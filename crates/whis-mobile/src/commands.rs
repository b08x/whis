use crate::state::{AppState, RecordingState};
use std::path::PathBuf;
use tauri::{Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_store::StoreExt;
use whis_core::config::TranscriptionProvider;
use whis_core::preset::Preset;
use whis_core::{OpenAIRealtimeProvider, PostProcessor, post_process};

/// Get the presets directory for this app using Tauri's path API.
/// This works correctly on Android where dirs::config_dir() returns None.
fn get_presets_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|p| p.join("presets"))
        .map_err(|e| format!("Failed to get app config dir: {}", e))
}

#[derive(serde::Serialize)]
pub struct StatusResponse {
    state: RecordingState,
    config_valid: bool,
}

#[tauri::command]
pub fn get_status(app: tauri::AppHandle, state: State<'_, AppState>) -> StatusResponse {
    let recording_state = *state.recording_state.lock().unwrap();

    // Check if API key is configured via store
    let config_valid = app
        .store("settings.json")
        .ok()
        .and_then(|store| {
            let provider = store
                .get("provider")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "openai".to_string());

            let key = match provider.as_str() {
                "openai" | "openai-realtime" => store.get("openai_api_key"),
                "mistral" => store.get("mistral_api_key"),
                _ => None,
            };

            key.and_then(|v| v.as_str().map(|s| !s.is_empty()))
        })
        .unwrap_or(false);

    StatusResponse {
        state: recording_state,
        config_valid,
    }
}

#[tauri::command]
pub fn validate_api_key(key: String, provider: String) -> bool {
    match provider.as_str() {
        "openai" | "openai-realtime" => key.starts_with("sk-") && key.len() > 20,
        "mistral" => key.len() > 20,
        _ => false,
    }
}

/// Check if post-processing is enabled (has post-processor and active preset)
fn is_post_processing_enabled(store: &tauri_plugin_store::Store<tauri::Wry>) -> bool {
    // Check post-processor setting
    let post_processor_str = store
        .get("post_processor")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "none".to_string());

    if post_processor_str == "none" {
        return false;
    }

    // Check active preset exists
    store
        .get("active_preset")
        .and_then(|v| v.as_str().map(String::from))
        .is_some()
}

/// Apply post-processing to transcription if enabled
///
/// Post-processing is applied when:
/// 1. An active preset is set, AND
/// 2. A post-processor is configured (not "none")
///
/// Returns the processed text, or original text on error/skip.
async fn apply_post_processing(
    app: &tauri::AppHandle,
    text: String,
    store: &tauri_plugin_store::Store<tauri::Wry>,
) -> String {
    // Get post-processor setting
    let post_processor_str = store
        .get("post_processor")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "none".to_string());

    let post_processor: PostProcessor = post_processor_str.parse().unwrap_or(PostProcessor::None);

    // Skip if disabled
    if post_processor == PostProcessor::None {
        return text;
    }

    // Get active preset - post-processing only works with a preset
    let active_preset = store
        .get("active_preset")
        .and_then(|v| v.as_str().map(String::from));

    let preset = match active_preset {
        Some(name) => {
            // Use Tauri's app config dir for presets (works on Android)
            let presets_dir = match get_presets_dir(app) {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("Failed to get presets dir: {}", e);
                    return text;
                }
            };
            match Preset::load_from(&name, &presets_dir) {
                Ok((preset, _)) => preset,
                Err(e) => {
                    eprintln!("Failed to load preset '{}': {}", name, e);
                    return text;
                }
            }
        }
        None => {
            // No preset active, skip post-processing
            return text;
        }
    };

    // Get API key for post-processor
    let api_key = match post_processor {
        PostProcessor::OpenAI => store.get("openai_api_key"),
        PostProcessor::Mistral => store.get("mistral_api_key"),
        _ => None,
    }
    .and_then(|v| v.as_str().map(String::from));

    let api_key = match api_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            eprintln!(
                "Post-processing: No API key configured for {}",
                post_processor
            );
            return text;
        }
    };

    // Apply post-processing with preset's prompt
    match post_process(&text, &post_processor, &api_key, &preset.prompt, None).await {
        Ok(processed) => processed,
        Err(e) => {
            eprintln!("Post-processing failed: {}", e);
            let _ = app.emit("post-process-warning", e.to_string());
            text // Return original on error
        }
    }
}

