use clap::{Parser, Subcommand, ValueHint};
use std::path::PathBuf;
use std::time::Duration;

/// Parse a duration string like "10s", "30s", "1m", "90"
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("duration cannot be empty".to_string());
    }

    // Check for suffix
    if let Some(num_str) = s.strip_suffix('s') {
        let secs: u64 = num_str
            .parse()
            .map_err(|_| format!("invalid number: {}", num_str))?;
        Ok(Duration::from_secs(secs))
    } else if let Some(num_str) = s.strip_suffix('m') {
        let mins: u64 = num_str
            .parse()
            .map_err(|_| format!("invalid number: {}", num_str))?;
        Ok(Duration::from_secs(mins * 60))
    } else {
        // No suffix, assume seconds
        let secs: u64 = s.parse().map_err(|_| format!("invalid duration: {}", s))?;
        Ok(Duration::from_secs(secs))
    }
}

#[derive(Parser)]
#[command(name = "whis")]
#[command(version)]
#[command(about = "Voice-to-text CLI with multiple transcription providers")]
#[command(after_help = "Run 'whis' without arguments to record once (press Enter to stop).")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose output for debugging (audio device, clipboard, etc.)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Post-process transcript with LLM (cleanup grammar, filler words)
    #[arg(long)]
    pub post_process: bool,

    /// Output preset for transcript (run 'whis presets' to see all)
    #[arg(long = "as", value_name = "PRESET")]
    pub preset: Option<String>,

    /// Transcribe audio from file instead of recording
    #[arg(short = 'f', long, value_hint = ValueHint::FilePath)]
    pub file: Option<PathBuf>,

    /// Read audio from stdin (use with pipes, e.g., `yt-dlp ... | whis --stdin`)
    #[arg(long)]
    pub stdin: bool,

    /// Input audio format when using --stdin (default: mp3)
    #[arg(long, default_value = "mp3")]
    pub format: String,

    /// Print output to stdout instead of copying to clipboard
    #[arg(long)]
    pub print: bool,

    /// Record for a fixed duration (e.g., "10s", "30s", "1m")
    /// Useful for non-interactive environments like AI assistant shell modes
    #[arg(short = 'd', long, value_parser = parse_duration)]
    pub duration: Option<Duration>,

    /// Disable Voice Activity Detection (records all audio including silence)
    #[arg(long)]
    pub no_vad: bool,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Start the background service that listens for hotkey triggers
    Listen {
        /// Hotkey to trigger recording (e.g., "ctrl+alt+w" or "cmd+option+w" on macOS)
        #[arg(short = 'k', long, default_value = "ctrl+alt+w")]
        hotkey: String,
    },

    /// Stop the background service
    Stop,

    /// Check service status
    Status,

    /// Configure settings (API keys, transcription service, post-processing, etc.)
    Config {
        /// Set your OpenAI API key
        #[arg(long)]
        openai_api_key: Option<String>,

        /// Set your Mistral API key
        #[arg(long)]
        mistral_api_key: Option<String>,

        /// Set your Groq API key
        #[arg(long)]
        groq_api_key: Option<String>,

        /// Set your Deepgram API key
        #[arg(long)]
        deepgram_api_key: Option<String>,

        /// Set your ElevenLabs API key
        #[arg(long)]
        elevenlabs_api_key: Option<String>,

        /// Set the transcription provider (openai, mistral, groq, deepgram, elevenlabs, local-whisper)
        #[arg(long)]
        provider: Option<String>,

        /// Path to whisper.cpp model file for local transcription (e.g., ~/.local/share/whis/models/ggml-small.bin)
        #[arg(long)]
        whisper_model_path: Option<String>,

        /// Ollama server URL for local post-processing (default: http://localhost:11434)
        #[arg(long)]
        ollama_url: Option<String>,

        /// Ollama model name for local post-processing (default: qwen2.5:1.5b)
        #[arg(long)]
        ollama_model: Option<String>,

        /// Set the language hint (ISO-639-1 code: en, de, fr, etc.) or "auto" for auto-detect
        #[arg(long)]
        language: Option<String>,

        /// Set the post-processor for transcript cleanup (none, openai, mistral, or ollama)
        #[arg(long)]
        post_processor: Option<String>,

        /// Set custom post-processing prompt for transcript cleanup
        #[arg(long)]
        post_processing_prompt: Option<String>,

        /// Enable Voice Activity Detection (skips silence during recording)
        #[arg(long)]
        vad: Option<bool>,

        /// VAD speech detection threshold (0.0-1.0, default 0.5)
        /// Lower = more sensitive, Higher = stricter
        #[arg(long)]
        vad_threshold: Option<f32>,

        /// Show current configuration
        #[arg(long)]
        show: bool,
    },

    /// Manage output presets
    Presets {
        #[command(subcommand)]
        action: Option<PresetsAction>,
    },

    /// Quick setup wizard for different usage modes
    Setup {
        #[command(subcommand)]
        mode: SetupMode,
    },

    /// List available models (whisper, ollama)
    Models {
        #[command(subcommand)]
        action: Option<ModelsAction>,
    },
}

#[derive(Subcommand)]
pub enum SetupMode {
    /// Setup for cloud providers (OpenAI, Mistral, Groq, etc.)
    Cloud,

    /// Setup for fully local (on-device) transcription and post-processing
    Local,
}

#[derive(Subcommand)]
pub enum PresetsAction {
    /// List all available presets (default)
    List,

    /// Show details of a specific preset
    Show {
        /// Name of the preset to show
        #[arg(value_hint = ValueHint::Other)]
        name: String,
    },

    /// Print a JSON template for creating a new preset
    New {
        /// Name for the new preset
        #[arg(value_hint = ValueHint::Other)]
        name: String,
    },

    /// Edit a preset in your editor ($EDITOR or $VISUAL)
    Edit {
        /// Name of the preset to edit (creates if doesn't exist)
        #[arg(value_hint = ValueHint::Other)]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ModelsAction {
    /// List available whisper models with install status (default)
    Whisper,

    /// List available Ollama models from server
    Ollama {
        /// Ollama server URL (default: http://localhost:11434)
        #[arg(long)]
        url: Option<String>,
    },
}
