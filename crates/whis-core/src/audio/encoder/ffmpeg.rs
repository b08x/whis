//! FFmpeg-based MP3 encoder implementation.

use anyhow::{Context, Result};
use hound::WavSpec;

use super::AudioEncoder;

/// MP3 encoder using FFmpeg command-line tool.
///
/// This encoder creates temporary WAV files and uses FFmpeg to convert them to MP3.
/// Supports parallel encoding with unique temporary file names.
pub struct FfmpegEncoder {
    channels: u16,
}

impl FfmpegEncoder {
    /// Create a new FFmpeg encoder.
    ///
    /// Always configured for mono (1 channel) output at 16kHz.
    pub fn new() -> Self {
        Self { channels: 1 }
    }

    /// Convert f32 samples to i16 PCM format.
    fn samples_to_i16(&self, samples: &[f32]) -> Vec<i16> {
        samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * i16::MAX as f32) as i16
            })
            .collect()
    }

    /// Generate unique temporary file paths for parallel encoding.
    fn generate_temp_paths(&self, suffix: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let temp_dir = std::env::temp_dir();
        let unique_id = format!(
            "{}_{}_{suffix}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        );
        let wav_path = temp_dir.join(format!("whis_{unique_id}.wav"));
        let mp3_path = temp_dir.join(format!("whis_{unique_id}.mp3"));
        (wav_path, mp3_path)
    }

    /// Write samples as WAV file.
    fn write_wav_file(&self, samples: &[i16], path: &std::path::Path, sample_rate: u32) -> Result<()> {
        let spec = WavSpec {
            channels: self.channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Execute FFmpeg conversion from WAV to MP3.
    fn execute_ffmpeg(&self, wav_path: &std::path::Path, mp3_path: &std::path::Path) -> Result<()> {
        let output = std::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                wav_path.to_str().unwrap(),
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
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("FFmpeg conversion failed: {stderr}");
        }

        Ok(())
    }
}

impl Default for FfmpegEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEncoder for FfmpegEncoder {
    fn encode_samples(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
        // Convert f32 samples to i16 for WAV format
        let i16_samples = self.samples_to_i16(samples);

        // Generate unique temporary file paths (supports parallel encoding)
        let (wav_path, mp3_path) = self.generate_temp_paths("audio");

        // Write WAV file
        self.write_wav_file(&i16_samples, &wav_path, sample_rate)?;

        // Convert WAV to MP3 using FFmpeg
        let result = self.execute_ffmpeg(&wav_path, &mp3_path);

        // Clean up WAV file immediately
        let _ = std::fs::remove_file(&wav_path);

        // Check conversion result
        result?;

        // Read the MP3 file
        let mp3_data = std::fs::read(&mp3_path).context("Failed to read converted MP3 file")?;

        // Clean up MP3 file
        let _ = std::fs::remove_file(&mp3_path);

        Ok(mp3_data)
    }
}
