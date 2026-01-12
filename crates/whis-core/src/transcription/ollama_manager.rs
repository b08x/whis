//! Ollama model warming for post-processing
//!
//! This module provides background warming of Ollama models to reduce latency
//! when post-processing transcriptions. Similar to model_manager.rs for Whisper.
//!
//! Call `preload_ollama()` when recording starts if using Ollama post-processing.
//! The warmup happens in a background thread and errors are logged (non-blocking).

use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

/// Cache of warmed (server_url, model) pairs to avoid redundant requests
static WARMUP_CACHE: OnceLock<RwLock<HashSet<(String, String)>>> = OnceLock::new();

fn get_cache() -> &'static RwLock<HashSet<(String, String)>> {
    WARMUP_CACHE.get_or_init(|| RwLock::new(HashSet::new()))
}

/// Check if Ollama model is already warmed up
fn is_warmed(server_url: &str, model: &str) -> bool {
    let cache = get_cache().read().unwrap();
    cache.contains(&(server_url.to_string(), model.to_string()))
}

/// Mark model as warmed in cache
fn set_warmed(server_url: &str, model: &str) {
    let mut cache = get_cache().write().unwrap();
    cache.insert((server_url.to_string(), model.to_string()));
}

/// Clear warmup cache (useful for testing or config changes)
pub fn clear_warmup_cache() {
    let mut cache = get_cache().write().unwrap();
    cache.clear();
}

/// Warm up Ollama model in background thread.
///
/// This function:
/// 1. Checks if already warmed (returns early if so)
/// 2. Spawns background thread that:
///    - Starts Ollama server if needed (localhost only)
///    - Checks if model exists (skips if not, no auto-pull)
///    - Sends minimal chat request to load model into memory
/// 3. Errors are logged but don't fail the recording
///
/// Call this when recording starts if post_processor == Ollama.
///
/// # Arguments
/// * `server_url` - Ollama server URL (e.g., "http://localhost:11434")
/// * `model` - Model name (e.g., "qwen2.5:1.5b")
/// * `keep_alive` - How long to keep model loaded (e.g., "5m", "10m", "-1")
pub fn preload_ollama(server_url: &str, model: &str, keep_alive: &str) {
    // Check if already warmed
    if is_warmed(server_url, model) {
        crate::verbose!("Ollama model already warmed, skipping preload");
        return;
    }

    let server_url = server_url.to_string();
    let model = model.to_string();
    let keep_alive = keep_alive.to_string();

    std::thread::spawn(move || {
        crate::verbose!("Preloading Ollama model '{}' in background...", model);

        // Step 1: Ensure Ollama is running (localhost only)
        if let Err(e) = super::ollama::ensure_ollama_running(&server_url) {
            crate::verbose!("Ollama preload: server startup failed: {}", e);
            return;
        }

        // Step 2: Check if model exists (skip pull during preload)
        match super::ollama::has_model(&server_url, &model) {
            Ok(true) => {
                crate::verbose!("Ollama preload: model '{}' found, warming up...", model);
            }
            Ok(false) => {
                crate::verbose!(
                    "Ollama preload: model '{}' not found, skipping warmup (will pull later if needed)",
                    model
                );
                return;
            }
            Err(e) => {
                crate::verbose!("Ollama preload: model check failed: {}", e);
                return;
            }
        }

        // Step 3: Send minimal chat request to warm up the model
        if let Err(e) = warm_model(&server_url, &model, &keep_alive) {
            crate::verbose!("Ollama preload: warmup request failed: {}", e);
            return;
        }

        // Mark as warmed in cache
        set_warmed(&server_url, &model);
        crate::verbose!("Ollama model '{}' preloaded successfully", model);
    });
}

/// Send minimal chat request to warm up the model
///
/// Uses empty messages array with keep_alive set to extend model lifetime.
fn warm_model(server_url: &str, model: &str, keep_alive: &str) -> Result<(), String> {
    let url = format!("{}/api/chat", server_url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "model": model,
            "messages": [],
            "stream": false,
            "keep_alive": keep_alive
        }))
        .send()
        .map_err(|e| {
            if e.is_connect() {
                format!("Cannot connect to Ollama at {}", server_url)
            } else {
                format!("Warmup request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Ollama warmup failed: {} - {}",
            response.status(),
            response.text().unwrap_or_default()
        ));
    }

    // Verify response indicates success (Ollama returns JSON)
    let response_text = response.text().unwrap_or_default();
    if response_text.is_empty() {
        return Err("Ollama warmup returned empty response".to_string());
    }

    // Basic check that we got valid JSON back
    serde_json::from_str::<serde_json::Value>(&response_text)
        .map_err(|e| format!("Invalid warmup response: {}", e))?;

    Ok(())
}
