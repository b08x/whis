use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fmt;

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const MISTRAL_CHAT_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub const DEFAULT_POLISH_PROMPT: &str = "Clean up this voice transcript. Fix grammar and punctuation. \
Remove filler words (um, uh, like, you know). \
If the speaker corrects themselves, keep only the correction. \
Preserve technical terms and proper nouns. Output only the cleaned text.";

/// Available polishing providers (LLM for transcript cleanup)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Polisher {
    #[default]
    None,
    OpenAI,
    Mistral,
    Ollama,
}

impl fmt::Display for Polisher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Polisher::None => write!(f, "none"),
            Polisher::OpenAI => write!(f, "openai"),
            Polisher::Mistral => write!(f, "mistral"),
            Polisher::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for Polisher {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Polisher::None),
            "openai" => Ok(Polisher::OpenAI),
            "mistral" => Ok(Polisher::Mistral),
            "ollama" => Ok(Polisher::Ollama),
            _ => Err(format!(
                "Unknown polisher: {}. Use 'none', 'openai', 'mistral', or 'ollama'",
                s
            )),
        }
    }
}

impl Polisher {
    /// Returns true if this polisher requires an API key (cloud providers)
    pub fn requires_api_key(&self) -> bool {
        matches!(self, Polisher::OpenAI | Polisher::Mistral)
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

/// Polish (clean up) a transcript using the specified LLM provider
///
/// For cloud providers (OpenAI, Mistral), `api_key_or_url` is the API key.
/// For Ollama, `api_key_or_url` is the server URL (e.g., http://localhost:11434).
pub async fn polish(
    text: &str,
    polisher: &Polisher,
    api_key_or_url: &str,
    prompt: &str,
    model: Option<&str>,
) -> Result<String> {
    match polisher {
        Polisher::None => Ok(text.to_string()),
        Polisher::OpenAI => polish_openai(text, api_key_or_url, prompt, model).await,
        Polisher::Mistral => polish_mistral(text, api_key_or_url, prompt, model).await,
        Polisher::Ollama => polish_ollama(text, api_key_or_url, prompt, model).await,
    }
}

const DEFAULT_OPENAI_MODEL: &str = "gpt-5-nano-2025-08-07";

async fn polish_openai(
    text: &str,
    api_key: &str,
    system_prompt: &str,
    model: Option<&str>,
) -> Result<String> {
    let model = model.unwrap_or(DEFAULT_OPENAI_MODEL);
    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ]
        }))
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("OpenAI polish failed: {}", error_text));
    }

    let chat_response: ChatResponse = response.json().await?;
    chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("No response from OpenAI"))
}

const DEFAULT_MISTRAL_MODEL: &str = "mistral-small-latest";

async fn polish_mistral(
    text: &str,
    api_key: &str,
    system_prompt: &str,
    model: Option<&str>,
) -> Result<String> {
    let model = model.unwrap_or(DEFAULT_MISTRAL_MODEL);
    let client = reqwest::Client::new();
    let response = client
        .post(MISTRAL_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ]
        }))
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Mistral polish failed: {}", error_text));
    }

    let chat_response: ChatResponse = response.json().await?;
    chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("No response from Mistral"))
}

const DEFAULT_OLLAMA_MODEL: &str = "ministral-3:3b";
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Ollama API response structure
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

async fn polish_ollama(
    text: &str,
    server_url: &str,
    system_prompt: &str,
    model: Option<&str>,
) -> Result<String> {
    let model = model.unwrap_or(DEFAULT_OLLAMA_MODEL);
    let base_url = if server_url.is_empty() {
        DEFAULT_OLLAMA_URL
    } else {
        server_url
    };
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ],
            "stream": false
        }))
        .timeout(std::time::Duration::from_secs(120)) // Longer timeout for local LLM
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                anyhow!(
                    "Cannot connect to Ollama at {}. Is Ollama running? Start with: ollama serve",
                    base_url
                )
            } else {
                anyhow!("Ollama request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Ollama polish failed: {}", error_text));
    }

    let ollama_response: OllamaResponse = response.json().await?;
    Ok(ollama_response.message.content.trim().to_string())
}
