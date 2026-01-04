//! XDG Portal GlobalShortcuts Implementation
//!
//! Implements global keyboard shortcuts for Wayland using the XDG Desktop Portal.
//! Works with GNOME 48+, KDE, and Hyprland.
//!
//! ## Architecture
//!
//! ```text
//! Portal Setup
//! ├── registry.rs    - App ID registration
//! ├── dconf.rs       - GNOME dconf reading (fallback)
//! ├── binding.rs     - Shortcut binding & configuration
//! └── mod.rs         - Main setup & event listening
//! ```

pub mod binding;
pub mod dconf;
pub mod registry;

// Re-export public APIs
pub use binding::{
    bind_shortcut_with_trigger, configure_with_preferred_trigger, open_configure_shortcuts,
};
pub use dconf::read_portal_shortcut_from_dconf;
pub use registry::register_app_with_portal;

use tauri::{AppHandle, Manager};

/// Setup global shortcuts using the XDG Portal (for Wayland with GNOME 48+, KDE)
#[cfg(target_os = "linux")]
pub async fn setup_portal_shortcuts<F>(
    shortcut_str: String,
    on_toggle: F,
    app_handle: AppHandle,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn() + Send + Sync + 'static,
{
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt;

    // IMPORTANT: Register app_id with portal BEFORE making any portal calls
    // This is required for native apps to have a valid app_id for GlobalShortcuts
    if let Err(e) = register_app_with_portal().await {
        eprintln!("Warning: Portal registration failed: {e}");
    }

    // Try to read existing shortcut from dconf first (works even if portal bind fails)
    if let Some(existing) = read_portal_shortcut_from_dconf() {
        println!("Found existing portal shortcut in dconf: {existing}");
        let state = app_handle.state::<crate::state::AppState>();
        *state.portal_shortcut.lock().unwrap() = Some(existing);
    }

    let shortcuts = GlobalShortcuts::new().await?;
    let session = shortcuts.create_session().await?;

    // Check for existing shortcuts first
    if let Ok(list_request) = shortcuts.list_shortcuts(&session).await
        && let Ok(list_response) = list_request.response()
    {
        let existing = list_response.shortcuts();
        if let Some(s) = existing.iter().find(|s| s.id() == "toggle-recording") {
            let trigger = s.trigger_description().to_string();
            println!("Found existing portal shortcut in session: {trigger}");
            let state = app_handle.state::<crate::state::AppState>();
            *state.portal_shortcut.lock().unwrap() = Some(trigger);
            // Skip binding, just listen for activations
            let mut activated = shortcuts.receive_activated().await?;
            while let Some(event) = activated.next().await {
                if event.shortcut_id() == "toggle-recording" {
                    println!("Portal shortcut triggered!");
                    on_toggle();
                }
            }
            return Ok(());
        }
    }

    // Define the toggle-recording shortcut
    let shortcut = NewShortcut::new("toggle-recording", "Toggle voice recording")
        .preferred_trigger(Some(shortcut_str.as_str()));

    // Try to bind - pass None for parent window (GNOME may show dialog to user)
    // Note: GNOME shows a configuration dialog that user must interact with
    match shortcuts.bind_shortcuts(&session, &[shortcut], None).await {
        Ok(request) => match request.response() {
            Ok(bind_response) => {
                if let Some(bound) = bind_response
                    .shortcuts()
                    .iter()
                    .find(|s| s.id() == "toggle-recording")
                {
                    let trigger = bound.trigger_description().to_string();
                    if !trigger.is_empty() {
                        println!("Portal bound shortcut: {trigger}");
                        let state = app_handle.state::<crate::state::AppState>();
                        *state.portal_shortcut.lock().unwrap() = Some(trigger);
                    }
                }
                println!("Portal shortcuts registered. Listening for activations...");
            }
            Err(e) => {
                let msg = format!("Portal bind response failed: {e}");
                eprintln!("{msg}");
                eprintln!("Will use dconf shortcut if available");
                let state = app_handle.state::<crate::state::AppState>();
                *state.portal_bind_error.lock().unwrap() = Some(msg);
            }
        },
        Err(e) => {
            let msg = format!("Portal bind_shortcuts failed: {e}");
            eprintln!("{msg}");
            eprintln!("Will use dconf shortcut if available");
            let state = app_handle.state::<crate::state::AppState>();
            *state.portal_bind_error.lock().unwrap() = Some(msg);
        }
    }

    // Listen for activations (this should still work even if bind failed)
    let mut activated = shortcuts.receive_activated().await?;
    while let Some(event) = activated.next().await {
        if event.shortcut_id() == "toggle-recording" {
            println!("Portal shortcut triggered!");
            on_toggle();
        }
    }

    Ok(())
}
