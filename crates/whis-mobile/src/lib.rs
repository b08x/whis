//! Whis Mobile - Voice-to-Text for Android/iOS
//!
//! ## Architecture
//!
//! ```text
//! whis-mobile/
//! ├── commands/           - Tauri command handlers (organized by domain)
//! │   ├── system.rs       - Status, validation
//! │   ├── presets.rs      - Preset CRUD
//! │   └── recording.rs    - Audio transcription (batch + streaming)
//! ├── recording/          - Recording business logic
//! │   ├── config.rs       - Transcription config from Tauri store
//! │   └── pipeline.rs     - Post-processing, clipboard, events
//! ├── state.rs            - Application state
//! └── lib.rs              - App entry point
//! ```
//!
//! ## Feature Differences from Desktop
//!
//! - **No local transcription** - Mobile uses cloud providers only
//! - **No VAD** - Voice Activity Detection requires ONNX Runtime (no Android binaries)
//! - **No system tray** - Mobile has different navigation paradigm
//! - **Tauri store** - Uses Tauri plugin for settings (not whis-core::Settings)

mod commands;
mod recording;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_floating_bubble::init())
        .setup(|app| {
            app.manage(AppState::new());

            // Initialize rustls-platform-verifier for Android HTTPS requests
            // This must be done before any reqwest calls are made
            // See: https://github.com/tauri-apps/tauri/issues/13267
            #[cfg(target_os = "android")]
            {
                app.get_webview_window("main")
                    .expect("no main window")
                    .with_webview(|webview| {
                        webview.jni_handle().exec(|env, context, _webview| {
                            let loader = env
                                .call_method(
                                    &context,
                                    "getClassLoader",
                                    "()Ljava/lang/ClassLoader;",
                                    &[],
                                )
                                .unwrap()
                                .l()
                                .unwrap();

                            rustls_platform_verifier::android::init_with_refs(
                                env.get_java_vm().unwrap(),
                                env.new_global_ref(&context).unwrap(),
                                env.new_global_ref(loader).unwrap(),
                            );
                        });
                    })?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // System commands
            commands::get_status,
            commands::validate_api_key,
            commands::warmup_connections,
            // Preset commands
            commands::list_presets,
            commands::get_preset_details,
            commands::set_active_preset,
            commands::get_active_preset,
            commands::create_preset,
            commands::update_preset,
            commands::delete_preset,
            // Recording commands (batch - legacy)
            commands::transcribe_audio,
            // Recording commands (progressive - matches CLI/desktop)
            commands::start_recording,
            commands::send_audio_chunk,
            commands::stop_recording,
            // Recording commands (OpenAI Realtime streaming)
            commands::transcribe_streaming_start,
            commands::transcribe_streaming_send_chunk,
            commands::transcribe_streaming_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
