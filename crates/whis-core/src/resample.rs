//! Audio resampling utilities for local transcription
//!
//! whisper.cpp requires 16kHz mono f32 PCM audio.

use anyhow::{Context, Result};

/// Target sample rate for whisper.cpp
pub const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Resample audio to 16kHz mono for whisper.cpp
///
/// # Arguments
/// * `samples` - Input samples (any sample rate, any channel count)
/// * `source_rate` - Source sample rate in Hz
/// * `channels` - Number of channels in input
///
/// # Returns
/// * 16kHz mono f32 samples ready for whisper-rs
#[cfg(feature = "local-whisper")]
pub fn resample_to_16k(samples: &[f32], source_rate: u32, channels: u16) -> Result<Vec<f32>> {
    use rubato::{FftFixedIn, Resampler};

    // Convert to mono first if stereo/multichannel
    let mono_samples = if channels > 1 {
        stereo_to_mono(samples, channels)
    } else {
        samples.to_vec()
    };

    // If already 16kHz, return as-is
    if source_rate == WHISPER_SAMPLE_RATE {
        return Ok(mono_samples);
    }

    // Create resampler
    let mut resampler = FftFixedIn::<f32>::new(
        source_rate as usize,
        WHISPER_SAMPLE_RATE as usize,
        1024, // chunk size
        2,    // sub-chunks
        1,    // channels (mono)
    )
    .context("Failed to create resampler")?;

    // Process in chunks
    let mut output = Vec::new();
    let chunk_size = resampler.input_frames_max();

    for chunk in mono_samples.chunks(chunk_size) {
        let mut padded = chunk.to_vec();
        if padded.len() < chunk_size {
            padded.resize(chunk_size, 0.0);
        }

        let result = resampler
            .process(&[padded], None)
            .context("Resampling failed")?;
        output.extend_from_slice(&result[0]);
    }

    Ok(output)
}

/// Convert multichannel audio to mono by averaging all channels
#[cfg(feature = "local-whisper")]
fn stereo_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    samples
        .chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
#[cfg(feature = "local-whisper")]
mod tests {
    use super::*;

    #[test]
    fn test_stereo_to_mono() {
        let stereo = vec![0.5, 0.3, 0.8, 0.2, 1.0, 0.0];
        let mono = stereo_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.4).abs() < 0.001);
        assert!((mono[1] - 0.5).abs() < 0.001);
        assert!((mono[2] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_resample_passthrough_16k() {
        let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = resample_to_16k(&samples, 16000, 1).unwrap();
        assert_eq!(result, samples);
    }
}
