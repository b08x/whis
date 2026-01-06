//! OpenAI Realtime API transcription provider
//!
//! Uses WebSocket to stream audio in real-time for lower latency transcription.
//! Audio is streamed during recording rather than buffered and uploaded after.
//!
//! See `realtime.rs` module docs for the common implementation pattern.

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
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
    OpenAIProvider, RealtimeTranscriptionBackend, TranscriptionBackend, TranscriptionRequest,
    TranscriptionResult,
};

const WS_URL: &str = "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview";
const REALTIME_SAMPLE_RATE: u32 = 24000;

/// OpenAI Realtime transcription provider
///
/// Streams audio via WebSocket for lower-latency transcription.
/// Uses the same API key as regular OpenAI.
#[derive(Debug, Default, Clone)]
pub struct OpenAIRealtimeProvider;

// WebSocket protocol messages (Beta API format)

#[derive(Serialize)]
struct SessionUpdate {
    #[serde(rename = "type")]
    msg_type: &'static str,
    session: SessionConfig,
}

#[derive(Serialize)]
struct SessionConfig {
    input_audio_format: &'static str,
    input_audio_transcription: InputAudioTranscription,
    /// Set to None to disable VAD (serializes as null in JSON)
    turn_detection: Option<TurnDetection>,
}

#[derive(Serialize)]
struct InputAudioTranscription {
    model: &'static str,
}

#[derive(Serialize)]
struct TurnDetection {
    #[serde(rename = "type")]
    detection_type: &'static str,
}

#[derive(Serialize)]
struct ResponseCreate {
    #[serde(rename = "type")]
    msg_type: &'static str,
}

#[derive(Serialize)]
struct AudioBufferAppend {
    #[serde(rename = "type")]
    msg_type: &'static str,
    audio: String,
}

#[derive(Serialize)]
struct AudioBufferCommit {
    #[serde(rename = "type")]
    msg_type: &'static str,
}

#[derive(Deserialize, Debug)]
struct RealtimeEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    transcript: Option<String>,
    #[serde(default)]
    error: Option<RealtimeError>,
}

#[derive(Deserialize, Debug)]
struct RealtimeError {
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    code: Option<String>,
}

