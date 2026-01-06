//! Recording and transcription commands.
//!
//! Handles audio transcription via batch and streaming modes.
//! Business logic is delegated to the `recording` module.

use crate::recording::pipeline::{apply_post_processing, is_post_processing_enabled};
use crate::state::{AppState, RecordingState};
use tauri::{Emitter, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_store::StoreExt;
use whis_core::OpenAIRealtimeProvider;
use whis_core::config::TranscriptionProvider;

// ========== Batch Transcription ==========

/// Transcribe audio data received from the WebView (batch mode).
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

    // Run transcription logic, capturing result
    let result = transcribe_audio_inner(&app, audio_data, mime_type).await;

    // Always reset state, regardless of success or failure
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Idle;
    }

    result
}

/// Inner transcription logic (separated for clean state management).
async fn transcribe_audio_inner(
    app: &tauri::AppHandle,
    audio_data: Vec<u8>,
    mime_type: String,
) -> Result<String, String> {
    // Get transcription config from store
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    let provider_str = store
        .get("provider")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| whis_core::DEFAULT_PROVIDER.as_str().to_string());

    let provider: TranscriptionProvider =
        provider_str.parse().unwrap_or(whis_core::DEFAULT_PROVIDER);

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

    // Transcribe using async API (blocking reqwest doesn't work on Android)
    let text = whis_core::transcribe_audio_async(
        &provider,
        &api_key,
        language.as_deref(),
        audio_data,
        Some(&mime_type),
        Some(filename),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Apply post-processing if enabled (requires active preset + post-processor)
    if is_post_processing_enabled(&store) {
        let _ = app.emit("post-processing-started", ());
    }
    let final_text = apply_post_processing(app, text, &store).await;

    // Copy to clipboard using Tauri plugin
    app.clipboard()
        .write_text(&final_text)
        .map_err(|e| e.to_string())?;

    Ok(final_text)
}

// ========== OpenAI Realtime Streaming ==========

/// Start streaming transcription with OpenAI Realtime API.
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

    // Create bounded channel for realtime streaming (separate from progressive)
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

    // Store sender in realtime_audio_tx (not audio_tx, which is for progressive)
    {
        let mut state_tx = state.realtime_audio_tx.lock().unwrap();
        *state_tx = Some(audio_tx);
    }

    // Get language setting
    let language: Option<String> = store
        .get("language")
        .and_then(|v| v.as_str().map(String::from));

    // Spawn transcription task
    let recording_state_arc = state.recording_state.clone();
    let realtime_tx_arc = state.realtime_audio_tx.clone();
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

        // Clear realtime_audio_tx
        {
            let mut state_tx = realtime_tx_arc.lock().unwrap();
            *state_tx = None;
        }
    });

    Ok(())
}

