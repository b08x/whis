//! Ollama Integration Commands
//!
//! Provides Tauri commands for interacting with Ollama server.
//! Includes connection testing, model listing/pulling, and server management.

/// Progress payload for Ollama pull events
#[derive(Clone, serde::Serialize)]
pub struct OllamaPullProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Ollama status check result
#[derive(Clone, serde::Serialize)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub error: Option<String>,
}

/// Test connection to Ollama server
/// Must be async with spawn_blocking because reqwest::blocking::Client
/// creates an internal tokio runtime that would panic if called from Tauri's async context
#[tauri::command]
pub async fn test_ollama_connection(url: String) -> Result<bool, String> {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    tauri::async_runtime::spawn_blocking(move || whis_core::ollama::is_ollama_running(&url))
        .await
        .map_err(|e| e.to_string())?
}

/// List available models from Ollama
#[tauri::command]
pub async fn list_ollama_models(url: String) -> Result<Vec<String>, String> {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    // Run blocking call in separate thread
    tauri::async_runtime::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));
        let response = client
            .get(&tags_url)
            .send()
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Ollama returned error: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct TagsResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(serde::Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let tags: TagsResponse = response
            .json()
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Pull an Ollama model with progress events
/// Emits 'ollama-pull-progress' events with { downloaded, total } during download
#[tauri::command]
pub async fn pull_ollama_model(
    app: tauri::AppHandle,
    url: String,
    model: String,
) -> Result<(), String> {
    use tauri::Emitter;

    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    // Run blocking calls in separate thread to avoid tokio runtime conflicts
    // (reqwest::blocking::Client creates its own runtime internally)
    tauri::async_runtime::spawn_blocking(move || {
        // Validate Ollama is running before attempting pull
        whis_core::ollama::ensure_ollama_running(&url).map_err(|e| e.to_string())?;

        whis_core::ollama::pull_model_with_progress(&url, &model, |downloaded, total| {
            let _ = app.emit(
                "ollama-pull-progress",
                OllamaPullProgress { downloaded, total },
            );
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Check Ollama installation and running status
/// Returns structured status without attempting to start Ollama
#[tauri::command]
pub async fn check_ollama_status(url: String) -> OllamaStatus {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    tauri::async_runtime::spawn_blocking(move || {
        let installed = whis_core::ollama::is_ollama_installed();

        if !installed {
            return OllamaStatus {
                installed: false,
                running: false,
                error: Some("Ollama is not installed".to_string()),
            };
        }

        match whis_core::ollama::is_ollama_running(&url) {
            Ok(true) => OllamaStatus {
                installed: true,
                running: true,
                error: None,
            },
            Ok(false) => OllamaStatus {
                installed: true,
                running: false,
                error: Some("Ollama is not running".to_string()),
            },
            Err(e) => OllamaStatus {
                installed: true,
                running: false,
                error: Some(e),
            },
        }
    })
    .await
    .unwrap_or(OllamaStatus {
        installed: false,
        running: false,
        error: Some("Failed to check status".to_string()),
    })
}

/// Start Ollama server if not running
/// Returns "started" if we started it, "running" if already running
/// Must be async with spawn_blocking because reqwest::blocking::Client
/// creates an internal tokio runtime that would panic if called from Tauri's async context
#[tauri::command]
pub async fn start_ollama(url: String) -> Result<String, String> {
    let url = if url.trim().is_empty() {
        whis_core::ollama::DEFAULT_OLLAMA_URL.to_string()
    } else {
        url
    };

    tauri::async_runtime::spawn_blocking(move || {
        match whis_core::ollama::ensure_ollama_running(&url) {
            Ok(true) => Ok("started".to_string()),  // Was started
            Ok(false) => Ok("running".to_string()), // Already running
            Err(e) => Err(e.to_string()),           // Not installed / error
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
