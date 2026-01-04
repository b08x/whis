//! Tray Event Handlers
//!
//! Handles tray menu clicks and tray icon interactions.
//! Platform-specific behavior for left-click on Linux vs macOS.

use super::menu::update_tray;
use crate::{recording, state::{AppState, RecordingState}};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Handle tray menu item clicks
pub fn handle_menu_event(app: AppHandle, event_id: &str) {
    match event_id {
        "record" => {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                toggle_recording(app_clone);
            });
        }
        "settings" => {
            open_settings_window(app);
        }
        "quit" => {
            // Emit event to frontend to flush settings before exit
            let _ = app.emit("tray-quit-requested", ());
        }
        _ => {}
    }
}

/// Handle tray icon clicks (Linux only - left-click toggles recording)
#[cfg(not(target_os = "macos"))]
pub fn handle_tray_icon_event(app: AppHandle, event: tauri::tray::TrayIconEvent) {
    use tauri::tray::TrayIconEvent;
    if let TrayIconEvent::Click { button, .. } = event
        && button == tauri::tray::MouseButton::Left
    {
        tauri::async_runtime::spawn(async move {
            toggle_recording(app);
        });
    }
}

#[cfg(target_os = "macos")]
pub fn handle_tray_icon_event(_app: AppHandle, _event: tauri::tray::TrayIconEvent) {
    // On macOS, menu shows on left-click so we don't handle icon events
}

/// Toggle recording with tray UI updates
/// Wraps the core recording logic and handles tray icon/menu updates
fn toggle_recording(app: AppHandle) {
    let state = app.state::<AppState>();
    let current_state = *state.state.lock().unwrap();

    match current_state {
        RecordingState::Idle => {
            // Start recording
            if let Err(e) = recording::start_recording_sync(&app, &state) {
                eprintln!("Failed to start recording: {e}");
            } else {
                update_tray(&app, RecordingState::Recording);
            }
        }
        RecordingState::Recording => {
            // Stop recording and transcribe
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                // Update tray to transcribing
                update_tray(&app_clone, RecordingState::Transcribing);

                // Run transcription pipeline
                if let Err(e) = recording::stop_and_transcribe(&app_clone).await {
                    eprintln!("Failed to transcribe: {e}");
                }

                // Update tray back to idle
                update_tray(&app_clone, RecordingState::Idle);
            });
        }
        RecordingState::Transcribing => {
            // Already transcribing, ignore
        }
    }
}

/// Open or focus the settings window
fn open_settings_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let window =
        WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("index.html".into()))
            .title("Whis Settings")
            .inner_size(600.0, 400.0)
            .min_inner_size(400.0, 300.0)
            .resizable(true)
            .decorations(false)
            .transparent(true)
            .build();

    // Fix Wayland window dragging by unsetting GTK titlebar
    // On Wayland, GTK's titlebar is required for dragging, but decorations(false)
    // removes it. By calling set_titlebar(None), we restore drag functionality
    // while keeping our custom chrome.
    match window {
        Ok(window) => {
            #[cfg(target_os = "linux")]
            {
                use gtk::prelude::GtkWindowExt;
                if let Ok(gtk_window) = window.gtk_window() {
                    gtk_window.set_titlebar(Option::<&gtk::Widget>::None);
                }
            }
            let _ = window.show();
        }
        Err(e) => eprintln!("Failed to create settings window: {e}"),
    }
}
