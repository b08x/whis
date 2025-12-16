//! Whisper model download utilities
//!
//! Helps download and manage whisper.cpp model files.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Available whisper models with their download URLs
pub const WHISPER_MODELS: &[(&str, &str, &str)] = &[
    (
        "tiny",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        "~75 MB - Fastest, lower quality",
    ),
    (
        "base",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        "~142 MB - Fast, decent quality",
    ),
    (
        "small",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        "~466 MB - Balanced (recommended)",
    ),
    (
        "medium",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        "~1.5 GB - Better quality, slower",
    ),
];

/// Default model for embedded setup
pub const DEFAULT_MODEL: &str = "small";

/// Get the default directory for storing whisper models
pub fn default_models_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whis")
        .join("models")
}

/// Get the default path for a whisper model
pub fn default_model_path(model_name: &str) -> PathBuf {
    default_models_dir().join(format!("ggml-{}.bin", model_name))
}

/// Check if a model exists at the given path
pub fn model_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Get the URL for a model by name
pub fn get_model_url(name: &str) -> Option<&'static str> {
    WHISPER_MODELS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, url, _)| *url)
}

/// Download a whisper model with progress indication
pub fn download_model(model_name: &str, dest: &Path) -> Result<()> {
    let url = get_model_url(model_name).ok_or_else(|| {
        anyhow!(
            "Unknown model: {}. Available: tiny, base, small, medium",
            model_name
        )
    })?;

    // Create parent directory if needed
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create models directory")?;
    }

    eprintln!("Downloading whisper model '{}'...", model_name);
    eprintln!("URL: {}", url);
    eprintln!("Destination: {}", dest.display());
    eprintln!();

    // Download with progress
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10 min timeout for large files
        .build()
        .context("Failed to create HTTP client")?;

    let mut response = client.get(url).send().context("Failed to start download")?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed: HTTP {}", response.status()));
    }

    let total_size = response.content_length();

    // Create temp file first, then rename on success
    let temp_path = dest.with_extension("bin.tmp");
    let mut file = fs::File::create(&temp_path).context("Failed to create temp file")?;

    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];
    let mut last_progress = 0;

    loop {
        let bytes_read = response.read(&mut buffer).context("Download interrupted")?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .context("Failed to write to file")?;
        downloaded += bytes_read as u64;

        // Show progress every 5%
        if let Some(total) = total_size {
            let progress = (downloaded * 100 / total) as usize;
            if progress >= last_progress + 5 {
                eprint!(
                    "\rDownloading: {}% ({:.1} MB / {:.1} MB)  ",
                    progress,
                    downloaded as f64 / 1_000_000.0,
                    total as f64 / 1_000_000.0
                );
                io::stderr().flush().ok();
                last_progress = progress;
            }
        }
    }

    eprintln!(
        "\rDownload complete: {:.1} MB                    ",
        downloaded as f64 / 1_000_000.0
    );

    // Rename temp to final
    fs::rename(&temp_path, dest).context("Failed to finalize download")?;

    Ok(())
}

/// Ensure a model is available, downloading if needed
pub fn ensure_model(model_name: &str) -> Result<PathBuf> {
    let path = default_model_path(model_name);

    if model_exists(&path) {
        return Ok(path);
    }

    download_model(model_name, &path)?;
    Ok(path)
}

/// List available models with descriptions
pub fn list_models() {
    eprintln!("Available whisper models:");
    eprintln!();
    for (name, _, desc) in WHISPER_MODELS {
        let path = default_model_path(name);
        let status = if model_exists(&path) {
            "[installed]"
        } else {
            ""
        };
        eprintln!("  {} - {} {}", name, desc, status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_path() {
        let path = default_model_path("small");
        assert!(path.to_string_lossy().contains("ggml-small.bin"));
    }

    #[test]
    fn test_get_model_url() {
        assert!(get_model_url("small").is_some());
        assert!(get_model_url("nonexistent").is_none());
    }
}
