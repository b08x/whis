use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::TranscriptionProvider;
use crate::polish::Polisher;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub shortcut: String,
    #[serde(default)]
    pub provider: TranscriptionProvider,
    /// Language hint for transcription (ISO-639-1 code, e.g., "en", "de", "fr")
    /// None = auto-detect, Some("en") = English, etc.
    #[serde(default)]
    pub language: Option<String>,
    /// API keys stored by provider name (e.g., "openai" -> "sk-...")
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
    /// LLM provider for polishing (cleaning up) transcripts
    #[serde(default)]
    pub polisher: Polisher,
    /// Custom prompt for polishing (uses default if None)
    #[serde(default)]
    pub polish_prompt: Option<String>,
    /// Path to whisper.cpp model file for local transcription
    /// (e.g., ~/.local/share/whis/models/ggml-small.bin)
    #[serde(default)]
    pub whisper_model_path: Option<String>,
    /// Ollama server URL for local LLM polishing (default: http://localhost:11434)
    #[serde(default)]
    pub ollama_url: Option<String>,
    /// Ollama model name for polishing (default: ministral-3:3b)
    #[serde(default)]
    pub ollama_model: Option<String>,
    /// Remote whisper server URL for self-hosted transcription
    /// (e.g., http://localhost:8765)
    #[serde(default)]
    pub remote_whisper_url: Option<String>,
    /// Currently active preset name (if any)
    #[serde(default)]
    pub active_preset: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Shift+R".to_string(),
            provider: TranscriptionProvider::default(),
            language: None, // Auto-detect
            api_keys: HashMap::new(),
            polisher: Polisher::default(),
            polish_prompt: None,
            whisper_model_path: None,
            ollama_url: None,
            ollama_model: None,
            remote_whisper_url: None,
            active_preset: None,
        }
    }
}

impl Settings {
    /// Get the settings file path (~/.config/whis/settings.json)
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whis")
            .join("settings.json")
    }

    /// Get the API key for the current provider, falling back to environment variables
    pub fn get_api_key(&self) -> Option<String> {
        self.get_api_key_for(&self.provider)
    }

    /// Get the API key for a specific provider
    ///
    /// Checks in order:
    /// 1. api_keys map
    /// 2. Environment variable
    pub fn get_api_key_for(&self, provider: &TranscriptionProvider) -> Option<String> {
        // Check api_keys map first
        if let Some(key) = self.api_keys.get(provider.as_str())
            && !key.is_empty()
        {
            return Some(key.clone());
        }

        // Fall back to environment variable
        std::env::var(provider.api_key_env_var()).ok()
    }

    /// Set the API key for a provider
    pub fn set_api_key(&mut self, provider: &TranscriptionProvider, key: String) {
        self.api_keys.insert(provider.as_str().to_string(), key);
    }

    /// Check if an API key is configured for the current provider
    pub fn has_api_key(&self) -> bool {
        self.get_api_key().is_some()
    }

    /// Check if the current provider is properly configured
    ///
    /// For cloud providers: checks for API key
    /// For LocalWhisper: checks for model path AND that file exists
    /// For RemoteWhisper: checks for server URL
    pub fn is_provider_configured(&self) -> bool {
        match self.provider {
            TranscriptionProvider::LocalWhisper => self
                .get_whisper_model_path()
                .map(|p| std::path::Path::new(&p).exists())
                .unwrap_or(false),
            TranscriptionProvider::RemoteWhisper => self.get_remote_whisper_url().is_some(),
            _ => self.has_api_key(),
        }
    }

    /// Get the API key for the polisher, falling back to environment variables
    /// Returns None for local polisher (Ollama uses URL instead)
    pub fn get_polisher_api_key(&self) -> Option<String> {
        match &self.polisher {
            Polisher::None | Polisher::Ollama => None,
            Polisher::OpenAI => self.get_api_key_for(&TranscriptionProvider::OpenAI),
            Polisher::Mistral => self.get_api_key_for(&TranscriptionProvider::Mistral),
        }
    }

    /// Get the whisper model path, falling back to environment variable
    pub fn get_whisper_model_path(&self) -> Option<String> {
        self.whisper_model_path
            .clone()
            .or_else(|| std::env::var("LOCAL_WHISPER_MODEL_PATH").ok())
    }

    /// Get the Ollama server URL, falling back to environment variable
    pub fn get_ollama_url(&self) -> Option<String> {
        self.ollama_url
            .clone()
            .or_else(|| std::env::var("OLLAMA_URL").ok())
    }

    /// Get the Ollama model name, falling back to environment variable
    pub fn get_ollama_model(&self) -> Option<String> {
        self.ollama_model
            .clone()
            .or_else(|| std::env::var("OLLAMA_MODEL").ok())
    }

    /// Get the remote whisper server URL, falling back to environment variable
    pub fn get_remote_whisper_url(&self) -> Option<String> {
        self.remote_whisper_url
            .clone()
            .or_else(|| std::env::var("REMOTE_WHISPER_URL").ok())
    }

    /// Load settings from disk
    pub fn load() -> Self {
        let path = Self::path();
        if let Ok(content) = fs::read_to_string(&path)
            && let Ok(settings) = serde_json::from_str(&content)
        {
            return settings;
        }
        Self::default()
    }

    /// Save settings to disk with 0600 permissions
    ///
    /// On Unix, creates the file with mode 0600 from the start to avoid
    /// a race condition where the file might briefly be world-readable.
    pub fn save(&self) -> Result<()> {
        use std::io::Write;

        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)?;
            file.write_all(content.as_bytes())?;
        }

        #[cfg(not(unix))]
        {
            fs::write(&path, &content)?;
        }

        Ok(())
    }
}
