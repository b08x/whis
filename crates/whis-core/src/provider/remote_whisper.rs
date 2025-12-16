//! Remote Whisper transcription provider
//!
//! Connects to any OpenAI-compatible whisper server (e.g., faster-whisper-server).
//! Enables self-hosted transcription for privacy or offline use.
//!
//! ## Authentication
//!
//! This provider sends a placeholder "no-auth" bearer token. Most self-hosted
//! whisper servers (like faster-whisper-server) don't require authentication,
//! but the OpenAI-compatible API format expects a bearer token header.
//!
//! If your server requires authentication, you'll need to modify this provider
//! or use a proxy that adds the appropriate credentials.

use anyhow::Result;
use async_trait::async_trait;

use super::{
    TranscriptionBackend, TranscriptionRequest, TranscriptionResult,
    openai_compatible_transcribe_async, openai_compatible_transcribe_sync,
};

/// Default model name for faster-whisper-server
const DEFAULT_MODEL: &str = "Systran/faster-whisper-small";

/// Remote whisper server transcription provider
#[derive(Debug, Default, Clone)]
pub struct RemoteWhisperProvider;

#[async_trait]
impl TranscriptionBackend for RemoteWhisperProvider {
    fn name(&self) -> &'static str {
        "remote-whisper"
    }

    fn display_name(&self) -> &'static str {
        "Remote Whisper"
    }

    fn transcribe_sync(
        &self,
        server_url: &str, // Repurposed: URL to whisper server (e.g., http://localhost:8765)
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        let api_url = build_api_url(server_url)?;
        // No auth needed for self-hosted, but API expects a bearer token
        openai_compatible_transcribe_sync(&api_url, DEFAULT_MODEL, "no-auth", request)
    }

    async fn transcribe_async(
        &self,
        client: &reqwest::Client,
        server_url: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        let api_url = build_api_url(server_url)?;
        openai_compatible_transcribe_async(client, &api_url, DEFAULT_MODEL, "no-auth", request)
            .await
    }
}

/// Build the full API URL from the server base URL
fn build_api_url(server_url: &str) -> Result<String> {
    if server_url.is_empty() {
        anyhow::bail!(
            "Remote whisper server URL not configured.\n\
             Set with: whis config --remote-whisper-url http://localhost:8765"
        );
    }

    // Validate URL format
    let trimmed = server_url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        anyhow::bail!(
            "Invalid remote whisper URL: must start with http:// or https://\n\
             Got: {}\n\
             Example: whis config --remote-whisper-url http://localhost:8765",
            trimmed
        );
    }

    // Basic validation: ensure there's a host after the scheme
    let after_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or("");
    if after_scheme.is_empty() || after_scheme.starts_with('/') {
        anyhow::bail!(
            "Invalid remote whisper URL: missing host\n\
             Got: {}\n\
             Example: whis config --remote-whisper-url http://localhost:8765",
            trimmed
        );
    }

    // Normalize URL: remove trailing slash, append transcriptions endpoint
    let base = trimmed.trim_end_matches('/');
    Ok(format!("{}/v1/audio/transcriptions", base))
}
