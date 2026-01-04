//! Portal Shortcut Binding
//!
//! Handles binding and configuration of shortcuts through the XDG Portal GlobalShortcuts interface.
//! Supports both Portal v1 (direct binding) and Portal v2+ (configuration dialog).

use super::registry::register_app_with_portal;
use crate::shortcuts::backend::portal_version;
use tauri::{AppHandle, Manager};

/// Open the system's shortcut configuration dialog (Portal v2+ only)
/// Falls back to direct binding on Portal v1
#[cfg(target_os = "linux")]
pub async fn open_configure_shortcuts(
    app_handle: AppHandle,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    bind_shortcut_with_trigger(None, app_handle).await
}

#[cfg(not(target_os = "linux"))]
pub async fn open_configure_shortcuts(
    _app_handle: AppHandle,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(None)
}

/// Bind a shortcut with an optional preferred trigger from in-app key capture
/// Works on Portal v1 and v2. On v2, also opens the configuration dialog.
/// Returns the actual binding after success.
#[cfg(target_os = "linux")]
pub async fn bind_shortcut_with_trigger(
    preferred_trigger: Option<&str>,
    app_handle: AppHandle,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};

    // IMPORTANT: Register app_id with portal BEFORE making any portal calls
    // This is required for native apps to have a valid app_id for GlobalShortcuts
    if let Err(e) = register_app_with_portal().await {
        eprintln!("Warning: Portal registration failed: {e}");
    }

    let version = portal_version();
    println!("Portal GlobalShortcuts version: {version}");

    let shortcuts = GlobalShortcuts::new().await?;
    let session = shortcuts.create_session().await?;

    // Check for existing shortcuts first (XDG spec: can only bind once per session)
    if let Ok(list_request) = shortcuts.list_shortcuts(&session).await
        && let Ok(list_response) = list_request.response()
    {
        let existing = list_response.shortcuts();
        if !existing.is_empty() {
            println!("Found {} existing shortcut(s) in session", existing.len());
            if let Some(s) = existing.iter().find(|s| s.id() == "toggle-recording") {
                let trigger = s.trigger_description().to_string();
                println!("Using existing shortcut: {trigger}");
                let state = app_handle.state::<crate::state::AppState>();
                *state.portal_shortcut.lock().unwrap() = Some(trigger.clone());
                return Ok(Some(trigger));
            }
        }
    }

    // Bind our shortcut ID with optional preferred trigger
    let mut shortcut = NewShortcut::new("toggle-recording", "Toggle voice recording");
    if let Some(trigger) = preferred_trigger {
        // Convert from Tauri format (Ctrl+Shift+R) to XDG format (<Control><Shift>r)
        let xdg_trigger = convert_to_xdg_format(trigger);
        println!("Requesting preferred trigger: {trigger} (XDG: {xdg_trigger})");
        shortcut = shortcut.preferred_trigger(Some(xdg_trigger.as_str()));
    }

    println!("Binding shortcut...");

    // Try to bind the shortcut (GNOME shows a dialog that user must interact with)
    let bind_result = shortcuts.bind_shortcuts(&session, &[shortcut], None).await;

    match bind_result {
        Ok(request) => {
            match request.response() {
                Ok(bind_response) => {
                    // Get the actual trigger that was bound
                    let trigger = bind_response
                        .shortcuts()
                        .iter()
                        .find(|s| s.id() == "toggle-recording")
                        .map(|s| s.trigger_description().to_string());

                    // On Portal v2+, also open configure dialog for confirmation
                    if version >= 2 {
                        println!("Portal v2: Opening configure dialog...");
                        let _ = shortcuts.configure_shortcuts(&session, None, None).await;

                        // Re-query after configure in case user changed it
                        if let Ok(list_request) = shortcuts.list_shortcuts(&session).await
                            && let Ok(list_response) = list_request.response()
                        {
                            let updated_trigger = list_response
                                .shortcuts()
                                .iter()
                                .find(|s| s.id() == "toggle-recording")
                                .map(|s| s.trigger_description().to_string());

                            if let Some(ref t) = updated_trigger {
                                let state = app_handle.state::<crate::state::AppState>();
                                *state.portal_shortcut.lock().unwrap() = Some(t.clone());
                                println!("Portal shortcut configured to: {t}");
                                return Ok(updated_trigger);
                            }
                        }
                    }

                    // Update AppState with bound trigger
                    if let Some(ref t) = trigger {
                        let state = app_handle.state::<crate::state::AppState>();
                        *state.portal_shortcut.lock().unwrap() = Some(t.clone());
                        println!("Portal shortcut bound to: {t}");
                    }

                    Ok(trigger)
                }
                Err(e) => Err(format!(
                    "Portal bind failed: {e}. The shortcut may conflict with an existing binding."
                )
                .into()),
            }
        }
        Err(e) => Err(format!("Portal request failed: {e}").into()),
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn bind_shortcut_with_trigger(
    _preferred_trigger: Option<&str>,
    _app_handle: AppHandle,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(None)
}

/// Legacy alias for configure_with_preferred_trigger
#[cfg(target_os = "linux")]
pub async fn configure_with_preferred_trigger(
    preferred_trigger: Option<&str>,
    app_handle: AppHandle,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    bind_shortcut_with_trigger(preferred_trigger, app_handle).await
}

#[cfg(not(target_os = "linux"))]
pub async fn configure_with_preferred_trigger(
    preferred_trigger: Option<&str>,
    app_handle: AppHandle,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    bind_shortcut_with_trigger(preferred_trigger, app_handle).await
}

/// Convert Tauri/human-readable shortcut format to XDG portal format
/// e.g., "Ctrl+Shift+R" -> "<Control><Shift>r"
#[cfg(target_os = "linux")]
fn convert_to_xdg_format(shortcut: &str) -> String {
    let parts: Vec<&str> = shortcut.split('+').collect();
    let mut result = String::new();

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => result.push_str("<Control>"),
            "shift" => result.push_str("<Shift>"),
            "alt" => result.push_str("<Alt>"),
            "super" | "meta" | "win" => result.push_str("<Super>"),
            key => result.push_str(&key.to_lowercase()),
        }
    }

    result
}
