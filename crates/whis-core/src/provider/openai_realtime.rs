//! OpenAI Realtime API transcription provider
//!
//! Streams audio via WebSocket for real-time transcription.
//!
//! Model: `gpt-4o-transcribe` (speech-to-text only, no AI responses).
//! No `gpt-5-transcribe` yet as of 2026-01-11.

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

const WS_URL: &str = "wss://api.openai.com/v1/realtime?intent=transcription";
const REALTIME_SAMPLE_RATE: u32 = 24000;

/// OpenAI Realtime transcription provider
///
/// Streams audio via WebSocket for lower-latency transcription.
/// Uses the same API key as regular OpenAI.
#[derive(Debug, Default, Clone)]
pub struct OpenAIRealtimeProvider;

// WebSocket protocol messages (GA API format)

#[derive(Serialize)]
struct SessionUpdate {
    #[serde(rename = "type")]
    msg_type: &'static str,
    session: SessionConfig,
}

/// GA API session configuration for transcription mode
#[derive(Serialize)]
struct SessionConfig {
    /// Session type: "transcription" for transcription-only mode
    #[serde(rename = "type")]
    session_type: &'static str,
    audio: AudioConfig,
}

#[derive(Serialize)]
struct AudioConfig {
    input: AudioInputConfig,
}

#[derive(Serialize)]
struct AudioInputConfig {
    format: AudioFormat,
    transcription: TranscriptionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    turn_detection: Option<TurnDetection>,
}

#[derive(Serialize)]
struct AudioFormat {
    #[serde(rename = "type")]
    format_type: &'static str,
    rate: u32,
}

#[derive(Serialize)]
struct TranscriptionConfig {
    model: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
}

#[derive(Serialize)]
struct TurnDetection {
    #[serde(rename = "type")]
    detection_type: &'static str,
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
        mut audio_rx: mpsc::UnboundedReceiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String> {
        // 1. Build WebSocket request with auth headers
        let mut request = WS_URL.into_client_request()?;
        request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))?,
        );

        // 2. Connect to WebSocket with timeout
        let (ws_stream, _response) = timeout(Duration::from_secs(30), connect_async(request))
            .await
            .context("Connection timeout")?
            .context("Failed to connect to OpenAI Realtime API")?;

        let (mut write, mut read) = ws_stream.split();

        // 3. Configure session for transcription-only mode (GA API format)
        let session_update = SessionUpdate {
            msg_type: "session.update",
            session: SessionConfig {
                session_type: "transcription",
                audio: AudioConfig {
                    input: AudioInputConfig {
                        format: AudioFormat {
                            format_type: "audio/pcm",
                            rate: REALTIME_SAMPLE_RATE,
                        },
                        transcription: TranscriptionConfig {
                            model: "gpt-4o-transcribe",
                            language: language.clone(),
                        },
                        turn_detection: None, // Server VAD disabled for manual control
                    },
                },
            },
        };

        write
            .send(Message::Text(
                serde_json::to_string(&session_update)?.into(),
            ))
            .await
            .context("Failed to send session configuration")?;

        // Wait for session.created or session.updated confirmation with timeout
        let setup_result = timeout(Duration::from_secs(30), async {
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
                            return Ok(());
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
        })
        .await;

        match setup_result {
            Ok(Ok(())) => {} // Setup successful
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(anyhow!("Timeout during session setup")),
        }

        // 4. Create done channel and spawn read task
        // error_tx: read task sends error if server sends one during streaming
        // done_tx: main task signals read task when streaming is complete (includes total_samples for dynamic timeout)
        let (error_tx, mut error_rx) = oneshot::channel::<anyhow::Error>();
        let (done_tx, done_rx) = oneshot::channel::<usize>(); // Now sends total_samples

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

        // 7. Commit audio buffer to trigger transcription (GA API - no response.create needed)
        let commit = AudioBufferCommit {
            msg_type: "input_audio_buffer.commit",
        };

        write
            .send(Message::Text(serde_json::to_string(&commit)?.into()))
            .await
            .context("Failed to commit audio buffer")?;

        // 8. Signal done_tx to notify read task with total_samples for dynamic timeout
        let _ = done_tx.send(total_samples);

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
        audio_rx: mpsc::UnboundedReceiver<Vec<f32>>,
        language: Option<String>,
    ) -> Result<String> {
        Self::transcribe_stream_impl(api_key, audio_rx, language).await
    }
}

