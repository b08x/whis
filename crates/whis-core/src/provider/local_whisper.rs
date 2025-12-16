//! Local transcription using whisper.cpp via whisper-rs
//!
//! This provider enables offline transcription without API calls.
//! Requires a whisper.cpp model file (e.g., ggml-small.bin).

use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{TranscriptionBackend, TranscriptionRequest, TranscriptionResult};

/// Local whisper.cpp transcription provider
#[derive(Debug, Default, Clone)]
pub struct LocalWhisperProvider;

#[async_trait]
impl TranscriptionBackend for LocalWhisperProvider {
    fn name(&self) -> &'static str {
        "local-whisper"
    }

    fn display_name(&self) -> &'static str {
        "Local Whisper"
    }

    fn transcribe_sync(
        &self,
        model_path: &str, // Repurposed: path to .bin model file
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

/// Perform local transcription using whisper-rs
fn transcribe_local(
    model_path: &str,
    request: TranscriptionRequest,
) -> Result<TranscriptionResult> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    // Suppress verbose whisper.cpp logging
    whisper_rs::install_logging_hooks();

    // Validate model path
    if model_path.is_empty() {
        anyhow::bail!(
            "Whisper model path not configured. Set LOCAL_WHISPER_MODEL_PATH or use: whis config --whisper-model-path <path>"
        );
    }

    if !std::path::Path::new(model_path).exists() {
        anyhow::bail!(
            "Whisper model not found at: {}\n\
             Download a model from: https://huggingface.co/ggerganov/whisper.cpp/tree/main",
            model_path
        );
    }

    // Load model
    // Note: For better performance, this should be cached globally
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .context("Failed to load whisper model")?;

    // Create state
    let mut state = ctx
        .create_state()
        .context("Failed to create whisper state")?;

    // Configure parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // Set language if provided
    if let Some(ref lang) = request.language {
        params.set_language(Some(lang));
    }

    // Disable printing to stdout
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Decode MP3 to PCM and resample to 16kHz mono
    let pcm_samples = decode_and_resample(&request.audio_data)?;

    // Run transcription
    state
        .full(params, &pcm_samples)
        .context("Transcription failed")?;

    // Extract text from segments
    let num_segments = state.full_n_segments();

    let mut text = String::new();
    for i in 0..num_segments {
        if let Some(segment) = state.get_segment(i)
            && let Ok(segment_text) = segment.to_str()
        {
            text.push_str(segment_text);
        }
    }

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
    })
}

/// Decode MP3 audio data and resample to 16kHz mono for whisper
fn decode_and_resample(mp3_data: &[u8]) -> Result<Vec<f32>> {
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

    // Resample to 16kHz mono
    crate::resample::resample_to_16k(&samples, sample_rate, channels)
}
