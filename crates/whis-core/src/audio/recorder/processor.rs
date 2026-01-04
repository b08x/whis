//! Sample processing abstraction for VAD and resampling.

use std::sync::{Arc, Mutex};

use crate::resample::FrameResampler;
use super::super::vad::VadProcessor;

/// Processes raw audio samples through resampling and optional VAD.
#[derive(Clone)]
pub(super) struct SampleProcessor {
    resampler: Arc<Mutex<FrameResampler>>,
    vad: Option<Arc<Mutex<VadProcessor>>>,
}

impl SampleProcessor {
    /// Create a new sample processor with resampling only.
    pub fn new(resampler: Arc<Mutex<FrameResampler>>) -> Self {
        Self {
            resampler,
            vad: None,
        }
    }

    /// Create a new sample processor with resampling and VAD.
    pub fn with_vad(
        resampler: Arc<Mutex<FrameResampler>>,
        vad: Arc<Mutex<VadProcessor>>,
    ) -> Self {
        Self {
            resampler,
            vad: Some(vad),
        }
    }

    /// Process raw audio samples through resampling and optional VAD.
    ///
    /// Returns the processed samples (16kHz mono, with silence filtered if VAD enabled).
    pub fn process(&self, raw_samples: &[f32]) -> Vec<f32> {
        // First, resample to 16kHz mono
        let resampled = self.resampler.lock().unwrap().process(raw_samples);

        if resampled.is_empty() {
            return Vec::new();
        }

        // Then apply VAD if enabled (filters out silence)
        if let Some(ref vad) = self.vad {
            return vad.lock().unwrap().process(&resampled);
        }

        resampled
    }

    /// Flush any buffered samples from the resampler and VAD.
    pub fn flush(&mut self) -> Vec<f32> {
        // Flush resampler
        let flushed_resampler = self.resampler.lock().unwrap().flush();

        // Process flushed resampler samples through VAD and flush VAD
        if let Some(ref vad) = self.vad {
            let mut vad = vad.lock().unwrap();
            let mut remaining = vad.process(&flushed_resampler);
            remaining.extend(vad.flush());
            return remaining;
        }

        flushed_resampler
    }
}
