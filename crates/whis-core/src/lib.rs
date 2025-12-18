pub mod audio;
#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod config;
pub mod model;
pub mod ollama;
pub mod polish;
pub mod preset;
pub mod provider;
pub mod resample;
pub mod settings;
pub mod transcribe;
pub mod verbose;

pub use audio::{AudioChunk, AudioRecorder, RecordingData, RecordingOutput};
#[cfg(feature = "clipboard")]
pub use clipboard::{ClipboardMethod, copy_to_clipboard};
pub use config::TranscriptionProvider;
pub use polish::{DEFAULT_POLISH_PROMPT, Polisher, polish};
pub use preset::{Preset, PresetSource};
pub use provider::{
    DEFAULT_TIMEOUT_SECS, TranscriptionBackend, TranscriptionRequest, TranscriptionResult, registry,
};
pub use settings::Settings;
pub use transcribe::{ChunkTranscription, parallel_transcribe, transcribe_audio};
pub use verbose::set_verbose;
