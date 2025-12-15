pub mod audio;
#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod config;
pub mod output_style;
pub mod polish;
pub mod settings;
pub mod transcribe;

pub use audio::{AudioChunk, AudioRecorder, RecordingData, RecordingOutput};
#[cfg(feature = "clipboard")]
pub use clipboard::copy_to_clipboard;
pub use config::TranscriptionProvider;
pub use output_style::OutputStyle;
pub use polish::{polish, Polisher, DEFAULT_POLISH_PROMPT};
pub use settings::Settings;
pub use transcribe::{parallel_transcribe, transcribe_audio, ChunkTranscription};
