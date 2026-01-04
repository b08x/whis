//! Audio transcription using provider registry
//!
//! This module provides transcription functionality using the extensible
//! provider architecture. It handles both single-file and chunked parallel
//! transcription with overlap merging.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::audio::AudioChunk;
use crate::config::TranscriptionProvider;
use crate::provider::{DEFAULT_TIMEOUT_SECS, ProgressCallback, TranscriptionRequest, registry};

/// Maximum concurrent API requests
const MAX_CONCURRENT_REQUESTS: usize = 3;
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

/// Transcribe multiple chunks in parallel with rate limiting
pub async fn parallel_transcribe(
    provider: &TranscriptionProvider,
    api_key: &str,
    language: Option<&str>,
    chunks: Vec<AudioChunk>,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
) -> Result<String> {
    let total_chunks = chunks.len();

    // Create shared HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .context("Failed to create HTTP client")?;

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
pub struct LocalAudioChunk {
    pub index: usize,
    pub samples: Vec<f32>,
    pub has_leading_overlap: bool,
}

/// Transcribe local audio chunks in parallel (for Parakeet progressive transcription)
///
/// Adapts the cloud provider parallel pattern for local CPU-bound transcription.
/// Uses tokio::task::spawn_blocking for each chunk to avoid blocking the async runtime.
///
/// # Arguments
/// * `model_path` - Path to Parakeet model directory
/// * `chunks` - Audio chunks to transcribe (raw 16kHz mono f32 samples)
/// * `num_workers` - Maximum concurrent workers (semaphore limit)
/// * `progress_callback` - Optional progress reporting callback
pub async fn parallel_transcribe_local(
    model_path: &str,
    chunks: Vec<LocalAudioChunk>,
    num_workers: usize,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
) -> Result<String> {
    let total_chunks = chunks.len();

    // Semaphore to limit concurrent workers
    let semaphore = Arc::new(Semaphore::new(num_workers));
    let model_path = Arc::new(model_path.to_string());
    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let progress_callback = progress_callback.map(Arc::new);
    let error_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Process chunks with worker limit (semaphore controls concurrency)
    let mut handles = Vec::with_capacity(total_chunks);

    for chunk in chunks {
        let model_path = model_path.clone();
        let completed = completed.clone();
        let progress_callback = progress_callback.clone();
        let error_flag = error_flag.clone();

        // Acquire semaphore permit BEFORE spawning blocking task
        // This limits concurrent workers: if num_workers permits are held,
        // this await will block until a worker finishes and releases its permit
        let permit = semaphore.clone().acquire_owned().await?;

        let handle = tokio::task::spawn_blocking(move || {
            // Hold permit for duration of transcription
            let _permit = permit;

            // Check if another worker already failed (fail-fast)
            if error_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(anyhow::anyhow!("Another worker failed, aborting"));
            }

            let chunk_index = chunk.index;
            let has_leading_overlap = chunk.has_leading_overlap;

            // Transcribe using Parakeet
            let result = crate::provider::transcribe_raw_parakeet(
                &model_path,
                chunk.samples,
            );

            let transcription = match result {
                Ok(r) => ChunkTranscription {
                    index: chunk_index,
                    text: r.text,
                    has_leading_overlap,
                },
                Err(e) => {
                    error_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    return Err(e);
                }
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

    // Merge transcriptions (reuse cloud provider overlap handling)
    Ok(merge_transcriptions(results))
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
