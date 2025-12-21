use std::sync::Mutex;
pub use whis_core::RecordingState;

pub struct AppState {
    pub recording_state: Mutex<RecordingState>,
}
