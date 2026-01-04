//! Audio Recording Module
//!
//! This module provides cross-platform audio recording with the following features:
//! - Real-time resampling to 16kHz mono
//! - Voice Activity Detection (optional, via `vad` feature)
//! - Multiple output formats (MP3 via FFmpeg or embedded encoder)
//!
//! # Architecture
//!
//! ```text
//! AudioRecorder
//!   ├── Stream (cpal) - Platform-specific audio capture
//!   ├── Resampler     - Real-time 16kHz conversion
//!   ├── VAD (optional)- Voice activity detection
//!   └── Encoder       - MP3 encoding
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use whis_core::audio::{AudioRecorder, RecorderConfig};
//!
//! let config = RecorderConfig::default();
//! let mut recorder = AudioRecorder::new(config)?;
//! recorder.start()?;
//! // ... wait for input ...
//! let output = recorder.stop()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Platform Notes
//!
//! - **macOS**: Uses message passing architecture to avoid `Send` issues with cpal::Stream
//! - **Linux**: ALSA stderr suppression via safe FFI wrapper

mod devices;
mod encoder;
pub mod error;
mod loader;
mod recorder;
mod types;
mod vad;

// Re-export public types
pub use devices::list_audio_devices;
pub use encoder::{AudioEncoder, create_encoder};
pub use error::AudioError;
pub use loader::{load_audio_file, load_audio_stdin};
pub use recorder::{AudioRecorder, AudioStreamSender, RecorderConfig, RecordingData};
pub use types::{AudioChunk, AudioDeviceInfo, RecordingOutput};

// Re-export VAD types (always available - no-op when feature disabled)
pub use vad::{VadConfig, VadProcessor};
