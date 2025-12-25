//! Ollama management utilities
//!
//! Handles checking, starting, and managing the local Ollama server.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::time::Duration;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::time::Instant;

/// Default Ollama server URL
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default model for post-processing
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5:1.5b";

/// Alternative models for post-processing (name, size, description)
/// qwen2.5:1.5b is the recommended default; these are additional options
pub const OLLAMA_MODEL_OPTIONS: &[(&str, &str, &str)] = &[
    ("qwen2.5:1.5b", "1.0 GB", "Recommended - fast, good quality"),
    ("qwen2.5:3b", "1.9 GB", "Better quality, still fast"),
    ("ministral:3b", "1.9 GB", "European model, good quality"),
    ("gemma2:2b", "1.6 GB", "Google model, efficient"),
];

/// Timeout for Ollama to start
#[cfg(any(target_os = "linux", target_os = "macos"))]
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll interval when waiting for Ollama to start
#[cfg(any(target_os = "linux", target_os = "macos"))]
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelInfo {
    name: String,
    #[serde(default)]
    size: u64,
}

/// Model info returned from list_models
#[derive(Debug, Clone)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
}

impl OllamaModel {
    /// Format size as human-readable string
    pub fn size_str(&self) -> String {
        if self.size == 0 {
            return String::new();
        }
        if self.size >= 1_000_000_000 {
            format!("{:.1} GB", self.size as f64 / 1_000_000_000.0)
        } else if self.size >= 1_000_000 {
            format!("{:.0} MB", self.size as f64 / 1_000_000.0)
        } else if self.size >= 1_000 {
            format!("{:.0} KB", self.size as f64 / 1_000.0)
        } else {
            format!("{} B", self.size)
        }
    }
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

/// Check if running inside a Flatpak sandbox
fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Check if Ollama binary is installed
pub fn is_ollama_installed() -> bool {
    // In Flatpak, we can't check for host binaries
    // Return true and rely on HTTP API check instead
    if is_flatpak() {
        return true;
    }

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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
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

/// List all models available in Ollama
pub fn list_models(url: &str) -> Result<Vec<OllamaModel>> {
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

    Ok(tags
        .models
        .into_iter()
        .map(|m| OllamaModel {
            name: m.name,
            size: m.size,
        })
        .collect())
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
