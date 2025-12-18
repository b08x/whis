use crate::state::{AppState, RecordingState, TranscriptionConfig};
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use whis_core::{
    AudioRecorder, DEFAULT_POLISH_PROMPT, Polisher, RecordingOutput, TranscriptionProvider,
    copy_to_clipboard, ollama, parallel_transcribe, polish, transcribe_audio,
};

// Static icons for each state (pre-loaded at compile time)
const ICON_IDLE: &[u8] = include_bytes!("../icons/icon-idle.png");
const ICON_RECORDING: &[u8] = include_bytes!("../icons/icon-recording.png");
const ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/icon-processing.png");

pub const TRAY_ID: &str = "whis-tray";

pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let record = MenuItem::with_id(app, "record", "Start Recording", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Whis", true, None::<&str>)?;

    // Store the record menu item for later updates
    if let Some(state) = app.try_state::<AppState>() {
        *state.record_menu_item.lock().unwrap() = Some(record.clone());
    }

    let menu = Menu::with_items(app, &[&record, &sep, &settings, &sep, &quit])?;

    // Use image crate for consistent rendering (same as set_tray_icon)
    let idle_bytes = include_bytes!("../icons/icon-idle.png");
    let img = image::load_from_memory(idle_bytes).expect("Failed to load idle icon");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let idle_icon = Image::new_owned(rgba.into_raw(), width, height);

    // Use app cache dir for tray icons so Flatpak host can access them
    // (default /tmp is sandboxed and GNOME AppIndicator can't read it)
    let cache_dir = app
        .path()
        .app_cache_dir()
        .expect("Failed to get app cache dir");

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(idle_icon)
        .temp_dir_path(cache_dir)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Whis - Click to record")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "record" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    toggle_recording(app_clone);
                });
            }
            "settings" => {
                open_settings_window(app.clone());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::TrayIconEvent;
            if let TrayIconEvent::Click { button, .. } = event
                && button == tauri::tray::MouseButton::Left
            {
                let app_handle = tray.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    toggle_recording(app_handle);
                });
            }
        })
        .build(app)?;

    Ok(())
}

fn open_settings_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let window = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("index.html".into()))
        .title("Whis Settings")
        .inner_size(600.0, 400.0)
        .min_inner_size(400.0, 300.0)
        .resizable(true)
        .decorations(false)
        .transparent(true)
        .build();

    // Fix Wayland window dragging by unsetting GTK titlebar
    // On Wayland, GTK's titlebar is required for dragging, but decorations(false)
    // removes it. By calling set_titlebar(None), we restore drag functionality
    // while keeping our custom chrome.
    match window {
        Ok(window) => {
            #[cfg(target_os = "linux")]
            {
                use gtk::prelude::GtkWindowExt;
                if let Ok(gtk_window) = window.gtk_window() {
                    gtk_window.set_titlebar(Option::<&gtk::Widget>::None);
                }
            }
            let _ = window.show();
        }
        Err(e) => eprintln!("Failed to create settings window: {e}"),
    }
}

