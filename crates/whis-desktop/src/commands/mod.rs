//! Tauri Command Handlers
//!
//! This module organizes all Tauri command handlers by domain.
//! Each sub-module contains related commands that are exposed to the frontend.
//!
//! Phase 1: validation and system commands extracted to separate modules
//! TODO Phase 3: Extract remaining commands (models, presets, ollama, settings, etc.)

mod system;
mod validation;

// Re-export Phase 1 commands
pub use system::*;
pub use validation::*;

// TODO Phase 3: These will be extracted to separate modules
use crate::shortcuts::ShortcutBackendInfo;
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};
use whis_core::{RecordingState, Settings};
use whis_core::model::{ModelType, WhisperModel};
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "local-transcription")]
use whis_core::model::ParakeetModel;

// Global locks for preventing concurrent model downloads
// TODO Phase 3: Move to commands/models/downloads.rs
static WHISPER_DOWNLOAD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static PARAKEET_DOWNLOAD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn get_whisper_lock() -> &'static Mutex<()> {
    WHISPER_DOWNLOAD_LOCK.get_or_init(|| Mutex::new(()))
}

fn get_parakeet_lock() -> &'static Mutex<()> {
    PARAKEET_DOWNLOAD_LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub state: String,
    pub config_valid: bool,
}

#[derive(serde::Serialize)]
pub struct SaveSettingsResponse {
    pub needs_restart: bool,
}

// TODO Phase 3: Move to commands/recording.rs
#[tauri::command]
pub async fn is_api_configured(state: State<'_, AppState>) -> Result<bool, String> {
    let settings = state.settings.lock().unwrap();
    Ok(settings.transcription.has_api_key())
}

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

#[tauri::command]
pub async fn toggle_recording(app: AppHandle) -> Result<(), String> {
    crate::recording::toggle_recording(app);
    Ok(())
}

// TODO Phase 3: Move to commands/settings.rs
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().unwrap();
    // Return cached state - settings are saved via save_settings() command
    Ok(settings.clone())
}

// TODO Phase 3: Move to commands/shortcuts.rs
#[tauri::command]
pub fn shortcut_backend() -> ShortcutBackendInfo {
    crate::shortcuts::backend_info()
}

#[tauri::command]
pub async fn configure_shortcut(app: AppHandle) -> Result<Option<String>, String> {
    crate::shortcuts::open_configure_shortcuts(app)
        .await
        .map_err(|e| e.to_string())
}

/// Configure shortcut with a preferred trigger from in-app key capture
/// The trigger should be in human-readable format like "Ctrl+Alt+W" or "Cmd+Option+W"
#[tauri::command]
pub async fn configure_shortcut_with_trigger(
    app: AppHandle,
    trigger: String,
) -> Result<Option<String>, String> {
    crate::shortcuts::configure_with_preferred_trigger(Some(&trigger), app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn portal_shortcut(state: State<'_, AppState>) -> Result<Option<String>, String> {
    // First check if we have it cached in state
    let cached = state.portal_shortcut.lock().unwrap().clone();
    if cached.is_some() {
        return Ok(cached);
    }

    // Otherwise try reading from dconf (GNOME stores shortcuts there)
    Ok(crate::shortcuts::read_portal_shortcut_from_dconf())
}

// TODO Phase 3: Move to commands/settings.rs
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
                || current.transcription.local_models.whisper_path != settings.transcription.local_models.whisper_path
                || current.transcription.local_models.parakeet_path != settings.transcription.local_models.parakeet_path,
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
        crate::shortcuts::update_shortcut(&app, &settings.ui.shortcut).map_err(|e| e.to_string())?
    } else {
        false
    };

    Ok(SaveSettingsResponse { needs_restart })
}

