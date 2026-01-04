//! Transcription Pipeline
//!
//! Orchestrates the full transcription pipeline:
//! 1. Finalize recording (encode audio)
//! 2. Transcribe audio (single or parallel chunks)
//! 3. Post-process transcription (optional)
//! 4. Copy to clipboard
//! 5. Emit completion event

use crate::state::{AppState, RecordingState};
use tauri::{AppHandle, Emitter, Manager};
use whis_core::{
    copy_to_clipboard, ollama, parallel_transcribe, post_process, transcribe_audio,
    PostProcessor, RecordingOutput, DEFAULT_POST_PROCESSING_PROMPT,
};

/// Stop recording and run the full transcription pipeline
/// Guarantees state cleanup on both success and failure
pub async fn stop_and_transcribe(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    // Update state to transcribing
    {
        *state.state.lock().unwrap() = RecordingState::Transcribing;
    }
    println!("Transcribing...");

    // Run transcription with guaranteed state cleanup on any error
    let result = do_transcription(app, &state).await;

    // Always reset state, regardless of success or failure
    {
        *state.state.lock().unwrap() = RecordingState::Idle;
    }

    result
}

/// Inner transcription logic - extracted so we can guarantee state cleanup
async fn do_transcription(app: &AppHandle, state: &AppState) -> Result<(), String> {
    // Get recorder and config
    let mut recorder = state
        .recorder
        .lock()
        .unwrap()
        .take()
        .ok_or("No active recording")?;

    let (provider, api_key, language) = {
        let config = state.transcription_config.lock().unwrap();
        let config_ref = config.as_ref().ok_or("Transcription config not loaded")?;
        (
            config_ref.provider.clone(),
            config_ref.api_key.clone(),
            config_ref.language.clone(),
        )
    };

    // Finalize recording (synchronous file encoding)
    let audio_result = recorder.finalize_recording().map_err(|e| e.to_string())?;

    // Transcribe
    let transcription = match audio_result {
        // Use spawn_blocking to run transcription on a dedicated thread pool.
        // This allows reqwest::blocking::Client (used by cloud providers and Ollama)
        // to create its internal tokio runtime safely. block_in_place() would panic
        // because it forbids runtime creation/destruction inside its context.
        RecordingOutput::Single(data) => {
            let provider = provider.clone();
            let api_key = api_key.clone();
            let language = language.clone();
            tauri::async_runtime::spawn_blocking(move || {
                transcribe_audio(&provider, &api_key, language.as_deref(), data)
            })
            .await
            .map_err(|e| format!("Task join failed: {e}"))?
            .map_err(|e| e.to_string())?
        }
        RecordingOutput::Chunked(chunks) => {
            // parallel_transcribe is async, so we can await it directly
            parallel_transcribe(&provider, &api_key, language.as_deref(), chunks, None)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    // Extract post-processing config and clipboard method from settings (lock scope limited)
    let (post_process_config, clipboard_method) = {
        let settings = state.settings.lock().unwrap();
        let clipboard_method = settings.ui.clipboard_method.clone();
        let post_process_config = if settings.post_processing.processor != PostProcessor::None {
            let post_processor = settings.post_processing.processor.clone();
            let prompt = settings
                .post_processing
                .prompt
                .clone()
                .unwrap_or_else(|| DEFAULT_POST_PROCESSING_PROMPT.to_string());
            let ollama_model = settings.services.ollama.model.clone();

            // Get API key or URL based on post-processor type
            let api_key_or_url = if post_processor.requires_api_key() {
                settings.post_processing.api_key(&settings.transcription.api_keys)
            } else if post_processor == PostProcessor::Ollama {
                let ollama_url = settings
                    .services
                    .ollama
                    .url()
                    .unwrap_or_else(|| ollama::DEFAULT_OLLAMA_URL.to_string());
                Some(ollama_url)
            } else {
                None
            };

            api_key_or_url.map(|key_or_url| (post_processor, prompt, ollama_model, key_or_url))
        } else {
            None
        };
        (post_process_config, clipboard_method)
    };

    // Apply post-processing if enabled (outside of lock scope)
    let final_text =
        if let Some((post_processor, prompt, ollama_model, key_or_url)) = post_process_config {
            // Auto-start Ollama if needed (and installed)
            // Use spawn_blocking because ensure_ollama_running uses reqwest::blocking::Client
            // which creates an internal tokio runtime that would panic if dropped in async context
            if post_processor == PostProcessor::Ollama {
                let url_for_check = key_or_url.clone();
                let ollama_result = tauri::async_runtime::spawn_blocking(move || {
                    ollama::ensure_ollama_running(&url_for_check)
                })
                .await
                .map_err(|e| format!("Task join failed: {e}"))?;

                if let Err(e) = ollama_result {
                    let warning = format!("Ollama: {e}");
                    eprintln!("Post-processing warning: {warning}");
                    let _ = app.emit("post-process-warning", &warning);
                    // Skip post-processing, return raw transcription
                    copy_to_clipboard(&transcription, clipboard_method)
                        .map_err(|e| e.to_string())?;
                    println!(
                        "Done (unprocessed): {}",
                        &transcription[..transcription.len().min(50)]
                    );
                    let _ = app.emit("transcription-complete", &transcription);
                    return Ok(());
                }
            }

            println!("Post-processing...");
            let _ = app.emit("post-process-started", ());

            let model = if post_processor == PostProcessor::Ollama {
                ollama_model.as_deref()
            } else {
                None
            };

            match post_process(&transcription, &post_processor, &key_or_url, &prompt, model).await
            {
                Ok(processed) => processed,
                Err(e) => {
                    let warning = e.to_string();
                    eprintln!("Post-processing warning: {warning}");
                    let _ = app.emit("post-process-warning", &warning);
                    transcription
                }
            }
        } else {
            transcription
        };

    // Copy to clipboard
    copy_to_clipboard(&final_text, clipboard_method).map_err(|e| e.to_string())?;

    println!("Done: {}", &final_text[..final_text.len().min(50)]);

    // Emit event to frontend so it knows transcription completed
    let _ = app.emit("transcription-complete", &final_text);

    Ok(())
}
