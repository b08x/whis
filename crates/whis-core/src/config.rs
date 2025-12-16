use serde::{Deserialize, Serialize};
use std::fmt;

/// Available transcription providers
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    #[default]
    OpenAI,
    Mistral,
    Groq,
    Deepgram,
    ElevenLabs,
    #[serde(rename = "local-whisper")]
    LocalWhisper,
    #[serde(rename = "remote-whisper")]
    RemoteWhisper,
}

impl TranscriptionProvider {
    /// Get the string identifier for this provider
    pub fn as_str(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "openai",
            TranscriptionProvider::Mistral => "mistral",
            TranscriptionProvider::Groq => "groq",
            TranscriptionProvider::Deepgram => "deepgram",
            TranscriptionProvider::ElevenLabs => "elevenlabs",
            TranscriptionProvider::LocalWhisper => "local-whisper",
            TranscriptionProvider::RemoteWhisper => "remote-whisper",
        }
    }

    /// Get the environment variable name for this provider's API key (or path/URL for local)
    pub fn api_key_env_var(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "OPENAI_API_KEY",
            TranscriptionProvider::Mistral => "MISTRAL_API_KEY",
            TranscriptionProvider::Groq => "GROQ_API_KEY",
            TranscriptionProvider::Deepgram => "DEEPGRAM_API_KEY",
            TranscriptionProvider::ElevenLabs => "ELEVENLABS_API_KEY",
            TranscriptionProvider::LocalWhisper => "LOCAL_WHISPER_MODEL_PATH",
            TranscriptionProvider::RemoteWhisper => "REMOTE_WHISPER_URL",
        }
    }

    /// List all available providers
    pub fn all() -> &'static [TranscriptionProvider] {
        &[
            TranscriptionProvider::OpenAI,
            TranscriptionProvider::Mistral,
            TranscriptionProvider::Groq,
            TranscriptionProvider::Deepgram,
            TranscriptionProvider::ElevenLabs,
            TranscriptionProvider::LocalWhisper,
            TranscriptionProvider::RemoteWhisper,
        ]
    }

    /// Human-readable display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "OpenAI",
            TranscriptionProvider::Mistral => "Mistral",
            TranscriptionProvider::Groq => "Groq",
            TranscriptionProvider::Deepgram => "Deepgram",
            TranscriptionProvider::ElevenLabs => "ElevenLabs",
            TranscriptionProvider::LocalWhisper => "Local Whisper",
            TranscriptionProvider::RemoteWhisper => "Remote Whisper",
        }
    }

    /// Whether this provider requires an API key (vs path/URL for local/remote)
    pub fn requires_api_key(&self) -> bool {
        !matches!(
            self,
            TranscriptionProvider::LocalWhisper | TranscriptionProvider::RemoteWhisper
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
            "mistral" => Ok(TranscriptionProvider::Mistral),
            "groq" => Ok(TranscriptionProvider::Groq),
            "deepgram" => Ok(TranscriptionProvider::Deepgram),
            "elevenlabs" => Ok(TranscriptionProvider::ElevenLabs),
            "local-whisper" | "localwhisper" | "whisper" => Ok(TranscriptionProvider::LocalWhisper),
            "remote-whisper" | "remotewhisper" => Ok(TranscriptionProvider::RemoteWhisper),
            _ => Err(format!(
                "Unknown provider: {}. Available: openai, mistral, groq, deepgram, elevenlabs, local-whisper, remote-whisper",
                s
            )),
        }
    }
}
