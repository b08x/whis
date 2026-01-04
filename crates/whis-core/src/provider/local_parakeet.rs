//! Local transcription using NVIDIA Parakeet via ONNX
//!
//! This provider enables offline transcription using Parakeet models.
//! Requires a Parakeet model directory containing ONNX files.
//!
//! Parakeet models offer high accuracy and speed for speech-to-text.

use anyhow::{Context, Result};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

use super::{TranscriptionBackend, TranscriptionRequest, TranscriptionResult, TranscriptionStage};

/// Local Parakeet transcription provider
#[derive(Debug, Default, Clone)]
pub struct LocalParakeetProvider;

#[async_trait]
impl TranscriptionBackend for LocalParakeetProvider {
    fn name(&self) -> &'static str {
        "local-parakeet"
    }

    fn display_name(&self) -> &'static str {
        "Local Parakeet"
    }

    fn transcribe_sync(
        &self,
        model_path: &str, // Path to Parakeet model directory
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        transcribe_local(model_path, request)
    }

    async fn transcribe_async(
        &self,
        _client: &reqwest::Client, // Not used for local transcription
        model_path: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        // Run CPU-bound transcription in blocking task
        let model_path = model_path.to_string();
        tokio::task::spawn_blocking(move || transcribe_local(&model_path, request))
            .await
            .context("Task join failed")?
    }
}

/// Perform local transcription using parakeet-rs
fn transcribe_local(
    model_path: &str,
    request: TranscriptionRequest,
) -> Result<TranscriptionResult> {
    // Report transcribing stage
    request.report(TranscriptionStage::Transcribing);

    // Decode MP3 to PCM samples
    let pcm_samples = decode_mp3_to_samples(&request.audio_data)?;

    // Transcribe the samples
    transcribe_samples(model_path, pcm_samples)
}

/// Transcribe raw f32 samples directly.
///
/// Use this for local recordings where samples are already 16kHz mono.
///
/// # Arguments
/// * `model_path` - Path to the Parakeet model directory
/// * `samples` - Raw f32 audio samples (must be 16kHz mono)
pub fn transcribe_raw(model_path: &str, samples: Vec<f32>) -> Result<TranscriptionResult> {
    transcribe_samples(model_path, samples)
}

/// Internal function to transcribe PCM samples using Parakeet
///
/// ONNX Runtime has memory constraints with long audio in Parakeet models.
/// This function automatically chunks audio longer than 90 seconds to avoid ORT errors.
fn transcribe_samples(model_path: &str, samples: Vec<f32>) -> Result<TranscriptionResult> {
    use transcribe_rs::engines::parakeet::{ParakeetInferenceParams, TimestampGranularity};

    // Empirically tested: Parakeet works well up to ~90 seconds
    // Beyond that, ONNX Runtime can hit memory limits (ORT error)
    const CHUNK_SIZE: usize = 1_440_000; // 90 seconds at 16kHz
    const OVERLAP: usize = 16_000; // 1 second overlap for context at chunk boundaries

    // Get or load shared engine from global cache
    let engine_mutex = get_or_load_engine(model_path)?;
    let mut engine = engine_mutex.lock().unwrap();

    // Configure inference parameters
    let params = ParakeetInferenceParams {
        timestamp_granularity: TimestampGranularity::Segment,
    };

    // If audio is short, transcribe directly (no chunking needed)
    if samples.len() <= CHUNK_SIZE {
        return transcribe_chunk_with_engine(&mut engine, samples, &params);
    }

    // Split long audio into chunks with overlap
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < samples.len() {
        let end = (start + CHUNK_SIZE).min(samples.len());
        chunks.push(&samples[start..end]);
        start += CHUNK_SIZE - OVERLAP;
    }

    // Transcribe each chunk using the same engine instance
    let mut results = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        crate::verbose!(
            "Transcribing chunk {}/{} ({:.1}s)...",
            i + 1,
            chunks.len(),
            chunk.len() as f32 / 16000.0
        );

        let result = transcribe_chunk_with_engine(&mut engine, chunk.to_vec(), &params)?;
        results.push(result.text);
    }

    // Concatenate chunk results with space separator
    Ok(TranscriptionResult {
        text: results.join(" "),
    })
}

