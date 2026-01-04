//! Settings Management Commands
//!
//! Provides Tauri commands for getting, saving, and validating application settings.

use crate::state::AppState;
use tauri::{AppHandle, State};
use whis_core::{
    model::{ModelType, ParakeetModel},
    Settings,
};

/// Save settings response
#[derive(serde::Serialize)]
pub struct SaveSettingsResponse {
    pub needs_restart: bool,
}

/// Configuration readiness check result
#[derive(serde::Serialize)]
pub struct ConfigReadiness {
    pub transcription_ready: bool,
    pub transcription_error: Option<String>,
    pub post_processing_ready: bool,
    pub post_processing_error: Option<String>,
}

/// Get current settings from state
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().unwrap();
    // Return cached state - settings are saved via save_settings() command
    Ok(settings.clone())
}

/// Save settings and handle side effects (clear cache, update shortcuts)
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SaveSettingsResponse, String> {
    // Check what changed
    let (config_changed, shortcut_changed) = {
        let current = state.settings.lock().unwrap();
        (
            current.transcription.provider != settings.transcription.provider
                || current.transcription.api_keys != settings.transcription.api_keys
                || current.transcription.language != settings.transcription.language
                || current.transcription.local_models.whisper_path
                    != settings.transcription.local_models.whisper_path
                || current.transcription.local_models.parakeet_path
                    != settings.transcription.local_models.parakeet_path,
            current.ui.shortcut != settings.ui.shortcut,
        )
    };

    {
        let mut state_settings = state.settings.lock().unwrap();
        *state_settings = settings.clone();
        state_settings.save().map_err(|e| e.to_string())?;
    }

    // Clear cached transcription config if provider or API key changed
    if config_changed {
        *state.transcription_config.lock().unwrap() = None;
    }

    // Only update shortcut if it actually changed
    let needs_restart = if shortcut_changed {
        crate::shortcuts::update_shortcut(&app, &settings.ui.shortcut)
            .map_err(|e| e.to_string())?
    } else {
        false
    };

    Ok(SaveSettingsResponse { needs_restart })
}

/// Check if transcription and post-processing are properly configured
/// Called on app load and settings changes to show proactive warnings
#[tauri::command]
pub async fn check_config_readiness(
    provider: String,
    post_processor: String,
    api_keys: std::collections::HashMap<String, String>,
    whisper_model_path: Option<String>,
    parakeet_model_path: Option<String>,
    ollama_url: Option<String>,
) -> ConfigReadiness {
    // Check transcription readiness
    let (transcription_ready, transcription_error) = match provider.as_str() {
        "local-whisper" => match &whisper_model_path {
            Some(path) if std::path::Path::new(path).exists() => (true, None),
            Some(_) => (false, Some("Whisper model file not found".to_string())),
            None => (false, Some("Whisper model path not configured".to_string())),
        },
        "local-parakeet" => match &parakeet_model_path {
            Some(path) if ParakeetModel.verify(std::path::Path::new(path)) => (true, None),
            Some(_) => (false, Some("Parakeet model not found or invalid".to_string())),
            None => (false, Some("Parakeet model not configured".to_string())),
        },
        provider => {
            // Normalize provider for API key lookup (openai-realtime uses openai key)
            let key_provider = if provider == "openai-realtime" {
                "openai"
            } else {
                provider
            };

            if api_keys.get(key_provider).is_none_or(|k| k.is_empty()) {
                (
                    false,
                    Some(format!("{} API key not configured", capitalize(provider))),
                )
            } else {
                (true, None)
            }
        }
    };

    // Check post-processing readiness
    let (post_processing_ready, post_processing_error) = match post_processor.as_str() {
        "none" => (true, None),
        "ollama" => {
            let url = ollama_url.unwrap_or_else(|| "http://localhost:11434".to_string());
            let result = tauri::async_runtime::spawn_blocking(move || {
                whis_core::ollama::is_ollama_running(&url)
            })
            .await
            .ok()
            .and_then(|r| r.ok());

            match result {
                Some(true) => (true, None),
                _ => (false, Some("Ollama not running".to_string())),
            }
        }
        post_processor => {
            if api_keys.get(post_processor).is_none_or(|k| k.is_empty()) {
                (
                    false,
                    Some(format!(
                        "{} API key not configured",
                        capitalize(post_processor)
                    )),
                )
            } else {
                (true, None)
            }
        }
    };

    ConfigReadiness {
        transcription_ready,
        transcription_error,
        post_processing_ready,
        post_processing_error,
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
