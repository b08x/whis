//! Transcription Provider Module
//!
//! This module provides an extensible architecture for adding new transcription providers.
//! All providers implement the `TranscriptionBackend` trait.
//!
//! # Architecture
//!
//! ```text
//! Provider System
//!   ├── Registry     - Provider lookup and lifecycle
//!   ├── Base         - Shared HTTP logic (OpenAI-compatible APIs)
//!   └── Providers    - Individual provider implementations
//!       ├── Cloud    - OpenAI, Mistral, Groq, Deepgram, ElevenLabs
//!       └── Local    - Whisper, Parakeet
//! ```
//!
//! # Provider Types
//!
//! **Cloud Providers** (OpenAI-compatible format):
//! - OpenAI Whisper API
//! - Groq Whisper API
//! - Mistral Voxtral API
//!
//! **Cloud Providers** (Custom format):
//! - Deepgram Nova API
//! - ElevenLabs API
//!
//! **Local Providers** (No API key required):
//! - Local Whisper (via transcribe-rs)
//! - Local Parakeet (via transcribe-rs)
//!
//! # Adding a New Provider
//!
//! 1. Create a new file in `provider/` (e.g., `myprovider.rs`)
//! 2. Implement `TranscriptionBackend` trait
//! 3. Add variant to `TranscriptionProvider` enum in `config.rs`
//! 4. Register in `ProviderRegistry::new()`
//! 5. Re-export in this module's public API
//!
//! For OpenAI-compatible APIs, use the shared helpers from `base::openai_compatible`.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Stages of the transcription workflow for progress reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptionStage {
    /// Recording audio from microphone
    Recording,
    /// Encoding audio to MP3
    Encoding,
    /// Uploading audio to cloud provider
    Uploading,
    /// Transcribing audio (cloud processing or local inference)
    Transcribing,
    /// Post-processing transcript with LLM
    PostProcessing,
    /// Transcription complete
    Complete,
}

impl TranscriptionStage {
    /// Get a human-readable status message for this stage
    pub fn message(&self) -> &'static str {
        match self {
            Self::Recording => "Recording...",
            Self::Encoding => "Encoding...",
            Self::Uploading => "Uploading...",
            Self::Transcribing => "Transcribing...",
            Self::PostProcessing => "Post-processing...",
            Self::Complete => "Done!",
        }
    }
}

/// Progress callback type for reporting transcription stages
pub type ProgressCallback = Arc<dyn Fn(TranscriptionStage) + Send + Sync>;

mod base;
mod deepgram;
mod elevenlabs;
pub mod error;
mod groq;
#[cfg(feature = "local-transcription")]
mod local_parakeet;
#[cfg(feature = "local-transcription")]
pub mod local_whisper;
mod mistral;
mod openai;
#[cfg(feature = "realtime")]
mod openai_realtime;

/// Default timeout for API requests (5 minutes)
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

pub use deepgram::DeepgramProvider;
pub use error::ProviderError;
pub use elevenlabs::ElevenLabsProvider;
pub use groq::GroqProvider;
#[cfg(feature = "local-transcription")]
pub use local_parakeet::LocalParakeetProvider;
#[cfg(feature = "local-transcription")]
pub use local_parakeet::transcribe_raw as transcribe_raw_parakeet;
#[cfg(feature = "local-transcription")]
pub use local_whisper::LocalWhisperProvider;
#[cfg(feature = "local-transcription")]
pub use local_whisper::transcribe_raw;
#[cfg(feature = "local-transcription")]
pub use local_whisper::{
    set_keep_loaded as whisper_set_keep_loaded,
    preload_model as whisper_preload_model,
    unload_model as whisper_unload_model,
};
pub use mistral::MistralProvider;
pub use openai::OpenAIProvider;
#[cfg(feature = "realtime")]
pub use openai_realtime::OpenAIRealtimeProvider;

use crate::config::TranscriptionProvider;

/// Request data for transcription
#[derive(Clone)]
pub struct TranscriptionRequest {
    pub audio_data: Vec<u8>,
    pub language: Option<String>,
    pub filename: String,
    pub mime_type: String,
    /// Optional progress callback for status updates
    pub progress: Option<ProgressCallback>,
}

impl TranscriptionRequest {
    /// Create a new request without progress callback
    pub fn new(audio_data: Vec<u8>, language: Option<String>) -> Self {
        Self {
            audio_data,
            language,
            filename: "audio.mp3".to_string(),
            mime_type: "audio/mpeg".to_string(),
            progress: None,
        }
    }

    /// Set the progress callback
    pub fn with_progress(mut self, callback: ProgressCallback) -> Self {
        self.progress = Some(callback);
        self
    }

    /// Report progress if callback is set
    pub fn report(&self, stage: TranscriptionStage) {
        if let Some(cb) = &self.progress {
            cb(stage);
        }
    }
}

/// Result of a transcription
pub struct TranscriptionResult {
    pub text: String,
}

// Import shared helpers from base module
pub(crate) use base::{openai_compatible_transcribe_async, openai_compatible_transcribe_sync};

/// Trait for transcription providers
///
/// Implement this trait to add a new transcription provider.
#[async_trait]
pub trait TranscriptionBackend: Send + Sync {
    /// Unique identifier for this provider (e.g., "openai", "deepgram")
    fn name(&self) -> &'static str;

    /// Display name for UI (e.g., "OpenAI Whisper", "Deepgram Nova")
    fn display_name(&self) -> &'static str;

    /// Synchronous transcription (for simple single-file case)
    fn transcribe_sync(
        &self,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult>;

    /// Async transcription for parallel chunk processing
    async fn transcribe_async(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult>;
}

/// Registry of all available transcription providers
pub struct ProviderRegistry {
    providers: HashMap<&'static str, Arc<dyn TranscriptionBackend>>,
}

impl ProviderRegistry {
    /// Create registry with all built-in providers
    pub fn new() -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn TranscriptionBackend>> = HashMap::new();

        providers.insert("openai", Arc::new(OpenAIProvider));
        #[cfg(feature = "realtime")]
        providers.insert("openai-realtime", Arc::new(OpenAIRealtimeProvider));
        providers.insert("mistral", Arc::new(MistralProvider));
        providers.insert("groq", Arc::new(GroqProvider));
        providers.insert("deepgram", Arc::new(DeepgramProvider));
        providers.insert("elevenlabs", Arc::new(ElevenLabsProvider));
        #[cfg(feature = "local-transcription")]
        providers.insert("local-whisper", Arc::new(LocalWhisperProvider));
        #[cfg(feature = "local-transcription")]
        providers.insert("local-parakeet", Arc::new(LocalParakeetProvider));

        Self { providers }
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn TranscriptionBackend>> {
        self.providers.get(name).cloned()
    }

    /// List all provider names
    pub fn list(&self) -> Vec<&'static str> {
        self.providers.keys().copied().collect()
    }

    /// Get provider for a TranscriptionProvider enum value
    ///
    /// Returns an error if the provider is not registered (should never happen
    /// if all enum variants have corresponding providers in the registry).
    pub fn get_by_kind(
        &self,
        kind: &TranscriptionProvider,
    ) -> Result<Arc<dyn TranscriptionBackend>> {
        self.get(kind.as_str()).ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' not found in registry. This is a bug - \
                 all TranscriptionProvider variants must have registered providers.",
                kind.as_str()
            )
        })
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global provider registry
pub fn registry() -> &'static ProviderRegistry {
    static REGISTRY: OnceLock<ProviderRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ProviderRegistry::new)
}
