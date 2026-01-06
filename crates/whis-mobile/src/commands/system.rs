//! System status and validation commands.

use crate::recording::provider::{api_key_store_key, validate_api_key_format};
use crate::state::{AppState, RecordingState};
use tauri::State;
use tauri_plugin_store::StoreExt;

/// Status response for the frontend.
#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub state: RecordingState,
    pub config_valid: bool,
}

/// Get current recording status and configuration state.
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
                .unwrap_or_else(|| whis_core::DEFAULT_PROVIDER.as_str().to_string());

            api_key_store_key(&provider)
                .and_then(|key| store.get(key))
                .and_then(|v| v.as_str().map(|s| !s.is_empty()))
        })
        .unwrap_or(false);

    StatusResponse {
        state: recording_state,
        config_valid,
    }
}

/// Validate API key format for a given provider.
#[tauri::command]
pub fn validate_api_key(key: String, provider: String) -> bool {
    validate_api_key_format(&key, &provider)
}
