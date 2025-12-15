mod commands;
mod state;

use state::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState {
            recording_state: Mutex::new(state::RecordingState::Idle),
            recorder: Mutex::new(None),
            settings: Mutex::new(whis_core::Settings::load()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_settings,
            commands::save_settings,
            commands::start_recording,
            commands::stop_recording,
            commands::validate_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