// TODO Phase 3: Move to commands/shortcuts.rs
/// Reset portal shortcuts by clearing dconf (GNOME)
/// This allows rebinding after restart
#[cfg(target_os = "linux")]
#[tauri::command]
pub fn reset_shortcut() -> Result<(), String> {
    std::process::Command::new("dconf")
        .args([
            "reset",
            "-f",
            "/org/gnome/settings-daemon/global-shortcuts/",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub fn reset_shortcut() -> Result<(), String> {
    Ok(())
}

/// Get any error from portal shortcut binding
#[tauri::command]
pub fn portal_bind_error(state: State<'_, AppState>) -> Option<String> {
    state.portal_bind_error.lock().unwrap().clone()
}

// TODO Phase 3: Move to commands/models/ directory
/// Progress event payload for model download
#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Download a whisper model for local transcription
/// Emits 'download-progress' events with { downloaded, total } during download
/// Returns the path where the model was saved
#[tauri::command]
pub async fn download_whisper_model(
    app: AppHandle,
    model_name: String
) -> Result<String, String> {
    use tauri::Emitter;

    // Run blocking download in a separate thread
    tauri::async_runtime::spawn_blocking(move || {
        // Try to acquire lock (non-blocking) to prevent concurrent downloads
        // Lock must be acquired inside spawn_blocking to avoid Send issues
        let _guard = get_whisper_lock().try_lock()
            .map_err(|_| "Download already in progress".to_string())?;

        // Get state from app handle (works inside spawn_blocking)
        let state = app.state::<AppState>();

        // Set download state in backend (survives window close/reopen)
        *state.active_download.lock().unwrap() = Some(crate::state::DownloadState {
            model_name: model_name.clone(),
            model_type: "whisper".to_string(),
            downloaded: 0,
            total: 0,
        });

        let path = WhisperModel.default_path(&model_name);

        // Skip download if model already exists
        if path.exists() {
            // Clear download state
            *state.active_download.lock().unwrap() = None;
            return Ok(path.to_string_lossy().to_string());
        }

        // Download with progress callback
        let result = whis_core::model::download::download_with_progress(&WhisperModel, &model_name, &path, |downloaded, total| {
            // Update progress in backend state
            if let Some(ref mut dl) = *state.active_download.lock().unwrap() {
                dl.downloaded = downloaded;
                dl.total = total;
            }
            let _ = app.emit("download-progress", DownloadProgress { downloaded, total });
        });

        // Clear download state on completion (success or failure)
        *state.active_download.lock().unwrap() = None;

        result.map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Check if the configured whisper model path points to an existing file
#[tauri::command]
pub fn is_whisper_model_valid(state: State<'_, AppState>) -> bool {
    let settings = state.settings.lock().unwrap();
    settings
        .transcription.whisper_model_path()
        .map(|p| std::path::Path::new(&p).exists())
        .unwrap_or(false)
}

/// Get available whisper models for download
#[tauri::command]
pub fn get_whisper_models() -> Vec<WhisperModelInfo> {
    WhisperModel.models()
        .iter()
        .map(|model| {
            let path = WhisperModel.default_path(model.name);
            WhisperModelInfo {
                name: model.name.to_string(),
                description: model.description.to_string(),
                installed: path.exists(),
                path: path.to_string_lossy().to_string(),
            }
        })
        .collect()
}

#[derive(serde::Serialize)]
pub struct WhisperModelInfo {
    pub name: String,
    pub description: String,
    pub installed: bool,
    pub path: String,
}

/// Parakeet model info for frontend
#[derive(serde::Serialize)]
pub struct ParakeetModelInfo {
    pub name: String,
    pub description: String,
    pub size: String,
    pub installed: bool,
    pub path: String,
}

/// Get available Parakeet models for download
#[tauri::command]
pub fn get_parakeet_models() -> Vec<ParakeetModelInfo> {
    ParakeetModel.models()
        .iter()
        .map(|model| {
            let path = ParakeetModel.default_path(model.name);
            ParakeetModelInfo {
                name: model.name.to_string(),
                description: model.description.to_string(),
                size: format!("~{} MB", model.size_mb.unwrap_or(0)),
                installed: ParakeetModel.verify(&path),
                path: path.to_string_lossy().to_string(),
            }
        })
        .collect()
}

/// Check if configured Parakeet model is valid
#[tauri::command]
pub fn is_parakeet_model_valid(state: State<'_, AppState>) -> bool {
    state
        .settings
        .lock()
        .unwrap()
        .transcription.parakeet_model_path()
        .map(|p| ParakeetModel.verify(std::path::Path::new(&p)))
        .unwrap_or(false)
}

/// Get current active download state (if any)
/// Used to restore download progress after window close/reopen
#[derive(serde::Serialize)]
pub struct ActiveDownloadInfo {
    pub model_name: String,
    pub model_type: String,
    pub downloaded: u64,
    pub total: u64,
}

#[tauri::command]
pub fn get_active_download(state: State<'_, AppState>) -> Option<ActiveDownloadInfo> {
    state.active_download.lock().unwrap().as_ref().map(|dl| ActiveDownloadInfo {
        model_name: dl.model_name.clone(),
        model_type: dl.model_type.clone(),
        downloaded: dl.downloaded,
        total: dl.total,
    })
}

/// Download a Parakeet model with progress events
#[tauri::command]
pub async fn download_parakeet_model(
    app: AppHandle,
    model_name: String
) -> Result<String, String> {
    use tauri::Emitter;

    tauri::async_runtime::spawn_blocking(move || {
        // Try to acquire lock (non-blocking) to prevent concurrent downloads
        // Lock must be acquired inside spawn_blocking to avoid Send issues
        let _guard = get_parakeet_lock().try_lock()
            .map_err(|_| "Download already in progress".to_string())?;

        // Get state from app handle (works inside spawn_blocking)
        let state = app.state::<AppState>();

        // Set download state in backend (survives window close/reopen)
        *state.active_download.lock().unwrap() = Some(crate::state::DownloadState {
            model_name: model_name.clone(),
            model_type: "parakeet".to_string(),
            downloaded: 0,
            total: 0,
        });

        let dest = ParakeetModel.default_path(&model_name);

        // Skip if already exists
        if ParakeetModel.verify(&dest) {
            // Clear download state
            *state.active_download.lock().unwrap() = None;
            return Ok(dest.to_string_lossy().to_string());
        }

        // Download with progress
        let result = whis_core::model::download::download_with_progress(&ParakeetModel,
            &model_name,
            &dest,
            |downloaded, total| {
                // Update progress in backend state
                if let Some(ref mut dl) = *state.active_download.lock().unwrap() {
                    dl.downloaded = downloaded;
                    dl.total = total;
                }
                let _ = app.emit("download-progress", DownloadProgress { downloaded, total });
            },
        );

        // Clear download state on completion (success or failure)
        *state.active_download.lock().unwrap() = None;

        result.map_err(|e| e.to_string())?;
        Ok(dest.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// TODO Phase 3: Move to commands/presets.rs
/// Preset info for the UI
#[derive(serde::Serialize)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
}

/// List all available presets (built-in + user)
#[tauri::command]
pub fn list_presets() -> Vec<PresetInfo> {
    use whis_core::preset::{Preset, PresetSource};

    Preset::list_all()
        .into_iter()
        .map(|(p, source)| PresetInfo {
            name: p.name,
            description: p.description,
            is_builtin: source == PresetSource::BuiltIn,
        })
        .collect()
}

/// Apply a preset - updates settings with the preset's configuration and sets it as active
#[tauri::command]
pub async fn apply_preset(name: String, state: State<'_, AppState>) -> Result<(), String> {
    use whis_core::preset::Preset;

    let (preset, _) = Preset::load(&name)?;

    {
        let mut settings = state.settings.lock().unwrap();

        // Apply preset's post-processing prompt
        settings.post_processing.prompt = Some(preset.prompt.clone());

        // Apply preset's post-processor override if specified
        if let Some(post_processor_str) = &preset.post_processor
            && let Ok(post_processor) = post_processor_str.parse()
        {
            settings.post_processing.processor = post_processor;
        }

        // Set this preset as active
        settings.ui.active_preset = Some(name);

        // Save the settings
        settings.save().map_err(|e| e.to_string())?;
    }

    // Clear cached transcription config since settings changed
    *state.transcription_config.lock().unwrap() = None;

    Ok(())
}

/// Get the active preset name (if any)
#[tauri::command]
pub fn get_active_preset(state: State<'_, AppState>) -> Option<String> {
    let settings = state.settings.lock().unwrap();
    settings.ui.active_preset.clone()
}

/// Set the active preset
#[tauri::command]
pub async fn set_active_preset(
    name: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.ui.active_preset = name;
    settings.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Full preset details for editing
#[derive(serde::Serialize)]
pub struct PresetDetails {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub post_processor: Option<String>,
    pub model: Option<String>,
    pub is_builtin: bool,
}

/// Input for creating a new preset
#[derive(serde::Deserialize)]
pub struct CreatePresetInput {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub post_processor: Option<String>,
    pub model: Option<String>,
}

/// Input for updating an existing preset
#[derive(serde::Deserialize)]
pub struct UpdatePresetInput {
    pub description: String,
    pub prompt: String,
    pub post_processor: Option<String>,
    pub model: Option<String>,
}

/// Get full details of a preset for viewing/editing
#[tauri::command]
pub fn get_preset_details(name: String) -> Result<PresetDetails, String> {
    use whis_core::preset::{Preset, PresetSource};

    let (preset, source) = Preset::load(&name)?;

    Ok(PresetDetails {
        name: preset.name,
        description: preset.description,
        prompt: preset.prompt,
        post_processor: preset.post_processor,
        model: preset.model,
        is_builtin: source == PresetSource::BuiltIn,
    })
}

/// Create a new user preset
#[tauri::command]
pub fn create_preset(input: CreatePresetInput) -> Result<PresetInfo, String> {
    use whis_core::preset::Preset;

    // Validate name
    Preset::validate_name(&input.name, false)?;

    // Check if preset already exists
    if Preset::load(&input.name).is_ok() {
        return Err(format!("A preset named '{}' already exists", input.name));
    }

    // Create and save the preset
    let preset = Preset {
        name: input.name.clone(),
        description: input.description.clone(),
        prompt: input.prompt,
        post_processor: input.post_processor,
        model: input.model,
    };

    preset.save()?;

    Ok(PresetInfo {
        name: input.name,
        description: input.description,
        is_builtin: false,
    })
}

/// Update an existing user preset
#[tauri::command]
pub fn update_preset(name: String, input: UpdatePresetInput) -> Result<PresetInfo, String> {
    use whis_core::preset::Preset;

    // Check it's not a built-in
    if Preset::is_builtin(&name) {
        return Err(format!("Cannot edit built-in preset '{}'", name));
    }

    // Check preset exists
    let (mut preset, _) = Preset::load(&name)?;

    // Update fields
    preset.description = input.description.clone();
    preset.prompt = input.prompt;
    preset.post_processor = input.post_processor;
    preset.model = input.model;

    // Save
    preset.save()?;

    Ok(PresetInfo {
        name,
        description: input.description,
        is_builtin: false,
    })
}

/// Delete a user preset
#[tauri::command]
pub fn delete_preset(name: String, state: State<'_, AppState>) -> Result<(), String> {
    use whis_core::preset::Preset;

    // Delete the preset file
    Preset::delete(&name)?;

    // If this was the active preset, clear it
    {
        let mut settings = state.settings.lock().unwrap();
        if settings.ui.active_preset.as_deref() == Some(&name) {
            settings.ui.active_preset = None;
            settings.save().map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// TODO Phase 3: Move to commands/ollama.rs
/// Test connection to Ollama server
/// Must be async with spawn_blocking because reqwest::blocking::Client
/// creates an internal tokio runtime that would panic if called from Tauri's async context
#[tauri::command]
pub async fn test_ollama_connection(url: String) -> Result<bool, String> {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    tauri::async_runtime::spawn_blocking(move || whis_core::ollama::is_ollama_running(&url))
        .await
        .map_err(|e| e.to_string())?
}

/// List available models from Ollama
#[tauri::command]
pub async fn list_ollama_models(url: String) -> Result<Vec<String>, String> {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    // Run blocking call in separate thread
    tauri::async_runtime::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));
        let response = client
            .get(&tags_url)
            .send()
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Ollama returned error: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct TagsResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(serde::Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let tags: TagsResponse = response
            .json()
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Progress payload for Ollama pull events
#[derive(Clone, serde::Serialize)]
pub struct OllamaPullProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Pull an Ollama model with progress events
/// Emits 'ollama-pull-progress' events with { downloaded, total } during download
#[tauri::command]
pub async fn pull_ollama_model(
    app: tauri::AppHandle,
    url: String,
    model: String,
) -> Result<(), String> {
    use tauri::Emitter;

    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    // Run blocking calls in separate thread to avoid tokio runtime conflicts
    // (reqwest::blocking::Client creates its own runtime internally)
    tauri::async_runtime::spawn_blocking(move || {
        // Validate Ollama is running before attempting pull
        whis_core::ollama::ensure_ollama_running(&url).map_err(|e| e.to_string())?;

        whis_core::ollama::pull_model_with_progress(&url, &model, |downloaded, total| {
            let _ = app.emit(
                "ollama-pull-progress",
                OllamaPullProgress { downloaded, total },
            );
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Ollama status check result
#[derive(Clone, serde::Serialize)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub error: Option<String>,
}

/// Check Ollama installation and running status
/// Returns structured status without attempting to start Ollama
#[tauri::command]
pub async fn check_ollama_status(url: String) -> OllamaStatus {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    tauri::async_runtime::spawn_blocking(move || {
        let installed = whis_core::ollama::is_ollama_installed();

        if !installed {
            return OllamaStatus {
                installed: false,
                running: false,
                error: Some("Ollama is not installed".to_string()),
            };
        }

        match whis_core::ollama::is_ollama_running(&url) {
            Ok(true) => OllamaStatus {
                installed: true,
                running: true,
                error: None,
            },
            Ok(false) => OllamaStatus {
                installed: true,
                running: false,
                error: Some("Ollama is not running".to_string()),
            },
            Err(e) => OllamaStatus {
                installed: true,
                running: false,
                error: Some(e),
            },
        }
    })
    .await
    .unwrap_or(OllamaStatus {
        installed: false,
        running: false,
        error: Some("Failed to check status".to_string()),
    })
}

/// Start Ollama server if not running
/// Returns "started" if we started it, "running" if already running
/// Must be async with spawn_blocking because reqwest::blocking::Client
/// creates an internal tokio runtime that would panic if called from Tauri's async context
#[tauri::command]
pub async fn start_ollama(url: String) -> Result<String, String> {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    tauri::async_runtime::spawn_blocking(move || {
        match whis_core::ollama::ensure_ollama_running(&url) {
            Ok(true) => Ok("started".to_string()),  // Was started
            Ok(false) => Ok("running".to_string()), // Already running
            Err(e) => Err(e.to_string()),           // Not installed / error
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

// TODO Phase 3: Move to commands/settings.rs
/// Configuration readiness check result
#[derive(serde::Serialize)]
pub struct ConfigReadiness {
    pub transcription_ready: bool,
    pub transcription_error: Option<String>,
    pub post_processing_ready: bool,
    pub post_processing_error: Option<String>,
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
            Some(path)
                if ParakeetModel.verify(std::path::Path::new(path)) =>
            {
                (true, None)
            }
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
