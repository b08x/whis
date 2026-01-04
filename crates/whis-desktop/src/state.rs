use std::sync::Mutex;
use tauri::menu::MenuItem;
pub use whis_core::RecordingState;
use whis_core::{AudioRecorder, Settings, TranscriptionProvider};

/// Cached transcription configuration (provider + API key + language)
pub struct TranscriptionConfig {
    pub provider: TranscriptionProvider,
    pub api_key: String,
    pub language: Option<String>,
}

/// Active model download state (persists across window close/reopen)
#[derive(Clone, Debug)]
pub struct DownloadState {
    pub model_name: String,
    pub model_type: String, // "whisper" or "parakeet"
    pub downloaded: u64,
    pub total: u64,
}

pub struct AppState {
    pub state: Mutex<RecordingState>,
    pub recorder: Mutex<Option<AudioRecorder>>,
    pub transcription_config: Mutex<Option<TranscriptionConfig>>,
    pub record_menu_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    pub settings: Mutex<Settings>,
    /// The actual shortcut binding from the XDG Portal (Wayland only)
    pub portal_shortcut: Mutex<Option<String>>,
    /// Error message if portal shortcut binding failed
    pub portal_bind_error: Mutex<Option<String>>,
    /// Whether system tray is available
    pub tray_available: Mutex<bool>,
    /// Active model download (if any)
    pub active_download: Mutex<Option<DownloadState>>,
}

impl AppState {
    pub fn new(settings: Settings, tray_available: bool) -> Self {
        Self {
            state: Mutex::new(RecordingState::Idle),
            recorder: Mutex::new(None),
            transcription_config: Mutex::new(None),
            record_menu_item: Mutex::new(None),
            settings: Mutex::new(settings),
            portal_shortcut: Mutex::new(None),
            portal_bind_error: Mutex::new(None),
            tray_available: Mutex::new(tray_available),
            active_download: Mutex::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Settings::default(), false)
    }
}
