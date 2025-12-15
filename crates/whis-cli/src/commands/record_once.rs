use anyhow::Result;
use std::io::{self, Write};
use whis_core::{
    AudioRecorder, OutputStyle, Polisher, RecordingOutput, Settings, copy_to_clipboard,
    parallel_transcribe, polish, transcribe_audio, DEFAULT_POLISH_PROMPT,
};
use crate::app;

pub fn run(polish_flag: bool, style: Option<String>) -> Result<()> {
    // Parse style if provided
    let style: Option<OutputStyle> = if let Some(style_name) = style {
        match style_name.parse() {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };
    // Create Tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()?;

    // Check if FFmpeg is available
    app::ensure_ffmpeg_installed()?;

    // Load transcription configuration (provider + API key)
    let config = app::load_transcription_config()?;

    // Create recorder and start recording
    let mut recorder = AudioRecorder::new()?;
    recorder.start_recording()?;

    print!("Recording... (press Enter to stop)");
    io::stdout().flush()?;
    app::wait_for_enter()?;

    // Finalize recording and get output
    let audio_result = recorder.finalize_recording()?;

    // Transcribe based on output type
    let transcription = match audio_result {
        RecordingOutput::Single(audio_data) => {
            // Small file - simple transcription
            print!("\rTranscribing...                        \n");
            io::stdout().flush()?;

            match transcribe_audio(
                &config.provider,
                &config.api_key,
                config.language.as_deref(),
                audio_data,
            ) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("Transcription error: {e}");
                    std::process::exit(1);
                }
            }
        }
        RecordingOutput::Chunked(chunks) => {
            // Large file - parallel transcription
            print!("\rTranscribing...                        \n");
            io::stdout().flush()?;

            runtime.block_on(async {
                match parallel_transcribe(
                    &config.provider,
                    &config.api_key,
                    config.language.as_deref(),
                    chunks,
                    None,
                )
                .await
                {
                    Ok(text) => text,
                    Err(e) => {
                        eprintln!("Transcription error: {e}");
                        std::process::exit(1);
                    }
                }
            })
        }
    };

    // Apply polishing if enabled (via flag, style, or settings)
    let settings = Settings::load();
    let should_polish = polish_flag || style.is_some() || settings.polisher != Polisher::None;

    let final_text = if should_polish {
        // Determine which polisher to use
        let polisher = if (polish_flag || style.is_some()) && settings.polisher == Polisher::None {
            // Flag/style enabled but no polisher configured - use transcription provider
            match config.provider {
                whis_core::TranscriptionProvider::OpenAI => Polisher::OpenAI,
                whis_core::TranscriptionProvider::Mistral => Polisher::Mistral,
            }
        } else {
            settings.polisher.clone()
        };

        // Get API key for polisher
        let api_key = match &polisher {
            Polisher::None => None,
            Polisher::OpenAI => settings
                .openai_api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok()),
            Polisher::Mistral => settings
                .mistral_api_key
                .clone()
                .or_else(|| std::env::var("MISTRAL_API_KEY").ok()),
        };

        if let Some(api_key) = api_key {
            print!("Polishing...");
            io::stdout().flush()?;

            // Priority: style prompt > settings prompt > default
            let prompt = if let Some(ref s) = style {
                s.prompt()
            } else {
                settings
                    .polish_prompt
                    .as_deref()
                    .unwrap_or(DEFAULT_POLISH_PROMPT)
            };

            match runtime.block_on(polish(&transcription, &polisher, &api_key, prompt)) {
                Ok(polished) => {
                    print!("\r              \r");
                    io::stdout().flush()?;
                    polished
                }
                Err(e) => {
                    eprintln!("\rPolish warning: {e}");
                    eprintln!("Falling back to raw transcript");
                    transcription
                }
            }
        } else {
            eprintln!("Warning: No API key for polisher, skipping polish");
            transcription
        }
    } else {
        transcription
    };

    // Copy to clipboard
    copy_to_clipboard(&final_text)?;

    println!("Copied to clipboard");

    Ok(())
}
