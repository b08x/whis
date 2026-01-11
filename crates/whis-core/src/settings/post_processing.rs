//! Post-processing settings for LLM-based transcript cleanup.

use serde::{Deserialize, Serialize};

use crate::config::TranscriptionProvider;
use crate::post_processing::PostProcessor;

/// Settings for post-processing transcripts with LLMs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingSettings {
    /// Whether post-processing is enabled
    #[serde(default)]
    pub enabled: bool,

    /// LLM provider for post-processing (grammar, punctuation, filler word removal)
    #[serde(default = "default_processor")]
    pub processor: PostProcessor,

    /// Custom prompt for post-processing (uses default if None)
    #[serde(default)]
    pub prompt: Option<String>,
}

fn default_processor() -> PostProcessor {
    crate::configuration::DEFAULT_POST_PROCESSOR
}

impl Default for PostProcessingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            processor: crate::configuration::DEFAULT_POST_PROCESSOR,
            prompt: Some(crate::transcription::DEFAULT_POST_PROCESSING_PROMPT.to_string()),
        }
    }
}

impl PostProcessingSettings {
    /// Get the API key for the post-processor, falling back to environment variables.
    ///
    /// Returns None for local post-processor (Ollama uses URL instead).
    pub fn api_key(
        &self,
        transcription_api_keys: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        // Check settings first (no env var fallback)
        if let Some(key) = self.api_key_from_settings(transcription_api_keys) {
            return Some(key);
        }

        // Fall back to environment variable
        match &self.processor {
            PostProcessor::None | PostProcessor::Ollama => None,
            PostProcessor::OpenAI => {
                std::env::var(TranscriptionProvider::OpenAI.api_key_env_var()).ok()
            }
            PostProcessor::Mistral => {
                std::env::var(TranscriptionProvider::Mistral.api_key_env_var()).ok()
            }
        }
    }

    /// Get the API key for the post-processor from settings only (no env var fallback).
    ///
    /// Used by desktop app which doesn't support env var configuration.
    pub fn api_key_from_settings(
        &self,
        transcription_api_keys: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        match &self.processor {
            PostProcessor::None | PostProcessor::Ollama => None,
            PostProcessor::OpenAI => transcription_api_keys.get("openai").cloned(),
            PostProcessor::Mistral => transcription_api_keys.get("mistral").cloned(),
        }
    }

    /// Check if post-processing is enabled and properly configured.
    pub fn is_configured(
        &self,
        transcription_api_keys: &std::collections::HashMap<String, String>,
    ) -> bool {
        match &self.processor {
            PostProcessor::None => true,   // No post-processing always valid
            PostProcessor::Ollama => true, // Ollama URL checked in services
            PostProcessor::OpenAI | PostProcessor::Mistral => {
                self.api_key(transcription_api_keys).is_some()
            }
        }
    }

    /// Validate post-processing settings.
    pub fn validate(
        &self,
        transcription_api_keys: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<()> {
        if !self.is_configured(transcription_api_keys) {
            anyhow::bail!(
                "Post-processor '{}' requires an API key. Please configure it.",
                match self.processor {
                    PostProcessor::OpenAI => "OpenAI",
                    PostProcessor::Mistral => "Mistral",
                    _ => "unknown",
                }
            );
        }
        Ok(())
    }
}
