mod commands;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            app.manage(AppState {
                recording_state: Mutex::new(state::RecordingState::Idle),
                recorder: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_recording,
            commands::stop_recording,
            commands::validate_api_key,
            commands::list_presets,
            commands::get_preset_details,
            commands::set_active_preset,
            commands::get_active_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
