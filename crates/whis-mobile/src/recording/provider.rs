//! Provider-related helpers for API key lookup and validation.
//!
//! This module centralizes provider → API key mapping and validation logic,
//! avoiding duplication across commands and config loading.

/// Get the Tauri store key for a provider's API key.
///
/// Maps provider strings (including realtime variants) to their
/// corresponding API key store keys.
///
/// # Examples
///
/// ```
/// assert_eq!(api_key_store_key("openai"), Some("openai_api_key"));
/// assert_eq!(api_key_store_key("openai-realtime"), Some("openai_api_key"));
/// assert_eq!(api_key_store_key("deepgram"), Some("deepgram_api_key"));
/// assert_eq!(api_key_store_key("unknown"), None);
/// ```
pub fn api_key_store_key(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" | "openai-realtime" => Some("openai_api_key"),
        "mistral" => Some("mistral_api_key"),
        "groq" => Some("groq_api_key"),
        "deepgram" | "deepgram-realtime" => Some("deepgram_api_key"),
        "elevenlabs" => Some("elevenlabs_api_key"),
        _ => None,
    }
}

/// Validate API key format for a given provider.
///
/// Performs basic format validation (prefix, length) without
/// making network calls. Returns `true` if the key looks valid.
///
/// # Provider-specific validation:
///
/// - **OpenAI**: Must start with `sk-` and be longer than 20 chars
/// - **Groq**: Must start with `gsk_` and be longer than 20 chars
/// - **Others**: Just checks minimum length (> 20 chars)
pub fn validate_api_key_format(key: &str, provider: &str) -> bool {
    match provider {
        "openai" | "openai-realtime" => key.starts_with("sk-") && key.len() > 20,
        "groq" => key.starts_with("gsk_") && key.len() > 20,
        "mistral" | "deepgram" | "deepgram-realtime" | "elevenlabs" => key.len() > 20,
        _ => false,
    }
}

/// Check if a provider supports realtime streaming.
///
/// Returns `true` for providers that can use WebSocket-based
/// realtime transcription.
pub fn supports_realtime_streaming(provider: &str) -> bool {
    matches!(
        provider,
        "openai" | "openai-realtime" | "deepgram" | "deepgram-realtime"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_store_key() {
        // OpenAI variants
        assert_eq!(api_key_store_key("openai"), Some("openai_api_key"));
        assert_eq!(api_key_store_key("openai-realtime"), Some("openai_api_key"));

        // Deepgram variants
        assert_eq!(api_key_store_key("deepgram"), Some("deepgram_api_key"));
        assert_eq!(
            api_key_store_key("deepgram-realtime"),
            Some("deepgram_api_key")
        );

        // Other providers
        assert_eq!(api_key_store_key("mistral"), Some("mistral_api_key"));
        assert_eq!(api_key_store_key("groq"), Some("groq_api_key"));
        assert_eq!(api_key_store_key("elevenlabs"), Some("elevenlabs_api_key"));

        // Unknown
        assert_eq!(api_key_store_key("unknown"), None);
        assert_eq!(api_key_store_key(""), None);
    }

    #[test]
    fn test_validate_api_key_format() {
        // OpenAI - needs sk- prefix
        assert!(validate_api_key_format(
            "sk-1234567890123456789012345",
            "openai"
        ));
        assert!(validate_api_key_format(
            "sk-1234567890123456789012345",
            "openai-realtime"
        ));
        assert!(!validate_api_key_format(
            "1234567890123456789012345",
            "openai"
        )); // No prefix
        assert!(!validate_api_key_format("sk-short", "openai")); // Too short

        // Groq - needs gsk_ prefix
        assert!(validate_api_key_format(
            "gsk_1234567890123456789012345",
            "groq"
        ));
        assert!(!validate_api_key_format(
            "1234567890123456789012345",
            "groq"
        )); // No prefix

        // Others - just length check
        assert!(validate_api_key_format(
            "1234567890123456789012345",
            "mistral"
        ));
        assert!(validate_api_key_format(
            "1234567890123456789012345",
            "deepgram"
        ));
        assert!(!validate_api_key_format("short", "mistral")); // Too short

        // Unknown provider
        assert!(!validate_api_key_format("anything", "unknown"));
    }

    #[test]
    fn test_supports_realtime_streaming() {
        assert!(supports_realtime_streaming("openai"));
        assert!(supports_realtime_streaming("openai-realtime"));
        assert!(supports_realtime_streaming("deepgram"));
        assert!(supports_realtime_streaming("deepgram-realtime"));

        assert!(!supports_realtime_streaming("mistral"));
        assert!(!supports_realtime_streaming("groq"));
        assert!(!supports_realtime_streaming("elevenlabs"));
        assert!(!supports_realtime_streaming("unknown"));
    }
}
