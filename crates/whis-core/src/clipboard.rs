use anyhow::{Context, Result};
use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

/// Clipboard method for copying text
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardMethod {
    /// Auto-detect: Flatpak→wl-copy, X11→xclip, Wayland→arboard
    #[default]
    Auto,
    /// Force xclip (X11)
    Xclip,
    /// Force wl-copy (Wayland)
    WlCopy,
    /// Force arboard (cross-platform)
    Arboard,
}

/// Check if running inside a Flatpak sandbox
fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Get the current session type (x11, wayland, or unknown)
fn session_type() -> &'static str {
    // Cache the result since env vars don't change
    static SESSION_TYPE: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    *SESSION_TYPE.get_or_init(|| {
        std::env::var("XDG_SESSION_TYPE")
            .map(|s| match s.to_lowercase().as_str() {
                "x11" => "x11",
                "wayland" => "wayland",
                _ => "unknown",
            })
            .unwrap_or("unknown")
    })
}

/// Copy to clipboard using bundled wl-copy
///
/// In Flatpak, we bundle wl-clipboard and call wl-copy directly.
/// This is required because GNOME/Mutter does not implement the wlr-data-control
/// Wayland protocol that arboard's wayland-data-control feature requires.
fn copy_via_wl_copy(text: &str) -> Result<()> {
    crate::verbose!("Using wl-copy for clipboard");

    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn wl-copy")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .context("Failed to write to wl-copy")?;
    }

    let status = child.wait().context("Failed to wait for wl-copy")?;
    if !status.success() {
        anyhow::bail!("wl-copy exited with non-zero status");
    }

    crate::verbose!("wl-copy succeeded");
    Ok(())
}

/// Copy to clipboard using xclip (for X11)
///
/// arboard has issues on some X11 setups where it reports success but
/// doesn't actually set the clipboard. xclip is more reliable.
fn copy_via_xclip(text: &str) -> Result<()> {
    crate::verbose!("Using xclip for clipboard");

    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn xclip. Install it with: sudo apt install xclip")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .context("Failed to write to xclip")?;
    }

    let status = child.wait().context("Failed to wait for xclip")?;
    if !status.success() {
        anyhow::bail!("xclip exited with non-zero status");
    }

    crate::verbose!("xclip succeeded");
    Ok(())
}

/// Copy to clipboard using arboard (cross-platform)
fn copy_via_arboard(text: &str) -> Result<()> {
    crate::verbose!("Using arboard for clipboard");

    let mut clipboard = Clipboard::new().context("Failed to access clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to copy text to clipboard")?;

    crate::verbose!("arboard succeeded");
    Ok(())
}

/// Copy text to clipboard using the specified method
pub fn copy_to_clipboard(text: &str, method: ClipboardMethod) -> Result<()> {
    crate::verbose!("Copying {} chars to clipboard", text.len());
    crate::verbose!(
        "Method: {:?}, Session: {}, Flatpak: {}",
        method,
        session_type(),
        is_flatpak()
    );

    match method {
        ClipboardMethod::Auto => {
            // Flatpak: use bundled wl-copy (GNOME doesn't support wlr-data-control)
            if is_flatpak() {
                return copy_via_wl_copy(text);
            }

            // X11: use xclip (arboard can fail silently on some setups)
            if session_type() == "x11" {
                return copy_via_xclip(text);
            }

            // Wayland (non-Flatpak): use arboard
            copy_via_arboard(text)
        }
        ClipboardMethod::Xclip => copy_via_xclip(text),
        ClipboardMethod::WlCopy => copy_via_wl_copy(text),
        ClipboardMethod::Arboard => copy_via_arboard(text),
    }
}
