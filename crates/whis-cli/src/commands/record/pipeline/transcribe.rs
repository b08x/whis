//! Transcription Pipeline Phase
//!
//! This module handles the conversion of recorded audio into text using the
//! configured transcription provider. It supports both cloud-based API providers
//! and local on-device transcription.
//!
//! # Provider Support
//!
//! - **Cloud Providers**: OpenAI, Mistral, Groq, Deepgram, ElevenLabs
//!   - Uses MP3-encoded audio for network efficiency
//!   - Supports chunked transcription for long audio
//!
//! - **Local Providers**: Whisper.cpp, Parakeet
//!   - Uses raw f32 samples directly
//!   - Sequential chunk processing with shared model cache
//!
//! # Processing Flow
//!
//! ```text
//! RecordResult
//!     │
//!     ├─ audio: RecordingOutput
//!     │   ├─ Single(Vec<u8>)  → transcribe_audio()
//!     │   └─ Chunked(Vec)     → batch_transcribe()
//!     │
//!     └─ raw_samples: Option<(Vec<f32>, u32)>
//!         └─ For local providers only
//! ```

use anyhow::Result;
use whis_core::{RecordingOutput, TranscriptionProvider, batch_transcribe, transcribe_audio};

use super::super::types::{RecordResult, TranscriptionResult};
use crate::app;

/// Transcription configuration
pub struct TranscriptionConfig {
    pub provider: TranscriptionProvider,
    pub api_key: String,
    pub language: Option<String>,
}

/// Execute transcription phase
pub async fn transcribe(
    record_result: RecordResult,
    config: &TranscriptionConfig,
    quiet: bool,
) -> Result<TranscriptionResult> {
    if !quiet {
        app::print_status(" Transcribing...", Some(&config.provider));
    }

    let text = match record_result.audio {
        RecordingOutput::Single(audio_data) => transcribe_audio(
            &config.provider,
            &config.api_key,
            config.language.as_deref(),
            audio_data,
        )?,
        RecordingOutput::Chunked(chunks) => {
            batch_transcribe(
                &config.provider,
                &config.api_key,
                config.language.as_deref(),
                chunks,
                None,
            )
            .await?
        }
    };

    // Print completion message immediately after transcription finishes
    if !quiet {
        println!(" Done.");
    }

    Ok(TranscriptionResult { text })
}
