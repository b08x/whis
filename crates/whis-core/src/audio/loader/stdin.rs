//! Stdin-based audio loading with FFmpeg conversion support.

use anyhow::{Context, Result};
use std::io::Read;

use super::classify_recording_output;
use crate::audio::RecordingOutput;

/// Load audio from stdin.
///
/// Supported formats: mp3, wav, m4a, ogg, flac, webm, aac, opus
///
/// # Parameters
/// - `format`: Format of the audio data (e.g., "mp3", "wav")
///
/// # Returns
/// RecordingOutput containing the MP3-encoded audio data
///
/// # Errors
/// Returns an error if:
/// - Format is not supported
/// - No data received from stdin
/// - FFmpeg conversion fails (for non-MP3 formats)
#[cfg(feature = "ffmpeg")]
pub fn load_audio_stdin(format: &str) -> Result<RecordingOutput> {
    let mut data = Vec::new();
    std::io::stdin()
        .read_to_end(&mut data)
        .context("Failed to read audio from stdin")?;

    if data.is_empty() {
        anyhow::bail!("No audio data received from stdin");
    }

    let mp3_data = match format.to_lowercase().as_str() {
        "mp3" => data, // Already MP3
        "wav" | "m4a" | "ogg" | "flac" | "webm" | "aac" | "opus" => {
            // Convert stdin data to MP3 using FFmpeg
            convert_stdin_to_mp3(&data, format)?
        }
        _ => {
            anyhow::bail!(
                "Unsupported stdin format: '{}'. Supported: mp3, wav, m4a, ogg, flac, webm, aac, opus",
                format
            );
        }
    };

    classify_recording_output(mp3_data)
}

#[cfg(not(feature = "ffmpeg"))]
pub fn load_audio_stdin(_format: &str) -> Result<RecordingOutput> {
    anyhow::bail!("Stdin input requires the 'ffmpeg' feature (not available in mobile builds)")
}

/// Convert stdin audio data to MP3 using FFmpeg.
#[cfg(feature = "ffmpeg")]
fn convert_stdin_to_mp3(data: &[u8], format: &str) -> Result<Vec<u8>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let temp_dir = std::env::temp_dir();
    let unique_id = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let mp3_path = temp_dir.join(format!("whis_stdin_{unique_id}.mp3"));

    crate::verbose!("Converting stdin ({} format) to MP3...", format);

    // Use FFmpeg with pipe input
    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            format,
            "-i",
            "pipe:0", // Read from stdin
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "128k",
            "-y",
            mp3_path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn ffmpeg process")?;

    // Write input data to FFmpeg's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(data)
            .context("Failed to write audio data to ffmpeg")?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&mp3_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg stdin conversion failed: {stderr}");
    }

    let mp3_data = std::fs::read(&mp3_path).context("Failed to read converted MP3 file")?;
    let _ = std::fs::remove_file(&mp3_path);

    crate::verbose!("Converted to {:.1} KB MP3", mp3_data.len() as f64 / 1024.0);

    Ok(mp3_data)
}
