//! Recording Configuration
//!
//! Handles loading and validation of transcription configuration from settings.

use crate::state::{AppState, TranscriptionConfig};
use whis_core::TranscriptionProvider;

/// Load transcription configuration from settings
/// Returns error if required API key or model path is missing
pub fn load_transcription_config(state: &AppState) -> Result<TranscriptionConfig, String> {
    let settings = state.settings.lock().unwrap();
    let provider = settings.transcription.provider.clone();

    // Get API key/model path based on provider type
    let api_key = match provider {
        TranscriptionProvider::LocalWhisper => settings
            .transcription
            .whisper_model_path()
            .ok_or_else(|| "Whisper model path not configured. Add it in Settings.".to_string())?,
        TranscriptionProvider::LocalParakeet => settings
            .transcription
            .parakeet_model_path()
            .ok_or_else(|| "Parakeet model not configured. Add it in Settings.".to_string())?,
        _ => settings
            .transcription
            .api_key_from_settings()
            .ok_or_else(|| format!("No {} API key configured. Add it in Settings.", provider))?,
    };

    let language = settings.transcription.language.clone();

    Ok(TranscriptionConfig {
        provider,
        api_key,
        language,
    })
}
