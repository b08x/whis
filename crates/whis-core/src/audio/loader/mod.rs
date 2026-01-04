//! Audio loading module for files and stdin.

mod file;
mod stdin;

use anyhow::Result;

pub use file::load_audio_file;
pub use stdin::load_audio_stdin;

use super::RecordingOutput;

/// Threshold for chunking (files larger than this get split)
pub(super) const CHUNK_THRESHOLD_BYTES: usize = 20 * 1024 * 1024; // 20 MB

/// Classify MP3 data into Single or Chunked based on size.
///
/// Files smaller than CHUNK_THRESHOLD_BYTES are returned as single files.
/// Larger files are also returned as single files for now, as chunking is
/// primarily designed for recordings where we have raw samples.
pub(super) fn classify_recording_output(mp3_data: Vec<u8>) -> Result<RecordingOutput> {
    if mp3_data.len() <= CHUNK_THRESHOLD_BYTES {
        Ok(RecordingOutput::Single(mp3_data))
    } else {
        // For pre-encoded MP3 files, we can't easily split by time
        // For now, just use as single file - chunking is mainly for recordings
        // where we have raw samples and can calculate exact time boundaries
        crate::verbose!(
            "Large file ({:.1} MB) - processing as single file",
            mp3_data.len() as f64 / 1024.0 / 1024.0
        );
        Ok(RecordingOutput::Single(mp3_data))
    }
}
