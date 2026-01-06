//! Audio transcription using provider registry
//!
//! This module supports two transcription modes:
//!
//! ## Progressive Transcription (Live Recording)
//! - Chunks are transcribed DURING recording (not after)
//! - Cloud: `progressive_transcribe_cloud()` - parallel (max 3 concurrent)
//! - Local: `progressive_transcribe_local()` - sequential (shared model cache)
//!
//! ## Batch Transcription (Pre-Recorded Audio)
//! - Used for files and stdin
//! - Single file: `transcribe_audio()`
//! - Chunked: `batch_transcribe()` - parallel for cloud, sequential for local
//!
//! All modes support overlap merging for seamless chunk boundaries.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::audio::AudioChunk;
use crate::config::TranscriptionProvider;
use crate::provider::{DEFAULT_TIMEOUT_SECS, ProgressCallback, TranscriptionRequest, registry};

/// Maximum concurrent API requests
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// Create an HTTP client configured for the current platform.
///
/// On mobile (embedded-encoder feature), uses bundled Mozilla CA certificates
/// to avoid Android's platform verifier JNI initialization issues.
/// On desktop, uses the default platform certificate verifier.
fn create_http_client() -> Result<reqwest::Client> {
    #[cfg(feature = "embedded-encoder")]
    {
        // Mobile: Use bundled webpki-roots to avoid Android TLS issues
        // Build root certificate store from Mozilla's CA bundle
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        // Build rustls config with the bundled roots
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // Create reqwest client with pre-configured TLS
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .use_preconfigured_tls(tls_config)
            .build()
            .context("Failed to create HTTP client")
    }

    #[cfg(not(feature = "embedded-encoder"))]
    {
        // Desktop: Use default platform verifier
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .context("Failed to create HTTP client")
    }
}
/// Maximum words to search for overlap between chunks
const MAX_OVERLAP_WORDS: usize = 15;

/// Result of transcribing a single chunk
pub struct ChunkTranscription {
    pub index: usize,
    pub text: String,
    pub has_leading_overlap: bool,
}

/// Transcribe a single audio file (blocking, for simple single-file case)
///
/// # Arguments
/// * `provider` - The transcription provider to use
/// * `api_key` - API key for the provider
/// * `language` - Optional language hint (ISO-639-1 code, e.g., "en", "de")
/// * `audio_data` - Audio data to transcribe
pub fn transcribe_audio(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    audio_data: Vec<u8>,
) -> Result<String> {
    transcribe_audio_with_progress(provider, api_key, language, audio_data, None, None, None)
}

/// Transcribe a single audio file with explicit format (blocking)
pub fn transcribe_audio_with_format(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    audio_data: Vec<u8>,
    mime_type: Option<&str>,
    filename: Option<&str>,
) -> Result<String> {
    transcribe_audio_with_progress(
        provider, api_key, language, audio_data, mime_type, filename, None,
    )
}

/// Transcribe a single audio file with progress callback (blocking)
pub fn transcribe_audio_with_progress(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    audio_data: Vec<u8>,
    mime_type: Option<&str>,
    filename: Option<&str>,
    progress: Option<ProgressCallback>,
) -> Result<String> {
    let provider_impl = registry().get_by_kind(provider)?;
    let request = TranscriptionRequest {
        audio_data,
        language: language.map(String::from),
        filename: filename.unwrap_or("audio.mp3").to_string(),
        mime_type: mime_type.unwrap_or("audio/mpeg").to_string(),
        progress,
    };

    let result = provider_impl.transcribe_sync(api_key, request)?;
    Ok(result.text)
}

/// Transcribe a single audio file with explicit format (async)
///
/// Use this version for mobile/environments where blocking HTTP is not supported.
pub async fn transcribe_audio_async(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    audio_data: Vec<u8>,
    mime_type: Option<&str>,
    filename: Option<&str>,
) -> Result<String> {
    let provider_impl = registry().get_by_kind(provider)?;
    let request = TranscriptionRequest {
        audio_data,
        language: language.map(String::from),
        filename: filename.unwrap_or("audio.mp3").to_string(),
        mime_type: mime_type.unwrap_or("audio/mpeg").to_string(),
        progress: None,
    };

    let client = create_http_client()?;

    let result = provider_impl.transcribe_async(&client, api_key, request).await?;
    Ok(result.text)
}