/// Collect transcripts from WebSocket messages.
///
/// Two-phase approach (matching Deepgram implementation pattern):
/// - Phase 1: During streaming, monitor for errors AND collect any early transcripts
/// - Phase 2: After done signal, wait for final transcript (if not already received)
///
/// IMPORTANT: OpenAI may send the transcript event during Phase 1 if the recording
/// is short. We must capture it to avoid losing transcripts.
///
/// The done_rx channel receives total_samples to calculate a dynamic timeout for Phase 2.
/// Longer recordings need more processing time after Finalize.
async fn collect_transcripts<S>(
    mut read: S,
    error_tx: oneshot::Sender<anyhow::Error>,
    mut done_rx: oneshot::Receiver<usize>,
) -> Result<String>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    // Store transcript if received early (during Phase 1)
    let mut early_transcript: Option<String> = None;

    // Phase 1: Monitor for errors AND collect transcripts during audio streaming
    // total_samples will be received when streaming completes
    let total_samples: usize;
    loop {
        tokio::select! {
            // Check if main task signaled streaming is complete
            result = &mut done_rx => {
                total_samples = result.unwrap_or(0);
                if crate::verbose::is_verbose() {
                    eprintln!("[openai-realtime] Finalize sent, switching to post-finalize phase");
                }
                // If we already got a transcript during streaming, return it now
                if let Some(transcript) = early_transcript {
                    if crate::verbose::is_verbose() {
                        eprintln!("[openai-realtime] Returning early transcript from Phase 1");
                    }
                    return Ok(transcript);
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

                        // Capture transcript if it arrives early (short recordings)
                        if event.event_type == "conversation.item.input_audio_transcription.completed"
                            && let Some(transcript) = event.transcript
                        {
                            if crate::verbose::is_verbose() {
                                eprintln!("[openai-realtime] Received transcript during Phase 1 (early)");
                            }
                            early_transcript = Some(transcript);
                        }
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
    // Calculate dynamic timeout based on recording duration
    // Formula: min(30s, audio_duration/5), capped at 120s
    // 16kHz sample rate, so audio_duration_secs = total_samples / 16000
    let audio_duration_secs = total_samples as f64 / 16000.0;
    let phase2_timeout_secs = (audio_duration_secs / 5.0).clamp(30.0, 120.0) as u64;

    if crate::verbose::is_verbose() {
        eprintln!(
            "[openai-realtime] Audio duration: {:.1}s, Phase 2 timeout: {}s",
            audio_duration_secs, phase2_timeout_secs
        );
    }

    let timeout_duration = Duration::from_secs(phase2_timeout_secs);
    let deadline = tokio::time::Instant::now() + timeout_duration;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            // Graceful degradation: return empty string with warning instead of error
            eprintln!(
                "[openai-realtime] WARNING: Phase 2 timeout after {}s - no transcript received",
                phase2_timeout_secs
            );
            return Ok(String::new());
        }

        tokio::select! {
            _ = tokio::time::sleep(remaining) => {
                // Graceful degradation: return empty string with warning instead of error
                eprintln!(
                    "[openai-realtime] WARNING: Phase 2 timeout after {}s - no transcript received",
                    phase2_timeout_secs
                );
                return Ok(String::new());
            }

            msg = read.next() => {
                match msg {
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
                        // Graceful degradation: connection closed, return empty string
                        eprintln!("[openai-realtime] WARNING: WebSocket closed before receiving transcription");
                        return Ok(String::new());
                    }
                    Some(Err(e)) => {
                        return Err(anyhow!("WebSocket error: {e}"));
                    }
                    None => {
                        // Graceful degradation: connection ended, return empty string
                        eprintln!("[openai-realtime] WARNING: Connection ended before receiving transcription");
                        return Ok(String::new());
                    }
                    _ => {} // Ignore Ping, Pong, Binary
                }
            }
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
        audio_rx: mpsc::UnboundedReceiver<Vec<f32>>,
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
