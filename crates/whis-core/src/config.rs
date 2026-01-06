use serde::{Deserialize, Serialize};
use std::fmt;

/// Available transcription providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    OpenAI,
    #[serde(rename = "openai-realtime")]
    OpenAIRealtime,
    Mistral,
    Groq,
    Deepgram,
    #[serde(rename = "deepgram-realtime")]
    DeepgramRealtime,
    ElevenLabs,
    #[serde(rename = "local-whisper")]
    LocalWhisper,
    #[serde(rename = "local-parakeet")]
    LocalParakeet,
}

impl Default for TranscriptionProvider {
    fn default() -> Self {
        crate::defaults::DEFAULT_PROVIDER
    }
}

impl TranscriptionProvider {
    /// Get the string identifier for this provider
    pub fn as_str(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "openai",
            TranscriptionProvider::OpenAIRealtime => "openai-realtime",
            TranscriptionProvider::Mistral => "mistral",
            TranscriptionProvider::Groq => "groq",
            TranscriptionProvider::Deepgram => "deepgram",
            TranscriptionProvider::DeepgramRealtime => "deepgram-realtime",
            TranscriptionProvider::ElevenLabs => "elevenlabs",
            TranscriptionProvider::LocalWhisper => "local-whisper",
            TranscriptionProvider::LocalParakeet => "local-parakeet",
        }
    }

    /// Get the environment variable name for this provider's API key (or path/URL for local)
    pub fn api_key_env_var(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI | TranscriptionProvider::OpenAIRealtime => {
                "OPENAI_API_KEY"
            }
            TranscriptionProvider::Mistral => "MISTRAL_API_KEY",
            TranscriptionProvider::Groq => "GROQ_API_KEY",
            TranscriptionProvider::Deepgram | TranscriptionProvider::DeepgramRealtime => {
                "DEEPGRAM_API_KEY"
            }
            TranscriptionProvider::ElevenLabs => "ELEVENLABS_API_KEY",
            TranscriptionProvider::LocalWhisper => "LOCAL_WHISPER_MODEL_PATH",
            TranscriptionProvider::LocalParakeet => "LOCAL_PARAKEET_MODEL_PATH",
        }
    }

    /// List all available providers (ordered by recommendation)
    pub fn all() -> &'static [TranscriptionProvider] {
        &[
            TranscriptionProvider::Deepgram,
            TranscriptionProvider::DeepgramRealtime,
            TranscriptionProvider::OpenAI,
            TranscriptionProvider::OpenAIRealtime,
            TranscriptionProvider::Mistral,
            TranscriptionProvider::Groq,
            TranscriptionProvider::ElevenLabs,
            TranscriptionProvider::LocalWhisper,
            TranscriptionProvider::LocalParakeet,
        ]
    }

    /// Human-readable display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "OpenAI",
            TranscriptionProvider::OpenAIRealtime => "OpenAI Realtime",
            TranscriptionProvider::Mistral => "Mistral",
            TranscriptionProvider::Groq => "Groq",
            TranscriptionProvider::Deepgram => "Deepgram",
            TranscriptionProvider::DeepgramRealtime => "Deepgram Realtime",
            TranscriptionProvider::ElevenLabs => "ElevenLabs",
            TranscriptionProvider::LocalWhisper => "Local Whisper",
            TranscriptionProvider::LocalParakeet => "Local Parakeet",
        }
    }

    /// Whether this provider requires an API key (vs path/URL for local/remote)
    pub fn requires_api_key(&self) -> bool {
        !matches!(
            self,
            TranscriptionProvider::LocalWhisper | TranscriptionProvider::LocalParakeet
        )
    }

    /// Whether this is a local provider (no cloud API)
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            TranscriptionProvider::LocalWhisper | TranscriptionProvider::LocalParakeet
        )
    }
}

impl fmt::Display for TranscriptionProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TranscriptionProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(TranscriptionProvider::OpenAI),
            "openai-realtime" | "openairealttime" | "realtime" => {
                Ok(TranscriptionProvider::OpenAIRealtime)
            }
            "mistral" => Ok(TranscriptionProvider::Mistral),
            "groq" => Ok(TranscriptionProvider::Groq),
            "deepgram" => Ok(TranscriptionProvider::Deepgram),
            "deepgram-realtime" | "deepgramrealtime" => Ok(TranscriptionProvider::DeepgramRealtime),
            "elevenlabs" => Ok(TranscriptionProvider::ElevenLabs),
            "local-whisper" | "localwhisper" | "whisper" => Ok(TranscriptionProvider::LocalWhisper),
            "local-parakeet" | "localparakeet" | "parakeet" => {
                Ok(TranscriptionProvider::LocalParakeet)
            }
            _ => Err(format!(
                "Unknown provider: {}. Available: openai, openai-realtime, mistral, groq, deepgram, deepgram-realtime, elevenlabs, local-whisper, local-parakeet",
                s
            )),
        }
    }
}
