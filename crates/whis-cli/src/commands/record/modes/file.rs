//! File transcription mode
//!
//! Reads audio from a file and transcribes it.

use anyhow::{Context, Result};
use std::path::Path;
use whis_core::resample::resample_to_16k;

/// Read a WAV file and return 16kHz mono f32 samples
pub fn read_audio_file(path: &Path) -> Result<Vec<f32>> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("wav") => read_wav(path),
        Some(ext) => anyhow::bail!(
            "Unsupported audio format: .{}\nCurrently supported: WAV",
            ext
        ),
        None => anyhow::bail!("File has no extension. Please provide a WAV file."),
    }
}

/// Read a WAV file and resample to 16kHz mono
fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::open(path).context("Failed to open WAV file")?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels;

    // Read samples based on format
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to read float samples")?,
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (1u32 << (bits - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max_val))
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to read int samples")?
        }
    };

    // Resample to 16kHz mono if needed
    resample_to_16k(&samples, sample_rate, channels)
}
