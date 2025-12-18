mod app;
mod args;
mod commands;
mod hotkey;
mod ipc;
mod service;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = args::Cli::parse();

    // Enable verbose logging if requested
    whis_core::set_verbose(cli.verbose);

    match cli.command {
        Some(args::Commands::Listen { hotkey }) => commands::listen::run(hotkey),
        Some(args::Commands::Stop) => commands::stop::run(),
        Some(args::Commands::Status) => commands::status::run(),
        Some(args::Commands::Config {
            openai_api_key,
            mistral_api_key,
            groq_api_key,
            deepgram_api_key,
            elevenlabs_api_key,
            provider,
            whisper_model_path,
            remote_whisper_url,
            ollama_url,
            ollama_model,
            language,
            polisher,
            polish_prompt,
            show,
        }) => commands::config::run(
            openai_api_key,
            mistral_api_key,
            groq_api_key,
            deepgram_api_key,
            elevenlabs_api_key,
            provider,
            whisper_model_path,
            remote_whisper_url,
            ollama_url,
            ollama_model,
            language,
            polisher,
            polish_prompt,
            show,
        ),
        Some(args::Commands::Presets { action }) => commands::presets::run(action),
        Some(args::Commands::Setup { mode }) => commands::setup::run(mode),
        Some(args::Commands::Models { action }) => commands::models::run(action),
        None => commands::record_once::run(cli.polish, cli.preset),
    }
}
