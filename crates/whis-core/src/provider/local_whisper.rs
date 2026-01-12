//! Local transcription using Whisper via transcribe-rs
//!
//! This provider enables offline transcription without API calls.
//! Requires a whisper.cpp model file (e.g., ggml-small.bin).
//!
//! Uses engine-level caching to avoid reloading the model on every
//! transcription (saves 200ms-2s per call in listen mode).

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use super::{TranscriptionBackend, TranscriptionRequest, TranscriptionResult};

// ============================================================================
// stderr Suppression for GGML Vulkan Output
// ============================================================================

/// Temporarily suppresses stderr during closure execution.
/// Used to suppress GGML Vulkan device detection lines that bypass logging callbacks.
/// whisper-rs-sys 0.11.x (used by transcribe-rs 0.2.1) writes these directly to std::cerr.
#[cfg(feature = "local-transcription")]
mod stderr_suppression {
    /// RAII guard that restores stderr when dropped
    #[cfg(unix)]
    pub struct StderrGuard {
        saved_fd: i32,
        stderr_fd: i32,
    }

    #[cfg(unix)]
    impl Drop for StderrGuard {
        fn drop(&mut self) {
            unsafe {
                libc::dup2(self.saved_fd, self.stderr_fd);
                libc::close(self.saved_fd);
            }
        }
    }

    /// Suppress stderr by redirecting it to /dev/null.
    /// Returns a guard that restores stderr when dropped.
    #[cfg(unix)]
    pub fn suppress() -> Option<StderrGuard> {
        use std::os::unix::io::AsRawFd;

        let stderr_fd = std::io::stderr().as_raw_fd();
        let saved_fd = unsafe { libc::dup(stderr_fd) };
        if saved_fd == -1 {
            return None;
        }

        // Open /dev/null and redirect stderr to it
        let devnull = std::fs::File::open("/dev/null").ok()?;
        let result = unsafe { libc::dup2(devnull.as_raw_fd(), stderr_fd) };
        if result == -1 {
            unsafe { libc::close(saved_fd) };
            return None;
        }

        Some(StderrGuard {
            saved_fd,
            stderr_fd,
        })
    }

    // Windows implementation
    #[cfg(windows)]
    pub struct StderrGuard {
        saved_handle: *mut std::ffi::c_void,
    }

    #[cfg(windows)]
    impl Drop for StderrGuard {
        fn drop(&mut self) {
            const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4; // -12 as u32
            extern "system" {
                fn SetStdHandle(nStdHandle: u32, hHandle: *mut std::ffi::c_void) -> i32;
            }
            unsafe {
                SetStdHandle(STD_ERROR_HANDLE, self.saved_handle);
            }
        }
    }

    #[cfg(windows)]
    pub fn suppress() -> Option<StderrGuard> {
        use std::os::windows::io::AsRawHandle;
        const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4; // -12 as u32

        extern "system" {
            fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
            fn SetStdHandle(nStdHandle: u32, hHandle: *mut std::ffi::c_void) -> i32;
        }

        let saved_handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) };

        // Open NUL device
        let nul = std::fs::OpenOptions::new().write(true).open("NUL").ok()?;

        let result = unsafe { SetStdHandle(STD_ERROR_HANDLE, nul.as_raw_handle() as *mut _) };
        if result == 0 {
            return None;
        }

        // Keep nul open by leaking it (will be restored when guard drops)
        std::mem::forget(nul);

        Some(StderrGuard { saved_handle })
    }
}

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
        _model_path: &str,
        _request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        anyhow::bail!(
            "File transcription not supported. Use microphone recording with transcribe_raw()."
        )
    }

    async fn transcribe_async(
        &self,
        _client: &reqwest::Client,
        _model_path: &str,
        _request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        anyhow::bail!(
            "File transcription not supported. Use microphone recording with transcribe_raw()."
        )
    }
}

/// Transcribe raw f32 samples directly.
///
/// Use this for local recordings where samples are already 16kHz mono.
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
    if let Some(ref cached) = *cache
        && cached.path == model_path
    {
        return Ok(()); // Already loaded
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

    // Suppress stderr during model loading to hide whisper.cpp noise.
    // The guard automatically restores stderr when dropped (RAII pattern).
    let _stderr_guard = stderr_suppression::suppress();

    engine
        .load_model(Path::new(model_path))
        .map_err(|e| anyhow::anyhow!("Failed to load whisper model: {}", e))?;

    // Explicitly drop the guard to restore stderr before any subsequent logging
    drop(_stderr_guard);

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

        // Suppress stderr during transcription to hide whisper.cpp noise
        let _stderr_guard = stderr_suppression::suppress();

        // Transcribe (transcribe-rs requires Vec<f32>, not &[f32])
        let result = cached
            .engine
            .transcribe_samples(samples.to_vec(), Some(params))
            .map_err(|e| anyhow::anyhow!("Transcription failed: {}", e))?;

        drop(_stderr_guard);

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
        if let Some(ref cached) = *cache
            && cached.path == path
        {
            crate::verbose!("Engine already cached, skipping preload");
            return;
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
