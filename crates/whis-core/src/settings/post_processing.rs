//! Post-processing settings for LLM-based transcript cleanup.

use serde::{Deserialize, Serialize};

use crate::config::TranscriptionProvider;
use crate::post_processing::PostProcessor;

/// Settings for post-processing transcripts with LLMs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingSettings {
    /// LLM provider for post-processing (grammar, punctuation, filler word removal)
    #[serde(default)]
    pub processor: PostProcessor,

    /// Custom prompt for post-processing (uses default if None)
    #[serde(default)]
    pub prompt: Option<String>,
}

impl Default for PostProcessingSettings {
    fn default() -> Self {
        Self {
            processor: PostProcessor::default(),
            prompt: None,
        }
    }
}

impl PostProcessingSettings {
    /// Get the API key for the post-processor, falling back to environment variables.
    ///
    /// Returns None for local post-processor (Ollama uses URL instead).
    pub fn api_key(&self, transcription_api_keys: &std::collections::HashMap<String, String>) -> Option<String> {
        match &self.processor {
            PostProcessor::None | PostProcessor::Ollama => None,
            PostProcessor::OpenAI => {
                // Check transcription settings API keys first
                if let Some(key) = transcription_api_keys.get("openai") {
                    return Some(key.clone());
                }
                // Fall back to environment variable
                std::env::var(TranscriptionProvider::OpenAI.api_key_env_var()).ok()
            }
            PostProcessor::Mistral => {
                // Check transcription settings API keys first
                if let Some(key) = transcription_api_keys.get("mistral") {
                    return Some(key.clone());
                }
                // Fall back to environment variable
                std::env::var(TranscriptionProvider::Mistral.api_key_env_var()).ok()
            }
        }
    }

    /// Check if post-processing is enabled and properly configured.
    pub fn is_configured(&self, transcription_api_keys: &std::collections::HashMap<String, String>) -> bool {
        match &self.processor {
            PostProcessor::None => true, // No post-processing always valid
            PostProcessor::Ollama => true, // Ollama URL checked in services
            PostProcessor::OpenAI | PostProcessor::Mistral => {
                self.api_key(transcription_api_keys).is_some()
            }
        }
    }

    /// Validate post-processing settings.
    pub fn validate(&self, transcription_api_keys: &std::collections::HashMap<String, String>) -> anyhow::Result<()> {
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
