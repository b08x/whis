use crate::recording;
use crate::state::{AppState, RecordingState};
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

// Static icons for each state (pre-loaded at compile time)
const ICON_IDLE: &[u8] = include_bytes!("../icons/icon-idle.png");
const ICON_RECORDING: &[u8] = include_bytes!("../icons/icon-recording.png");
const ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/icon-processing.png");

pub const TRAY_ID: &str = "whis-tray";

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
    let idle_bytes = include_bytes!("../icons/icon-idle.png");
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
        .on_menu_event(|app, event| match event.id.as_ref() {
            "record" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    toggle_recording(app_clone);
                });
            }
            "settings" => {
                open_settings_window(app.clone());
            }
            "quit" => {
                // Emit event to frontend to flush settings before exit
                let _ = app.emit("tray-quit-requested", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // On Linux, left-click toggles recording (menu is on right-click)
            // On macOS, menu shows on left-click so we don't need this handler
            #[cfg(not(target_os = "macos"))]
            {
                use tauri::tray::TrayIconEvent;
                if let TrayIconEvent::Click { button, .. } = event
                    && button == tauri::tray::MouseButton::Left
                {
                    let app_handle = tray.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        toggle_recording(app_handle);
                    });
                }
            }
            #[cfg(target_os = "macos")]
            {
                // Suppress unused variable warning
                let _ = (tray, event);
            }
        })
        .build(app)?;

    Ok(())
}

fn open_settings_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let window = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("index.html".into()))
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

fn update_tray(app: &AppHandle, new_state: RecordingState) {
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

fn set_tray_icon(tray: &tauri::tray::TrayIcon, icon_bytes: &[u8]) {
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
