//! Provider-related constants and helpers for setup

use whis_core::{Settings, TranscriptionProvider};

/// Cloud providers for transcription (excludes local providers)
/// Note: OpenAIRealtime is excluded - it's selected via method choice in cloud.rs
pub const CLOUD_PROVIDERS: &[TranscriptionProvider] = &[
    TranscriptionProvider::OpenAI,
    TranscriptionProvider::Mistral,
    TranscriptionProvider::Groq,
    TranscriptionProvider::Deepgram,
    TranscriptionProvider::ElevenLabs,
];

/// Cloud post-processing providers
pub const PP_PROVIDERS: &[TranscriptionProvider] = &[
    TranscriptionProvider::OpenAI,
    TranscriptionProvider::Mistral,
];

/// Provider descriptions for display
pub fn provider_description(provider: &TranscriptionProvider) -> &'static str {
    match provider {
        TranscriptionProvider::OpenAI => "High quality, most popular",
        TranscriptionProvider::OpenAIRealtime => "Streaming, lower latency",
        TranscriptionProvider::Mistral => "European provider, good quality",
        TranscriptionProvider::Groq => "Very fast, good for real-time",
        TranscriptionProvider::Deepgram => "Fast, good for conversations",
        TranscriptionProvider::DeepgramRealtime => "Streaming, very fast (~150ms)",
        TranscriptionProvider::ElevenLabs => "Good multilingual support",
        _ => "",
    }
}

/// Get the API key URL for a provider
pub fn api_key_url(provider: &TranscriptionProvider) -> &'static str {
    match provider {
        TranscriptionProvider::OpenAI | TranscriptionProvider::OpenAIRealtime => {
            "https://platform.openai.com/api-keys"
        }
        TranscriptionProvider::Mistral => "https://console.mistral.ai/api-keys",
        TranscriptionProvider::Groq => "https://console.groq.com/keys",
        TranscriptionProvider::Deepgram | TranscriptionProvider::DeepgramRealtime => {
            "https://console.deepgram.com"
        }
        TranscriptionProvider::ElevenLabs => "https://elevenlabs.io/app/settings/api-keys",
        _ => "",
    }
}

/// Get configured and unconfigured cloud providers
pub fn get_provider_status(
    settings: &Settings,
) -> (
    Vec<(TranscriptionProvider, String)>,
    Vec<TranscriptionProvider>,
) {
    let mut configured = Vec::new();
    let mut unconfigured = Vec::new();

    for provider in CLOUD_PROVIDERS {
        if let Some(key) = settings.transcription.api_key_for(provider) {
            configured.push((provider.clone(), key));
        } else {
            unconfigured.push(provider.clone());
        }
    }

    (configured, unconfigured)
}
