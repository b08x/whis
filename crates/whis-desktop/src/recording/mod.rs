//! Recording Orchestration Module
//!
//! Manages the complete recording workflow:
//! - Configuration loading and validation
//! - Audio recording control (start/stop)
//! - Transcription pipeline (transcribe → post-process → clipboard)
//!
//! ## Architecture
//!
//! ```text
//! recording/
//! ├── config.rs      - Configuration loading from settings
//! ├── control.rs     - Start/stop recording logic
//! ├── pipeline.rs    - Transcription pipeline orchestration
//! └── mod.rs         - Public API (toggle, start, stop)
//! ```

pub mod config;
pub mod control;
pub mod pipeline;

// Re-export public APIs
pub use config::load_transcription_config;
pub use control::start_recording_sync;
pub use pipeline::stop_and_transcribe;

use crate::state::{AppState, RecordingState};
use tauri::{AppHandle, Manager};

/// Toggle recording state (start if idle, stop if recording)
/// Called from global shortcuts, tray menu, and IPC
pub fn toggle_recording(app: AppHandle) {
    let state = app.state::<AppState>();
    let current_state = *state.state.lock().unwrap();

    match current_state {
        RecordingState::Idle => {
            // Start recording
            if let Err(e) = start_recording_sync(&app, &state) {
                eprintln!("Failed to start recording: {e}");
            }
        }
        RecordingState::Recording => {
            // Stop recording and transcribe
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = stop_and_transcribe(&app_clone).await {
                    eprintln!("Failed to transcribe: {e}");
                }
            });
        }
        RecordingState::Transcribing => {
            // Already transcribing, ignore
        }
    }
}
