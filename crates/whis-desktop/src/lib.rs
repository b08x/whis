mod commands;
pub mod recording;
pub mod shortcuts;
mod state;
pub mod tray;
mod window;

use tauri::{Emitter, Manager};
use whis_core::Settings;

pub fn run(start_in_tray: bool) {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
if !args.contains(&"--start-in-tray".to_string()) {
                match app.get_webview_window("main") {
                    Some(window) => {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    None => {
                        let _ = window::show_main_window(app);
                    }
                }
            }
        }))
        .plugin(tauri_plugin_process::init())
        .setup(move |app| {
            // Load settings from disk
            let loaded_settings = Settings::load();

            // Initialize state with tray availability
            app.manage(state::AppState::new(loaded_settings, true));

            // Initialize system tray (optional - may fail on tray-less environments)
            let _tray_available = match tray::setup_tray(app) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Tray unavailable: {e}. Running in window mode.");
                    false
                }
            };

            // Setup global shortcuts (hybrid: Tauri plugin / Portal / CLI fallback)
            shortcuts::setup_shortcuts(app);

            // Start IPC listener for --toggle CLI commands
            shortcuts::start_ipc_listener(app.handle().clone());

            // Only show main window if NOT starting in tray
            if !start_in_tray {
                window::show_main_window(app.handle())?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            use tauri::WindowEvent;
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    // Prevent immediate close - emit event to frontend for graceful shutdown
                    api.prevent_close();
                    let _ = window.emit("window-close-requested", ());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            // System commands (Phase 1)
            commands::get_toggle_command,
            commands::can_reopen_window,
            commands::list_audio_devices,
            commands::exit_app,
            // Validation commands (Phase 1)
            commands::validate_openai_api_key,
            commands::validate_mistral_api_key,
            commands::validate_groq_api_key,
            commands::validate_deepgram_api_key,
            commands::validate_elevenlabs_api_key,
            // TODO: Phase 3 - migrate these commands to modular structure
            commands::get_status,
            commands::is_api_configured,
            commands::get_settings,
            commands::save_settings,
            commands::shortcut_backend,
            commands::configure_shortcut,
            commands::configure_shortcut_with_trigger,
            commands::portal_shortcut,
            commands::reset_shortcut,
            commands::portal_bind_error,
            commands::toggle_recording,
            commands::download_whisper_model,
            commands::test_ollama_connection,
            commands::list_ollama_models,
            commands::pull_ollama_model,
            commands::start_ollama,
            commands::check_ollama_status,
            commands::check_config_readiness,
            commands::get_whisper_models,
            commands::is_whisper_model_valid,
            commands::get_parakeet_models,
            commands::is_parakeet_model_valid,
            commands::download_parakeet_model,
            commands::get_active_download,
            commands::list_presets,
            commands::apply_preset,
            commands::get_active_preset,
            commands::set_active_preset,
            commands::get_preset_details,
            commands::create_preset,
            commands::update_preset,
            commands::delete_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
