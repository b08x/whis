//! Sane Defaults for Whis
//!
//! This module defines the default configuration values used across
//! CLI, Desktop, and Mobile applications. When changing defaults
//! (e.g., switching default provider), update this file.
//!
//! # Changelog
//!
//! - **2026-01-05**: Default provider changed from OpenAI to Deepgram
//!   - Rationale: Deepgram offers $200 free credit vs OpenAI's limited trial
//!   - Better first-time user experience (critical first 5 seconds)
//!
//! # How to Change Defaults in Future
//!
//! To change the default transcription provider (or any other default):
//! 1. Edit the constant in this file (e.g., `DEFAULT_PROVIDER`)
//! 2. Add a changelog entry with date and rationale
//! 3. Rebuild all apps (CLI, Desktop, Mobile) - they'll all use the new default
//! 4. Update documentation if needed
//!
//! # Where Defaults Are Used
//!
//! - `config.rs`: TranscriptionProvider::default() impl
//! - `settings/transcription.rs`: TranscriptionSettings::default() impl
//! - `settings/ui.rs`: UiSettings::default() and VadSettings::default() impls
//! - `settings/services.rs`: OllamaConfig::default() impl
//! - `settings/post_processing.rs`: PostProcessor::default() and PostProcessingSettings::default() impls
//! - `whis-mobile/src/commands.rs`: Hardcoded fallbacks for Tauri store

use crate::{PostProcessor, TranscriptionProvider};

// =============================================================================
// TRANSCRIPTION DEFAULTS
// =============================================================================

/// Default transcription provider for new users
///
/// **Current:** Deepgram (since 2026-01-05)
///
/// **Previous:** OpenAI (until 2026-01-05)
///
/// **Rationale:** Deepgram offers $200 free credit, making it more accessible
/// for new users compared to OpenAI's limited trial. This improves the critical
/// first 5 seconds of user experience - users can start transcribing immediately
/// without needing to purchase API credits.
pub const DEFAULT_PROVIDER: TranscriptionProvider = TranscriptionProvider::Deepgram;

/// Default language (None = auto-detect)
///
/// When set to None, the transcription provider will automatically detect
/// the spoken language. Users can override this in settings or via CLI args.
pub const DEFAULT_LANGUAGE: Option<&str> = None;

// =============================================================================
// POST-PROCESSING DEFAULTS
// =============================================================================

/// Default post-processor provider when enabled
///
/// **Current:** OpenAI (since 2026-01-11)
///
/// **Rationale:** OpenAI provides excellent grammar and filler word cleanup.
/// Post-processing is disabled by default (via `enabled: false` in settings),
/// but when users enable it, OpenAI is the recommended starting point.
///
/// **Note:** The default post-processing prompt is defined in
/// `post_processing.rs` as `DEFAULT_POST_PROCESSING_PROMPT`.
pub const DEFAULT_POST_PROCESSOR: PostProcessor = PostProcessor::OpenAI;

// =============================================================================
// UI DEFAULTS
// =============================================================================

/// Default shortcut mode for triggering recording
///
/// - "system": User configures a keyboard shortcut in desktop settings to run `whis toggle`
/// - "direct": Whis captures the hotkey directly (requires input group membership on Linux)
///
/// Default is "system" because it's more reliable and requires no special permissions.
pub const DEFAULT_SHORTCUT_MODE: &str = "system";

/// Default keyboard shortcut for recording toggle (used when shortcut_mode is "direct")
///
/// Cross-platform shortcut that works on Windows, macOS, and Linux.
/// Users can customize this via `whis config shortcut <your-shortcut>`.
pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+W";

/// Default VAD (Voice Activity Detection) enabled state
///
/// VAD is disabled by default to ensure all audio is captured.
/// Enable with `whis config vad true` to automatically skip silence.
pub const DEFAULT_VAD_ENABLED: bool = false;

/// Default VAD threshold (0.0 = silence, 1.0 = loud speech)
///
/// Threshold of 0.5 provides good balance between skipping silence
/// and capturing soft speech. Adjust via `whis config vad-threshold <value>`.
pub const DEFAULT_VAD_THRESHOLD: f32 = 0.5;

/// Default chunk duration for progressive transcription (seconds)
///
/// 90 seconds provides a good balance between transcription quality
/// (more context) and response time. Adjust via `whis config chunk-size <seconds>`.
/// Smaller values (30s) feel more real-time, larger values (120s) improve accuracy.
pub const DEFAULT_CHUNK_DURATION_SECS: u64 = 90;

// =============================================================================
// SERVICE DEFAULTS
// =============================================================================

/// Default Ollama service URL
///
/// Points to local Ollama instance. If you run Ollama on a different
/// host/port, configure it via `whis config ollama-url <url>`.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default Ollama model for post-processing
///
/// qwen2.5:1.5b is chosen as default because it's:
/// - Small enough to run on most machines (< 1GB)
/// - Fast for post-processing tasks
/// - Good quality for grammar/filler word cleanup
///
/// Users can switch to larger models via `whis config ollama-model <model>`.
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5:1.5b";
