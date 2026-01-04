//! Shared implementation for OpenAI-compatible transcription APIs.
//!
//! This module provides helper functions for transcription providers that use
//! the OpenAI Whisper API format:
//! - OpenAI Whisper API
//! - Groq Whisper API
//! - Mistral Voxtral API
//!
//! All three providers use identical request/response formats:
//! - Multipart form upload with `model` and `file` fields
//! - Authorization via `Bearer` token
//! - JSON response with `text` field

use anyhow::{Context, Result};
use serde::Deserialize;

use super::super::{
    DEFAULT_TIMEOUT_SECS, TranscriptionRequest, TranscriptionResult, TranscriptionStage,
};

/// Response structure for OpenAI-compatible APIs
#[derive(Deserialize)]
struct OpenAICompatibleResponse {
    text: String,
}

/// Transcribe audio using an OpenAI-compatible API (synchronous).
///
/// # Parameters
/// - `api_url`: The API endpoint URL (e.g., "https://api.openai.com/v1/audio/transcriptions")
/// - `model`: The model name to use (e.g., "whisper-1")
/// - `api_key`: Bearer token for authentication
/// - `request`: Transcription request with audio data and options
///
/// # Returns
/// Transcription result containing the text transcript
pub(crate) fn openai_compatible_transcribe_sync(
    api_url: &str,
    model: &str,
    api_key: &str,
    request: TranscriptionRequest,
) -> Result<TranscriptionResult> {
    // Report uploading stage
    request.report(TranscriptionStage::Uploading);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .context("Failed to create HTTP client")?;

    let mut form = reqwest::blocking::multipart::Form::new()
        .text("model", model.to_string())
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(request.audio_data.clone())
                .file_name(request.filename.clone())
                .mime_str(&request.mime_type)?,
        );

    if let Some(lang) = request.language.clone() {
        form = form.text("language", lang);
    }

    // Report transcribing stage (request sent, waiting for response)
    request.report(TranscriptionStage::Transcribing);

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .context("Failed to send request")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("API error ({status}): {error_text}");
    }

    let text = response.text().context("Failed to get response text")?;
    let resp: OpenAICompatibleResponse =
        serde_json::from_str(&text).context("Failed to parse API response")?;

    Ok(TranscriptionResult { text: resp.text })
}

/// Transcribe audio using an OpenAI-compatible API (asynchronous).
///
/// # Parameters
/// - `client`: Shared reqwest client for connection pooling
/// - `api_url`: The API endpoint URL
/// - `model`: The model name to use
/// - `api_key`: Bearer token for authentication
/// - `request`: Transcription request with audio data and options
///
/// # Returns
/// Transcription result containing the text transcript
pub(crate) async fn openai_compatible_transcribe_async(
    client: &reqwest::Client,
    api_url: &str,
    model: &str,
    api_key: &str,
    request: TranscriptionRequest,
) -> Result<TranscriptionResult> {
    // Report uploading stage
    request.report(TranscriptionStage::Uploading);

    let mut form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(request.audio_data.clone())
                .file_name(request.filename.clone())
                .mime_str(&request.mime_type)?,
        );

    if let Some(lang) = request.language.clone() {
        form = form.text("language", lang);
    }

    // Report transcribing stage
    request.report(TranscriptionStage::Transcribing);

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .context("Failed to send request")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("API error ({status}): {error_text}");
    }

    let text = response
        .text()
        .await
        .context("Failed to get response text")?;
    let resp: OpenAICompatibleResponse =
        serde_json::from_str(&text).context("Failed to parse API response")?;

    Ok(TranscriptionResult { text: resp.text })
}
