//! User interface settings for desktop applications.

use serde::{Deserialize, Serialize};

#[cfg(feature = "clipboard")]
use crate::clipboard::ClipboardMethod;

/// Settings for UI behavior and device configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// Global keyboard shortcut (desktop only)
    pub shortcut: String,

    /// Clipboard method for copying text (auto, xclip, wl-copy, arboard)
    #[cfg(feature = "clipboard")]
    #[serde(default)]
    pub clipboard_method: ClipboardMethod,

    /// Selected microphone device name (None = system default)
    #[serde(default)]
    pub microphone_device: Option<String>,

    /// Voice Activity Detection settings
    #[serde(default)]
    pub vad: VadSettings,

    /// Currently active preset name (if any)
    #[serde(default)]
    pub active_preset: Option<String>,
}

/// Voice Activity Detection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadSettings {
    /// Enable Voice Activity Detection to skip silence during recording
    #[serde(default)]
    pub enabled: bool,

    /// VAD speech probability threshold (0.0-1.0, default 0.5)
    #[serde(default = "default_vad_threshold")]
    pub threshold: f32,
}

fn default_vad_threshold() -> f32 {
    0.5
}

impl Default for VadSettings {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for conservative behavior
            threshold: default_vad_threshold(),
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Alt+W".to_string(),
            #[cfg(feature = "clipboard")]
            clipboard_method: ClipboardMethod::default(),
            microphone_device: None,
            vad: VadSettings::default(),
            active_preset: None,
        }
    }
}