/// Batch transcription for pre-recorded audio (files/stdin)
///
/// Transcribes multiple pre-chunked audio segments in parallel with rate limiting.
/// This is NOT used for progressive transcription during recording—use
/// `progressive_transcribe_cloud()` or `progressive_transcribe_local()` for that.
///
/// # Rate Limiting
/// Cloud providers: Max 3 concurrent API requests
/// Local providers: Sequential processing (not truly parallel)
pub async fn batch_transcribe(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    chunks: Vec<AudioChunk>,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
) -> Result<String> {
    let total_chunks = chunks.len();

    // Create shared HTTP client
    let client = create_http_client()?;

    // Semaphore to limit concurrent requests
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let client = Arc::new(client);
    let api_key = Arc::new(api_key.to_string());
    let language = language.map(|s| Arc::new(s.to_string()));
    let provider_impl = registry().get_by_kind(provider)?;
    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let progress_callback = progress_callback.map(Arc::new);

    // Spawn ALL tasks immediately - they'll wait on semaphore inside
    let mut handles = Vec::with_capacity(total_chunks);

    for chunk in chunks {
        let semaphore = semaphore.clone();
        let client = client.clone();
        let api_key = api_key.clone();
        let language = language.clone();
        let provider_impl = provider_impl.clone();
        let completed = completed.clone();
        let progress_callback = progress_callback.clone();

        let handle = tokio::spawn(async move {
            // Acquire permit INSIDE the task
            let _permit = semaphore.acquire_owned().await?;

            let chunk_index = chunk.index;
            let has_leading_overlap = chunk.has_leading_overlap;

            let request = TranscriptionRequest {
                audio_data: chunk.data,
                language: language.as_ref().map(|s| s.to_string()),
                filename: format!("audio_chunk_{chunk_index}.mp3"),
                mime_type: "audio/mpeg".to_string(),
                progress: None,
            };

            let result = provider_impl
                .transcribe_async(&client, &api_key, request)
                .await;

            let transcription = match result {
                Ok(r) => ChunkTranscription {
                    index: chunk_index,
                    text: r.text,
                    has_leading_overlap,
                },
                Err(e) => return Err(e),
            };

            let done = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if let Some(ref cb) = progress_callback {
                cb(done, total_chunks);
            }
            Ok(transcription)
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = Vec::with_capacity(total_chunks);
    let mut errors = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok(transcription)) => results.push(transcription),
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(anyhow::anyhow!("Task panicked: {e}")),
        }
    }

    // If any chunks failed, return error with details
    if !errors.is_empty() {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        anyhow::bail!(
            "Failed to transcribe {} of {} chunks:\n{}",
            errors.len(),
            total_chunks,
            error_msgs.join("\n")
        );
    }

    // Sort by index to ensure correct order
    results.sort_by_key(|r| r.index);

    // Merge transcriptions
    Ok(merge_transcriptions(results))
}

/// Local audio chunk (raw f32 samples instead of encoded bytes)
#[cfg(feature = "local-transcription")]
pub struct LocalAudioChunk {
    pub index: usize,
    pub samples: Vec<f32>,
    pub has_leading_overlap: bool,
}

