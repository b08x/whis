//! Audio recorder configuration.

use super::super::vad::VadConfig;

/// Configuration for the audio recorder.
#[derive(Debug, Clone, Default)]
pub struct RecorderConfig {
    /// Device name to use (None = system default)
    pub device_name: Option<String>,

    /// Voice Activity Detection configuration (no-op when vad feature disabled)
    pub vad: VadConfig,
}

impl RecorderConfig {
    /// Create a new recorder configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the device name.
    pub fn with_device(mut self, device_name: impl Into<String>) -> Self {
        self.device_name = Some(device_name.into());
        self
    }

    /// Set VAD configuration.
    pub fn with_vad(mut self, vad: VadConfig) -> Self {
        self.vad = vad;
        self
    }

    /// Disable VAD.
    pub fn without_vad(mut self) -> Self {
        self.vad = VadConfig::disabled();
        self
    }
}
