use crate::state::{AppState, RecordingState};
use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_store::StoreExt;
use whis_core::config::TranscriptionProvider;
use whis_core::{AudioRecorder, RecordingOutput};

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
                "openai" => store.get("openai_api_key"),
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
        "openai" => key.starts_with("sk-") && key.len() > 20,
        "mistral" => key.len() > 20,
        _ => false,
    }
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut recording_state = state.recording_state.lock().unwrap();
    if *recording_state != RecordingState::Idle {
        return Err("Already recording or transcribing".to_string());
    }

    let mut recorder = AudioRecorder::new().map_err(|e| e.to_string())?;
    recorder.start_recording().map_err(|e| e.to_string())?;

    *state.recorder.lock().unwrap() = Some(recorder);
    *recording_state = RecordingState::Recording;

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Stop recording and get data
    let recording_data = {
        let mut recording_state = state.recording_state.lock().unwrap();
        if *recording_state != RecordingState::Recording {
            return Err("Not currently recording".to_string());
        }

        let mut recorder_guard = state.recorder.lock().unwrap();
        let recorder = recorder_guard.as_mut().ok_or("No recorder available")?;

        let data = recorder.stop_recording().map_err(|e| e.to_string())?;
        *recorder_guard = None;
        *recording_state = RecordingState::Transcribing;
        data
    };

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
        "openai" => store.get("openai_api_key"),
        "mistral" => store.get("mistral_api_key"),
        _ => None,
    }
    .and_then(|v| v.as_str().map(String::from))
    .ok_or("No API key configured")?;

    let language: Option<String> = store
        .get("language")
        .and_then(|v| v.as_str().map(String::from));

    // Finalize recording (convert to MP3)
    let output = tokio::task::spawn_blocking(move || recording_data.finalize())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // Transcribe (wrap blocking calls in spawn_blocking to avoid tokio panic)
    let text = match output {
        RecordingOutput::Single(data) => tokio::task::spawn_blocking(move || {
            whis_core::transcribe_audio(&provider, &api_key, language.as_deref(), data)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?,
        RecordingOutput::Chunked(chunks) => {
            whis_core::parallel_transcribe(&provider, &api_key, language.as_deref(), chunks, None)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    // Copy to clipboard using Tauri plugin
    app.clipboard()
        .write_text(&text)
        .map_err(|e| e.to_string())?;

    // Reset state
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Idle;
    }

    Ok(text)
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

    Preset::list_all()
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
pub fn get_preset_details(name: String) -> Result<PresetDetails, String> {
    use whis_core::preset::{Preset, PresetSource};

    let (preset, source) = Preset::load(&name)?;

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
