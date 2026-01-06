//! Realtime Transcription Provider Trait
//!
//! This module defines the trait and common patterns for realtime (WebSocket-based)
//! transcription providers. Realtime providers stream audio during recording for
//! lower latency, rather than buffering and uploading after recording stops.
//!
//! # Implementation Pattern
//!
//! All realtime providers should follow this structure in their `transcribe_stream` method:
//!
//! ```text
//! 1. Build WebSocket URL/request with auth headers
//! 2. Connect to WebSocket
//! 3. Configure session (if required by protocol)
//! 4. Spawn read task (collect_transcripts) with done_rx channel
//! 5. (Optional) Spawn keepalive task if required by provider
//! 6. Stream audio chunks in loop (convert f32 → PCM16, encode if needed)
//! 7. Send finalize message to flush buffer
//! 8. Signal done_tx to notify read task
//! 9. Wait for read task with timeout (30s)
//! 10. Close connection gracefully
//! ```
//!
//! # Read Task Pattern (collect_transcripts)
//!
//! The read task should have two phases:
//!
//! - **Phase 1 (Streaming):** Monitor for errors while audio is being sent.
//!   Use `tokio::select!` to check both `done_rx` and incoming messages.
//!   
//! - **Phase 2 (Post-Finalize):** After receiving done signal, wait for the
//!   final transcript with a short timeout (provider-specific).
//!
//! # Provider Differences
//!
//! | Aspect | OpenAI Realtime | Deepgram Realtime |
//! |--------|-----------------|-------------------|
//! | Sample Rate | 24kHz (resampling needed) | 16kHz (native) |
//! | Audio Format | Base64-encoded PCM16 | Raw binary PCM16 |
//! | Auth Header | `Authorization: Bearer {key}` | `Authorization: Token {key}` |
//! | Session Config | `session.update` message | Query parameters |
//! | Finalize | `commit` + `response.create` | `{"type":"Finalize"}` |
//! | KeepAlive | Not needed | Required (every 5s) |
//! | Final Event | `conversation.item.input_audio_transcription.completed` | `Results` with `from_finalize=true` |

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Trait for realtime (WebSocket-based) transcription providers.
///
/// Realtime providers stream audio during recording rather than buffering
/// and uploading after recording stops. This reduces latency but requires
/// a persistent WebSocket connection.
///
/// Providers implementing this trait should also implement `TranscriptionBackend`
/// to support batch transcription as a fallback for file input.
#[async_trait]
pub trait RealtimeTranscriptionBackend: Send + Sync {
    /// Transcribe audio from a channel of f32 samples (16kHz mono).
    ///
    /// Connects to the provider's WebSocket API and streams audio chunks
    /// as they arrive. Returns the final transcript when the channel closes.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Provider-specific API key
    /// * `audio_rx` - Channel receiving audio chunks as f32 samples at 16kHz
    /// * `language` - Optional language code (e.g., "en", "es")
    ///
    /// # Returns
    ///
    /// The final transcript text, or an error if transcription failed.
    ///
    /// # Implementation Notes
    ///
    /// Implementations should:
    /// - Handle resampling if needed (OpenAI requires 24kHz)
    /// - Convert f32 to PCM16 (multiply by i16::MAX, clamp to -1.0..1.0)
    /// - Follow the standard implementation pattern documented at module level
    async fn transcribe_stream(
        &self,
        api_key: &str,
        audio_rx: mpsc::Receiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String>;

    /// Required sample rate for this provider's WebSocket API.
    ///
    /// Input audio at 16kHz will be resampled to this rate if different.
    fn sample_rate(&self) -> u32;

    /// Whether this provider requires keepalive messages during streaming.
    ///
    /// Deepgram requires keepalive messages every ~5 seconds during silence.
    /// OpenAI does not require keepalive.
    fn requires_keepalive(&self) -> bool {
        false
    }
}
