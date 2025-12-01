use anyhow::{Context, Result};
use arboard::Clipboard;
use std::io::Write;
use std::process::{Command, Stdio};

/// Check if running inside a Flatpak sandbox
fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Copy to clipboard using bundled wl-copy
///
/// In Flatpak, we bundle wl-clipboard and call wl-copy directly.
/// This is required because GNOME/Mutter does not implement the wlr-data-control
/// Wayland protocol that arboard's wayland-data-control feature requires.
fn copy_via_wl_copy(text: &str) -> Result<()> {
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

    Ok(())
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    // In Flatpak, use bundled wl-copy directly.
    // This is necessary because GNOME doesn't support wlr-data-control protocol.
    if is_flatpak() {
        return copy_via_wl_copy(text);
    }

    // Standard approach for non-Flatpak environments
    let mut clipboard = Clipboard::new().context("Failed to access clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to copy text to clipboard")?;

    Ok(())
}
