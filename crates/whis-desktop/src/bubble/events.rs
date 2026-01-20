//! Bubble Event Handlers
//!
//! Controls bubble visibility and state updates.

use tauri::{AppHandle, Emitter, Manager};

use crate::state::{AppState, RecordingState};

/// Show the bubble with current recording state
pub fn show_bubble(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Check if bubble is enabled
    if !state.with_settings(|s| s.ui.bubble.enabled) {
        return;
    }

    let Some(window) = app.get_webview_window("bubble") else {
        return;
    };

    // Only update position on platforms that support it.
    // On Wayland, let the compositor place the window naturally.
    if whis_core::platform::supports_window_positioning()
        && let Ok((x, y)) = super::window::calculate_bubble_position(app)
    {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }

    let current_state = state.get_state();
    let _ = window.show();
    let _ = window.emit("bubble-state", state_to_string(current_state));
}

/// Hide the bubble
pub fn hide_bubble(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("bubble") {
        let _ = window.emit("bubble-hide", ());
        // Delay hide for fade-out animation (use async to avoid spawning OS thread)
        let window_clone = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            let _ = window_clone.hide();
        });
    }
}

/// Update bubble state without changing visibility
pub fn update_bubble_state(app: &AppHandle, new_state: RecordingState) {
    let state = app.state::<AppState>();

    // Only update if bubble is enabled
    if !state.with_settings(|s| s.ui.bubble.enabled) {
        return;
    }

    if let Some(window) = app.get_webview_window("bubble") {
        let _ = window.emit("bubble-state", state_to_string(new_state));
    }
}

/// Convert RecordingState to string for frontend
fn state_to_string(state: RecordingState) -> &'static str {
    match state {
        RecordingState::Idle => "idle",
        RecordingState::Recording => "recording",
        RecordingState::Transcribing => "transcribing",
    }
}
