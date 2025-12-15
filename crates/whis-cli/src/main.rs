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

    match cli.command {
        Some(args::Commands::Listen { hotkey }) => commands::listen::run(hotkey),
        Some(args::Commands::Stop) => commands::stop::run(),
        Some(args::Commands::Status) => commands::status::run(),
        Some(args::Commands::Config {
            openai_api_key,
            mistral_api_key,
            provider,
            language,
            polisher,
            polish_prompt,
            show,
        }) => commands::config::run(
            openai_api_key,
            mistral_api_key,
            provider,
            language,
            polisher,
            polish_prompt,
            show,
        ),
        None => commands::record_once::run(cli.polish, cli.style),
    }
}
