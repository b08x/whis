use clap::{Parser, Subcommand, ValueHint};

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

    /// Polish transcript with LLM (cleanup grammar, filler words)
    #[arg(long)]
    pub polish: bool,

    /// Output preset for transcript (run 'whis presets' to see all)
    #[arg(long = "as", value_name = "PRESET")]
    pub preset: Option<String>,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Start the background service that listens for hotkey triggers
    Listen {
        /// Hotkey to trigger recording (e.g., "ctrl+shift+r")
        #[arg(short = 'k', long, default_value = "ctrl+shift+r")]
        hotkey: String,
    },

    /// Stop the background service
    Stop,

    /// Check service status
    Status,

    /// Configure settings (API keys, provider, etc.)
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

        /// Set the transcription provider (openai, mistral, groq, deepgram, elevenlabs, local-whisper, remote-whisper)
        #[arg(long)]
        provider: Option<String>,

        /// Path to whisper.cpp model file for local transcription (e.g., ~/.local/share/whis/models/ggml-small.bin)
        #[arg(long)]
        whisper_model_path: Option<String>,

        /// Remote whisper server URL for self-hosted transcription (e.g., http://localhost:8765)
        #[arg(long)]
        remote_whisper_url: Option<String>,

        /// Ollama server URL for local polishing (default: http://localhost:11434)
        #[arg(long)]
        ollama_url: Option<String>,

        /// Ollama model name for local polishing (default: ministral-3:3b)
        #[arg(long)]
        ollama_model: Option<String>,

        /// Set the language hint (ISO-639-1 code: en, de, fr, etc.) or "auto" for auto-detect
        #[arg(long)]
        language: Option<String>,

        /// Set the polisher for transcript cleanup (none, openai, mistral, or ollama)
        #[arg(long)]
        polisher: Option<String>,

        /// Set custom polish prompt for transcript cleanup
        #[arg(long)]
        polish_prompt: Option<String>,

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

    /// Setup for fully local (on-device) transcription and polishing
    Local,

    /// Setup for self-hosted server (Docker)
    SelfHosted {
        /// URL of your self-hosted whisper server (e.g., http://myserver:8765)
        #[arg(long)]
        url: Option<String>,
    },
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