impl OpenAIRealtimeProvider {
    /// Transcribe audio from a channel of f32 samples (16kHz mono)
    ///
    /// Connects to OpenAI Realtime API via WebSocket and streams audio chunks
    /// as they arrive. Returns the final transcript when the channel closes.
    async fn transcribe_stream_impl(
        api_key: &str,
        mut audio_rx: mpsc::Receiver<Vec<f32>>,
        _language: Option<String>,
    ) -> Result<String> {
        // 1. Build WebSocket request with auth headers
        let mut request = WS_URL.into_client_request()?;
        request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))?,
        );
        request
            .headers_mut()
            .insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));

        // 2. Connect to WebSocket
        let (ws_stream, _response) = connect_async(request)
            .await
            .context("Failed to connect to OpenAI Realtime API")?;

        let (mut write, mut read) = ws_stream.split();

        // 3. Configure session
        // Disable VAD - we'll explicitly trigger transcription with response.create
        // Server VAD causes issues with longer recordings because it waits for "turn" boundaries
        let session_update = SessionUpdate {
            msg_type: "session.update",
            session: SessionConfig {
                input_audio_format: "pcm16",
                input_audio_transcription: InputAudioTranscription { model: "whisper-1" },
                turn_detection: None, // Disable VAD for transcription-only mode
            },
        };

        write
            .send(Message::Text(
                serde_json::to_string(&session_update)?.into(),
            ))
            .await
            .context("Failed to send session configuration")?;

        // Wait for session.created or session.updated confirmation
        loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    let event: RealtimeEvent =
                        serde_json::from_str(&text).context("Failed to parse server event")?;

                    if event.event_type == "error"
                        && let Some(err) = event.error
                    {
                        return Err(anyhow!("OpenAI Realtime error: {}", err.message));
                    }

                    if event.event_type == "session.created"
                        || event.event_type == "session.updated"
                    {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    return Err(anyhow!("WebSocket closed unexpectedly during setup"));
                }
                Some(Err(e)) => {
                    return Err(anyhow!("WebSocket error during setup: {e}"));
                }
                None => {
                    return Err(anyhow!("WebSocket connection closed during setup"));
                }
                _ => {} // Ignore other message types (Ping, Pong, Binary)
            }
        }

        // 4. Create done channel and spawn read task
        // error_tx: read task sends error if server sends one during streaming
        // done_tx: main task signals read task when streaming is complete
        let (error_tx, mut error_rx) = oneshot::channel::<anyhow::Error>();
        let (done_tx, done_rx) = oneshot::channel::<()>();

        let read_handle =
            tokio::spawn(async move { collect_transcripts(read, error_tx, done_rx).await });

        // 5. (No keepalive needed for OpenAI)

        // 6. Stream audio chunks
        let mut chunk_count = 0;
        let mut total_samples = 0;

        loop {
            // Check if read task detected an error
            match error_rx.try_recv() {
                Ok(err) => return Err(err),
                Err(oneshot::error::TryRecvError::Closed) => {
                    return Err(anyhow!("WebSocket read task ended unexpectedly"));
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // No error yet, continue
                }
            }

            // Receive next audio chunk
            let Some(samples) = audio_rx.recv().await else {
                break; // Audio channel closed - streaming complete
            };

            if samples.is_empty() {
                continue;
            }

            chunk_count += 1;
            total_samples += samples.len();

            // Resample from 16kHz to 24kHz (OpenAI requires 24kHz)
            let resampled = resample_16k_to_24k(&samples);

            // Convert f32 samples to PCM16 (i16)
            let pcm16: Vec<i16> = resampled
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .collect();

            // Convert to bytes (little-endian)
            let bytes: Vec<u8> = pcm16.iter().flat_map(|&s| s.to_le_bytes()).collect();

            // Encode as base64 (OpenAI requires base64)
            let audio_base64 = BASE64.encode(&bytes);

            // Send audio append message
            let append = AudioBufferAppend {
                msg_type: "input_audio_buffer.append",
                audio: audio_base64,
            };

            write
                .send(Message::Text(serde_json::to_string(&append)?.into()))
                .await
                .context("Failed to send audio chunk")?;
        }

        if crate::verbose::is_verbose() {
            eprintln!(
                "[openai-realtime] Sent {} chunks, {} total samples",
                chunk_count, total_samples
            );
        }

        // 7. Send finalize messages (commit + response.create)
        let commit = AudioBufferCommit {
            msg_type: "input_audio_buffer.commit",
        };

        write
            .send(Message::Text(serde_json::to_string(&commit)?.into()))
            .await
            .context("Failed to commit audio buffer")?;

        // Trigger transcription explicitly (required when VAD is disabled)
        let response = ResponseCreate {
            msg_type: "response.create",
        };
        write
            .send(Message::Text(serde_json::to_string(&response)?.into()))
            .await
            .context("Failed to create response")?;

        // 8. Signal done_tx to notify read task
        let _ = done_tx.send(());

        // 9. Wait for read task with timeout (30s)
        let transcript_result = timeout(Duration::from_secs(30), read_handle).await;

        // 10. Close connection gracefully
        let _ = write.send(Message::Close(None)).await;

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

