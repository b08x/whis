mod app;
mod args;
mod commands;
mod error;
mod hotkey;
mod ipc;
mod service;
mod ui;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    // Run CLI and handle errors with helpful messages
    if let Err(err) = run() {
        error::display_anyhow_error(err);
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = args::Cli::parse();

    // Enable verbose logging if requested
    whis_core::set_verbose(cli.verbose);

    match cli.command {
        Some(args::Commands::Start { hotkey }) => commands::start::run(hotkey),
        Some(args::Commands::Stop) => commands::stop::run(),
        Some(args::Commands::Restart { hotkey }) => commands::restart::run(hotkey),
        Some(args::Commands::Status) => commands::status::run(),
        Some(args::Commands::Config {
            key,
            value,
            list,
            path,
        }) => commands::config::run(key, value, list, path),
        Some(args::Commands::Preset { action }) => commands::preset::run(action),
        Some(args::Commands::Setup { mode }) => commands::setup::run(mode),
        Some(args::Commands::Model { action }) => commands::model::run(action),
        None => {
            // Determine input source from CLI arguments
            let input_source = if let Some(path) = cli.input.file {
                commands::record::InputSource::File(path)
            } else if cli.input.stdin {
                commands::record::InputSource::Stdin {
                    format: cli.input.format.clone(),
                }
            } else {
                commands::record::InputSource::Microphone
            };

            // Create configuration and run record command
            let config = commands::record::RecordConfig::new(
                input_source,
                cli.processing.post_process,
                cli.processing.preset,
                cli.output.print,
                cli.processing.duration,
                cli.processing.no_vad,
                cli.output.save_raw,
            )?;
            commands::record::run(config)
        }
    }
}
