//! File-based audio loading with FFmpeg conversion support.

use anyhow::{Context, Result};
use std::path::Path;

use super::classify_recording_output;
use crate::audio::RecordingOutput;

/// Load audio from a file, converting to MP3 if needed.
///
/// Supported formats: mp3, wav, m4a, ogg, flac, webm, aac, opus
///
/// # Parameters
/// - `path`: Path to the audio file
///
/// # Returns
/// RecordingOutput containing the MP3-encoded audio data
///
/// # Errors
/// Returns an error if:
/// - File format is not supported
/// - File cannot be read
/// - FFmpeg conversion fails (for non-MP3 files)
#[cfg(feature = "ffmpeg")]
pub fn load_audio_file(path: &Path) -> Result<RecordingOutput> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mp3_data = match extension.as_str() {
        "mp3" => {
            // Read MP3 directly
            std::fs::read(path).context("Failed to read MP3 file")?
        }
        "wav" | "m4a" | "ogg" | "flac" | "webm" | "aac" | "opus" => {
            // Convert to MP3 using FFmpeg
            convert_file_to_mp3(path)?
        }
        _ => {
            anyhow::bail!(
                "Unsupported audio format: '{}'. Supported: mp3, wav, m4a, ogg, flac, webm, aac, opus",
                extension
            );
        }
    };

    classify_recording_output(mp3_data)
}

#[cfg(not(feature = "ffmpeg"))]
pub fn load_audio_file(_path: &Path) -> Result<RecordingOutput> {
    anyhow::bail!("File input requires the 'ffmpeg' feature (not available in mobile builds)")
}

/// Convert an audio file to MP3 using FFmpeg.
#[cfg(feature = "ffmpeg")]
fn convert_file_to_mp3(input_path: &Path) -> Result<Vec<u8>> {
    let temp_dir = std::env::temp_dir();
    let unique_id = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let mp3_path = temp_dir.join(format!("whis_convert_{unique_id}.mp3"));

    crate::verbose!("Converting {} to MP3...", input_path.display());

    let output = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            input_path.to_str().unwrap(),
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "128k",
            "-y",
            mp3_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to execute ffmpeg. Make sure ffmpeg is installed.")?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&mp3_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg conversion failed: {stderr}");
    }

    let mp3_data = std::fs::read(&mp3_path).context("Failed to read converted MP3 file")?;
    let _ = std::fs::remove_file(&mp3_path);

    crate::verbose!("Converted to {:.1} KB MP3", mp3_data.len() as f64 / 1024.0);

    Ok(mp3_data)
}