/// Collect transcripts from WebSocket messages.
///
/// Two-phase approach (matching Deepgram implementation pattern):
/// - Phase 1: During streaming, monitor for errors while audio is being sent
/// - Phase 2: After done signal, wait for final transcript
///
/// This function is named `collect_transcripts` to match Deepgram's pattern.
async fn collect_transcripts<S>(
    mut read: S,
    error_tx: oneshot::Sender<anyhow::Error>,
    mut done_rx: oneshot::Receiver<()>,
) -> Result<String>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    // Phase 1: Monitor for errors during audio streaming
    loop {
        tokio::select! {
            // Check if main task signaled streaming is complete
            _ = &mut done_rx => {
                if crate::verbose::is_verbose() {
                    eprintln!("[openai-realtime] Finalize sent, switching to post-finalize phase");
                }
                break;
            }

            // Process WebSocket messages
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let event: RealtimeEvent = match serde_json::from_str(&text) {
                            Ok(e) => e,
                            Err(e) => {
                                let err = anyhow!("Failed to parse server event: {e}");
                                let _ = error_tx.send(anyhow::Error::msg(err.to_string()));
                                return Err(err);
                            }
                        };

                        if event.event_type == "error"
                            && let Some(e) = event.error
                        {
                            let err = anyhow!("OpenAI Realtime error: {}", e.message);
                            let _ = error_tx.send(anyhow::Error::msg(err.to_string()));
                            return Err(err);
                        }
                        // Ignore other events during streaming (status updates, etc.)
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let err = anyhow!("WebSocket closed during streaming: {:?}", frame);
                        let _ = error_tx.send(anyhow::Error::msg(err.to_string()));
                        return Err(err);
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                        // Tungstenite handles ping/pong automatically
                    }
                    Some(Err(e)) => {
                        let err = anyhow!("WebSocket error during streaming: {e}");
                        let _ = error_tx.send(anyhow::Error::msg(err.to_string()));
                        return Err(err);
                    }
                    None => {
                        let err = anyhow!("WebSocket closed unexpectedly during streaming");
                        let _ = error_tx.send(anyhow::Error::msg(err.to_string()));
                        return Err(err);
                    }
                    _ => {} // Ignore Binary
                }
            }
        }
    }

    // Phase 2: Wait for final transcript after streaming completes
    loop {
        match read.next().await {
            Some(Ok(Message::Text(text))) => {
                let event: RealtimeEvent =
                    serde_json::from_str(&text).context("Failed to parse server event")?;

                if crate::verbose::is_verbose() {
                    eprintln!("[openai-realtime] event: {}", event.event_type);
                }

                match event.event_type.as_str() {
                    "error" => {
                        if let Some(err) = event.error {
                            return Err(anyhow!("OpenAI Realtime error: {}", err.message));
                        }
                    }
                    "conversation.item.input_audio_transcription.completed" => {
                        if let Some(transcript) = event.transcript {
                            return Ok(transcript);
                        }
                    }
                    // Ignore other events (response.created, response.done, etc.)
                    _ => {}
                }
            }
            Some(Ok(Message::Close(_))) => {
                return Err(anyhow!("WebSocket closed before receiving transcription"));
            }
            Some(Err(e)) => {
                return Err(anyhow!("WebSocket error: {e}"));
            }
            None => {
                return Err(anyhow!("Connection ended before receiving transcription"));
            }
            _ => {} // Ignore Ping, Pong, Binary
        }
    }
}

/// Simple linear interpolation to resample from 16kHz to 24kHz
///
/// For each output sample, we interpolate between two input samples.
/// Ratio: 24000/16000 = 1.5 (3 output samples per 2 input samples)
fn resample_16k_to_24k(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let ratio = REALTIME_SAMPLE_RATE as f64 / 16000.0; // 1.5
    let output_len = ((samples.len() as f64) * ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 / ratio;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(samples.len() - 1);
        let frac = (src_idx - idx0 as f64) as f32;

        let sample = samples[idx0] * (1.0 - frac) + samples[idx1] * frac;
        output.push(sample);
    }

    output
}

#[async_trait]
impl RealtimeTranscriptionBackend for OpenAIRealtimeProvider {
    async fn transcribe_stream(
        &self,
        api_key: &str,
        audio_rx: mpsc::Receiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String> {
        Self::transcribe_stream_impl(api_key, audio_rx, language).await
    }

    fn sample_rate(&self) -> u32 {
        REALTIME_SAMPLE_RATE // 24000
    }

    fn requires_keepalive(&self) -> bool {
        false
    }
}

#[async_trait]
impl TranscriptionBackend for OpenAIRealtimeProvider {
    fn name(&self) -> &'static str {
        "openai-realtime"
    }

    fn display_name(&self) -> &'static str {
        "OpenAI Realtime"
    }

    /// For file input, fall back to regular OpenAI API
    ///
    /// The Realtime API is designed for streaming mic input.
    /// For pre-recorded files, the standard whisper-1 API is more appropriate.
    fn transcribe_sync(
        &self,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        // Delegate to regular OpenAI provider for file-based transcription
        OpenAIProvider.transcribe_sync(api_key, request)
    }

    /// For async file input, fall back to regular OpenAI API
    async fn transcribe_async(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        // Delegate to regular OpenAI provider for file-based transcription
        OpenAIProvider
            .transcribe_async(client, api_key, request)
            .await
    }
}
