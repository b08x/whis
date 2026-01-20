//! Platform and Compositor Detection
//!
//! Shared platform detection logic for CLI and Desktop applications.
//! Identifies the operating system, display server, and desktop compositor.

use serde::Serialize;
use std::env;

/// Operating system platform
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Platform {
    MacOS,
    Windows,
    LinuxX11,
    LinuxWayland,
}

impl Platform {
    /// Whether this platform uses Wayland
    pub fn is_wayland(&self) -> bool {
        matches!(self, Platform::LinuxWayland)
    }

    /// Whether this platform uses X11
    pub fn is_x11(&self) -> bool {
        matches!(self, Platform::LinuxX11)
    }

    /// Whether this is a Linux platform
    pub fn is_linux(&self) -> bool {
        matches!(self, Platform::LinuxX11 | Platform::LinuxWayland)
    }
}

/// Desktop compositor/environment
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Compositor {
    /// macOS or Windows (native windowing)
    Native,
    /// X11 without specific desktop environment detected
    X11,
    /// GNOME (including Ubuntu's Unity-like GNOME)
    Gnome,
    /// KDE Plasma
    KdePlasma,
    /// Sway (i3-compatible Wayland compositor)
    Sway,
    /// Hyprland
    Hyprland,
    /// Generic wlroots-based compositor
    Wlroots,
    /// Unknown compositor
    Unknown(String),
}

impl Compositor {
    /// Human-readable display name
    pub fn display_name(&self) -> &str {
        match self {
            Compositor::Native => "Native",
            Compositor::X11 => "X11",
            Compositor::Gnome => "GNOME",
            Compositor::KdePlasma => "KDE Plasma",
            Compositor::Sway => "Sway",
            Compositor::Hyprland => "Hyprland",
            Compositor::Wlroots => "wlroots",
            Compositor::Unknown(name) => name,
        }
    }
}

/// Complete platform information
#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub compositor: Compositor,
    pub portal_version: u32,
    pub is_flatpak: bool,
}

/// Check if running inside a Flatpak sandbox
pub fn is_flatpak() -> bool {
    std::env::var("FLATPAK_ID").is_ok() || std::path::Path::new("/.flatpak-info").exists()
}

/// Check if programmatic window positioning is supported.
/// Returns false for pure Wayland where compositors ignore set_position().
pub fn supports_window_positioning() -> bool {
    #[cfg(target_os = "linux")]
    {
        let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        if !is_wayland {
            return true; // X11 always supports positioning
        }
        // On Wayland, only XWayland (non-Flatpak) supports positioning
        // Flatpak sandbox prevents XWayland from working reliably
        !is_flatpak() && std::env::var("DISPLAY").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true // macOS and Windows support positioning
    }
}

/// Detect the current platform, compositor, and portal version
pub fn detect_platform() -> PlatformInfo {
    #[cfg(target_os = "macos")]
    {
        PlatformInfo {
            platform: Platform::MacOS,
            compositor: Compositor::Native,
            portal_version: 0,
            is_flatpak: false,
        }
    }

    #[cfg(target_os = "windows")]
    {
        PlatformInfo {
            platform: Platform::Windows,
            compositor: Compositor::Native,
            portal_version: 0,
            is_flatpak: false,
        }
    }

    #[cfg(target_os = "linux")]
    {
        let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let wayland_display = env::var("WAYLAND_DISPLAY").is_ok();

        let platform = if session_type == "wayland" || wayland_display {
            Platform::LinuxWayland
        } else {
            Platform::LinuxX11
        };

        let compositor = detect_compositor_linux();
        let portal_version = if platform == Platform::LinuxWayland {
            query_portal_version()
        } else {
            0
        };

        PlatformInfo {
            platform,
            compositor,
            portal_version,
            is_flatpak: is_flatpak(),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        PlatformInfo {
            platform: Platform::LinuxX11, // fallback
            compositor: Compositor::Unknown("Unknown OS".into()),
            portal_version: 0,
            is_flatpak: false,
        }
    }
}

/// Detect the Linux desktop compositor from environment variables
#[cfg(target_os = "linux")]
fn detect_compositor_linux() -> Compositor {
    let desktop = env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_default()
        .to_lowercase();

    if desktop.contains("gnome") || desktop.contains("ubuntu") {
        Compositor::Gnome
    } else if desktop.contains("kde") || desktop.contains("plasma") {
        Compositor::KdePlasma
    } else if desktop.contains("sway") {
        Compositor::Sway
    } else if desktop.contains("hyprland") {
        Compositor::Hyprland
    } else if env::var("WAYLAND_DISPLAY").is_ok() {
        // Wayland but unknown compositor - likely wlroots-based
        if desktop.is_empty() {
            Compositor::Wlroots
        } else {
            Compositor::Unknown(desktop)
        }
    } else if desktop.is_empty() {
        Compositor::X11
    } else {
        Compositor::Unknown(desktop)
    }
}

/// Query the XDG Portal GlobalShortcuts version
#[cfg(target_os = "linux")]
fn query_portal_version() -> u32 {
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
