use anyhow::{Result, anyhow};
use std::io::{self, Write};
use whis_core::{
    AudioRecorder, DEFAULT_POLISH_PROMPT, Polisher, Preset, RecordingOutput, Settings,
    TranscriptionProvider, copy_to_clipboard, ollama, parallel_transcribe, polish,
    transcribe_audio,
};

use crate::app;

/// Resolve which polisher to use based on priority:
/// 1. Preset override (if specified and valid)
/// 2. Settings polisher (if configured)
/// 3. Transcription provider fallback (OpenAI/Mistral only, others check key availability)
fn resolve_polisher(
    preset: &Option<Preset>,
    settings: &Settings,
    provider: &TranscriptionProvider,
) -> Polisher {
    // 1. Preset override
    if let Some(p) = preset
        && let Some(polisher_str) = &p.polisher
    {
        match polisher_str.parse() {
            Ok(polisher) => return polisher,
            Err(_) => eprintln!("Warning: Invalid polisher '{}' in preset", polisher_str),
        }
    }

    // 2. Settings polisher
    if settings.polisher != Polisher::None {
        return settings.polisher.clone();
    }

    // 3. Transcription provider fallback
    // OpenAI and Mistral have built-in polish capabilities
    // Other providers need an available OpenAI or Mistral key
    match provider {
        TranscriptionProvider::OpenAI => Polisher::OpenAI,
        TranscriptionProvider::Mistral => Polisher::Mistral,
        // Cloud providers without built-in LLM: try OpenAI/Mistral keys
        TranscriptionProvider::Groq
        | TranscriptionProvider::Deepgram
        | TranscriptionProvider::ElevenLabs => {
            if settings
                .get_api_key_for(&TranscriptionProvider::OpenAI)
                .is_some()
            {
                Polisher::OpenAI
            } else if settings
                .get_api_key_for(&TranscriptionProvider::Mistral)
                .is_some()
            {
                Polisher::Mistral
            } else {
                Polisher::None
            }
        }
        // Self-hosted transcription: default to Ollama (likely also self-hosted)
        TranscriptionProvider::LocalWhisper | TranscriptionProvider::RemoteWhisper => {
            Polisher::Ollama
        }
    }
}

pub fn run(polish_flag: bool, preset_name: Option<String>) -> Result<()> {
    // Load preset if provided
    let preset: Option<Preset> = if let Some(name) = preset_name {
        let (p, _source) = Preset::load(&name).map_err(|e| anyhow!("{}", e))?;
        Some(p)
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

    println!("Press Enter to stop recording");
    print!("Recording...");
    io::stdout().flush()?;
    app::wait_for_enter()?;

    // Finalize recording and get output
    let audio_result = recorder.finalize_recording()?;

    // Transcribe based on output type
    // Continue on same line (raw mode doesn't echo newline)
    app::typewriter(" Transcribing...", 25);
    let transcription = match audio_result {
        RecordingOutput::Single(audio_data) => {
            // Small file - simple transcription
            transcribe_audio(
                &config.provider,
                &config.api_key,
                config.language.as_deref(),
                audio_data,
            )?
        }
        RecordingOutput::Chunked(chunks) => {
            // Large file - parallel transcription
            runtime.block_on(parallel_transcribe(
                &config.provider,
                &config.api_key,
                config.language.as_deref(),
                chunks,
                None,
            ))?
        }
    };

    // Apply polishing if enabled (via flag or preset)
    let settings = Settings::load();
    let should_polish = polish_flag || preset.is_some();

    let final_text = if should_polish {
        let polisher = resolve_polisher(&preset, &settings, &config.provider);

        // Get API key or URL depending on polisher type
        // For cloud polishers: need API key
        // For Ollama: need server URL (defaults to localhost:11434)
        let api_key_or_url = if polisher.requires_api_key() {
            settings.get_polisher_api_key()
        } else if polisher == Polisher::Ollama {
            // For local-whisper provider, auto-start Ollama if not running
            let ollama_url = settings
                .get_ollama_url()
                .unwrap_or_else(|| ollama::DEFAULT_OLLAMA_URL.to_string());

            if matches!(config.provider, TranscriptionProvider::LocalWhisper) {
                // Auto-start Ollama for embedded (local) mode
                match ollama::ensure_ollama_running(&ollama_url) {
                    Ok(_) => Some(ollama_url),
                    Err(e) => {
                        eprintln!("Warning: Could not start Ollama: {}", e);
                        eprintln!("Skipping polish. Start Ollama manually: ollama serve");
                        None // Skip polish
                    }
                }
            } else {
                Some(ollama_url)
            }
        } else {
            None
        };

        if let Some(key_or_url) = api_key_or_url {
            app::typewriter(" Polishing...", 25);

            // Priority: preset prompt > settings prompt > default
            let prompt = if let Some(ref p) = preset {
                p.prompt.as_str()
            } else {
                settings
                    .polish_prompt
                    .as_deref()
                    .unwrap_or(DEFAULT_POLISH_PROMPT)
            };

            // For Ollama, use settings model if preset doesn't specify one
            let ollama_model = settings.get_ollama_model();
            let model = if let Some(ref p) = preset {
                p.model.as_deref()
            } else if polisher == Polisher::Ollama {
                ollama_model.as_deref()
            } else {
                None
            };
            match runtime.block_on(polish(
                &transcription,
                &polisher,
                &key_or_url,
                prompt,
                model,
            )) {
                Ok(polished) => polished,
                Err(e) => {
                    eprintln!("Polish warning: {e}");
                    eprintln!("Falling back to raw transcript");
                    transcription
                }
            }
        } else {
            // Warn when polish was requested but we can't perform it
            // (Ollama case already warned above when auto-start failed)
            if polisher != Polisher::None && polisher.requires_api_key() {
                eprintln!(
                    "Warning: No API key for {} polisher, skipping polish",
                    polisher
                );
            }
            transcription
        }
    } else {
        transcription
    };

    // Copy to clipboard
    copy_to_clipboard(&final_text)?;

    app::typewriter(" Done", 20);
    println!();  // End the status line
    println!("Copied to clipboard");

    Ok(())
}
