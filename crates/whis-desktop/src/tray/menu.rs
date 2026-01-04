//! Tray Menu Management
//!
//! Handles tray menu creation and dynamic updates based on recording state.
//! Platform-specific implementations for macOS (rebuild menu) and Linux (update text).

use super::TRAY_ID;
use super::icons::{ICON_IDLE, ICON_RECORDING, ICON_TRANSCRIBING, set_tray_icon};
use crate::state::{AppState, RecordingState};
use tauri::{
    AppHandle, Manager,
};

/// Update tray menu and icon for new recording state
pub fn update_tray(app: &AppHandle, new_state: RecordingState) {
    // Rebuild menu on macOS (workaround for menu item updates not reflecting)
    #[cfg(target_os = "macos")]
    {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let text = match new_state {
                RecordingState::Idle => "Start Recording",
                RecordingState::Recording => "Stop Recording",
                RecordingState::Transcribing => "Transcribing...",
            };
            let enabled = new_state != RecordingState::Transcribing;

            // Rebuild menu with updated state
            if let Ok(record) = MenuItem::with_id(app, "record", text, enabled, None::<&str>) {
                if let Ok(settings) =
                    MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
                {
                    if let Ok(sep) = PredefinedMenuItem::separator(app) {
                        if let Ok(quit) =
                            MenuItem::with_id(app, "quit", "Quit Whis", true, None::<&str>)
                        {
                            if let Ok(menu) =
                                Menu::with_items(app, &[&record, &sep, &settings, &sep, &quit])
                            {
                                let _ = tray.set_menu(Some(menu));
                                println!("Rebuilt tray menu to: {}", text);
                            }
                        }
                    }
                }
            }
        }
    }

    // Update menu item text using stored reference (Linux)
    #[cfg(not(target_os = "macos"))]
    {
        let app_state = app.state::<AppState>();
        if let Some(ref menu_item) = *app_state.record_menu_item.lock().unwrap() {
            let text = match new_state {
                RecordingState::Idle => "Start Recording",
                RecordingState::Recording => "Stop Recording",
                RecordingState::Transcribing => "Transcribing...",
            };
            if let Err(e) = menu_item.set_text(text) {
                eprintln!("Failed to update menu item text: {e}");
            }
            if let Err(e) = menu_item.set_enabled(new_state != RecordingState::Transcribing) {
                eprintln!("Failed to update menu item enabled state: {e}");
            }
            println!("Updated tray menu to: {}", text);
        } else {
            eprintln!("Menu item not found in state");
        }
    }

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        // Update tooltip (platform-specific behavior)
        #[cfg(target_os = "macos")]
        let tooltip = match new_state {
            RecordingState::Idle => "Whis",
            RecordingState::Recording => "Whis - Recording...",
            RecordingState::Transcribing => "Whis - Transcribing...",
        };
        #[cfg(not(target_os = "macos"))]
        let tooltip = match new_state {
            RecordingState::Idle => "Whis - Click to record",
            RecordingState::Recording => "Whis - Recording... Click to stop",
            RecordingState::Transcribing => "Whis - Transcribing...",
        };
        let _ = tray.set_tooltip(Some(tooltip));

        // Set static icon based on state
        let icon = match new_state {
            RecordingState::Idle => ICON_IDLE,
            RecordingState::Recording => ICON_RECORDING,
            RecordingState::Transcribing => ICON_TRANSCRIBING,
        };
        set_tray_icon(&tray, icon);
    }
}
