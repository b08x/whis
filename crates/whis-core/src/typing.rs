//! Cross-platform text typing via virtual keyboard.
//!
//! This module provides the ability to type text directly into the active window
//! by simulating keyboard input. Multiple backends are supported for different
//! platforms and display servers.
//!
//! # Backends
//!
//! - **wrtype** - Wayland (uses zwp_virtual_keyboard_v1 protocol)
//! - **enigo** - X11, macOS, Windows (cross-platform input simulation)
//!
//! # Platform Support
//!
//! | Platform | Backend |
//! |----------|---------|
//! | Wayland  | wrtype  |
//! | X11      | enigo   |
//! | macOS    | enigo   |
//! | Windows  | enigo   |
//!
//! # Permissions
//!
//! - **macOS**: Requires Accessibility permission in System Preferences
//! - **Windows**: Cannot type into elevated (admin) windows
//! - **Wayland**: Requires compositor support for virtual keyboard protocol
//!
//! # Usage
//!
//! ```ignore
//! use whis_core::typing::{type_text, TypingBackend};
//!
//! // Auto-detect best backend for current platform
//! type_text("Hello, world!", TypingBackend::Auto, None)?;
//!
//! // With inter-key delay for slow applications
//! type_text("Hello", TypingBackend::Auto, Some(10))?;
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::platform::{Platform, detect_platform};

/// Backend for typing text into the active window
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TypingBackend {
    /// Auto-detect based on platform:
    /// - Wayland → wrtype
    /// - X11/macOS/Windows → enigo
    #[default]
    Auto,
    /// Force wrtype (Wayland only)
    Wrtype,
    /// Force enigo (X11, macOS, Windows)
    Enigo,
}

/// How to output transcribed text
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMethod {
    /// Copy to clipboard only (default, current behavior)
    #[default]
    Clipboard,
    /// Type directly into active window
    TypeToWindow,
    /// Both clipboard and type to window
    Both,
}

impl std::fmt::Display for OutputMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMethod::Clipboard => write!(f, "clipboard"),
            OutputMethod::TypeToWindow => write!(f, "type to window"),
            OutputMethod::Both => write!(f, "clipboard + type to window"),
        }
    }
}

/// Type text into the active window using the specified backend
///
/// # Arguments
///
/// * `text` - The text to type
/// * `backend` - Which typing backend to use
/// * `delay_ms` - Optional delay between keystrokes (milliseconds)
///
/// # Errors
///
/// Returns an error if the typing backend fails or is not available
/// for the current platform.
pub fn type_text(text: &str, backend: TypingBackend, delay_ms: Option<u32>) -> Result<()> {
    crate::verbose!("Typing {} chars to active window", text.len());
    crate::verbose!("Backend: {:?}, Delay: {:?}ms", backend, delay_ms);

    match backend {
        TypingBackend::Auto => type_auto(text, delay_ms),
        TypingBackend::Wrtype => type_via_wrtype(text),
        TypingBackend::Enigo => type_via_enigo(text, delay_ms),
    }
}

/// Auto-detect the best typing backend for the current platform
fn type_auto(text: &str, delay_ms: Option<u32>) -> Result<()> {
    let platform_info = detect_platform();
    crate::verbose!(
        "Auto-detecting typing backend for {:?}",
        platform_info.platform
    );

    match platform_info.platform {
        Platform::LinuxWayland => {
            crate::verbose!("Using wrtype for Wayland");
            type_via_wrtype(text)
        }
        Platform::LinuxX11 | Platform::MacOS | Platform::Windows => {
            crate::verbose!("Using enigo for {:?}", platform_info.platform);
            type_via_enigo(text, delay_ms)
        }
    }
}

/// Type text using wrtype (Wayland virtual keyboard)
///
/// Uses the zwp_virtual_keyboard_v1 Wayland protocol to simulate keyboard input.
/// Only works on Wayland compositors that support this protocol (GNOME, KDE, Sway, etc.)
#[cfg(target_os = "linux")]
fn type_via_wrtype(text: &str) -> Result<()> {
    use wrtype::WrtypeClient;

    crate::verbose!("Connecting to Wayland for virtual keyboard");

    let mut client = WrtypeClient::new()
        .context("Failed to create Wayland virtual keyboard client. Is your compositor running and does it support zwp_virtual_keyboard_v1?")?;

    client
        .type_text(text)
        .context("Failed to type text via Wayland virtual keyboard")?;

    crate::verbose!("wrtype succeeded");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn type_via_wrtype(_text: &str) -> Result<()> {
    anyhow::bail!("wrtype is only available on Linux/Wayland")
}

/// Type text using enigo (cross-platform input simulation)
///
/// - X11: Uses XTest extension
/// - macOS: Uses CoreGraphics/CGEvent (requires Accessibility permission)
/// - Windows: Uses SendInput API (cannot type into elevated windows)
fn type_via_enigo(text: &str, delay_ms: Option<u32>) -> Result<()> {
    use enigo::{Enigo, Keyboard, Settings};

    crate::verbose!("Initializing enigo");

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow::anyhow!("Failed to initialize enigo: {}", e))?;

    if let Some(delay) = delay_ms {
        crate::verbose!("Typing with {}ms delay between characters", delay);
        // Type character by character with delay
        for c in text.chars() {
            enigo
                .text(&c.to_string())
                .map_err(|e| anyhow::anyhow!("Failed to type character '{}': {}", c, e))?;
            std::thread::sleep(std::time::Duration::from_millis(delay as u64));
        }
    } else {
        enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("Failed to type text: {}", e))?;
    }

    crate::verbose!("enigo succeeded");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typing_backend_default() {
        assert_eq!(TypingBackend::default(), TypingBackend::Auto);
    }

    #[test]
    fn test_output_method_default() {
        assert_eq!(OutputMethod::default(), OutputMethod::Clipboard);
    }

    #[test]
    fn test_typing_backend_serde() {
        let backend: TypingBackend = serde_json::from_str(r#""auto""#).unwrap();
        assert_eq!(backend, TypingBackend::Auto);

        let backend: TypingBackend = serde_json::from_str(r#""wrtype""#).unwrap();
        assert_eq!(backend, TypingBackend::Wrtype);

        let backend: TypingBackend = serde_json::from_str(r#""enigo""#).unwrap();
        assert_eq!(backend, TypingBackend::Enigo);
    }

    #[test]
    fn test_output_method_serde() {
        let method: OutputMethod = serde_json::from_str(r#""clipboard""#).unwrap();
        assert_eq!(method, OutputMethod::Clipboard);

        let method: OutputMethod = serde_json::from_str(r#""type_to_window""#).unwrap();
        assert_eq!(method, OutputMethod::TypeToWindow);

        let method: OutputMethod = serde_json::from_str(r#""both""#).unwrap();
        assert_eq!(method, OutputMethod::Both);
    }
}
