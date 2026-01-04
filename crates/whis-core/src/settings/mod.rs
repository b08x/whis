//! Application Settings Module
//!
//! This module provides a hierarchical settings structure organized by concern:
//!
//! # Architecture
//!
//! ```text
//! Settings (Aggregate Root)
//!   ├── Transcription  - Provider, API keys, local models
//!   ├── PostProcessing - LLM processor, prompts
//!   ├── Services       - Ollama, external services
//!   └── UI             - Shortcut, clipboard, microphone, VAD, presets
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use whis_core::settings::Settings;
//!
//! // Load settings from disk
//! let settings = Settings::load();
//!
//! // Access nested settings
//! println!("Provider: {}", settings.transcription.provider);
//! println!("Post-processor: {:?}", settings.post_processing.processor);
//!
//! // Modify and save
//! let mut settings = settings;
//! settings.transcription.provider = whis_core::config::TranscriptionProvider::Mistral;
//! settings.save().expect("Failed to save settings");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # File Location
//!
//! Settings are stored at `~/.config/whis/settings.json` with 0600 permissions
//! to protect API keys.

mod post_processing;
mod services;
mod transcription;
mod ui;

pub use post_processing::PostProcessingSettings;
pub use services::{OllamaConfig, ServicesSettings};
pub use transcription::{LocalModelsConfig, TranscriptionSettings};
pub use ui::{UiSettings, VadSettings};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings (aggregate root).
///
/// Settings are organized hierarchically by concern:
/// - `transcription`: Provider configuration and API keys
/// - `post_processing`: LLM post-processing settings
/// - `services`: External service configuration (Ollama, etc.)
/// - `ui`: User interface preferences
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub transcription: TranscriptionSettings,
    pub post_processing: PostProcessingSettings,
    pub services: ServicesSettings,
    pub ui: UiSettings,
}

impl Settings {
    /// Get the settings file path (~/.config/whis/settings.json).
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whis")
            .join("settings.json")
    }

    /// Load settings from disk.
    ///
    /// Returns default settings if file doesn't exist or cannot be parsed.
    pub fn load() -> Self {
        let path = Self::path();
        if let Ok(content) = fs::read_to_string(&path)
            && let Ok(settings) = serde_json::from_str(&content)
        {
            return settings;
        }
        Self::default()
    }

    /// Save settings to disk with 0600 permissions.
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

    /// Validate all settings.
    ///
    /// Checks that:
    /// - Transcription provider is properly configured
    /// - Post-processing (if enabled) has required credentials
    pub fn validate(&self) -> Result<()> {
        self.transcription.validate()?;
        self.post_processing
            .validate(&self.transcription.api_keys)?;
        Ok(())
    }
}
