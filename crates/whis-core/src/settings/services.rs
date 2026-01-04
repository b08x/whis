//! External service configuration (Ollama, etc.).

use serde::{Deserialize, Serialize};

/// Settings for external services.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServicesSettings {
    /// Ollama configuration for local LLM post-processing
    #[serde(default)]
    pub ollama: OllamaConfig,
}

/// Configuration for Ollama local LLM service.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaConfig {
    /// Ollama server URL (default: http://localhost:11434)
    #[serde(default)]
    pub url: Option<String>,

    /// Ollama model name for post-processing (default: qwen2.5:1.5b)
    #[serde(default)]
    pub model: Option<String>,
}

impl OllamaConfig {
    /// Get the Ollama server URL, falling back to environment variable.
    pub fn url(&self) -> Option<String> {
        self.url
            .clone()
            .or_else(|| std::env::var("OLLAMA_URL").ok())
    }

    /// Get the Ollama model name, falling back to environment variable.
    pub fn model(&self) -> Option<String> {
        self.model
            .clone()
            .or_else(|| std::env::var("OLLAMA_MODEL").ok())
    }
}
