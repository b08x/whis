//! Record Command Types
//!
//! This module defines the core data structures used throughout the record command
//! pipeline. These types flow through the four phases: Record → Transcribe → Process → Output.
//!
//! # Type Flow
//!
//! ```text
//! RecordConfig
//!     ↓
//! ┌─────────────────┐
//! │  Record Phase   │  → RecordResult { audio, raw_samples }
//! └─────────────────┘
//!     ↓
//! ┌─────────────────┐
//! │ Transcribe Phase│  → TranscriptionResult { text }
//! └─────────────────┘
//!     ↓
//! ┌─────────────────┐
//! │  Process Phase  │  → ProcessedResult { text }
//! └─────────────────┘
//!     ↓
//! ┌─────────────────┐
//! │  Output Phase   │  → Final output (clipboard/stdout)
//! └─────────────────┘
//! ```
//!
//! # Key Types
//!
//! - `RecordConfig`: User-provided configuration (flags, presets, output mode)
//! - `InputSource`: Audio source (microphone, file, or stdin)
//! - `RecordResult`: Raw audio data + optional raw samples for local transcription
//! - `TranscriptionResult`: Raw transcript text from provider
//! - `ProcessedResult`: Final processed text after LLM cleanup/preset transform

use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use whis_core::{Preset, RecordingOutput};

/// Configuration for the record command
#[derive(Debug, Clone)]
pub struct RecordConfig {
    /// Audio input source
    pub input_source: InputSource,
    /// Whether to enable post-processing
    pub post_process: bool,
    /// Preset to apply to output
    pub preset: Option<Preset>,
    /// Whether to print to stdout instead of clipboard
    pub print: bool,
    /// Recording duration (None = until silence/manual stop)
    pub duration: Option<Duration>,
    /// Disable Voice Activity Detection
    pub no_vad: bool,
    /// Save raw audio samples to file
    pub save_raw: Option<PathBuf>,
}

impl RecordConfig {
    /// Create a new record configuration
    pub fn new(
        input_source: InputSource,
        post_process: bool,
        preset_name: Option<String>,
        print: bool,
        duration: Option<Duration>,
        no_vad: bool,
        save_raw: Option<PathBuf>,
    ) -> Result<Self> {
        // Load preset if provided
        let preset = if let Some(name) = preset_name {
            let (p, _source) = Preset::load(&name).map_err(|e| anyhow::anyhow!("{}", e))?;
            Some(p)
        } else {
            None
        };

        Ok(Self {
            input_source,
            post_process,
            preset,
            print,
            duration,
            no_vad,
            save_raw,
        })
    }

    /// Check if output should be quiet (for clean stdout)
    pub fn is_quiet(&self) -> bool {
        self.print
    }
}

/// Audio input source for recording
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Record from microphone
    Microphone,
    /// Read from audio file
    File(PathBuf),
    /// Read from stdin with format
    Stdin { format: String },
}

/// Result of audio recording/loading phase
pub struct RecordResult {
    /// The recorded or loaded audio
    pub audio: RecordingOutput,
    /// Raw samples (for optional saving)
    pub raw_samples: Option<(Vec<f32>, u32)>, // (samples, sample_rate)
}

/// Result of transcription phase
#[derive(Debug)]
pub struct TranscriptionResult {
    /// The transcribed text
    pub text: String,
}

/// Result of post-processing phase
#[derive(Debug)]
pub struct ProcessedResult {
    /// The processed text
    pub text: String,
}
