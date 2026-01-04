//! Local transcription using Whisper via transcribe-rs
//!
//! This provider enables offline transcription without API calls.
//! Requires a whisper.cpp model file (e.g., ggml-small.bin).
//!
//! Uses engine-level caching to avoid reloading the model on every
//! transcription (saves 200ms-2s per call in listen mode).

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use super::{TranscriptionBackend, TranscriptionRequest, TranscriptionResult, TranscriptionStage};

/// Local Whisper transcription provider using transcribe-rs
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
        model_path: &str, // Path to .bin model file
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

/// Perform local transcription using transcribe-rs WhisperEngine
fn transcribe_local(
    model_path: &str,
    request: TranscriptionRequest,
) -> Result<TranscriptionResult> {
    // Report transcribing stage
    request.report(TranscriptionStage::Transcribing);

    // Decode MP3 to PCM and resample to 16kHz mono
    let pcm_samples = decode_and_resample(&request.audio_data)?;

    // Transcribe the samples
    transcribe_samples(model_path, &pcm_samples, request.language.as_deref())
}

/// Transcribe raw f32 samples directly (skips MP3 decoding).
///
/// Use this for local recordings where samples are already 16kHz mono.
/// This is faster than going through MP3 encoding/decoding.
///
/// # Arguments
/// * `model_path` - Path to the whisper.cpp model file (.bin)
/// * `samples` - Raw f32 audio samples (must be 16kHz mono)
/// * `language` - Optional language code (e.g., "en", "de")
pub fn transcribe_raw(
    model_path: &str,
    samples: &[f32],
    language: Option<&str>,
) -> Result<TranscriptionResult> {
    transcribe_samples(model_path, samples, language)
}

// ============================================================================
// Engine Caching (replaces model_manager.rs)
// ============================================================================

static WHISPER_ENGINE: OnceLock<Mutex<Option<CachedWhisperEngine>>> = OnceLock::new();
static KEEP_LOADED: AtomicBool = AtomicBool::new(false);

struct CachedWhisperEngine {
    engine: transcribe_rs::engines::whisper::WhisperEngine,
    path: String,
}

fn get_cache() -> &'static Mutex<Option<CachedWhisperEngine>> {
    WHISPER_ENGINE.get_or_init(|| Mutex::new(None))
}

/// Get or load the WhisperEngine, caching it for future use.
fn get_or_load_engine(model_path: &str) -> Result<()> {
    let mut cache = get_cache().lock().unwrap();

    // Check if already loaded with same path
    if let Some(ref cached) = *cache {
        if cached.path == model_path {
            return Ok(()); // Already loaded
        }
    }

    // Validate model path
    if model_path.is_empty() {
        anyhow::bail!(
            "Whisper model path not configured. Set LOCAL_WHISPER_MODEL_PATH or use: whis config --whisper-model-path <path>"
        );
    }

    if !Path::new(model_path).exists() {
        anyhow::bail!(
            "Whisper model not found at: {}\n\
             Download a model from: https://huggingface.co/ggerganov/whisper.cpp/tree/main",
            model_path
        );
    }

    crate::verbose!("Loading whisper model from: {}", model_path);

    // Create and load engine
    use transcribe_rs::TranscriptionEngine;
    let mut engine = transcribe_rs::engines::whisper::WhisperEngine::new();
    engine
        .load_model(Path::new(model_path))
        .map_err(|e| anyhow::anyhow!("Failed to load whisper model: {}", e))?;

    crate::verbose!("Whisper model loaded successfully");

    *cache = Some(CachedWhisperEngine {
        engine,
        path: model_path.to_string(),
    });

    Ok(())
}

/// Internal function to transcribe PCM samples using cached WhisperEngine
fn transcribe_samples(
    model_path: &str,
    samples: &[f32],
    language: Option<&str>,
) -> Result<TranscriptionResult> {
    use transcribe_rs::TranscriptionEngine;
    use transcribe_rs::engines::whisper::WhisperInferenceParams;

    // Get or load engine
    get_or_load_engine(model_path)?;

    // Perform transcription with locked access to engine
    let text = {
        let mut cache = get_cache().lock().unwrap();
        let cached = cache
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Engine not loaded"))?;

        // Configure inference parameters
        let params = WhisperInferenceParams {
            language: language.map(|s| s.to_string()),
            translate: false,
            print_special: false,
            print_progress: false,
            print_realtime: false,
            print_timestamps: false,
            suppress_blank: true,
            suppress_non_speech_tokens: true,
            no_speech_thold: 0.2,
            initial_prompt: None,
        };

        // Transcribe (transcribe-rs requires Vec<f32>, not &[f32])
        let result = cached
            .engine
            .transcribe_samples(samples.to_vec(), Some(params))
            .map_err(|e| anyhow::anyhow!("Transcription failed: {}", e))?;

        result.text
    };

    // Conditionally unload based on KEEP_LOADED flag
    maybe_unload();

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
    })
}

// ============================================================================
// Public API for model lifecycle management
// ============================================================================

/// Set whether to keep the model loaded after transcription.
///
/// When `true`, the model stays in memory for faster subsequent transcriptions.
/// When `false` (default), the model is unloaded after each use.
///
/// # Arguments
/// * `keep` - Whether to keep the model loaded
pub fn set_keep_loaded(keep: bool) {
    KEEP_LOADED.store(keep, Ordering::SeqCst);
    crate::verbose!("Whisper engine keep_loaded set to: {}", keep);
}

/// Check if models should be kept loaded.
pub fn should_keep_loaded() -> bool {
    KEEP_LOADED.load(Ordering::SeqCst)
}

/// Unload the cached model (if any).
///
/// This frees the memory used by the model. Call this when you're done
/// with transcription and don't expect more requests soon.
pub fn unload_model() {
    let mut cache = get_cache().lock().unwrap();
    if cache.is_some() {
        crate::verbose!("Unloading whisper engine from cache");
        *cache = None;
    }
}

/// Called after transcription to conditionally unload the model.
fn maybe_unload() {
    if !should_keep_loaded() {
        unload_model();
    }
}

/// Preload the whisper model in a background thread.
///
/// Call this when recording starts to overlap model loading with recording.
/// By the time recording finishes, the model should already be loaded.
///
/// # Arguments
/// * `path` - Path to the whisper model file (.bin)
pub fn preload_model(path: &str) {
    // Check if model is already loaded
    {
        let cache = get_cache().lock().unwrap();
        if let Some(ref cached) = *cache {
            if cached.path == path {
                crate::verbose!("Engine already cached, skipping preload");
                return;
            }
        }
    }

    let path = path.to_string();
    std::thread::spawn(move || {
        crate::verbose!("Preloading whisper model in background...");
        if let Err(e) = get_or_load_engine(&path) {
            crate::verbose!("Preload failed: {}", e);
        }
    });
}

// ============================================================================
// Audio Decoding
// ============================================================================

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
