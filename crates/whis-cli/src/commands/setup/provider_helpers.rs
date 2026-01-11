//! Provider-related constants and helpers for setup

use whis_core::TranscriptionProvider;

/// Get cloud providers for setup wizard (ordered by recommendation from whis-core).
///
/// Excludes local providers and realtime variants (realtime is selected via
/// method choice in cloud.rs after selecting the base provider).
pub fn cloud_providers() -> Vec<TranscriptionProvider> {
    TranscriptionProvider::cloud_providers().cloned().collect()
}

/// Cloud post-processing providers
pub const PP_PROVIDERS: &[TranscriptionProvider] = &[
    TranscriptionProvider::OpenAI,
    TranscriptionProvider::Mistral,
];

/// Provider descriptions for display
pub fn provider_description(provider: &TranscriptionProvider) -> &'static str {
    match provider {
        TranscriptionProvider::OpenAIRealtime | TranscriptionProvider::DeepgramRealtime => {
            "Streaming"
        }
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
