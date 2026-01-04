//! Tray Icon Management
//!
//! Manages system tray icons for different recording states.
//! Icons are pre-loaded at compile time for fast state transitions.

use tauri::image::Image;

// Static icons for each state (pre-loaded at compile time)
pub const ICON_IDLE: &[u8] = include_bytes!("../../icons/icon-idle.png");
pub const ICON_RECORDING: &[u8] = include_bytes!("../../icons/icon-recording.png");
pub const ICON_TRANSCRIBING: &[u8] = include_bytes!("../../icons/icon-processing.png");

/// Load and set a tray icon from raw PNG bytes
pub fn set_tray_icon(tray: &tauri::tray::TrayIcon, icon_bytes: &[u8]) {
    match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let icon = Image::new_owned(rgba.into_raw(), width, height);
            if let Err(e) = tray.set_icon(Some(icon)) {
                eprintln!("Failed to set tray icon: {e}");
            }
        }
        Err(e) => eprintln!("Failed to load tray icon: {e}"),
    }
}
