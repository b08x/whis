//! Bubble Window Creation and Positioning
//!
//! Creates the floating bubble overlay window with platform-specific configuration.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

/// Bubble window dimensions
const BUBBLE_SIZE: f64 = 48.0;

/// Offset from screen edges
const BUBBLE_OFFSET: f64 = 50.0;

/// Create the bubble overlay window (hidden by default)
pub fn create_bubble_window(app: &AppHandle) -> Result<(), String> {
    let window = WebviewWindowBuilder::new(
        app,
        "bubble",
        WebviewUrl::App("src/bubble/index.html".into()),
    )
    .title("Whis Bubble")
    .inner_size(BUBBLE_SIZE, BUBBLE_SIZE)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    {
        // On Wayland (when positioning is not supported), remove invisible titlebar
        // geometry so the compositor can properly center the window.
        // This aligns the window's geometric center with its visual center.
        if !whis_core::platform::supports_window_positioning() {
            use gtk::prelude::GtkWindowExt;
            if let Ok(gtk_window) = window.gtk_window() {
                gtk_window.set_titlebar(Option::<&gtk::Widget>::None);
            }
        }
    }

    // Only set position on platforms that support it.
    // On Wayland, let the compositor place the window naturally (centered).
    if whis_core::platform::supports_window_positioning() {
        let (x, y) = calculate_bubble_position(app).unwrap_or((100, 100));
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }

    Ok(())
}

/// Calculate bubble position in physical pixels.
///
/// Uses custom_position if set (from user dragging), otherwise defaults to
/// bottom-center of the primary monitor's work area.
///
/// Returns physical pixel coordinates to avoid precision loss with fractional
/// scaling factors (1.25x, 1.5x) that can cause off-center positioning.
///
/// # Known Limitation (Linux/Dev Mode)
///
/// Bubble positioning may not work correctly when running via `cargo tauri dev`.
/// This is due to Tauri/GTK window positioning limitations on Linux:
/// - Wayland does not support programmatic window positioning by design
/// - Dev mode initializes GTK differently than production builds
///
/// The bubble position works correctly when installed via `just install-desktop`.
///
/// Related Tauri issues:
/// - <https://github.com/tauri-apps/tauri/issues/7376> (Linux positioning)
/// - <https://github.com/tauri-apps/tauri/issues/12411> (Wayland limitations)
pub fn calculate_bubble_position(app: &AppHandle) -> Result<(i32, i32), String> {
    let state = app.state::<AppState>();
    let custom_position = state.with_settings(|s| s.ui.bubble.custom_position);

    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor")?;

    let scale = monitor.scale_factor();

    // Only use custom_position on platforms that support dragging
    if whis_core::platform::supports_window_positioning()
        && let Some((x, y)) = custom_position
    {
        // custom_position is stored in logical, convert to physical
        return Ok(((x * scale) as i32, (y * scale) as i32));
    }

    // Work area is already in physical pixels
    let work_area = monitor.work_area();
    let bubble_physical = (BUBBLE_SIZE * scale) as i32;
    let offset_physical = (BUBBLE_OFFSET * scale) as i32;

    // Calculate in physical pixels (no precision loss)
    let x = work_area.position.x + (work_area.size.width as i32 - bubble_physical) / 2;
    let y = work_area.position.y + work_area.size.height as i32 - bubble_physical - offset_physical;

    Ok((x, y))
}
