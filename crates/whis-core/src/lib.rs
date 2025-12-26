pub mod audio;
#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod config;
pub mod model;
#[cfg(feature = "local-transcription")]
pub mod model_manager;
pub mod ollama;
pub mod ollama_manager;
pub mod post_processing;
pub mod preset;
pub mod provider;
pub mod resample;
pub mod settings;
pub mod state;
pub mod transcribe;
#[cfg(feature = "vad")]
pub mod vad;
pub mod verbose;

#[cfg(feature = "vad")]
pub use audio::VadConfig;
pub use audio::{
    AudioChunk, AudioDeviceInfo, AudioRecorder, RecordingData, RecordingOutput, list_audio_devices,
    load_audio_file, load_audio_stdin,
};
#[cfg(feature = "clipboard")]
pub use clipboard::{ClipboardMethod, copy_to_clipboard};
pub use config::TranscriptionProvider;
pub use ollama_manager::{clear_warmup_cache, preload_ollama};
pub use post_processing::{DEFAULT_POST_PROCESSING_PROMPT, PostProcessor, post_process};
pub use preset::{Preset, PresetSource};
#[cfg(feature = "realtime")]
pub use provider::OpenAIRealtimeProvider;
#[cfg(feature = "local-transcription")]
pub use provider::transcribe_raw;
#[cfg(feature = "local-transcription")]
pub use provider::transcribe_raw_parakeet;
pub use provider::{
    DEFAULT_TIMEOUT_SECS, ProgressCallback, TranscriptionBackend, TranscriptionRequest,
    TranscriptionResult, TranscriptionStage, registry,
};
pub use settings::Settings;
pub use state::RecordingState;
pub use transcribe::{
    ChunkTranscription, parallel_transcribe, transcribe_audio, transcribe_audio_with_format,
    transcribe_audio_with_progress,
};
pub use verbose::set_verbose;
