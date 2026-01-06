//! Deepgram Live Streaming API transcription provider
//!
//! Uses WebSocket to stream audio in real-time for lower latency transcription.
//! Simpler and faster than OpenAI Realtime API.
//!
//! Key advantages over OpenAI Realtime:
//! - No base64 encoding overhead (sends raw binary)
//! - No resampling needed (16kHz native vs OpenAI's 24kHz)
//! - Simpler protocol (just send binary frames vs complex message types)
//! - Lower latency (~150ms vs 300-500ms)
//! - Supports interim results for progressive transcription

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use futures_util::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        http::header::{AUTHORIZATION, HeaderValue},
    },
};

use super::{
    DeepgramProvider, RealtimeTranscriptionBackend, TranscriptionBackend, TranscriptionRequest,
    TranscriptionResult,
};

const WS_URL: &str = "wss://api.deepgram.com/v1/listen";
const MODEL: &str = "nova-2";
const SAMPLE_RATE: u32 = 16000;
const KEEPALIVE_INTERVAL_SECS: u64 = 5;

/// Deepgram Live Streaming provider
///
/// Streams audio via WebSocket for real-time, low-latency transcription.
/// Uses the same API key as batch Deepgram (DEEPGRAM_API_KEY).
#[derive(Debug, Default, Clone)]
pub struct DeepgramRealtimeProvider;

// Response message types

#[derive(Deserialize, Debug)]
struct DeepgramEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    is_final: bool,
    #[serde(default)]
    #[allow(dead_code)]
    speech_final: bool,
    #[serde(default)]
    channel: Option<Channel>,
    #[serde(default)]
    description: Option<String>,
    /// Set to true when this result is from a Finalize message
    #[serde(default)]
    from_finalize: bool,
}

#[derive(Deserialize, Debug)]
struct Channel {
    alternatives: Vec<Alternative>,
}

#[derive(Deserialize, Debug)]
struct Alternative {
    transcript: String,
    #[allow(dead_code)]
    confidence: f64,
}

impl DeepgramRealtimeProvider {
    /// Transcribe audio from a channel of f32 samples (16kHz mono)
    ///
    /// Connects to Deepgram Live Streaming API via WebSocket and streams audio chunks
    /// as they arrive. Returns the final transcript when the channel closes.
    async fn transcribe_stream_impl(
        api_key: &str,
        mut audio_rx: mpsc::Receiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String> {
        // 1. Build WebSocket URL with query params
        let mut url = format!(
            "{WS_URL}?model={MODEL}&encoding=linear16&sample_rate={SAMPLE_RATE}\
             &channels=1&smart_format=true&interim_results=true"
        );

        if let Some(lang) = language {
            url.push_str(&format!("&language={}", lang));
        }

        // 2. Build request with Authorization header
        let mut request = url.into_client_request()?;
        request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Token {api_key}"))?,
        );

        // 3. Connect WebSocket
        let (ws_stream, _response) = connect_async(request)
            .await
            .context("Failed to connect to Deepgram Live Streaming API")?;

        let (write, read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));

        // 4. Create done channel to signal when Finalize is sent
        let (done_tx, done_rx) = oneshot::channel();

        // 5. Spawn read task to collect transcripts
        let read_handle = tokio::spawn(async move { collect_transcripts(read, done_rx).await });

        // 6. Spawn keepalive task
        let (keepalive_cancel_tx, keepalive_cancel_rx) = oneshot::channel();
        let keepalive_handle = tokio::spawn({
            let write = Arc::clone(&write);
            async move { keepalive_task(write, keepalive_cancel_rx).await }
        });

        // 7. Stream audio chunks as binary frames
        let mut chunk_count = 0;
        let mut total_samples = 0;

        while let Some(samples) = audio_rx.recv().await {
            if samples.is_empty() {
                continue;
            }

            chunk_count += 1;
            total_samples += samples.len();

            // Convert f32 to PCM16 i16
            let pcm16: Vec<i16> = samples
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .collect();

            // Convert to bytes (little-endian)
            let bytes: Vec<u8> = pcm16.iter().flat_map(|&s| s.to_le_bytes()).collect();

            // Send as binary WebSocket message (NOT base64!)
            write
                .lock()
                .await
                .send(Message::Binary(bytes.into()))
                .await
                .context("Failed to send audio chunk")?;
        }

