//! Audio resampling utilities for transcription
//!
//! All transcription providers benefit from 16kHz mono audio:
//! - Smaller file sizes for cloud uploads
//! - Direct input for local whisper.cpp

use anyhow::{Context, Result};
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

/// Target sample rate for transcription (16kHz mono)
pub const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Real-time frame-by-frame resampler for audio callbacks.
///
/// This resampler converts audio from the device's sample rate to 16kHz mono
/// in real-time during recording, reducing file size for all providers.
pub struct FrameResampler {
    /// The rubato resampler (None if source is already 16kHz mono)
    resampler: Option<Fft<f32>>,
    /// Number of input channels
    channels: u16,
    /// Buffer for accumulating input samples until we have enough for a chunk
    input_buffer: Vec<f32>,
    /// Chunk size required by the resampler
    chunk_size: usize,
}

impl FrameResampler {
    /// Create a new frame resampler.
    ///
    /// # Arguments
    /// * `source_rate` - Source sample rate in Hz (e.g., 44100, 48000)
    /// * `channels` - Number of input channels (1 for mono, 2 for stereo)
    pub fn new(source_rate: u32, channels: u16) -> Result<Self> {
        // If already 16kHz mono, no resampling needed
        if source_rate == WHISPER_SAMPLE_RATE && channels == 1 {
            return Ok(Self {
                resampler: None,
                channels,
                input_buffer: Vec::new(),
                chunk_size: 0,
            });
        }

        // Create resampler: source_rate -> 16kHz
        let resampler = Fft::<f32>::new(
            source_rate as usize,
            WHISPER_SAMPLE_RATE as usize,
            1024,              // chunk size
            2,                 // sub-chunks for better quality
            1,                 // output channels (mono)
            FixedSync::Input,  // fixed input size
        )
        .context("Failed to create frame resampler")?;

        let chunk_size = resampler.input_frames_max();

        Ok(Self {
            resampler: Some(resampler),
            channels,
            input_buffer: Vec::with_capacity(chunk_size * 2),
            chunk_size,
        })
    }

    /// Process incoming audio samples and return resampled 16kHz mono output.
    ///
    /// This method accumulates samples until it has enough for a full chunk,
    /// then resamples and returns the output. May return an empty vec if
    /// not enough samples have accumulated yet.
    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        // If no resampling needed, just return input as-is
        let Some(resampler) = &mut self.resampler else {
            return samples.to_vec();
        };

        // Convert to mono first if multichannel
        let mono_samples = if self.channels > 1 {
            stereo_to_mono(samples, self.channels)
        } else {
            samples.to_vec()
        };

        // Add to input buffer
        self.input_buffer.extend_from_slice(&mono_samples);

        // Process complete chunks
        let mut output = Vec::new();
        while self.input_buffer.len() >= self.chunk_size {
            let chunk: Vec<f32> = self.input_buffer.drain(..self.chunk_size).collect();
            // Wrap chunk in AudioAdapter (1 channel, chunk_size frames)
            if let Ok(adapter) = InterleavedSlice::new(&chunk, 1, chunk.len()) {
                if let Ok(resampled) = resampler.process(&adapter, 0, None) {
                    output.extend_from_slice(&resampled.take_data());
                }
            }
        }

        output
    }

    /// Flush remaining samples at end of recording.
    ///
    /// Call this after the last `process()` to get any remaining buffered samples.
    pub fn flush(&mut self) -> Vec<f32> {
        let Some(resampler) = &mut self.resampler else {
            return std::mem::take(&mut self.input_buffer);
        };

        if self.input_buffer.is_empty() {
            return Vec::new();
        }

        // Pad remaining samples to chunk size
        let mut padded = std::mem::take(&mut self.input_buffer);
        padded.resize(self.chunk_size, 0.0);

        // Process final chunk with AudioAdapter
        if let Ok(adapter) = InterleavedSlice::new(&padded, 1, padded.len()) {
            if let Ok(resampled) = resampler.process(&adapter, 0, None) {
                resampled.take_data()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }
}

/// Resample audio to 16kHz mono for transcription.
///
/// This is used by local_whisper for batch resampling of already-recorded audio.
/// For real-time resampling during recording, use `FrameResampler` instead.
///
/// # Arguments
/// * `samples` - Input samples (any sample rate, any channel count)
/// * `source_rate` - Source sample rate in Hz
/// * `channels` - Number of channels in input
///
/// # Returns
/// * 16kHz mono f32 samples ready for transcription
#[cfg(feature = "local-transcription")]
pub fn resample_to_16k(samples: &[f32], source_rate: u32, channels: u16) -> Result<Vec<f32>> {
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
    let mut resampler = Fft::<f32>::new(
        source_rate as usize,
        WHISPER_SAMPLE_RATE as usize,
        1024,              // chunk size
        2,                 // sub-chunks
        1,                 // channels (mono)
        FixedSync::Input,  // fixed input size
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

        // Wrap in AudioAdapter (1 channel, padded.len() frames)
        let adapter = InterleavedSlice::new(&padded, 1, padded.len())
            .context("Failed to create audio adapter")?;
        let result = resampler
            .process(&adapter, 0, None)
            .context("Resampling failed")?;
        output.extend_from_slice(&result.take_data());
    }

    Ok(output)
}

/// Convert multichannel audio to mono by averaging all channels
fn stereo_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    samples
        .chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