/// Send audio chunk to ongoing streaming transcription.
///
/// Frontend calls this continuously with audio samples from Web Audio API.
#[tauri::command]
pub async fn transcribe_streaming_send_chunk(
    state: State<'_, AppState>,
    chunk: Vec<f32>,
) -> Result<(), String> {
    let audio_tx = state.realtime_audio_tx.lock().unwrap();

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

/// Stop streaming transcription.
///
/// Drops the realtime_audio_tx to signal end of stream, causing WebSocket to commit
/// and request final transcription from OpenAI.
#[tauri::command]
pub async fn transcribe_streaming_stop(state: State<'_, AppState>) -> Result<(), String> {
    // Drop realtime_audio_tx to signal end of stream
    {
        let mut audio_tx = state.realtime_audio_tx.lock().unwrap();
        *audio_tx = None;
    }

    Ok(())
}

// ========== Progressive Transcription ==========
//
// Progressive transcription matches the CLI/desktop architecture:
// - Audio samples are chunked every ~90 seconds
// - Chunks are transcribed in parallel (cloud providers)
// - Results are combined when recording stops

use crate::recording::config::load_transcription_config;
use tokio::sync::{mpsc, oneshot};
use whis_core::{ChunkerConfig, ProgressiveChunker, progressive_transcribe_cloud};

/// Default chunk duration in seconds for progressive transcription.
const DEFAULT_CHUNK_DURATION_SECS: u64 = 90;

/// Start recording with progressive transcription.
///
/// Initializes the chunker and transcription pipeline. Frontend should call
/// `send_audio_chunk()` repeatedly with audio samples, then `stop_recording()`
/// to get the final result.
///
/// This matches the CLI/desktop architecture where audio is chunked and
/// transcribed progressively during recording.
#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Load transcription config
    let config = load_transcription_config(&app, &state)?;
    let provider = config.provider.clone();
    let api_key = config.api_key.clone();
    let language = config.language.clone();

    // Set state to recording
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Recording;
    }

    // Create unbounded channel for audio samples from frontend
    let (audio_tx, audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();

    // Store sender in state so frontend can send chunks
    {
        let mut state_tx = state.audio_tx.lock().unwrap();
        *state_tx = Some(audio_tx);
    }

    // Create channel for chunker output
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    // Create chunker config (no VAD on mobile)
    let chunker_config = ChunkerConfig {
        target_duration_secs: DEFAULT_CHUNK_DURATION_SECS,
        min_duration_secs: DEFAULT_CHUNK_DURATION_SECS * 2 / 3,
        max_duration_secs: DEFAULT_CHUNK_DURATION_SECS * 4 / 3,
        vad_aware: false, // No VAD on mobile
    };

    // Spawn chunker task
    let mut chunker = ProgressiveChunker::new(chunker_config, chunk_tx);
    tokio::spawn(async move {
        let _ = chunker.consume_stream(audio_rx, None).await;
    });

    // Create oneshot channel for transcription result
    let (result_tx, result_rx) = oneshot::channel();

    // Spawn transcription task
    tokio::spawn(async move {
        let result =
            progressive_transcribe_cloud(&provider, &api_key, language.as_deref(), chunk_rx, None)
                .await
                .map_err(|e| e.to_string());

        let _ = result_tx.send(result);
    });

    // Store result receiver for later retrieval
    {
        let mut rx_guard = state.transcription_rx.lock().unwrap();
        *rx_guard = Some(result_rx);
    }

    Ok(())
}

/// Send audio samples to the progressive transcription pipeline.
///
/// Frontend should call this repeatedly with audio samples from Web Audio API.
/// Samples should be f32 PCM at 16kHz sample rate.
#[tauri::command]
pub async fn send_audio_chunk(state: State<'_, AppState>, samples: Vec<f32>) -> Result<(), String> {
    let audio_tx = state.audio_tx.lock().unwrap();

    if let Some(tx) = audio_tx.as_ref() {
        // Use unbounded send (won't block)
        if tx.send(samples).is_err() {
            return Err("Audio channel closed".to_string());
        }
    } else {
        return Err("No active recording".to_string());
    }

    Ok(())
}

/// Stop recording and get the transcription result.
///
/// Drops the audio channel to signal end of stream, waits for transcription
/// to complete, applies post-processing, and copies to clipboard.
#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Set state to transcribing
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Transcribing;
    }

    // Drop audio_tx to signal end of stream
    {
        let mut audio_tx = state.audio_tx.lock().unwrap();
        *audio_tx = None;
    }

    // Get the result receiver
    let result_rx = {
        let mut rx_guard = state.transcription_rx.lock().unwrap();
        rx_guard.take()
    };

    let result_rx = result_rx.ok_or("No transcription in progress")?;

    // Wait for transcription result
    let transcription = result_rx
        .await
        .map_err(|_| "Transcription task was cancelled")?
        .map_err(|e| format!("Transcription failed: {}", e))?;

    // Apply post-processing if enabled
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    if is_post_processing_enabled(&store) {
        let _ = app.emit("post-processing-started", ());
    }
    let final_text = apply_post_processing(&app, transcription, &store).await;

    // Copy to clipboard
    app.clipboard()
        .write_text(&final_text)
        .map_err(|e| format!("Clipboard error: {}", e))?;

    // Emit completion event
    let _ = app.emit("transcription-complete", final_text.clone());

    // Reset state
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Idle;
    }

    // Clear config cache (in case settings changed)
    {
        let mut config_guard = state.transcription_config.lock().unwrap();
        *config_guard = None;
    }

    Ok(final_text)
}