/// Transcribe a single chunk of audio using an already-loaded engine
///
/// This function is used internally by `transcribe_samples()` to reuse the same
/// engine instance across multiple chunks, avoiding repeated model loading.
fn transcribe_chunk_with_engine(
    engine: &mut transcribe_rs::engines::parakeet::ParakeetEngine,
    samples: Vec<f32>,
    params: &transcribe_rs::engines::parakeet::ParakeetInferenceParams,
) -> Result<TranscriptionResult> {
    use transcribe_rs::TranscriptionEngine;

    // Transcribe the audio samples using the pre-loaded engine
    // transcribe-rs expects 16kHz mono samples
    let result = engine
        .transcribe_samples(samples, Some(params.clone()))
        .map_err(|e| anyhow::anyhow!("Parakeet transcription failed: {}", e))?;

    Ok(TranscriptionResult {
        text: result.text.trim().to_string(),
    })
}

/// Decode MP3 audio data to f32 samples at 16kHz mono
fn decode_mp3_to_samples(mp3_data: &[u8]) -> Result<Vec<f32>> {
    use minimp3::{Decoder, Frame};

    let mut decoder = Decoder::new(mp3_data);
    let mut samples = Vec::new();
    let mut sample_rate = 0u32;
    let mut channels = 0u16;

    // Decode all MP3 frames
    loop {
        match decoder.next_frame() {
            Ok(Frame {
                data,
                sample_rate: sr,
                channels: ch,
                ..
            }) => {
                sample_rate = sr as u32;
                channels = ch as u16;
                // Convert i16 samples to f32 normalized to [-1.0, 1.0]
                samples.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
            }
            Err(minimp3::Error::Eof) => break,
            Err(e) => anyhow::bail!("MP3 decode error: {:?}", e),
        }
    }

    if samples.is_empty() {
        anyhow::bail!("No audio data decoded from MP3");
    }

    // Resample to 16kHz mono if needed
    crate::resample::resample_to_16k(&samples, sample_rate, channels)
}

/// Global shared Parakeet engine (loaded once, reused for all transcriptions)
static PARAKEET_ENGINE: OnceCell<Mutex<transcribe_rs::engines::parakeet::ParakeetEngine>> =
    OnceCell::new();

/// Get or load the shared Parakeet engine
///
/// This function ensures the model is loaded only once and then cached globally.
/// All subsequent calls reuse the same engine instance, reducing memory usage
/// and eliminating repeated model loading overhead.
fn get_or_load_engine(
    model_path: &str,
) -> Result<&'static Mutex<transcribe_rs::engines::parakeet::ParakeetEngine>> {
    use std::path::Path;
    use transcribe_rs::TranscriptionEngine;
    use transcribe_rs::engines::parakeet::{ParakeetEngine, ParakeetModelParams};

    PARAKEET_ENGINE.get_or_try_init(|| {
        crate::verbose!("Loading Parakeet model: {}", model_path);

        let mut engine = ParakeetEngine::new();
        engine
            .load_model_with_params(Path::new(model_path), ParakeetModelParams::int8())
            .map_err(|e| anyhow::anyhow!("Failed to load Parakeet model: {}", e))?;

        crate::verbose!("✓ Parakeet model loaded");
        Ok(Mutex::new(engine))
    })
}

/// Preload Parakeet model in background to reduce first-transcription latency
///
/// This function spawns a background thread that loads the Parakeet model
/// into memory. This reduces the latency when transcription actually starts,
/// as the model will already be loaded.
///
/// The preloaded model is cached in a static variable and reused for all
/// subsequent transcription calls.
///
/// # Arguments
/// * `model_path` - Path to the Parakeet model directory
///
/// # Example
/// ```no_run
/// use whis_core::preload_parakeet;
/// preload_parakeet("/path/to/parakeet/model");
/// // Model loads in background while recording...
/// ```
pub fn preload_parakeet(model_path: &str) {
    let model_path = model_path.to_string();
    std::thread::spawn(move || {
        crate::verbose!("Preloading Parakeet model: {}", model_path);

        // Load into shared static cache using get_or_load_engine
        if let Err(e) = get_or_load_engine(&model_path) {
            eprintln!("Warning: Failed to preload Parakeet model: {}", e);
            return;
        }

        crate::verbose!("✓ Parakeet model preloaded");
    });
}
