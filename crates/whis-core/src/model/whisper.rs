//! Whisper model type implementation

use super::types::{ModelInfo, ModelType};
use std::path::{Path, PathBuf};

/// Available whisper models
const MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "tiny",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        description: "~75 MB - Fastest, lower quality",
        size_mb: Some(75),
    },
    ModelInfo {
        name: "base",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        description: "~142 MB - Fast, decent quality",
        size_mb: Some(142),
    },
    ModelInfo {
        name: "small",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        description: "~466 MB - Balanced (recommended)",
        size_mb: Some(466),
    },
    ModelInfo {
        name: "medium",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        description: "~1.5 GB - Better quality, slower",
        size_mb: Some(1500),
    },
];

/// Default model for whisper
pub const DEFAULT_MODEL: &str = "small";

/// Whisper model type
pub struct WhisperModel;

impl ModelType for WhisperModel {
    fn name(&self) -> &'static str {
        "whisper"
    }

    fn models(&self) -> &[ModelInfo] {
        MODELS
    }

    fn default_dir(&self) -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whis")
            .join("models")
    }

    fn default_path(&self, model_name: &str) -> PathBuf {
        self.default_dir().join(format!("ggml-{}.bin", model_name))
    }

    fn verify(&self, path: &Path) -> bool {
        path.exists() && path.is_file()
    }

    fn download_extension(&self) -> &'static str {
        ".bin"
    }
}
