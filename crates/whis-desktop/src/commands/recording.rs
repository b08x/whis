//! Recording Status Commands
//!
//! Provides Tauri commands for checking recording status and API configuration.

use crate::state::AppState;
use tauri::{AppHandle, State};
use whis_core::RecordingState;

/// Status response for the UI
#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub state: String,
    pub config_valid: bool,
}

/// Check if API/model is configured for the current transcription provider
#[tauri::command]
pub async fn is_api_configured(state: State<'_, AppState>) -> Result<bool, String> {
    let settings = state.settings.lock().unwrap();
    Ok(settings.transcription.has_api_key())
}

/// Get current recording status and configuration validity
#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<StatusResponse, String> {
    let current_state = *state.state.lock().unwrap();

    // Check if API key is configured for the current provider
    let config_valid = {
        let has_cached_config = state.transcription_config.lock().unwrap().is_some();
        let settings = state.settings.lock().unwrap();
        has_cached_config || settings.transcription.is_configured()
    };

    Ok(StatusResponse {
        state: match current_state {
            RecordingState::Idle => "Idle".to_string(),
            RecordingState::Recording => "Recording".to_string(),
            RecordingState::Transcribing => "Transcribing".to_string(),
        },
        config_valid,
    })
}

/// Toggle recording state (start if idle, stop if recording)
#[tauri::command]
pub async fn toggle_recording(app: AppHandle) -> Result<(), String> {
    crate::recording::toggle_recording(app);
    Ok(())
}
