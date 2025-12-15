use std::sync::Mutex;
use whis_core::{AudioRecorder, Settings};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Idle,
    Recording,
    Transcribing,
}

pub struct AppState {
    pub recording_state: Mutex<RecordingState>,
    pub recorder: Mutex<Option<AudioRecorder>>,
    pub settings: Mutex<Settings>,
}
