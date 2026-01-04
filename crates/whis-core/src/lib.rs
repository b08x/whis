pub mod audio;
#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod config;
pub mod error;
pub mod model;
pub mod ollama;
pub mod ollama_manager;
pub mod post_processing;
pub mod preset;
pub mod provider;
pub mod resample;
pub mod settings;
pub mod state;
pub mod transcribe;
pub mod verbose;

pub use audio::{
    AudioChunk, AudioDeviceInfo, AudioRecorder, RecordingData, RecordingOutput, VadConfig,
    list_audio_devices, load_audio_file, load_audio_stdin,
};
pub use error::{AudioError, ProviderError, Result, WhisError};
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
#[cfg(feature = "local-transcription")]
pub use provider::{whisper_preload_model, whisper_set_keep_loaded, whisper_unload_model};
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
#[cfg(feature = "local-transcription")]
pub use transcribe::{LocalAudioChunk, parallel_transcribe_local};
pub use verbose::set_verbose;
