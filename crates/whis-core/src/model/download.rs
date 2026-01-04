//! Shared download logic for models

use super::types::ModelType;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// Download a model with default progress indication (prints to stderr)
pub fn download<M: ModelType>(
    model_type: &M,
    model_name: &str,
    dest: &Path,
) -> Result<()> {
    download_with_progress(model_type, model_name, dest, |downloaded, total| {
        let progress = if total > 0 {
            (downloaded * 100 / total) as usize
        } else {
            0
        };
        eprint!(
            "\rDownloading: {}% ({:.1} MB / {:.1} MB)  ",
            progress,
            downloaded as f64 / 1_000_000.0,
            total as f64 / 1_000_000.0
        );
        io::stderr().flush().ok();
    })
}

/// Download a model with a custom progress callback
///
/// The callback receives (downloaded_bytes, total_bytes) and is called
/// approximately every 1% of progress or every 500KB, whichever is more frequent.
pub fn download_with_progress<M, F>(
    model_type: &M,
    model_name: &str,
    dest: &Path,
    on_progress: F,
) -> Result<()>
where
    M: ModelType,
    F: Fn(u64, u64),
{
    let url = model_type.get_url(model_name).ok_or_else(|| {
        let available: Vec<_> = model_type.models().iter().map(|m| m.name).collect();
        anyhow!(
            "Unknown {} model: {}. Available: {}",
            model_type.name(),
            model_name,
            available.join(", ")
        )
    })?;

    // Create parent directory if needed
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create models directory")?;
    }

    eprintln!("Downloading {} model '{}'...", model_type.name(), model_name);
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

    let total_size = response.content_length().unwrap_or(0);

    // Create temp file first, then rename on success
    let temp_path = if model_type.needs_extraction() {
        // For archives, use the download extension for temp file
        dest.with_extension(format!("tmp{}", model_type.download_extension()))
    } else {
        dest.with_extension(format!("{}tmp", model_type.download_extension()))
    };

    let mut file = fs::File::create(&temp_path).context("Failed to create temp file")?;

    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];
    let mut last_callback_bytes: u64 = 0;

    // Emit initial progress
    on_progress(0, total_size);

    loop {
        let bytes_read = response.read(&mut buffer).context("Download interrupted")?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .context("Failed to write to file")?;
        downloaded += bytes_read as u64;

        // Emit progress every ~1% or 500KB, whichever is more frequent
        let threshold = if total_size > 0 {
            (total_size / 100).min(500_000)
        } else {
            500_000
        };

        if downloaded - last_callback_bytes >= threshold {
            on_progress(downloaded, total_size);
            last_callback_bytes = downloaded;
        }
    }

    // Final progress callback
    on_progress(downloaded, total_size);

    eprintln!(
        "\rDownload complete: {:.1} MB                    ",
        downloaded as f64 / 1_000_000.0
    );
    eprintln!();

    // Handle extraction if needed
    if model_type.needs_extraction() {
        eprintln!("Extracting...");
        if let Some(parent) = dest.parent() {
            model_type.extract(&temp_path, parent)?;
        } else {
            return Err(anyhow!("No parent directory for extraction"));
        }
        // Remove temp archive after extraction
        fs::remove_file(&temp_path).ok();
        eprintln!("Extraction complete!");
    } else {
        // Rename temp file to final destination
        fs::rename(&temp_path, dest).context("Failed to finalize download")?;
    }

    Ok(())
}

/// Ensure a model is available, downloading it if necessary
pub fn ensure<M: ModelType>(
    model_type: &M,
    model_name: &str,
) -> Result<()> {
    let path = model_type.default_path(model_name);

    if model_type.verify(&path) {
        return Ok(());
    }

    download(model_type, model_name, &path)
}
