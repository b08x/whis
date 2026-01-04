//! Recording Control
//!
//! Handles starting and stopping audio recording with state management.

use crate::state::{AppState, RecordingState};
use super::config::load_transcription_config;
use tauri::AppHandle;
use whis_core::{AudioRecorder, PostProcessor, ollama, preload_ollama};

/// Start recording with configuration loading and validation
pub fn start_recording_sync(_app: &AppHandle, state: &AppState) -> Result<(), String> {
    // Load transcription config if not already loaded
    {
        let mut config_guard = state.transcription_config.lock().unwrap();
        if config_guard.is_none() {
            *config_guard = Some(load_transcription_config(state)?);
        }
    }

    // Start recording with selected microphone device
    let mut recorder = AudioRecorder::new().map_err(|e| e.to_string())?;
    let device_name = state.settings.lock().unwrap().ui.microphone_device.clone();
    recorder
        .start_recording_with_device(device_name.as_deref())
        .map_err(|e| e.to_string())?;

    *state.recorder.lock().unwrap() = Some(recorder);
    *state.state.lock().unwrap() = RecordingState::Recording;

    // Preload Ollama model in background if using Ollama post-processing
    // This overlaps model loading with recording to reduce latency
    {
        let settings = state.settings.lock().unwrap();
        if settings.post_processing.processor == PostProcessor::Ollama {
            let ollama_url = settings
                .services
                .ollama
                .url()
                .unwrap_or_else(|| ollama::DEFAULT_OLLAMA_URL.to_string());
            let ollama_model = settings
                .services
                .ollama
                .model()
                .unwrap_or_else(|| ollama::DEFAULT_OLLAMA_MODEL.to_string());

            preload_ollama(&ollama_url, &ollama_model);
        }
    }

    println!("Recording started...");
    Ok(())
}