/// Transcribe audio data received from the WebView
///
/// The frontend records audio using MediaRecorder (webm/opus format)
/// and sends the raw bytes here for transcription.
#[tauri::command]
pub async fn transcribe_audio(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    audio_data: Vec<u8>,
    mime_type: String,
) -> Result<String, String> {
    // Set state to transcribing
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Transcribing;
    }

    // Get transcription config from store
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    let provider_str = store
        .get("provider")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "openai".to_string());

    let provider: TranscriptionProvider = provider_str
        .parse()
        .unwrap_or(TranscriptionProvider::OpenAI);

    let api_key = match provider_str.as_str() {
        "openai" | "openai-realtime" => store.get("openai_api_key"),
        "mistral" => store.get("mistral_api_key"),
        _ => None,
    }
    .and_then(|v| v.as_str().map(String::from))
    .ok_or("No API key configured")?;

    let language: Option<String> = store
        .get("language")
        .and_then(|v| v.as_str().map(String::from));

    // Determine filename extension based on mime type
    let filename = if mime_type.contains("webm") {
        "audio.webm"
    } else if mime_type.contains("ogg") {
        "audio.ogg"
    } else if mime_type.contains("mp4") || mime_type.contains("m4a") {
        "audio.m4a"
    } else {
        "audio.mp3"
    };

    // Transcribe
    let text = tokio::task::spawn_blocking(move || {
        whis_core::transcribe_audio_with_format(
            &provider,
            &api_key,
            language.as_deref(),
            audio_data,
            Some(&mime_type),
            Some(filename),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Apply post-processing if enabled (requires active preset + post-processor)
    if is_post_processing_enabled(&store) {
        let _ = app.emit("post-processing-started", ());
    }
    let final_text = apply_post_processing(&app, text, &store).await;

    // Copy to clipboard using Tauri plugin
    app.clipboard()
        .write_text(&final_text)
        .map_err(|e| e.to_string())?;

    // Reset state
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Idle;
    }

    Ok(final_text)
}

// ========== Preset Commands ==========

/// Preset info for the UI
#[derive(serde::Serialize)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub is_active: bool,
}

/// Full preset details for viewing
#[derive(serde::Serialize)]
pub struct PresetDetails {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub is_builtin: bool,
}

/// List all available presets (built-in + user)
#[tauri::command]
pub fn list_presets(app: tauri::AppHandle) -> Vec<PresetInfo> {
    use whis_core::preset::{Preset, PresetSource};

    // Get active preset from store
    let active_preset = app.store("settings.json").ok().and_then(|store| {
        store
            .get("active_preset")
            .and_then(|v| v.as_str().map(String::from))
    });

    // Use Tauri's app config dir for presets (works on Android)
    let presets = match get_presets_dir(&app) {
        Ok(dir) => Preset::list_all_from(&dir),
        Err(_) => {
            // Fall back to built-ins only if we can't get the dir
            Preset::builtins()
                .into_iter()
                .map(|p| (p, PresetSource::BuiltIn))
                .collect()
        }
    };

    presets
        .into_iter()
        .map(|(p, source)| PresetInfo {
            is_active: active_preset.as_ref().is_some_and(|a| a == &p.name),
            name: p.name,
            description: p.description,
            is_builtin: source == PresetSource::BuiltIn,
        })
        .collect()
}

/// Get full details of a preset
#[tauri::command]
pub fn get_preset_details(app: tauri::AppHandle, name: String) -> Result<PresetDetails, String> {
    use whis_core::preset::{Preset, PresetSource};

    let presets_dir = get_presets_dir(&app)?;
    let (preset, source) = Preset::load_from(&name, &presets_dir)?;

    Ok(PresetDetails {
        name: preset.name,
        description: preset.description,
        prompt: preset.prompt,
        is_builtin: source == PresetSource::BuiltIn,
    })
}

/// Set the active preset
#[tauri::command]
pub fn set_active_preset(app: tauri::AppHandle, name: Option<String>) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    if let Some(preset_name) = name {
        store.set("active_preset", serde_json::json!(preset_name));
    } else {
        store.delete("active_preset");
    }

    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Get the active preset name
#[tauri::command]
pub fn get_active_preset(app: tauri::AppHandle) -> Option<String> {
    app.store("settings.json").ok().and_then(|store| {
        store
            .get("active_preset")
            .and_then(|v| v.as_str().map(String::from))
    })
}

// ========== Preset CRUD Commands ==========

/// Input for creating a new preset
#[derive(serde::Deserialize)]
pub struct CreatePresetInput {
    pub name: String,
    pub description: String,
    pub prompt: String,
}

/// Input for updating an existing preset
#[derive(serde::Deserialize)]
pub struct UpdatePresetInput {
    pub description: String,
    pub prompt: String,
}

/// Create a new user preset
#[tauri::command]
pub fn create_preset(
    app: tauri::AppHandle,
    input: CreatePresetInput,
) -> Result<PresetInfo, String> {
    use whis_core::preset::Preset;

    let presets_dir = get_presets_dir(&app)?;

    // Validate name
    Preset::validate_name(&input.name, false)?;

    // Check if preset already exists (check user preset in custom dir)
    if Preset::load_user_from(&input.name, &presets_dir).is_some() {
        return Err(format!("Preset '{}' already exists", input.name));
    }

    // Create and save the preset
    let preset = Preset {
        name: input.name.clone(),
        description: input.description.clone(),
        prompt: input.prompt,
        post_processor: None,
        model: None,
    };

    preset.save_to(&presets_dir)?;

    Ok(PresetInfo {
        name: input.name,
        description: input.description,
        is_builtin: false,
        is_active: false,
    })
}

