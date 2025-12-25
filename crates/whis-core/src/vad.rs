//! Voice Activity Detection (VAD) using Silero model
//!
//! This module provides real-time speech detection to skip silence during recording,
//! reducing audio size and improving transcription quality.
//!
//! Uses "Smoothed VAD" approach with:
//! - **Prefill**: Buffers audio BEFORE speech detected to capture word beginnings
//! - **Onset detection**: Requires consecutive speech frames to prevent noise false-positives
//! - **Hangover**: Continues recording after silence to catch trailing syllables

use std::collections::VecDeque;

use anyhow::{Context, Result};
use voice_activity_detector::VoiceActivityDetector;

use crate::resample::WHISPER_SAMPLE_RATE;

/// VAD processes 512 samples at a time (32ms at 16kHz)
pub const VAD_CHUNK_SIZE: usize = 512;

/// Default prefill frames (~480ms at 32ms/frame)
const DEFAULT_PREFILL_FRAMES: usize = 15;

/// Default onset frames (require 2 consecutive speech frames)
const DEFAULT_ONSET_FRAMES: usize = 2;

/// Default hangover frames (~480ms trailing capture)
const DEFAULT_HANGOVER_FRAMES: usize = 15;

/// Voice Activity Detection processor
///
/// Wraps the Silero VAD model to detect speech in real-time audio streams.
/// Uses Smoothed VAD approach for better speech capture.
pub struct VadProcessor {
    detector: VoiceActivityDetector,
    threshold: f32,
    is_enabled: bool,
    /// Buffer to accumulate samples until we have a full chunk
    buffer: Vec<f32>,
    /// Whether speech is currently detected
    is_speaking: bool,

    // Smoothed VAD fields
    /// Circular buffer of recent frames for prefill
    frame_buffer: VecDeque<Vec<f32>>,
    /// Number of frames to buffer before speech (captures word beginnings)
    prefill_frames: usize,
    /// Number of consecutive speech frames required to confirm onset
    onset_frames: usize,
    /// Number of frames to continue after silence (captures trailing syllables)
    hangover_frames: usize,
    /// Counter for consecutive speech frames (onset detection)
    onset_counter: usize,
    /// Remaining hangover frames before transitioning to silence
    hangover_counter: usize,
}

impl VadProcessor {
    /// Create a new VAD processor
    ///
    /// # Arguments
    /// * `enabled` - Whether VAD is enabled
    /// * `threshold` - Speech probability threshold (0.0-1.0, default 0.5)
    pub fn new(enabled: bool, threshold: f32) -> Result<Self> {
        // VAD expects 16kHz audio with 512-sample chunks
        let detector = VoiceActivityDetector::builder()
            .sample_rate(WHISPER_SAMPLE_RATE as i64)
            .chunk_size(VAD_CHUNK_SIZE)
            .build()
            .context("Failed to create VAD detector")?;

        Ok(Self {
            detector,
            threshold: threshold.clamp(0.0, 1.0),
            is_enabled: enabled,
            buffer: Vec::with_capacity(VAD_CHUNK_SIZE * 2),
            is_speaking: false,
            // Smoothed VAD initialization
            frame_buffer: VecDeque::with_capacity(DEFAULT_PREFILL_FRAMES + 1),
            prefill_frames: DEFAULT_PREFILL_FRAMES,
            onset_frames: DEFAULT_ONSET_FRAMES,
            hangover_frames: DEFAULT_HANGOVER_FRAMES,
            onset_counter: 0,
            hangover_counter: 0,
        })
    }

    /// Create a disabled VAD processor (passthrough)
    pub fn disabled() -> Result<Self> {
        Self::new(false, 0.5)
    }

    /// Check if VAD is enabled
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Process audio samples and return samples that contain speech.
    ///
    /// Uses Smoothed VAD approach:
    /// - Prefill: Buffers recent frames to emit when speech starts (captures word beginnings)
    /// - Onset: Requires consecutive speech frames to confirm speech (prevents noise false-positives)
    /// - Hangover: Continues recording after silence (captures trailing syllables)
    ///
    /// When VAD is disabled, returns all samples.
    /// When enabled, only returns samples during detected speech.
    ///
    /// # Arguments
    /// * `samples` - 16kHz mono f32 audio samples
    ///
    /// # Returns
    /// * Samples that should be kept (speech only when VAD enabled)
    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        if !self.is_enabled {
            return samples.to_vec();
        }

        let mut output = Vec::new();
        self.buffer.extend_from_slice(samples);

        // Process complete chunks
        while self.buffer.len() >= VAD_CHUNK_SIZE {
            let chunk: Vec<f32> = self.buffer.drain(..VAD_CHUNK_SIZE).collect();

            // 1. Buffer for prefill (keep last N frames)
            self.frame_buffer.push_back(chunk.clone());
            while self.frame_buffer.len() > self.prefill_frames + 1 {
                self.frame_buffer.pop_front();
            }

            // 2. Get VAD prediction
            let probability = self.detector.predict(chunk.iter().copied());
            let is_voice = probability >= self.threshold;

            // 3. State machine (Smoothed VAD approach)
            match (self.is_speaking, is_voice) {
                // Potential speech onset - waiting for confirmation
                (false, true) => {
                    self.onset_counter += 1;
                    if self.onset_counter >= self.onset_frames {
                        // Confirmed speech - emit prefill + current frame
                        self.is_speaking = true;
                        self.hangover_counter = self.hangover_frames;
                        self.onset_counter = 0;

                        // Emit all buffered frames (prefill captures word beginning)
                        for frame in &self.frame_buffer {
                            output.extend_from_slice(frame);
                        }
                    }
                    // else: still waiting for onset confirmation
                }

                // Ongoing speech - reset hangover and emit
                (true, true) => {
                    self.hangover_counter = self.hangover_frames;
                    output.extend_from_slice(&chunk);
                }

                // Potential speech end - use hangover
                (true, false) => {
                    if self.hangover_counter > 0 {
                        self.hangover_counter -= 1;
                        output.extend_from_slice(&chunk);
                    } else {
                        self.is_speaking = false;
                    }
                }

                // Confirmed silence - reset onset counter
                (false, false) => {
                    self.onset_counter = 0;
                }
            }
        }

        output
    }

    /// Reset the VAD state for a new recording session.
    ///
    /// Clears all buffers and resets counters while keeping configuration.
    pub fn reset(&mut self) {
        self.frame_buffer.clear();
        self.onset_counter = 0;
        self.hangover_counter = 0;
        self.is_speaking = false;
        self.buffer.clear();
    }

    /// Flush remaining buffered samples.
    ///
    /// Call this at the end of recording to get any remaining samples.
    /// Resets state for the next recording session.
    pub fn flush(&mut self) -> Vec<f32> {
        if !self.is_enabled {
            return std::mem::take(&mut self.buffer);
        }

        let mut output = Vec::new();

        // If we were speaking, return remaining buffer
        if self.is_speaking {
            output.extend(std::mem::take(&mut self.buffer));
        }

        // Reset state for next recording
        self.reset();

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_disabled_passthrough() {
        let mut vad = VadProcessor::disabled().unwrap();
        let samples = vec![0.1, 0.2, 0.3];
        let output = vad.process(&samples);
        assert_eq!(output, samples);
    }
}
