//! Record Command Types
//!
//! This module defines the core data structures used throughout the record command
//! pipeline. Microphone input uses progressive transcription.
//!
//! # Type Flow
//!
//! ```text
//! RecordConfig
//!     ↓
//! ┌─────────────────┐
//! │  Record Phase   │  → Vec<f32> samples (16kHz mono)
//! └─────────────────┘
//!     ↓
//! ┌─────────────────┐
//! │  Progressive    │  → TranscriptionResult { text }
//! │  Transcription  │
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
//! - `TranscriptionResult`: Raw transcript text from provider
//! - `ProcessedResult`: Final processed text after LLM cleanup/preset transform

use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use whis_core::Preset;

use crate::args::{InputOptions, OutputFormat, OutputOptions, ProcessingOptions};

/// Configuration for the record command
#[derive(Debug, Clone)]
pub struct RecordConfig {
    /// Input file path (None = record from microphone)
    pub input_file: Option<PathBuf>,
    /// Whether to enable post-processing
    pub post_process: bool,
    /// Preset to apply to output
    pub preset: Option<Preset>,
    /// Whether to print to stdout instead of clipboard
    pub print: bool,
    /// Output file path (None = clipboard)
    pub output_path: Option<PathBuf>,
    /// Output format (txt, srt, vtt)
    pub format: OutputFormat,
    /// Recording duration (None = until silence/manual stop)
    pub duration: Option<Duration>,
    /// Disable Voice Activity Detection
    pub no_vad: bool,
    /// Language override (None = use configured language)
    pub language: Option<String>,
}

impl RecordConfig {
    /// Create configuration from CLI options
    pub fn from_cli(
        input: &InputOptions,
        processing: &ProcessingOptions,
        output: &OutputOptions,
    ) -> Result<Self> {
        // Load preset if provided
        let preset = if let Some(name) = &processing.preset {
            let (p, _source) = Preset::load(name).map_err(|e| anyhow::anyhow!("{}", e))?;
            Some(p)
        } else {
            None
        };

        // Auto-detect format from file extension if not explicitly set
        let format = if output.format == OutputFormat::Txt {
            output
                .output
                .as_ref()
                .and_then(|p| OutputFormat::from_extension(p))
                .unwrap_or(output.format)
        } else {
            output.format
        };

        Ok(Self {
            input_file: input.file.clone(),
            post_process: processing.post_process,
            preset,
            print: output.print,
            output_path: output.output.clone(),
            format,
            duration: processing.duration,
            no_vad: processing.no_vad,
            language: processing.language.clone(),
        })
    }

    /// Check if output should be quiet (for clean stdout)
    pub fn is_quiet(&self) -> bool {
        self.print
    }
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