/// Update an existing user preset
#[tauri::command]
pub fn update_preset(
    app: tauri::AppHandle,
    name: String,
    input: UpdatePresetInput,
) -> Result<(), String> {
    use whis_core::preset::Preset;

    let presets_dir = get_presets_dir(&app)?;

    // Check it's not a built-in
    if Preset::is_builtin(&name) {
        return Err(format!("Cannot edit built-in preset '{}'", name));
    }

    // Check preset exists
    let (mut preset, _) = Preset::load_from(&name, &presets_dir)?;

    // Update fields
    preset.description = input.description;
    preset.prompt = input.prompt;

    // Save
    preset.save_to(&presets_dir)?;

    Ok(())
}

/// Delete a user preset
#[tauri::command]
pub fn delete_preset(app: tauri::AppHandle, name: String) -> Result<(), String> {
    use whis_core::preset::Preset;

    let presets_dir = get_presets_dir(&app)?;

    // Delete the preset file
    Preset::delete_from(&name, &presets_dir)?;

    // If this was the active preset, clear it
    if let Ok(store) = app.store("settings.json")
        && let Some(active) = store
            .get("active_preset")
            .and_then(|v| v.as_str().map(String::from))
        && active == name
    {
        store.delete("active_preset");
        let _ = store.save();
    }

    Ok(())
}

// ========== Streaming Transcription Commands ==========

/// Start streaming transcription with OpenAI Realtime API
///
/// Creates a WebSocket connection and audio channel for real-time streaming.
/// Frontend sends audio chunks via transcribe_streaming_send_chunk.
#[tauri::command]
pub async fn transcribe_streaming_start(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    // Normalize provider for API key lookup (openai-realtime uses openai key)
    let provider_str = store
        .get("provider")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "openai".to_string());

    let api_key = if provider_str == "openai-realtime" || provider_str == "openai" {
        store.get("openai_api_key")
    } else {
        None
    }
    .and_then(|v| v.as_str().map(String::from))
    .ok_or("No OpenAI API key configured")?;

    // Set state to transcribing
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Transcribing;
    }

    // Create channel for audio samples
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

    // Store sender in state so frontend can send chunks
    {
        let mut state_tx = state.audio_tx.lock().unwrap();
        *state_tx = Some(audio_tx);
    }

    // Get language setting
    let language: Option<String> = store
        .get("language")
        .and_then(|v| v.as_str().map(String::from));

    // Spawn transcription task
    let recording_state_arc = state.recording_state.clone();
    let audio_tx_arc = state.audio_tx.clone();
    tokio::spawn(async move {
        match OpenAIRealtimeProvider::transcribe_stream(&api_key, audio_rx, language).await {
            Ok(transcript) => {
                // Apply post-processing if enabled
                let final_text = if let Ok(store) = app.store("settings.json") {
                    if is_post_processing_enabled(&store) {
                        let _ = app.emit("post-processing-started", ());
                    }
                    apply_post_processing(&app, transcript, &store).await
                } else {
                    transcript
                };

                // Copy to clipboard
                if let Err(e) = app.clipboard().write_text(&final_text) {
                    let _ = app.emit("transcription-error", format!("Clipboard error: {}", e));
                    return;
                }

                // Emit event with result
                let _ = app.emit("transcription-complete", final_text);
            }
            Err(e) => {
                let _ = app.emit("transcription-error", e.to_string());
            }
        }

        // Reset state
        {
            let mut recording_state = recording_state_arc.lock().unwrap();
            *recording_state = RecordingState::Idle;
        }

        // Clear audio_tx
        {
            let mut state_tx = audio_tx_arc.lock().unwrap();
            *state_tx = None;
        }
    });

    Ok(())
}

/// Send audio chunk to ongoing streaming transcription
///
/// Frontend calls this continuously with audio samples from Web Audio API.
#[tauri::command]
pub async fn transcribe_streaming_send_chunk(
    state: State<'_, AppState>,
    chunk: Vec<f32>,
) -> Result<(), String> {
    let audio_tx = state.audio_tx.lock().unwrap();

    if let Some(tx) = audio_tx.as_ref() {
        // Send chunk with error handling
        // Use try_send to avoid blocking if channel is full
        if tx.try_send(chunk).is_err() {
            return Err("Audio channel closed or full".to_string());
        }
    } else {
        return Err("No active streaming transcription".to_string());
    }

    Ok(())
}

/// Stop streaming transcription
///
/// Drops the audio_tx to signal end of stream, causing WebSocket to commit
/// and request final transcription from OpenAI.
#[tauri::command]
pub async fn transcribe_streaming_stop(state: State<'_, AppState>) -> Result<(), String> {
    // Drop audio_tx to signal end of stream
    {
        let mut audio_tx = state.audio_tx.lock().unwrap();
        *audio_tx = None;
    }

    Ok(())
}
