//! Model type abstractions and shared types

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Information about an available model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: &'static str,
    pub url: &'static str,
    pub description: &'static str,
    pub size_mb: Option<u64>,
}

/// Trait defining operations for a model type (Whisper, Parakeet, etc.)
///
/// This trait provides a consistent interface for working with different
/// transcription model types, allowing shared code for download, validation,
/// and path management.
pub trait ModelType: Send + Sync {
    /// Display name for this model type
    fn name(&self) -> &'static str;

    /// List of available models for this type
    fn models(&self) -> &[ModelInfo];

    /// Default model directory for this type
    fn default_dir(&self) -> PathBuf;

    /// Default path for a specific model
    fn default_path(&self, model_name: &str) -> PathBuf;

    /// Check if a model exists and is valid at the given path
    fn verify(&self, path: &Path) -> bool;

    /// Get download URL for a model by name
    fn get_url(&self, name: &str) -> Option<&'static str> {
        self.models()
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.url)
    }

    /// Whether this model type needs extraction after download
    fn needs_extraction(&self) -> bool {
        false
    }

    /// Extract downloaded archive (for models that need extraction)
    fn extract(&self, _archive_path: &Path, _dest: &Path) -> Result<()> {
        Ok(())
    }

    /// Get file extension for downloads
    fn download_extension(&self) -> &'static str {
        ".bin"
    }
}
