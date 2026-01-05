//! Record Command - Voice-to-Text Pipeline
//!
//! This module implements the main recording/transcription workflow using a clean
//! pipeline architecture. It handles audio input from multiple sources, transcribes
//! it using configured providers, optionally post-processes the text, and outputs
//! the result.
//!
//! # Architecture
//!
//! The record command follows a four-phase pipeline:
//!
//! ```text
//! ┌─────────────┐      ┌──────────────┐      ┌─────────────┐      ┌────────────┐
//! │   Record    │  →   │  Transcribe  │  →   │   Process   │  →   │   Output   │
//! │  (modes/)   │      │ (pipeline/)  │      │ (pipeline/) │      │(pipeline/) │
//! └─────────────┘      └──────────────┘      └─────────────┘      └────────────┘
//!     ↓                      ↓                     ↓                    ↓
//! Microphone          Cloud/Local API         LLM Cleanup         Clipboard/
//! File Input          Parallel Chunks         Preset Transform    Stdout
//! Stdin Stream        Language Detection
//! ```
//!
//! # Pipeline Phases
//!
//! 1. **Record Phase** (`modes/`): Capture or load audio
//!    - `MicrophoneMode`: Record from system microphone with VAD
//!    - `FileMode`: Load and decode audio file
//!    - `StdinMode`: Stream audio from stdin
//!
//! 2. **Transcribe Phase** (`pipeline/transcribe.rs`): Convert audio to text
//!    - Cloud providers: Single or chunked API calls
//!    - Local providers: Raw samples with progressive transcription
//!
//! 3. **Process Phase** (`pipeline/process.rs`): Enhance transcript
//!    - Apply LLM post-processing (grammar, filler words)
//!    - Transform with output presets
//!
//! 4. **Output Phase** (`pipeline/output.rs`): Deliver result
//!    - Copy to clipboard (default)
//!    - Print to stdout (--print flag)
//!
//! # Execution Paths
//!
//! ## Progressive Mode (Microphone Only)
//! - Recording and transcription happen CONCURRENTLY
//! - 90-second chunks sent to API as they arrive
//! - Faster perceived latency (overlapped phases)
//!
//! ## Batch Mode (File/Stdin)
//! - Recording completes first, then transcription begins
//! - Sequential phases: Record → Transcribe → Process → Output
//!
//! # Usage
//!
//! ```rust
//! // Default: Record from microphone, copy to clipboard
//! commands::record::run(false, None, None, false, "mp3", false, None, false, None)?;
//!
//! // Transcribe file with post-processing
//! commands::record::run(
//!     true,                             // post_process
//!     None,                             // preset_name
//!     Some(PathBuf::from("audio.mp3")), // file_path
//!     false,                            // stdin_mode
//!     "mp3",                            // input_format
//!     false,                            // print
//!     None,                             // duration
//!     false,                            // no_vad
//!     None,                             // save_raw
//! )?;
//! ```
//!
//! # Configuration
//!
//! The record command respects user settings from `~/.config/whis/config.toml`:
//! - Transcription provider and API keys
//! - Post-processing preferences
//! - VAD settings and hotkeys
//! - Clipboard method

mod modes;
mod pipeline;
mod types;

// Re-export public types for external use
pub use types::{InputSource, RecordConfig};

use anyhow::Result;
use std::path::PathBuf;

use crate::app;

