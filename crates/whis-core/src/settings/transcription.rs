//! Transcription settings for provider configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::TranscriptionProvider;

#[cfg(feature = "local-transcription")]
use crate::model::{ModelType, ParakeetModel};

/// Settings for transcription providers and models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSettings {
    /// Active transcription provider
    #[serde(default)]
    pub provider: TranscriptionProvider,

    /// Language hint for transcription (ISO-639-1 code, e.g., "en", "de", "fr")
    /// None = auto-detect, Some("en") = English, etc.
    #[serde(default)]
    pub language: Option<String>,

    /// API keys stored by provider name (e.g., "openai" -> "sk-...")
    #[serde(default)]
    pub api_keys: HashMap<String, String>,

    /// Local model configuration
    #[serde(default)]
    pub local_models: LocalModelsConfig,
}

/// Configuration for local transcription models.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalModelsConfig {
    /// Path to whisper.cpp model file for local transcription
    /// (e.g., ~/.local/share/whis/models/ggml-small.bin)
    #[serde(default)]
    pub whisper_path: Option<String>,

    /// Path to Parakeet model directory for local transcription
    /// (e.g., ~/.local/share/whis/models/parakeet/parakeet-tdt-0.6b-v3-int8)
    #[serde(default)]
    pub parakeet_path: Option<String>,

    /// Number of parallel workers for Parakeet progressive transcription
    /// (1 = sequential ~2GB RAM, 2 = parallel ~4GB, 3 = parallel ~6GB)
    #[serde(default = "default_parakeet_progressive_workers")]
    pub parakeet_progressive_workers: usize,
}

fn default_parakeet_progressive_workers() -> usize {
    1 // Conservative default: single worker, minimal memory usage (~2GB)
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            provider: TranscriptionProvider::default(),
            language: None,
            api_keys: HashMap::new(),
            local_models: LocalModelsConfig::default(),
        }
    }
}

impl TranscriptionSettings {
    /// Get the API key for the current provider, falling back to environment variables.
    pub fn api_key(&self) -> Option<String> {
        self.api_key_for(&self.provider)
    }

    /// Get the API key for a specific provider.
    ///
    /// Checks in order:
    /// 1. api_keys map
    /// 2. Environment variable
    pub fn api_key_for(&self, provider: &TranscriptionProvider) -> Option<String> {
        // Normalize provider for API key lookup (OpenAI Realtime uses same key as OpenAI)
        let key_provider = match provider {
            TranscriptionProvider::OpenAIRealtime => "openai",
            _ => provider.as_str(),
        };

        // Check api_keys map first
        if let Some(key) = self.api_keys.get(key_provider)
            && !key.is_empty()
        {
            return Some(key.clone());
        }

        // Fall back to environment variable
        std::env::var(provider.api_key_env_var()).ok()
    }

    /// Set the API key for a provider.
    pub fn set_api_key(&mut self, provider: &TranscriptionProvider, key: String) {
        self.api_keys.insert(provider.as_str().to_string(), key);
    }

    /// Check if an API key is configured for the current provider.
    pub fn has_api_key(&self) -> bool {
        self.api_key().is_some()
    }

    /// Check if the current provider is properly configured.
    ///
    /// For cloud providers: checks for API key
    /// For LocalWhisper: checks for model path AND that file exists
    /// For LocalParakeet: checks for model directory AND it's valid
    pub fn is_configured(&self) -> bool {
        match self.provider {
            TranscriptionProvider::LocalWhisper => self
                .whisper_model_path()
                .map(|p| std::path::Path::new(&p).exists())
                .unwrap_or(false),
            #[cfg(feature = "local-transcription")]
            TranscriptionProvider::LocalParakeet => self
                .parakeet_model_path()
                .map(|p| ParakeetModel.verify(std::path::Path::new(&p)))
                .unwrap_or(false),
            #[cfg(not(feature = "local-transcription"))]
            TranscriptionProvider::LocalParakeet => false,
            _ => self.has_api_key(),
        }
    }

    /// Get the whisper model path, falling back to environment variable.
    pub fn whisper_model_path(&self) -> Option<String> {
        self.local_models
            .whisper_path
            .clone()
            .or_else(|| std::env::var("LOCAL_WHISPER_MODEL_PATH").ok())
    }

    /// Get the Parakeet model path, falling back to environment variable.
    pub fn parakeet_model_path(&self) -> Option<String> {
        self.local_models
            .parakeet_path
            .clone()
            .or_else(|| std::env::var("LOCAL_PARAKEET_MODEL_PATH").ok())
    }

    /// Validate transcription settings.
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.is_configured() {
            anyhow::bail!(
                "Provider '{}' is not configured. Please run setup.",
                self.provider
            );
        }
        Ok(())
    }
}
