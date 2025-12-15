use crate::state::{AppState, RecordingState};
use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;
use whis_core::{AudioRecorder, RecordingOutput, Settings};

#[derive(serde::Serialize)]
pub struct StatusResponse {
    state: RecordingState,
    config_valid: bool,
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> StatusResponse {
    let recording_state = *state.recording_state.lock().unwrap();
    let settings = state.settings.lock().unwrap();
    let config_valid = settings.get_api_key().is_some();

    StatusResponse {
        state: recording_state,
        config_valid,
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().unwrap();
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let mut current = state.settings.lock().unwrap();
    *current = settings.clone();
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_api_key(key: String, provider: String) -> bool {
    match provider.as_str() {
        "openai" => key.starts_with("sk-") && key.len() > 20,
        "mistral" => key.len() > 20,
        _ => false,
    }
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut recording_state = state.recording_state.lock().unwrap();
    if *recording_state != RecordingState::Idle {
        return Err("Already recording or transcribing".to_string());
    }

    let mut recorder = AudioRecorder::new().map_err(|e| e.to_string())?;
    recorder.start_recording().map_err(|e| e.to_string())?;

    *state.recorder.lock().unwrap() = Some(recorder);
    *recording_state = RecordingState::Recording;

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Stop recording and get data
    let recording_data = {
        let mut recording_state = state.recording_state.lock().unwrap();
        if *recording_state != RecordingState::Recording {
            return Err("Not currently recording".to_string());
        }

        let mut recorder_guard = state.recorder.lock().unwrap();
        let recorder = recorder_guard
            .as_mut()
            .ok_or("No recorder available")?;

        let data = recorder.stop_recording().map_err(|e| e.to_string())?;
        *recorder_guard = None;
        *recording_state = RecordingState::Transcribing;
        data
    };

    // Get transcription config
    let (provider, api_key, language) = {
        let settings = state.settings.lock().unwrap();
        let provider = settings.provider.clone();
        let api_key = settings
            .get_api_key()
            .ok_or("No API key configured")?;
        let language = settings.language.clone();
        (provider, api_key, language)
    };

    // Finalize recording (convert to MP3)
    let output = tokio::task::spawn_blocking(move || recording_data.finalize())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // Transcribe (wrap blocking calls in spawn_blocking to avoid tokio panic)
    let text = match output {
        RecordingOutput::Single(data) => {
            tokio::task::spawn_blocking(move || {
                whis_core::transcribe_audio(&provider, &api_key, language.as_deref(), data)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
        }
        RecordingOutput::Chunked(chunks) => {
            whis_core::parallel_transcribe(&provider, &api_key, language.as_deref(), chunks, None)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    // Copy to clipboard using Tauri plugin
    app.clipboard()
        .write_text(&text)
        .map_err(|e| e.to_string())?;

    // Reset state
    {
        let mut recording_state = state.recording_state.lock().unwrap();
        *recording_state = RecordingState::Idle;
    }

    Ok(text)
}
