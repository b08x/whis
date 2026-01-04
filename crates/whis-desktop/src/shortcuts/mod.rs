//! Global Keyboard Shortcuts Module
//!
//! Provides cross-platform global keyboard shortcut support with multiple backends:
//! - **TauriPlugin**: X11, macOS, Windows (native shortcuts)
//! - **PortalGlobalShortcuts**: Wayland with XDG Desktop Portal (GNOME 48+, KDE, Hyprland)
//! - **ManualSetup**: Fallback to compositor configuration + IPC toggle
//!
//! ## Architecture
//!
//! ```text
//! shortcuts/
//! ├── backend.rs           - Backend detection & capability
//! ├── tauri_plugin.rs      - X11/macOS/Windows implementation
//! ├── portal/              - Wayland portal implementation
//! │   ├── mod.rs           - Portal setup & event listening
//! │   ├── binding.rs       - Shortcut binding & configuration
//! │   ├── registry.rs      - App ID registration
//! │   └── dconf.rs         - GNOME dconf integration
//! ├── ipc.rs               - Unix socket toggle server
//! ├── manual.rs            - Manual setup instructions
//! └── mod.rs               - Public API
//! ```

pub mod backend;
pub mod ipc;
pub mod manual;
pub mod portal;
pub mod tauri_plugin;

// Re-export backend detection
pub use backend::{
    backend_info, detect_backend, portal_version, ShortcutBackend, ShortcutBackendInfo,
    ShortcutCapability,
};

// Re-export portal functions
pub use portal::{
    bind_shortcut_with_trigger, configure_with_preferred_trigger, open_configure_shortcuts,
    read_portal_shortcut_from_dconf, register_app_with_portal, setup_portal_shortcuts,
};

// Re-export tauri plugin functions
pub use tauri_plugin::{setup_tauri_shortcut, update_tauri_shortcut};

// Re-export IPC functions
pub use ipc::{send_toggle_command, start_ipc_listener};

// Re-export manual instructions
pub use manual::print_manual_setup_instructions;

use tauri::{AppHandle, Manager};

/// Setup shortcuts based on detected backend
pub fn setup_shortcuts(app: &tauri::App) {
    let capability = detect_backend();
    let state = app.state::<crate::state::AppState>();
    let settings = state.settings.lock().unwrap();
    let shortcut_str = settings.ui.shortcut.clone();
    drop(settings);

    println!(
        "Detected environment: {} (backend: {:?})",
        capability.compositor, capability.backend
    );

    match capability.backend {
        ShortcutBackend::TauriPlugin => {
            if let Err(e) = setup_tauri_shortcut(app, &shortcut_str) {
                eprintln!("Failed to setup Tauri shortcut: {e}");
                eprintln!("Falling back to manual setup mode");
                print_manual_setup_instructions(&capability.compositor, &shortcut_str);
            }
        }
        #[cfg(target_os = "linux")]
        ShortcutBackend::PortalGlobalShortcuts => {
            let app_handle = app.handle().clone();
            let app_handle_for_state = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let toggle_handle = app_handle.clone();
                if let Err(e) = setup_portal_shortcuts(
                    shortcut_str,
                    move || {
                        let handle = toggle_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            crate::recording::toggle_recording(handle);
                        });
                    },
                    app_handle_for_state,
                )
                .await
                {
                    eprintln!("Portal shortcuts failed: {e}");
                    eprintln!("Falling back to CLI mode");
                }
            });
        }
        #[cfg(not(target_os = "linux"))]
        ShortcutBackend::PortalGlobalShortcuts => {
            // Portal shortcuts only available on Linux
            print_manual_setup_instructions(&capability.compositor, &shortcut_str);
        }
        ShortcutBackend::ManualSetup => {
            print_manual_setup_instructions(&capability.compositor, &shortcut_str);
        }
    }
}

/// Update shortcut. Returns Ok(true) if restart is needed, Ok(false) if applied immediately.
pub fn update_shortcut(
    app: &AppHandle,
    new_shortcut: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let capability = detect_backend();

    match capability.backend {
        ShortcutBackend::TauriPlugin => {
            update_tauri_shortcut(app, new_shortcut)?;
            Ok(false) // No restart needed
        }
        _ => {
            // For portals and CLI, dynamic updates require restart.
            println!("Shortcut saved. Restart required for changes to take effect.");
            Ok(true) // Restart needed
        }
    }
}
