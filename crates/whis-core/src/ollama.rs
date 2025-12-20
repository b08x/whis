//! Ollama management utilities
//!
//! Handles checking, starting, and managing the local Ollama server.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Default Ollama server URL
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default model for post-processing
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5:1.5b";

/// Timeout for Ollama to start
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll interval when waiting for Ollama to start
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

/// Progress response from Ollama pull API (streaming NDJSON)
#[derive(Debug, Deserialize)]
struct PullProgress {
    status: String,
    #[serde(default)]
    completed: u64,
    #[serde(default)]
    total: u64,
}

/// Check if Ollama is reachable at the given URL
///
/// Returns `Ok(true)` if connected successfully, or an error with details about why
/// the connection failed (not running, not installed, connection refused, etc.)
pub fn is_ollama_running(url: &str) -> Result<bool, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));

    match client.get(&tags_url).send() {
        Ok(resp) if resp.status().is_success() => Ok(true),
        Ok(resp) => Err(format!("Ollama returned status {}", resp.status())),
        Err(e) if e.is_connect() => Err("Connection refused - Ollama not running".to_string()),
        Err(e) if e.is_timeout() => Err("Connection timed out".to_string()),
        Err(e) => Err(format!("Failed to connect: {}", e)),
    }
}

/// Check if Ollama binary is installed
pub fn is_ollama_installed() -> bool {
    Command::new("ollama")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Start Ollama server if not running
///
/// Returns Ok(true) if Ollama was started, Ok(false) if already running.
/// Returns Err if Ollama couldn't be started.
pub fn ensure_ollama_running(url: &str) -> Result<bool> {
    // Already running?
    if is_ollama_running(url).unwrap_or(false) {
        return Ok(false);
    }

    // Only auto-start for default URL (localhost)
    if !url.contains("localhost") && !url.contains("127.0.0.1") {
        return Err(anyhow!(
            "Ollama not reachable at {}.\n\
             For remote Ollama servers, ensure the server is running.",
            url
        ));
    }

    // Check if ollama is installed
    if !is_ollama_installed() {
        return Err(anyhow!(
            "Ollama is not installed.\n\
             Install from: https://ollama.com/download\n\
             \n\
             Linux:   curl -fsSL https://ollama.com/install.sh | sh\n\
             macOS:   brew install ollama"
        ));
    }

    // Start ollama serve in background
    eprintln!("Starting Ollama server...");

    // Use setsid on Linux to detach from terminal, nohup-style behavior
    #[cfg(target_os = "linux")]
    {
        Command::new("setsid")
            .args(["ollama", "serve"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Ollama server")?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("ollama")
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Ollama server")?;
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        return Err(anyhow!(
            "Auto-starting Ollama is not supported on this platform.\n\
             Please start Ollama manually: ollama serve"
        ));
    }

    // Wait for Ollama to become ready
    let start = Instant::now();
    while start.elapsed() < STARTUP_TIMEOUT {
        if is_ollama_running(url).unwrap_or(false) {
            eprintln!("Ollama server started.");
            return Ok(true);
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    Err(anyhow!(
        "Ollama server did not start within {} seconds.\n\
         Try starting it manually: ollama serve",
        STARTUP_TIMEOUT.as_secs()
    ))
}

/// Check if a specific model is available in Ollama
pub fn has_model(url: &str, model: &str) -> Result<bool> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("Failed to create HTTP client")?;

    let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));
    let response = client
        .get(&tags_url)
        .send()
        .context("Failed to connect to Ollama")?;

    if !response.status().is_success() {
        return Err(anyhow!("Ollama returned error: {}", response.status()));
    }

    let tags: TagsResponse = response.json().context("Failed to parse Ollama response")?;

    // Model names can have tags like "qwen2.5:1.5b:latest", check for prefix match
    let model_base = model.split(':').next().unwrap_or(model);
    Ok(tags
        .models
        .iter()
        .any(|m| m.name.starts_with(model_base) || m.name == model))
}

/// Pull a model from Ollama registry
///
/// Shows progress to stderr.
/// Note: Uses the ollama CLI for better progress display.
pub fn pull_model(_url: &str, model: &str) -> Result<()> {
    eprintln!("Pulling Ollama model '{}'...", model);

    // Use ollama CLI for pulling (better progress display)
    let status = Command::new("ollama")
        .args(["pull", model])
        .status()
        .context("Failed to run ollama pull")?;

    if !status.success() {
        return Err(anyhow!("Failed to pull model '{}'", model));
    }

    eprintln!("Model '{}' is ready.", model);
    Ok(())
}

/// Pull a model from Ollama registry with progress callback
///
/// Uses the Ollama HTTP API for streaming progress updates.
/// Calls `on_progress(completed_bytes, total_bytes)` during download.
pub fn pull_model_with_progress(
    url: &str,
    model: &str,
    on_progress: impl Fn(u64, u64),
) -> Result<()> {
    use std::io::BufRead;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3600)) // 1 hour for large models
        .build()
        .context("Failed to create HTTP client")?;

    let pull_url = format!("{}/api/pull", url.trim_end_matches('/'));

    let response = client
        .post(&pull_url)
        .json(&serde_json::json!({ "name": model }))
        .send()
        .context("Failed to connect to Ollama")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Ollama pull failed: {} - {}",
            response.status(),
            response.text().unwrap_or_default()
        ));
    }

    // Stream the response line by line (NDJSON format)
    let reader = std::io::BufReader::new(response);
    for line in reader.lines() {
        let line = line.context("Failed to read response")?;
        if line.is_empty() {
            continue;
        }

        // Parse the JSON progress
        if let Ok(progress) = serde_json::from_str::<PullProgress>(&line) {
            // Report progress when we have total size info
            if progress.total > 0 {
                on_progress(progress.completed, progress.total);
            }

            // Check for error status
            if progress.status.contains("error") {
                return Err(anyhow!("Pull failed: {}", progress.status));
            }
        }
    }

    Ok(())
}

/// Ensure Ollama is running and has the specified model
pub fn ensure_ollama_ready(url: &str, model: &str) -> Result<()> {
    // Start Ollama if needed
    ensure_ollama_running(url)?;

    // Check if model is available
    if !has_model(url, model)? {
        pull_model(url, model)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ollama_installed() {
        // This test just verifies the function doesn't panic
        let _ = is_ollama_installed();
    }

    #[test]
    fn test_default_url_is_localhost() {
        assert!(DEFAULT_OLLAMA_URL.contains("localhost"));
    }
}
