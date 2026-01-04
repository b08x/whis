//! Whisper Model Management Commands
//!
//! Provides Tauri commands for downloading, validating, and listing Whisper models.

use super::downloads::get_whisper_lock;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, State};
use whis_core::model::{ModelType, WhisperModel};

/// Progress event payload for model download
#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Whisper model info for frontend
#[derive(serde::Serialize)]
pub struct WhisperModelInfo {
    pub name: String,
    pub description: String,
    pub installed: bool,
    pub path: String,
}

/// Download a whisper model for local transcription
/// Emits 'download-progress' events with { downloaded, total } during download
/// Returns the path where the model was saved
#[tauri::command]
pub async fn download_whisper_model(
    app: AppHandle,
    model_name: String,
) -> Result<String, String> {
    // Run blocking download in a separate thread
    tauri::async_runtime::spawn_blocking(move || {
        // Try to acquire lock (non-blocking) to prevent concurrent downloads
        // Lock must be acquired inside spawn_blocking to avoid Send issues
        let _guard = get_whisper_lock()
            .try_lock()
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
        let result = whis_core::model::download::download_with_progress(
            &WhisperModel,
            &model_name,
            &path,
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
        .transcription
        .whisper_model_path()
        .map(|p| std::path::Path::new(&p).exists())
        .unwrap_or(false)
}

/// Get available whisper models for download
#[tauri::command]
pub fn get_whisper_models() -> Vec<WhisperModelInfo> {
    WhisperModel
        .models()
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