        if crate::verbose::is_verbose() {
            eprintln!(
                "[deepgram-realtime] Sent {} chunks, {} total samples",
                chunk_count, total_samples
            );
        }

        // 8. Cancel keepalive task
        let _ = keepalive_cancel_tx.send(());
        let _ = keepalive_handle.await;

        // 9. Send Finalize message to flush buffer
        write
            .lock()
            .await
            .send(Message::Text(r#"{"type":"Finalize"}"#.to_string().into()))
            .await
            .context("Failed to send Finalize message")?;

        // 10. Signal read task that Finalize was sent
        let _ = done_tx.send(());

        // 11. Wait for final transcript with timeout
        let transcript_result = timeout(Duration::from_secs(30), read_handle).await;

        // 12. Close connection gracefully
        let _ = write.lock().await.send(Message::Close(None)).await;

        match transcript_result {
            Ok(Ok(Ok(transcript))) => Ok(transcript),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(e)) => Err(anyhow!("Read task panicked: {e}")),
            Err(_) => Err(anyhow!("Timeout waiting for transcription result")),
        }
    }

    /// Transcribe audio from a channel of f32 samples (16kHz mono)
    ///
    /// This is a convenience method that delegates to the trait implementation.
    pub async fn transcribe_stream(
        api_key: &str,
        audio_rx: mpsc::Receiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String> {
        Self::transcribe_stream_impl(api_key, audio_rx, language).await
    }
}

/// KeepAlive task that sends periodic messages during silence
///
/// Deepgram requires KeepAlive or audio data within 10 seconds.
/// This task sends KeepAlive every 5 seconds to prevent timeout.
async fn keepalive_task<W>(write: Arc<Mutex<W>>, mut cancel_rx: oneshot::Receiver<()>) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let mut interval = tokio::time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if crate::verbose::is_verbose() {
                    eprintln!("[deepgram-realtime] Sending KeepAlive");
                }

                let msg = r#"{"type":"KeepAlive"}"#;
                if write.lock().await.send(Message::Text(msg.to_string().into())).await.is_err() {
                    break;
                }
            }
            _ = &mut cancel_rx => {
                if crate::verbose::is_verbose() {
                    eprintln!("[deepgram-realtime] KeepAlive task cancelled");
                }
                break;
            }
        }
    }

    Ok(())
}

/// Timeout for waiting for final results after Finalize is sent.
/// Deepgram docs say from_finalize is not guaranteed, so we use a short timeout.
const POST_FINALIZE_TIMEOUT_MS: u64 = 500;

/// Collect final transcripts from WebSocket messages
///
/// Two-phase approach (similar to OpenAI implementation):
/// - Phase 1: During streaming, collect final transcripts as they arrive
/// - Phase 2: After done signal (Finalize sent), wait for from_finalize=true
///   or use a short timeout since from_finalize is not guaranteed by Deepgram
///
/// This avoids waiting for WebSocket close, which can take many seconds.
async fn collect_transcripts<S>(mut read: S, mut done_rx: oneshot::Receiver<()>) -> Result<String>
where
    S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let mut final_transcript = String::new();

    // Phase 1: Collect transcripts during streaming
    loop {
        tokio::select! {
            // Check if main task signaled that Finalize was sent
            _ = &mut done_rx => {
                if crate::verbose::is_verbose() {
                    eprintln!("[deepgram-realtime] Finalize sent, switching to post-finalize phase");
                }
                // Break to phase 2
                break;
            }

            // Process WebSocket messages
            msg = read.next() => {
                if let Some(result) = process_message(msg, &mut final_transcript)? {
                    return Ok(result);
                }
            }
        }
    }

    // Phase 2: Wait for final results with timeout
    // Use a short timeout since from_finalize is not guaranteed by Deepgram
    let timeout_duration = Duration::from_millis(POST_FINALIZE_TIMEOUT_MS);
    let deadline = tokio::time::Instant::now() + timeout_duration;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            if crate::verbose::is_verbose() {
                eprintln!(
                    "[deepgram-realtime] Post-finalize timeout, returning with current transcript"
                );
            }
            return Ok(final_transcript.trim().to_string());
        }

        tokio::select! {
            _ = tokio::time::sleep(remaining) => {
                if crate::verbose::is_verbose() {
                    eprintln!("[deepgram-realtime] Post-finalize timeout, returning with current transcript");
                }
                return Ok(final_transcript.trim().to_string());
            }

            msg = read.next() => {
                if let Some(result) = process_message(msg, &mut final_transcript)? {
                    return Ok(result);
                }
                // Continue waiting - don't reset the deadline, just process more messages
            }
        }
    }
}

