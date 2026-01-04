//! Parakeet model type implementation

#[cfg(feature = "local-transcription")]
use super::types::{ModelInfo, ModelType};
#[cfg(feature = "local-transcription")]
use anyhow::{Context, Result};
#[cfg(feature = "local-transcription")]
use std::path::{Path, PathBuf};

/// Available Parakeet models
#[cfg(feature = "local-transcription")]
const MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "parakeet-v3",
        url: "https://blob.handy.computer/parakeet-v3-int8.tar.gz",
        description: "~478 MB - Fast & accurate (recommended)",
        size_mb: Some(478),
    },
    ModelInfo {
        name: "parakeet-v2",
        url: "https://blob.handy.computer/parakeet-v2-int8.tar.gz",
        description: "~478 MB - Previous version",
        size_mb: Some(478),
    },
];

/// Default Parakeet model
#[cfg(feature = "local-transcription")]
pub const DEFAULT_MODEL: &str = "parakeet-v3";

/// Directory name after extraction (tar.gz contains this directory)
#[cfg(feature = "local-transcription")]
fn dirname(model_name: &str) -> String {
    match model_name {
        "parakeet-v3" => "parakeet-tdt-0.6b-v3-int8".to_string(),
        "parakeet-v2" => "parakeet-tdt-0.6b-v2-int8".to_string(),
        _ => format!("{}-int8", model_name),
    }
}

/// Parakeet model type
#[cfg(feature = "local-transcription")]
pub struct ParakeetModel;

#[cfg(feature = "local-transcription")]
impl ModelType for ParakeetModel {
    fn name(&self) -> &'static str {
        "parakeet"
    }

    fn models(&self) -> &[ModelInfo] {
        MODELS
    }

    fn default_dir(&self) -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whis")
            .join("models")
            .join("parakeet")
    }

    fn default_path(&self, model_name: &str) -> PathBuf {
        self.default_dir().join(dirname(model_name))
    }

    fn verify(&self, path: &Path) -> bool {
        // Parakeet models are directories containing ONNX files
        if !path.exists() || !path.is_dir() {
            return false;
        }

        // Check for essential files
        let encoder = path.join("encoder-model.int8.onnx");
        let decoder = path.join("decoder_joint-model.int8.onnx");
        let vocab = path.join("vocab.txt");

        encoder.exists() && decoder.exists() && vocab.exists()
    }

    fn needs_extraction(&self) -> bool {
        true
    }

    fn extract(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::fs::File;
        use tar::Archive;

        let tar_gz = File::open(archive_path)
            .context("Failed to open archive for extraction")?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        archive
            .unpack(dest_dir)
            .context("Failed to extract archive")?;

        Ok(())
    }

    fn download_extension(&self) -> &'static str {
        ".tar.gz"
    }
}
