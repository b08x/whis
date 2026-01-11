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
    DEFAULT_POST_PROCESSING_PROMPT, PostProcessor, copy_to_clipboard, ollama, post_process,
    preload_ollama, warn,
};

/// Stop recording and run the full transcription pipeline (progressive mode)
/// Guarantees state cleanup on both success and failure
pub async fn stop_and_transcribe(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    // Stop recording (closes audio stream, signals chunker/transcription to finish)
    {
        let mut recorder = state.recorder.lock().unwrap().take();
        if let Some(ref mut rec) = recorder {
            rec.stop_recording().map_err(|e| e.to_string())?;
        }
    }

    // Update state to transcribing
    {
        *state.state.lock().unwrap() = RecordingState::Transcribing;
    }
    println!("Transcribing...");

    // Run transcription with guaranteed state cleanup on any error
    let result = do_progressive_transcription(app, &state).await;

    // Always reset state, regardless of success or failure
    {
        *state.state.lock().unwrap() = RecordingState::Idle;
    }

    result
}

/// Progressive transcription logic - receives result from background task
async fn do_progressive_transcription(app: &AppHandle, state: &AppState) -> Result<(), String> {
    // Receive transcription result from background task
    let rx = {
        let mut rx_guard = state.transcription_rx.lock().unwrap();
        rx_guard
            .take()
            .ok_or("No progressive transcription in progress")?
    };

    // Wait for transcription to complete (rx_guard dropped, so this is Send-safe)
    let transcription = rx
        .await
        .map_err(|_| "Transcription task dropped unexpectedly".to_string())?
        .map_err(|e| format!("Transcription failed: {e}"))?;

    // Extract post-processing config and clipboard method from settings
    let (post_process_config, clipboard_method) = {
        let settings = state.settings.lock().unwrap();
        let clipboard_method = settings.ui.clipboard_backend.clone();
        let post_process_config = if settings.post_processing.enabled
            && settings.post_processing.processor != PostProcessor::None
        {
            let post_processor = settings.post_processing.processor.clone();
            let prompt = settings
                .post_processing
                .prompt
                .clone()
                .unwrap_or_else(|| DEFAULT_POST_PROCESSING_PROMPT.to_string());
            let ollama_model = settings.services.ollama.model.clone();

            let api_key_or_url = if post_processor.requires_api_key() {
                settings
                    .post_processing
                    .api_key_from_settings(&settings.transcription.api_keys)
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

    // Apply post-processing if configured
    let final_text = if let Some((post_processor, prompt, ollama_model, key_or_url)) =
        post_process_config
    {
        if post_processor == PostProcessor::Ollama {
            let url_for_check = key_or_url.clone();
            let ollama_result = tauri::async_runtime::spawn_blocking(move || {
                ollama::ensure_ollama_running(&url_for_check)
            })
            .await
            .map_err(|e| format!("Task join failed: {e}"))?;

            if let Err(e) = ollama_result {
                let warning = format!("Ollama: {e}");
                warn!("Post-processing: {warning}");
                let _ = app.emit("post-process-warning", &warning);
                copy_to_clipboard(&transcription, clipboard_method).map_err(|e| e.to_string())?;
                println!(
                    "Done (unprocessed): {}",
                    &transcription[..transcription.len().min(50)]
                );
                let _ = app.emit("transcription-complete", &transcription);
                return Ok(());
            }
        }

        // Re-warm Ollama model (in case it unloaded during long recording > 5 min)
        if post_processor == PostProcessor::Ollama
            && let Some(model_name) = ollama_model.as_deref()
        {
            preload_ollama(&key_or_url, model_name);
            // Brief pause to allow warmup to complete (runs in background thread)
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        println!("Post-processing...");
        let _ = app.emit("post-process-started", ());

        let model = if post_processor == PostProcessor::Ollama {
            ollama_model.as_deref()
        } else {
            None
        };

        match post_process(&transcription, &post_processor, &key_or_url, &prompt, model).await {
            Ok(processed) => processed,
            Err(e) => {
                let warning = e.to_string();
                warn!("Post-processing: {warning}");
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

    // Emit event to frontend
    let _ = app.emit("transcription-complete", &final_text);

    Ok(())
}