/// Process a single WebSocket message.
/// Returns Ok(Some(transcript)) if we should return immediately,
/// Ok(None) to continue processing, or Err on error.
fn process_message(
    msg: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    final_transcript: &mut String,
) -> Result<Option<String>> {
    match msg {
        Some(Ok(Message::Text(text))) => {
            let event: DeepgramEvent =
                serde_json::from_str(&text).context("Failed to parse Deepgram event")?;

            if crate::verbose::is_verbose() && event.event_type != "Metadata" {
                eprintln!(
                    "[deepgram-realtime] event: {} (is_final={}, from_finalize={})",
                    event.event_type, event.is_final, event.from_finalize
                );
            }

            match event.event_type.as_str() {
                "Results" => {
                    // Only collect final results (ignore interim results where is_final=false)
                    if event.is_final
                        && let Some(channel) = event.channel
                        && let Some(alt) = channel.alternatives.first()
                        && !alt.transcript.is_empty()
                    {
                        final_transcript.push_str(&alt.transcript);
                        final_transcript.push(' ');
                    }

                    // If this result is from Finalize, we're done immediately
                    if event.from_finalize {
                        if crate::verbose::is_verbose() {
                            eprintln!(
                                "[deepgram-realtime] Received from_finalize result, returning"
                            );
                        }
                        return Ok(Some(final_transcript.trim().to_string()));
                    }
                }
                "Metadata" => {
                    // Connection metadata, ignore
                }
                "error" => {
                    if let Some(desc) = event.description {
                        return Err(anyhow!("Deepgram error: {}", desc));
                    }
                    return Err(anyhow!("Deepgram error (no description)"));
                }
                _ => {
                    // Unknown event type, ignore
                }
            }
            Ok(None)
        }
        Some(Ok(Message::Close(frame))) => {
            if crate::verbose::is_verbose() {
                eprintln!("[deepgram-realtime] WebSocket closed: {:?}", frame);
            }
            // Return current transcript on close
            Ok(Some(final_transcript.trim().to_string()))
        }
        Some(Ok(Message::Ping(_) | Message::Pong(_))) => {
            // Tungstenite handles ping/pong automatically
            Ok(None)
        }
        Some(Ok(Message::Binary(_))) => {
            // Unexpected binary message from server, ignore
            Ok(None)
        }
        Some(Ok(Message::Frame(_))) => {
            // Raw frame, ignore
            Ok(None)
        }
        Some(Err(e)) => Err(anyhow!("WebSocket error: {e}")),
        None => {
            // Stream ended, return current transcript
            Ok(Some(final_transcript.trim().to_string()))
        }
    }
}

#[async_trait]
impl RealtimeTranscriptionBackend for DeepgramRealtimeProvider {
    async fn transcribe_stream(
        &self,
        api_key: &str,
        audio_rx: mpsc::Receiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String> {
        Self::transcribe_stream_impl(api_key, audio_rx, language).await
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE // 16000
    }

    fn requires_keepalive(&self) -> bool {
        true
    }
}

#[async_trait]
impl TranscriptionBackend for DeepgramRealtimeProvider {
    fn name(&self) -> &'static str {
        "deepgram-realtime"
    }

    fn display_name(&self) -> &'static str {
        "Deepgram Realtime"
    }

    /// For file input, fall back to regular Deepgram API
    ///
    /// The Live Streaming API is designed for real-time mic input.
    /// For pre-recorded files, the batch API is more appropriate.
    fn transcribe_sync(
        &self,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        // Delegate to regular Deepgram provider for file-based transcription
        DeepgramProvider.transcribe_sync(api_key, request)
    }

    /// For async file input, fall back to regular Deepgram API
    async fn transcribe_async(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        // Delegate to regular Deepgram provider for file-based transcription
        DeepgramProvider
            .transcribe_async(client, api_key, request)
            .await
    }
}
