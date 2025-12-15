use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "whis")]
#[command(version)]
#[command(about = "Voice-to-text CLI using OpenAI Whisper or Mistral Voxtral")]
#[command(after_help = "Run 'whis' without arguments to record once (press Enter to stop).")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Polish transcript with LLM (cleanup grammar, filler words)
    #[arg(long)]
    pub polish: bool,

    /// Output style for transcript [possible values: ai-prompt, email, notes]
    #[arg(long = "as", value_name = "STYLE")]
    pub style: Option<String>,
}

#[derive(Subcommand)]
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

        /// Set the transcription provider (openai or mistral)
        #[arg(long)]
        provider: Option<String>,

        /// Set the language hint (ISO-639-1 code: en, de, fr, etc.) or "auto" for auto-detect
        #[arg(long)]
        language: Option<String>,

        /// Set the polisher for transcript cleanup (none, openai, or mistral)
        #[arg(long)]
        polisher: Option<String>,

        /// Set custom polish prompt for transcript cleanup
        #[arg(long)]
        polish_prompt: Option<String>,

        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
}