/// Merge transcription results, handling overlaps
fn merge_transcriptions(transcriptions: Vec<ChunkTranscription>) -> String {
    if transcriptions.is_empty() {
        return String::new();
    }

    if transcriptions.len() == 1 {
        return transcriptions.into_iter().next().unwrap().text;
    }

    let mut merged = String::new();

    for (i, transcription) in transcriptions.into_iter().enumerate() {
        let text = transcription.text.trim();

        if i == 0 {
            // First chunk - use as-is
            merged.push_str(text);
        } else if transcription.has_leading_overlap {
            // This chunk has overlap - try to find and remove duplicate words
            let cleaned_text = remove_overlap(&merged, text);

            // Skip completely deduplicated chunks to avoid extra whitespace
            if cleaned_text.trim().is_empty() {
                crate::verbose!(
                    "Chunk {} completely deduplicated after overlap removal",
                    transcription.index
                );
                continue;
            }

            if !merged.ends_with(' ') && !cleaned_text.is_empty() && !cleaned_text.starts_with(' ')
            {
                merged.push(' ');
            }
            merged.push_str(&cleaned_text);
        } else {
            // No overlap - just append with space
            if !merged.ends_with(' ') && !text.is_empty() && !text.starts_with(' ') {
                merged.push(' ');
            }
            merged.push_str(text);
        }
    }

    merged
}

/// Remove overlapping text from the beginning of new_text that matches end of existing_text
fn remove_overlap(existing: &str, new_text: &str) -> String {
    let existing_words: Vec<&str> = existing.split_whitespace().collect();
    let new_words: Vec<&str> = new_text.split_whitespace().collect();

    if existing_words.is_empty() || new_words.is_empty() {
        return new_text.to_string();
    }

    // Look for overlap in the last N words of existing and first N words of new
    // ~2 seconds of audio overlap = roughly 5-15 words
    let search_end = existing_words.len().min(MAX_OVERLAP_WORDS);
    let search_new = new_words.len().min(MAX_OVERLAP_WORDS);

    // Find the longest matching overlap
    let mut best_overlap = 0;

    for overlap_len in 1..=search_end.min(search_new) {
        let end_slice = &existing_words[existing_words.len() - overlap_len..];
        let start_slice = &new_words[..overlap_len];

        // Case-insensitive comparison
        let matches = end_slice
            .iter()
            .zip(start_slice.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            best_overlap = overlap_len;
        }
    }

    if best_overlap > 0 {
        // Skip the overlapping words
        new_words[best_overlap..].join(" ")
    } else {
        new_text.to_string()
    }
}

//
// Progressive Transcription Functions
//

use crate::audio::chunker::AudioChunk as ProgressiveChunk;

/// Progressive transcription for cloud providers
///
/// Transcribes audio chunks DURING recording (true progressive). As each 90-second
/// chunk is produced, it's immediately sent to the API for transcription in parallel
/// (max 3 concurrent requests). Results are collected and merged when recording ends.
///
/// # Arguments
/// * `provider` - The transcription provider to use
/// * `api_key` - API key for the provider
/// * `language` - Optional language hint
/// * `chunk_rx` - Channel receiving audio chunks during recording
/// * `progress_callback` - Optional progress reporting
pub async fn progressive_transcribe_cloud(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    mut chunk_rx: tokio::sync::mpsc::UnboundedReceiver<ProgressiveChunk>,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
) -> Result<String> {
    // Create shared HTTP client
    let client = Arc::new(create_http_client()?);

    // Semaphore to limit concurrent requests (max 3)
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let api_key = Arc::new(api_key.to_string());
    let language = language.map(|s| Arc::new(s.to_string()));
    let provider_impl = registry().get_by_kind(provider)?;
    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let total_chunks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let progress_callback = progress_callback.map(Arc::new);

    // Spawn tasks as chunks arrive (true progressive)
    let mut handles = Vec::new();

    while let Some(chunk) = chunk_rx.recv().await {
        // Increment total chunks
        total_chunks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Extract chunk data (MP3 encoding happens inside task to avoid orphaned tasks on error)
        let samples = chunk.samples;
        let chunk_index = chunk.index;
        let has_leading_overlap = chunk.has_leading_overlap;

        // Clone Arc references for task
        let semaphore = semaphore.clone();
        let client = client.clone();
        let api_key = api_key.clone();
        let language = language.clone();
        let provider_impl = provider_impl.clone();
        let completed = completed.clone();
        let total_chunks = total_chunks.clone();
        let progress_callback = progress_callback.clone();

        // Spawn task immediately (don't wait for more chunks)
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit (limits to 3 concurrent)
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("Semaphore is never closed");

            // Convert to MP3 inside task so encoding errors only affect this chunk
            let mp3_data = samples_to_mp3(&samples)?;

            let request = TranscriptionRequest {
                audio_data: mp3_data,
                language: language.as_ref().map(|s| s.to_string()),
                filename: format!("audio_chunk_{chunk_index}.mp3"),
                mime_type: "audio/mpeg".to_string(),
                progress: None,
            };

            let result = provider_impl
                .transcribe_async(&client, &api_key, request)
                .await;

            let transcription = match result {
                Ok(r) => ChunkTranscription {
                    index: chunk_index,
                    text: r.text,
                    has_leading_overlap,
                },
                Err(e) => return Err(e),
            };

            // Report progress (total can increase as new chunks arrive during recording)
            let done = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let total = total_chunks.load(std::sync::atomic::Ordering::SeqCst);
            if let Some(ref cb) = progress_callback {
                cb(done, total);
            }

            Ok(transcription)
        });

        handles.push(handle);
    }

    // Collect results
    let total = total_chunks.load(std::sync::atomic::Ordering::SeqCst);
    let mut results = Vec::with_capacity(total);
    let mut errors = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok(transcription)) => results.push(transcription),
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(anyhow::anyhow!("Task panicked: {e}")),
        }
    }

    // If any chunks failed, return error with details
    if !errors.is_empty() {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        anyhow::bail!(
            "Failed to transcribe {} of {} chunks:\n{}",
            errors.len(),
            total,
            error_msgs.join("\n")
        );
    }

    // Sort by index to ensure correct order
    results.sort_by_key(|r| r.index);

    // Merge with overlap deduplication
    Ok(merge_transcriptions(results))
}