fn toggle_recording(app: AppHandle) {
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

fn start_recording_sync(app: &AppHandle, state: &AppState) -> Result<(), String> {
    // Load transcription config if not already loaded
    {
        let mut config_guard = state.transcription_config.lock().unwrap();
        if config_guard.is_none() {
            let settings = state.settings.lock().unwrap();
            let provider = settings.provider.clone();

            // Get API key/model path/URL based on provider type
            let api_key = match provider {
                TranscriptionProvider::LocalWhisper => {
                    settings.get_whisper_model_path().ok_or_else(|| {
                        "Whisper model path not configured. Add it in Settings.".to_string()
                    })?
                }
                TranscriptionProvider::RemoteWhisper => {
                    settings.get_remote_whisper_url().ok_or_else(|| {
                        "Remote Whisper URL not configured. Add it in Settings.".to_string()
                    })?
                }
                _ => settings.get_api_key().ok_or_else(|| {
                    format!("No {} API key configured. Add it in Settings.", provider)
                })?,
            };

            let language = settings.language.clone();

            *config_guard = Some(TranscriptionConfig {
                provider,
                api_key,
                language,
            });
        }
    }

    // Start recording
    let mut recorder = AudioRecorder::new().map_err(|e| e.to_string())?;
    recorder.start_recording().map_err(|e| e.to_string())?;

    *state.recorder.lock().unwrap() = Some(recorder);
    *state.state.lock().unwrap() = RecordingState::Recording;

    // Update tray
    update_tray(app, RecordingState::Recording);
    println!("Recording started...");

    Ok(())
}

async fn stop_and_transcribe(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    // Update state to transcribing
    {
        *state.state.lock().unwrap() = RecordingState::Transcribing;
    }
    update_tray(app, RecordingState::Transcribing);
    println!("Transcribing...");

    // Run transcription with guaranteed state cleanup on any error
    let result = do_transcription(app, &state).await;

    // Always reset state, regardless of success or failure
    {
        *state.state.lock().unwrap() = RecordingState::Idle;
    }
    update_tray(app, RecordingState::Idle);

    result
}

/// Inner transcription logic - extracted so we can guarantee state cleanup
async fn do_transcription(app: &AppHandle, state: &AppState) -> Result<(), String> {
    // Get recorder and config
    let mut recorder = state
        .recorder
        .lock()
        .unwrap()
        .take()
        .ok_or("No active recording")?;

    let (provider, api_key, language) = {
        let config = state.transcription_config.lock().unwrap();
        let config_ref = config.as_ref().ok_or("Transcription config not loaded")?;
        (
            config_ref.provider.clone(),
            config_ref.api_key.clone(),
            config_ref.language.clone(),
        )
    };

    // Finalize recording (synchronous file encoding)
    let audio_result = recorder.finalize_recording().map_err(|e| e.to_string())?;

    // Transcribe
    let transcription = match audio_result {
        // transcribe_audio is synchronous (blocking HTTP), so we should wrap it in spawn_blocking
        // to avoid blocking the async runtime
        RecordingOutput::Single(data) => {
            let provider = provider.clone();
            let api_key = api_key.clone();
            let language = language.clone();
            tauri::async_runtime::spawn_blocking(move || {
                transcribe_audio(&provider, &api_key, language.as_deref(), data)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
        }
        RecordingOutput::Chunked(chunks) => {
            // parallel_transcribe is async, so we can await it directly
            parallel_transcribe(&provider, &api_key, language.as_deref(), chunks, None)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    // Extract polishing config and clipboard method from settings (lock scope limited)
    let (polish_config, clipboard_method) = {
        let settings = state.settings.lock().unwrap();
        let clipboard_method = settings.clipboard_method.clone();
        let polish_config = if settings.polisher != Polisher::None {
            let polisher = settings.polisher.clone();
            let prompt = settings
                .polish_prompt
                .clone()
                .unwrap_or_else(|| DEFAULT_POLISH_PROMPT.to_string());
            let ollama_model = settings.ollama_model.clone();

            // Get API key or URL based on polisher type
            let api_key_or_url = if polisher.requires_api_key() {
                settings.get_polisher_api_key()
            } else if polisher == Polisher::Ollama {
                let ollama_url = settings
                    .get_ollama_url()
                    .unwrap_or_else(|| ollama::DEFAULT_OLLAMA_URL.to_string());
                Some(ollama_url)
            } else {
                None
            };

            api_key_or_url.map(|key_or_url| (polisher, prompt, ollama_model, key_or_url))
        } else {
            None
        };
        (polish_config, clipboard_method)
    };

    // Apply polishing if enabled (outside of lock scope)
    let final_text = if let Some((polisher, prompt, ollama_model, key_or_url)) = polish_config {
        // Auto-start Ollama if needed (and installed)
        if polisher == Polisher::Ollama {
            if let Err(e) = ollama::ensure_ollama_running(&key_or_url) {
                let warning = format!("Ollama: {e}");
                eprintln!("Polish warning: {warning}");
                let _ = app.emit("polish-warning", &warning);
                // Skip polishing, return raw transcription
                copy_to_clipboard(&transcription, clipboard_method).map_err(|e| e.to_string())?;
                println!(
                    "Done (unpolished): {}",
                    &transcription[..transcription.len().min(50)]
                );
                let _ = app.emit("transcription-complete", &transcription);
                return Ok(());
            }
        }

        println!("Polishing...");
        let _ = app.emit("polish-started", ());

        let model = if polisher == Polisher::Ollama {
            ollama_model.as_deref()
        } else {
            None
        };

        match polish(&transcription, &polisher, &key_or_url, &prompt, model).await {
            Ok(polished) => polished,
            Err(e) => {
                let warning = e.to_string();
                eprintln!("Polish warning: {warning}");
                let _ = app.emit("polish-warning", &warning);
                transcription
            }
        }
    } else {
        transcription
    };

    // Copy to clipboard
    copy_to_clipboard(&final_text, clipboard_method).map_err(|e| e.to_string())?;

    println!("Done: {}", &final_text[..final_text.len().min(50)]);

    // Emit event to frontend so it knows transcription completed
    let _ = app.emit("transcription-complete", &final_text);

    Ok(())
}

fn update_tray(app: &AppHandle, new_state: RecordingState) {
    // Update menu item text using stored reference
    let app_state = app.state::<AppState>();
    if let Some(ref menu_item) = *app_state.record_menu_item.lock().unwrap() {
        let text = match new_state {
            RecordingState::Idle => "Start Recording",
            RecordingState::Recording => "Stop Recording",
            RecordingState::Transcribing => "Transcribing...",
        };
        let _ = menu_item.set_text(text);
        let _ = menu_item.set_enabled(new_state != RecordingState::Transcribing);
    }

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        // Update tooltip
        let tooltip = match new_state {
            RecordingState::Idle => "Whis - Click to record",
            RecordingState::Recording => "Whis - Recording... Click to stop",
            RecordingState::Transcribing => "Whis - Transcribing...",
        };
        let _ = tray.set_tooltip(Some(tooltip));

        // Set static icon based on state
        let icon = match new_state {
            RecordingState::Idle => ICON_IDLE,
            RecordingState::Recording => ICON_RECORDING,
            RecordingState::Transcribing => ICON_TRANSCRIBING,
        };
        set_tray_icon(&tray, icon);
    }
}

fn set_tray_icon(tray: &tauri::tray::TrayIcon, icon_bytes: &[u8]) {
    match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let icon = Image::new_owned(rgba.into_raw(), width, height);
            if let Err(e) = tray.set_icon(Some(icon)) {
                eprintln!("Failed to set tray icon: {e}");
            }
        }
        Err(e) => eprintln!("Failed to load tray icon: {e}"),
    }
}

/// Public wrapper for toggle_recording to be called from global shortcuts
pub fn toggle_recording_public(app: AppHandle) {
    toggle_recording(app);
}
