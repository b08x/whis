//! Core audio types used throughout the audio module.

use serde::{Deserialize, Serialize};

/// Information about an available audio input device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// Device name as reported by the system
    pub name: String,
    /// Whether this is the default input device
    pub is_default: bool,
}

/// A chunk of audio data ready for transcription.
#[derive(Clone)]
pub struct AudioChunk {
    /// MP3 audio data
    pub data: Vec<u8>,
    /// Chunk index (0-based, for ordering)
    pub index: usize,
    /// Whether this chunk has overlap from the previous chunk
    pub has_leading_overlap: bool,
}

/// Output from a recording session - either a single file or multiple chunks.
pub enum RecordingOutput {
    /// Small file that can be transcribed directly
    Single(Vec<u8>),
    /// Large file split into chunks for parallel transcription
    Chunked(Vec<AudioChunk>),
}

