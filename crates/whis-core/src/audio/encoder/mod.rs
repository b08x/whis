//! Audio encoding module providing MP3 encoding via FFmpeg or embedded encoder.

#[cfg(feature = "ffmpeg")]
mod ffmpeg;

#[cfg(feature = "embedded-encoder")]
mod embedded;

use anyhow::Result;

/// Trait for encoding raw audio samples to compressed formats.
pub trait AudioEncoder: Send + Sync {
    /// Encode raw f32 PCM samples to MP3.
    ///
    /// # Parameters
    /// - `samples`: Raw audio samples (f32 PCM, expected to be 16kHz mono)
    /// - `sample_rate`: Sample rate of the input audio
    ///
    /// # Returns
    /// Encoded MP3 data as bytes
    fn encode_samples(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<u8>>;
}

/// Create the appropriate audio encoder based on available features.
///
/// Priority:
/// 1. FFmpeg encoder (if `ffmpeg` feature enabled)
/// 2. Embedded encoder (if `embedded-encoder` feature enabled)
/// 3. Panic if no encoder available
pub fn create_encoder() -> Box<dyn AudioEncoder> {
    #[cfg(feature = "ffmpeg")]
    {
        Box::new(ffmpeg::FfmpegEncoder::new())
    }

    #[cfg(all(feature = "embedded-encoder", not(feature = "ffmpeg")))]
    {
        return Box::new(embedded::EmbeddedEncoder::new());
    }

    #[cfg(not(any(feature = "ffmpeg", feature = "embedded-encoder")))]
    {
        panic!("No audio encoder available. Enable either 'ffmpeg' or 'embedded-encoder' feature.");
    }
}