/// Progressive transcription for local providers (Whisper + Parakeet)
///
/// Transcribes audio chunks DURING recording (true progressive). As each 90-second
/// chunk is produced, it's immediately transcribed using the shared cached model
/// (sequential processing). The model is loaded once and reused to minimize memory
/// usage (constant 2GB, compared to 6GB with the previous parallel worker architecture).
///
/// # Arguments
/// * `model_path` - Path to local model directory
/// * `chunk_rx` - Channel receiving audio chunks during recording
/// * `progress_callback` - Optional progress reporting
#[cfg(feature = "local-transcription")]
pub async fn progressive_transcribe_local(
    model_path: &str,
    mut chunk_rx: tokio::sync::mpsc::UnboundedReceiver<ProgressiveChunk>,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
) -> Result<String> {
    let mut transcriptions = Vec::new();
    let mut chunk_count = 0;

    // Process chunks sequentially as they arrive (true progressive)
    while let Some(chunk) = chunk_rx.recv().await {
        chunk_count += 1;
        let chunk_index = chunk.index;
        let has_leading_overlap = chunk.has_leading_overlap;
        let samples = chunk.samples;
        let model_path_owned = model_path.to_string();

        // Run transcription in blocking task (CPU-bound work)
        let result = tokio::task::spawn_blocking(move || {
            crate::provider::transcribe_raw_parakeet(&model_path_owned, samples)
        })
        .await
        .context("Transcription task panicked")?
        .context("Transcription failed")?;

        transcriptions.push(ChunkTranscription {
            index: chunk_index,
            text: result.text,
            has_leading_overlap,
        });

        // Progress reporting (total unknown until channel closes)
        if let Some(ref callback) = progress_callback {
            callback(chunk_count, 0); // Total is 0 since we don't know how many more chunks will arrive
        }
    }

    // Results are already in correct order (sequential processing, no sorting needed)
    Ok(merge_transcriptions(transcriptions))
}

/// Convert f32 samples to MP3 bytes
fn samples_to_mp3(samples: &[f32]) -> Result<Vec<u8>> {
    use crate::audio::create_encoder;
    let encoder = create_encoder();
    encoder
        .encode_samples(samples, crate::resample::WHISPER_SAMPLE_RATE)
        .context("Failed to encode audio to MP3")
}