/// Execute the record command with clean pipeline phases
pub fn run(config: RecordConfig) -> Result<()> {
    let quiet = config.is_quiet();

    // Create Tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()?;

    // Ensure FFmpeg is available
    app::ensure_ffmpeg_installed()?;

    // Load transcription configuration
    let transcription_config = app::load_transcription_config()?;

    // Microphone always uses progressive (record + transcribe concurrently)
    // File/stdin use batch (record fully, then transcribe)
    let use_progressive = matches!(config.input_source, InputSource::Microphone);

    let transcription_result = if use_progressive {
        // Progressive path: Record and transcribe concurrently (overlap recording with transcription)
        let mic_config = modes::MicrophoneConfig {
            duration: config.duration,
            no_vad: config.no_vad,
            provider: transcription_config.provider.clone(),
            will_post_process: config.post_process || config.preset.is_some(),
        };
        runtime.block_on(progressive_record_and_transcribe(
            mic_config,
            &transcription_config,
            quiet,
        ))?
    } else {
        // Batch path: Record first, then transcribe (for pre-recorded audio)
        // Note: Microphone always uses progressive path (see line 119)
        let record_result = match config.input_source {
            InputSource::File(path) => {
                let mode = modes::FileMode::new(path);
                mode.execute(quiet)?
            }
            InputSource::Stdin { format } => {
                let mode = modes::StdinMode::new(format);
                mode.execute(quiet)?
            }
            InputSource::Microphone => {
                unreachable!("Microphone input always uses progressive transcription path")
            }
        };

        // Save raw samples if requested
        if let Some(save_path) = &config.save_raw
            && let Some((samples, sample_rate)) = &record_result.raw_samples
        {
            save_raw_samples_as_wav(samples, *sample_rate, save_path)?;
            if !quiet {
                eprintln!("✓ Saved raw audio to: {}", save_path.display());
            }
        }

        // Phase 2: Transcribe audio to text
        let transcription_cfg = pipeline::TranscriptionConfig {
            provider: transcription_config.provider,
            api_key: transcription_config.api_key,
            language: transcription_config.language,
        };
        runtime.block_on(pipeline::transcribe(
            record_result,
            &transcription_cfg,
            quiet,
        ))?
    };

    // Phase 3: Post-process and apply presets
    let processing_cfg = pipeline::ProcessingConfig {
        enabled: config.post_process,
        preset: config.preset,
    };
    let processed_result = runtime.block_on(pipeline::process(
        transcription_result,
        &processing_cfg,
        quiet,
    ))?;

    // Phase 4: Output (print or clipboard)
    let output_mode = if config.print {
        pipeline::OutputMode::Print
    } else {
        pipeline::OutputMode::Clipboard
    };
    pipeline::output(processed_result, output_mode, quiet)?;

    Ok(())
}

