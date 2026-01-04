//! Tray Initialization
//!
//! Handles initial system tray setup with menu creation and event handlers.

use super::{events, TRAY_ID};
use crate::state::AppState;
use tauri::{
    Manager,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

/// Initialize system tray with menu and event handlers
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let record = MenuItem::with_id(app, "record", "Start Recording", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Whis", true, None::<&str>)?;

    // Store the record menu item for later updates
    if let Some(state) = app.try_state::<AppState>() {
        *state.record_menu_item.lock().unwrap() = Some(record.clone());
    }

    let menu = Menu::with_items(app, &[&record, &sep, &settings, &sep, &quit])?;

    // Use image crate for consistent rendering (same as set_tray_icon)
    let idle_bytes = include_bytes!("../../icons/icon-idle.png");
    let img = image::load_from_memory(idle_bytes).expect("Failed to load idle icon");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let idle_icon = Image::new_owned(rgba.into_raw(), width, height);

    // Use app cache dir for tray icons so Flatpak host can access them
    // (default /tmp is sandboxed and GNOME AppIndicator can't read it)
    let cache_dir = app
        .path()
        .app_cache_dir()
        .expect("Failed to get app cache dir");

    // On macOS, show menu on left-click (standard behavior)
    // On Linux, use right-click for menu and left-click for quick record
    #[cfg(target_os = "macos")]
    let show_menu_on_left = true;
    #[cfg(not(target_os = "macos"))]
    let show_menu_on_left = false;

    #[cfg(target_os = "macos")]
    let tooltip = "Whis";
    #[cfg(not(target_os = "macos"))]
    let tooltip = "Whis - Click to record";

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(idle_icon)
        .temp_dir_path(cache_dir)
        .menu(&menu)
        .show_menu_on_left_click(show_menu_on_left)
        .tooltip(tooltip)
        .on_menu_event(|app, event| {
            events::handle_menu_event(app.clone(), event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            events::handle_tray_icon_event(tray.app_handle().clone(), event);
        })
        .build(app)?;

    Ok(())
}
