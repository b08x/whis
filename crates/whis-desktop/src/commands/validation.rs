//! API Key Validation Commands
//!
//! Provides Tauri commands for validating API keys from various transcription providers.
//! These commands perform basic format validation before saving to settings.

#[tauri::command]
pub fn validate_openai_api_key(api_key: String) -> Result<bool, String> {
    // Validate format: OpenAI keys start with "sk-"
    if api_key.is_empty() {
        return Ok(true); // Empty is valid (will fall back to env var)
    }

    if !api_key.starts_with("sk-") {
        return Err("Invalid key format. OpenAI keys start with 'sk-'".to_string());
    }

    Ok(true)
}

#[tauri::command]
pub fn validate_mistral_api_key(api_key: String) -> Result<bool, String> {
    // Empty is valid (will fall back to env var)
    if api_key.is_empty() {
        return Ok(true);
    }

    // Basic validation: Mistral keys should be reasonably long
    let trimmed = api_key.trim();
    if trimmed.len() < 20 {
        return Err("Invalid Mistral API key: key appears too short".to_string());
    }

    Ok(true)
}

#[tauri::command]
pub fn validate_groq_api_key(api_key: String) -> Result<bool, String> {
    if api_key.is_empty() {
        return Ok(true); // Empty is valid (will fall back to env var)
    }

    if !api_key.starts_with("gsk_") {
        return Err("Invalid key format. Groq keys start with 'gsk_'".to_string());
    }

    Ok(true)
}

#[tauri::command]
pub fn validate_deepgram_api_key(api_key: String) -> Result<bool, String> {
    if api_key.is_empty() {
        return Ok(true);
    }

    if api_key.trim().len() < 20 {
        return Err("Invalid Deepgram API key: key appears too short".to_string());
    }

    Ok(true)
}

#[tauri::command]
pub fn validate_elevenlabs_api_key(api_key: String) -> Result<bool, String> {
    if api_key.is_empty() {
        return Ok(true);
    }

    if api_key.trim().len() < 20 {
        return Err("Invalid ElevenLabs API key: key appears too short".to_string());
    }

    Ok(true)
}
