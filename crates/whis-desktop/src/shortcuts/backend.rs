//! Shortcut Backend Detection
//!
//! Detects the appropriate global shortcut backend for the current platform:
//! - TauriPlugin: X11, macOS, Windows
//! - PortalGlobalShortcuts: Wayland with portal support
//! - ManualSetup: Wayland without portal (fallback to IPC)

use serde::Serialize;
use std::env;

/// Backend for global keyboard shortcuts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutBackend {
    /// Tauri plugin - works on X11, macOS, Windows
    TauriPlugin,
    /// XDG Portal GlobalShortcuts - works on Wayland with GNOME 48+, KDE, Hyprland
    PortalGlobalShortcuts,
    /// Manual setup - user configures compositor to run `whis-desktop --toggle`
    ManualSetup,
}

/// Information about shortcut capability on current system
pub struct ShortcutCapability {
    pub backend: ShortcutBackend,
    pub compositor: String,
}

/// Backend info for frontend consumption
#[derive(Debug, Clone, Serialize)]
pub struct ShortcutBackendInfo {
    pub backend: String,
    pub requires_restart: bool,
    pub compositor: String,
    pub portal_version: u32,
}

/// Get the GlobalShortcuts portal version (0 if unavailable)
#[cfg(target_os = "linux")]
pub fn portal_version() -> u32 {
    std::process::Command::new("busctl")
        .args([
            "--user",
            "get-property",
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.GlobalShortcuts",
            "version",
        ])
        .output()
        .ok()
        .and_then(|o| {
            let output = String::from_utf8_lossy(&o.stdout);
            // Output format: "u 1" or "u 2"
            output.split_whitespace().last()?.parse().ok()
        })
        .unwrap_or(0)
}

#[cfg(not(target_os = "linux"))]
pub fn portal_version() -> u32 {
    0
}

/// Get backend info for the frontend
pub fn backend_info() -> ShortcutBackendInfo {
    let capability = detect_backend();
    let portal_version = if capability.backend == ShortcutBackend::PortalGlobalShortcuts {
        portal_version()
    } else {
        0
    };

    ShortcutBackendInfo {
        backend: format!("{:?}", capability.backend),
        requires_restart: !matches!(capability.backend, ShortcutBackend::TauriPlugin),
        compositor: capability.compositor,
        portal_version,
    }
}

/// Detect the best shortcut backend for the current environment
pub fn detect_backend() -> ShortcutCapability {
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland_display = env::var("WAYLAND_DISPLAY").is_ok();

    // Check if running on Wayland
    if session_type == "wayland" || wayland_display {
        if is_portal_available() {
            ShortcutCapability {
                backend: ShortcutBackend::PortalGlobalShortcuts,
                compositor: detect_compositor(),
            }
        } else {
            ShortcutCapability {
                backend: ShortcutBackend::ManualSetup,
                compositor: detect_compositor(),
            }
        }
    } else {
        // X11 or other - use Tauri plugin
        ShortcutCapability {
            backend: ShortcutBackend::TauriPlugin,
            compositor: "X11".into(),
        }
    }
}

/// Check if GlobalShortcuts portal is available via D-Bus
#[cfg(target_os = "linux")]
fn is_portal_available() -> bool {
    std::process::Command::new("busctl")
        .args([
            "--user",
            "introspect",
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("GlobalShortcuts"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn is_portal_available() -> bool {
    false
}

/// Detect the current desktop compositor
fn detect_compositor() -> String {
    env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "Unknown".into())
}
