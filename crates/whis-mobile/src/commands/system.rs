//! System status and validation commands.

use crate::recording::provider::api_key_store_key;
use crate::state::{AppState, RecordingState};
use tauri::State;
use tauri_plugin_store::StoreExt;
use whis_core::{WarmupConfig, warmup_configured};

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
    crate::recording::provider::validate_api_key_format(&key, &provider)
}

/// Warm up HTTP client and cloud connections based on current configuration.
///
/// This should be called after the app is mounted to reduce latency
/// on the first transcription request. The warmup is best-effort and
/// will not block the UI.
#[tauri::command]
pub async fn warmup_connections(app: tauri::AppHandle) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    // Read current provider configuration
    let provider = store
        .get("provider")
        .and_then(|v| v.as_str().map(String::from));

    let provider_api_key = provider.as_ref().and_then(|p| {
        api_key_store_key(p)
            .and_then(|key| store.get(key))
            .and_then(|v| v.as_str().map(String::from))
    });

    // Read post-processor configuration
    let post_processor = store
        .get("post_processor")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|p| p != "none");

    let post_processor_api_key = post_processor.as_ref().and_then(|p| {
        // Post-processor uses same key lookup pattern
        api_key_store_key(p)
            .and_then(|key| store.get(key))
            .and_then(|v| v.as_str().map(String::from))
    });

    // Build warmup config
    let config = WarmupConfig {
        provider,
        provider_api_key,
        post_processor,
        post_processor_api_key,
    };

    // Run warmup (best-effort, errors are logged but not propagated)
    warmup_configured(&config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
