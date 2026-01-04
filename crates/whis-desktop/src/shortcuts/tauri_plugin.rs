//! Tauri Plugin Shortcut Implementation
//!
//! Implements global keyboard shortcuts using the Tauri plugin.
//! Works on X11, macOS, and Windows platforms where native shortcuts are supported.

use std::str::FromStr;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Setup global shortcuts using Tauri plugin (for X11, macOS, Windows)
pub fn setup_tauri_shortcut(
    app: &tauri::App,
    shortcut_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // Attempt to parse the shortcut
    let shortcut =
        Shortcut::from_str(shortcut_str).map_err(|e| format!("Invalid shortcut: {e}"))?;

    // Initialize plugin with generic handler
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    println!("Tauri shortcut triggered!");
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        crate::recording::toggle_recording(handle);
                    });
                }
            })
            .build(),
    )?;

    // Register the shortcut
    app.global_shortcut().register(shortcut)?;
    println!("Tauri global shortcut registered: {shortcut_str}");

    Ok(())
}

/// Update shortcut. Returns Ok(true) if restart is needed, Ok(false) if applied immediately.
pub fn update_tauri_shortcut(
    app: &AppHandle,
    new_shortcut: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Unregister all existing shortcuts
    app.global_shortcut().unregister_all()?;

    // Parse and register new one
    let shortcut =
        Shortcut::from_str(new_shortcut).map_err(|e| format!("Invalid shortcut: {e}"))?;
    app.global_shortcut().register(shortcut)?;
    println!("Updated Tauri global shortcut to: {new_shortcut}");
    Ok(false) // No restart needed
}
