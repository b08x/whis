//! Transcription configuration loading from Tauri store.
//!
//! This module handles loading and caching transcription configuration,
//! mirroring the pattern used in whis-desktop's recording/config.rs.

use super::provider::api_key_store_key;
use crate::state::{AppState, TranscriptionConfig};
use tauri_plugin_store::StoreExt;
use whis_core::config::TranscriptionProvider;

/// Load transcription configuration from Tauri store.
///
/// Checks the cached config first. If not cached or if provider/key changed,
/// loads fresh from the store.
///
/// Returns the provider, API key, and optional language setting.
pub fn load_transcription_config(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<TranscriptionConfig, String> {
    // Check if we have a cached config
    {
        let config_guard = state.transcription_config.lock().unwrap();
        if let Some(ref config) = *config_guard {
            return Ok(config.clone());
        }
    }

    // Load from Tauri store
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    let provider_str = store
        .get("provider")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| whis_core::DEFAULT_PROVIDER.as_str().to_string());

    let provider: TranscriptionProvider =
        provider_str.parse().unwrap_or(whis_core::DEFAULT_PROVIDER);

    // Get API key based on provider
    let api_key = api_key_store_key(&provider_str)
        .and_then(|key| store.get(key))
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| format!("No API key configured for provider: {}", provider_str))?;

    let language: Option<String> = store
        .get("language")
        .and_then(|v| v.as_str().map(String::from));

    let config = TranscriptionConfig {
        provider,
        api_key,
        language,
    };

    // Cache the config
    {
        let mut config_guard = state.transcription_config.lock().unwrap();
        *config_guard = Some(config.clone());
    }

    Ok(config)
}

/// Clear the cached transcription config.
///
/// Call this when settings change (provider, API key, language).
pub fn clear_transcription_config(state: &AppState) {
    let mut config_guard = state.transcription_config.lock().unwrap();
    *config_guard = None;
}
