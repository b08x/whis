//! Parakeet Model Management Commands
//!
//! Provides Tauri commands for downloading, validating, and listing Parakeet models.
//! Only available when `local-transcription` feature is enabled.

use super::downloads::get_parakeet_lock;
use super::whisper::DownloadProgress;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, State};
use whis_core::model::{ModelType, ParakeetModel};

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
    ParakeetModel
        .models()
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
        .transcription
        .parakeet_model_path()
        .map(|p| ParakeetModel.verify(std::path::Path::new(&p)))
        .unwrap_or(false)
}

/// Download a Parakeet model with progress events
#[tauri::command]
pub async fn download_parakeet_model(
    app: AppHandle,
    model_name: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // Try to acquire lock (non-blocking) to prevent concurrent downloads
        // Lock must be acquired inside spawn_blocking to avoid Send issues
        let _guard = get_parakeet_lock()
            .try_lock()
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
        let result = whis_core::model::download::download_with_progress(
            &ParakeetModel,
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
    state
        .active_download
        .lock()
        .unwrap()
        .as_ref()
        .map(|dl| ActiveDownloadInfo {
            model_name: dl.model_name.clone(),
            model_type: dl.model_type.clone(),
            downloaded: dl.downloaded,
            total: dl.total,
        })
}
