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

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            provider: crate::configuration::DEFAULT_PROVIDER,
            language: crate::configuration::DEFAULT_LANGUAGE.map(String::from),
            api_keys: HashMap::new(),
            local_models: LocalModelsConfig::default(),
        }
    }
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
        // Check api_keys map first (no env var fallback)
        if let Some(key) = self.api_key_from_settings_for(provider) {
            return Some(key);
        }

        // Fall back to environment variable
        std::env::var(provider.api_key_env_var()).ok()
    }

    /// Get the API key for the current provider from settings only (no env var fallback).
    ///
    /// Used by desktop app which doesn't support env var configuration.
    pub fn api_key_from_settings(&self) -> Option<String> {
        self.api_key_from_settings_for(&self.provider)
    }

    /// Get the API key for a specific provider from settings only (no env var fallback).
    pub fn api_key_from_settings_for(&self, provider: &TranscriptionProvider) -> Option<String> {
        self.api_keys
            .get(provider.api_key_name())
            .filter(|k| !k.is_empty())
            .cloned()
    }

    /// Check if an API key is explicitly configured in settings (not just in environment).
    ///
    /// Returns true only if the key exists in the api_keys HashMap.
    /// Use this to distinguish between:
    /// - [configured]: Key in settings.json
    /// - [available]: Key only in environment variable
    pub fn has_configured_api_key(&self, provider: &TranscriptionProvider) -> bool {
        self.api_keys
            .get(provider.api_key_name())
            .map(|k| !k.is_empty())
            .unwrap_or(false)
    }

    /// Set the API key for a provider.
    pub fn set_api_key(&mut self, provider: &TranscriptionProvider, key: String) {
        self.api_keys.insert(provider.api_key_name().to_string(), key);
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
