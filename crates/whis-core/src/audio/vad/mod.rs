//! Voice Activity Detection (VAD) for audio recording
//!
//! This module provides VAD functionality that adapts based on feature flags:
//! - With `feature = "vad"`: Full Silero VAD with speech detection
//! - Without `feature = "vad"`: No-op passthrough implementation
//!
//! This design eliminates cfg blocks in consuming code - VadProcessor
//! always exists with the same API regardless of features enabled.

// Feature-gated module pattern: export real or no-op implementation

#[cfg(feature = "vad")]
mod processor;

#[cfg(not(feature = "vad"))]
mod processor_noop;

// Re-export the appropriate implementation
#[cfg(feature = "vad")]
pub use processor::VadProcessor;

#[cfg(not(feature = "vad"))]
pub use processor_noop::VadProcessor;

// VadConfig is always available (not feature-gated)

/// Configuration for Voice Activity Detection.
#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// Whether VAD is enabled
    pub enabled: bool,
    /// VAD threshold (0.0-1.0), higher values are more sensitive
    pub threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 0.5,
        }
    }
}

impl VadConfig {
    /// Create a new VAD configuration.
    pub fn new(enabled: bool, threshold: f32) -> Self {
        Self { enabled, threshold }
    }

    /// Create a disabled VAD configuration.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            threshold: 0.5,
        }
    }

    /// Create an enabled VAD configuration with the given threshold.
    pub fn enabled_with_threshold(threshold: f32) -> Self {
        Self {
            enabled: true,
            threshold,
        }
    }
}