/// Progressive recording + transcription (combines recording and transcription phases)
///
/// This function overlaps recording and transcription using the progressive
/// architecture. Audio is chunked during recording and transcribed immediately
/// (cloud providers transcribe chunks in parallel, local providers sequentially).
async fn progressive_record_and_transcribe(
    mic_config: modes::MicrophoneConfig,
    transcription_config: &app::TranscriptionConfig,
    quiet: bool,
) -> Result<types::TranscriptionResult> {
    use tokio::sync::mpsc;
    #[cfg(feature = "local-transcription")]
    use whis_core::progressive_transcribe_local;
    use whis_core::{
        AudioRecorder, ChunkerConfig, ProgressiveChunker, Settings, TranscriptionProvider,
        progressive_transcribe_cloud,
    };

    // Create recorder
    let mut recorder = AudioRecorder::new()?;

    // Configure VAD
    let settings = Settings::load();
    let vad_enabled = settings.ui.vad.enabled && !mic_config.no_vad;
    recorder.set_vad(vad_enabled, settings.ui.vad.threshold);

    // Preload models in background (same as batch mode)
    preload_models(&mic_config);

    // Start streaming recording
    let mut audio_rx_bounded = recorder.start_recording_streaming()?;

    // Create unbounded channel for chunker (adapter pattern)
    let (audio_tx_unbounded, audio_rx_unbounded) = mpsc::unbounded_channel();

    // Spawn adapter task to forward from bounded to unbounded channel
    tokio::spawn(async move {
        while let Some(samples) = audio_rx_bounded.recv().await {
            if audio_tx_unbounded.send(samples).is_err() {
                break; // Receiver dropped
            }
        }
    });

    // Create channels for progressive chunking
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    // Create chunker config
    let chunker_config = ChunkerConfig {
        target_duration_secs: 90,
        min_duration_secs: 60,
        max_duration_secs: 120,
        vad_aware: vad_enabled,
    };

    // Spawn chunker task
    let mut chunker = ProgressiveChunker::new(chunker_config, chunk_tx);
    let chunker_task = tokio::spawn(async move {
        // Note: VAD state streaming not yet implemented, using fixed-duration chunking
        chunker
            .consume_stream(audio_rx_unbounded, None)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    });

    // Spawn transcription task based on provider
    let transcription_task = {
        let provider = transcription_config.provider.clone();
        let api_key = transcription_config.api_key.clone();
        let language = transcription_config.language.clone();

        tokio::spawn(async move {
            #[cfg(feature = "local-transcription")]
            if provider == TranscriptionProvider::LocalParakeet {
                // Local Parakeet progressive transcription
                let model_path = Settings::load()
                    .transcription
                    .parakeet_model_path()
                    .ok_or_else(|| anyhow::anyhow!("Parakeet model path not configured"))?;

                return progressive_transcribe_local(&model_path, chunk_rx, None).await;
            }

            // Cloud provider progressive transcription
            progressive_transcribe_cloud(&provider, &api_key, language.as_deref(), chunk_rx, None)
                .await
        })
    };

    // Wait for recording to complete (user input or duration)
    if let Some(dur) = mic_config.duration {
        // Timed recording
        if !quiet {
            println!("Recording for {} seconds...", dur.as_secs());
        }
        tokio::time::sleep(dur).await;
    } else {
        // Interactive mode
        if !quiet {
            let hotkey = &settings.ui.shortcut;
            println!("Press Enter or {} to stop", hotkey);
            print!("Recording...");
            use std::io::Write;
            std::io::stdout().flush()?;
        }

        // Wait for user to stop (blocking operation, run in spawn_blocking)
        let hotkey = settings.ui.shortcut.clone();
        tokio::task::spawn_blocking(move || app::wait_for_stop(&hotkey)).await??;

        if !quiet && whis_core::verbose::is_verbose() {
            println!();
        }
    }

    // Stop recording (closes audio stream, signals chunker to finish)
    recorder.stop_recording()?;

    // Wait for chunker to finish
    chunker_task.await??;

    // Wait for transcription to finish
    if !quiet {
        app::print_status(" Transcribing...", Some(&transcription_config.provider));
    }

    let text = transcription_task.await??;

    // Print completion message immediately after transcription finishes
    if !quiet {
        println!(" Done.");
    }

    Ok(types::TranscriptionResult { text })
}

/// Preload models in background to reduce latency (extracted from MicrophoneMode)
fn preload_models(config: &modes::MicrophoneConfig) {
    #[cfg(feature = "local-transcription")]
    {
        let settings = whis_core::Settings::load();

        // Preload the configured local model (Whisper OR Parakeet, not both)
        match config.provider {
            whis_core::TranscriptionProvider::LocalWhisper => {
                if let Some(model_path) = settings.transcription.whisper_model_path() {
                    whis_core::whisper_preload_model(&model_path);
                }
            }
            whis_core::TranscriptionProvider::LocalParakeet => {
                if let Some(model_path) = settings.transcription.parakeet_model_path() {
                    whis_core::preload_parakeet(&model_path);
                }
            }
            _ => {} // Cloud providers don't need preload
        }
    }

    // Preload Ollama if post-processing enabled
    if config.will_post_process {
        let settings = whis_core::Settings::load();
        if settings.post_processing.processor == whis_core::PostProcessor::Ollama
            && let (Some(url), Some(model)) = (
                settings.services.ollama.url(),
                settings.services.ollama.model(),
            )
        {
            whis_core::preload_ollama(&url, &model);
        }
    }
}

/// Save raw audio samples as WAV file
fn save_raw_samples_as_wav(samples: &[f32], sample_rate: u32, path: &PathBuf) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)?;

    for &sample in samples {
        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    Ok(())
}
