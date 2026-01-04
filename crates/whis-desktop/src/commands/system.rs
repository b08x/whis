//! System Utility Commands
//!
//! Provides Tauri commands for system-level operations like audio device listing,
//! CLI toggle command retrieval, window reopening checks, and app exit.

use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_toggle_command() -> String {
    if std::path::Path::new("/.flatpak-info").exists() {
        "flatpak run ink.whis.Whis --toggle".to_string()
    } else {
        "whis-desktop --toggle".to_string()
    }
}

/// Check if user can reopen the window after closing
/// Returns true if tray is available OR a working shortcut exists
#[tauri::command]
pub fn can_reopen_window(state: State<'_, AppState>) -> bool {
    // If tray is available, user can always reopen from there
    if *state.tray_available.lock().unwrap() {
        return true;
    }

    // Check shortcut backend - some always work, some need verification
    let backend_info = crate::shortcuts::backend_info();
    match backend_info.backend.as_str() {
        "TauriPlugin" => true, // X11 shortcuts always work
        "ManualSetup" => true, // IPC toggle always available
        "PortalGlobalShortcuts" => {
            // Portal needs a bound shortcut without errors
            let has_shortcut = state.portal_shortcut.lock().unwrap().is_some();
            let no_error = state.portal_bind_error.lock().unwrap().is_none();
            has_shortcut && no_error
        }
        _ => false,
    }
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<whis_core::AudioDeviceInfo>, String> {
    whis_core::list_audio_devices().map_err(|e| e.to_string())
}

/// Exit the application gracefully
/// Called after settings have been flushed to disk
#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}
