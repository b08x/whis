//! Unified audio stream building with platform-specific handling.

use anyhow::Result;
use cpal::traits::DeviceTrait;
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

use super::processor::SampleProcessor;
use super::AudioStreamSender;

/// Build a unified audio input stream that works with or without VAD.
///
/// This function eliminates the code duplication between VAD and non-VAD builds
/// by using the SampleProcessor abstraction.
pub(super) fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    processor: SampleProcessor,
    stream_tx: Option<Arc<AudioStreamSender>>,
) -> Result<Stream>
where
    T: cpal::Sample + cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    // Wrap processor in Arc<Mutex> for sharing with audio callback
    let processor = Arc::new(Mutex::new(processor));
    let err_fn = |err| eprintln!("Error in audio stream: {err}");

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            // Convert to f32
            let f32_samples: Vec<f32> = data
                .iter()
                .map(|&s| cpal::Sample::from_sample(s))
                .collect();

            // Process through resampler and VAD (if enabled)
            let processed_samples = processor.lock().unwrap().process(&f32_samples);

            // Store processed samples (speech only if VAD enabled)
            if !processed_samples.is_empty() {
                samples
                    .lock()
                    .unwrap()
                    .extend_from_slice(&processed_samples);

                // Stream samples if channel is configured (for real-time transcription)
                if let Some(ref tx) = stream_tx {
                    // Use try_send to avoid blocking the audio thread
                    let _ = tx.try_send(processed_samples);
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
