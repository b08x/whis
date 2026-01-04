//! Shortcut Configuration Commands
//!
//! Provides Tauri commands for configuring and managing global keyboard shortcuts.

use crate::shortcuts::ShortcutBackendInfo;
use crate::state::AppState;
use tauri::{AppHandle, State};

/// Get the current shortcut backend information
#[tauri::command]
pub fn shortcut_backend() -> ShortcutBackendInfo {
    crate::shortcuts::backend_info()
}

/// Open shortcut configuration dialog (Portal v2+) or bind directly (Portal v1)
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

/// Get the currently configured portal shortcut
/// Returns cached value or reads from dconf (GNOME)
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
