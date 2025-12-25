use anyhow::{Result, anyhow};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::Duration;
use whis_core::{
    AudioRecorder, DEFAULT_POST_PROCESSING_PROMPT, PostProcessor, Preset, RecordingOutput,
    Settings, TranscriptionProvider, copy_to_clipboard, load_audio_file, load_audio_stdin, ollama,
    parallel_transcribe, post_process, transcribe_audio,
};

use crate::app;

/// Resolve which post-processor to use based on priority:
/// 1. Preset override (if specified and valid)
/// 2. Settings post-processor (if configured)
/// 3. Transcription provider fallback (OpenAI/Mistral only, others check key availability)
fn resolve_post_processor(
    preset: &Option<Preset>,
    settings: &Settings,
    provider: &TranscriptionProvider,
) -> PostProcessor {
    // 1. Preset override
    if let Some(p) = preset
        && let Some(post_processor_str) = &p.post_processor
    {
        match post_processor_str.parse() {
            Ok(post_processor) => return post_processor,
            Err(_) => eprintln!(
                "Warning: Invalid post-processor '{}' in preset",
                post_processor_str
            ),
        }
    }

    // 2. Settings post-processor
    if settings.post_processor != PostProcessor::None {
        return settings.post_processor.clone();
    }

    // 3. Transcription provider fallback
    // OpenAI and Mistral have built-in post-processing capabilities
    // Other providers need an available OpenAI or Mistral key
    match provider {
        TranscriptionProvider::OpenAI => PostProcessor::OpenAI,
        TranscriptionProvider::Mistral => PostProcessor::Mistral,
        // Cloud providers without built-in LLM: try OpenAI/Mistral keys
        TranscriptionProvider::Groq
        | TranscriptionProvider::Deepgram
        | TranscriptionProvider::ElevenLabs => {
            if settings
                .get_api_key_for(&TranscriptionProvider::OpenAI)
                .is_some()
            {
                PostProcessor::OpenAI
            } else if settings
                .get_api_key_for(&TranscriptionProvider::Mistral)
                .is_some()
            {
                PostProcessor::Mistral
            } else {
                PostProcessor::None
            }
        }
        // Local transcription: default to Ollama (local post-processing)
        TranscriptionProvider::LocalWhisper => PostProcessor::Ollama,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    post_process_flag: bool,
    preset_name: Option<String>,
    file_path: Option<PathBuf>,
    stdin_mode: bool,
    input_format: &str,
    print_flag: bool,
    duration: Option<Duration>,
    no_vad: bool,
) -> Result<()> {
    // Use --print flag for output mode (explicit user choice)
    // When --print is set, suppress status messages for clean stdout output
    let quiet = print_flag;

    // Load preset if provided
    let preset: Option<Preset> = if let Some(name) = preset_name {
        let (p, _source) = Preset::load(&name).map_err(|e| anyhow!("{}", e))?;
        Some(p)
    } else {
        None
    };

    // Create Tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()?;

    // Check if FFmpeg is available (needed for all input modes)
    app::ensure_ffmpeg_installed()?;

    // Load transcription configuration (provider + API key)
    let config = app::load_transcription_config()?;

    // Determine audio source based on input mode
    let audio_result = if let Some(path) = file_path {
        // File input mode
        if !quiet {
            println!("Loading audio file: {}", path.display());
        }
        load_audio_file(&path)?
    } else if stdin_mode {
        // Stdin input mode
        if !quiet {
            println!("Reading audio from stdin ({} format)...", input_format);
        }
        load_audio_stdin(input_format)?
    } else {
        // Microphone recording mode (default)
        let mut recorder = AudioRecorder::new()?;

        // Configure VAD based on settings and --no-vad flag
        #[cfg(feature = "vad")]
        {
            let settings = Settings::load();
            // VAD is enabled if settings say so AND --no-vad is not passed
            let vad_enabled = settings.vad_enabled && !no_vad;
            recorder.set_vad(vad_enabled, settings.vad_threshold);
        }
        #[cfg(not(feature = "vad"))]
        let _ = no_vad; // Suppress unused variable warning

        recorder.start_recording()?;

        if let Some(dur) = duration {
            // Timed recording mode (for non-interactive environments like AI assistants)
            if !quiet {
                println!("Recording for {} seconds...", dur.as_secs());
                io::stdout().flush()?;
            }
            std::thread::sleep(dur);
        } else {
            // Interactive mode: wait for Enter or hotkey to stop
            let settings = Settings::load();
            let hotkey = &settings.shortcut;

            if !quiet {
                if std::io::stdin().is_terminal() {
                    println!("Press Enter or {} to stop", hotkey);
                } else {
                    println!("Press {} to stop", hotkey);
                }
                print!("Recording...");
                io::stdout().flush()?;
            }
            app::wait_for_stop(hotkey)?;
        }

        // In verbose mode (non-quiet only), print newline so verbose output appears cleanly
        if !quiet && whis_core::verbose::is_verbose() {
            println!();
        }

        recorder.finalize_recording()?
    };

    // Transcribe (silent when --print)
    if !quiet {
        if whis_core::verbose::is_verbose() {
            println!("Transcribing...");
        } else {
            app::typewriter(" Transcribing...", 25);
        }
    }
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

    // Apply post-processing if enabled (via flag or preset)
    let settings = Settings::load();
    let should_post_process = post_process_flag || preset.is_some();

    let final_text = if should_post_process {
        let post_processor = resolve_post_processor(&preset, &settings, &config.provider);

        // Get API key or URL depending on post-processor type
        // For cloud post-processors: need API key
        // For Ollama: need server URL (defaults to localhost:11434)
        let api_key_or_url = if post_processor.requires_api_key() {
            settings.get_post_processor_api_key()
        } else if post_processor == PostProcessor::Ollama {
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
                        eprintln!("Skipping post-processing. Start Ollama manually: ollama serve");
                        None // Skip post-processing
                    }
                }
            } else {
                Some(ollama_url)
            }
        } else {
            None
        };

        if let Some(key_or_url) = api_key_or_url {
            // Silent when --print
            if !quiet {
                app::typewriter(" Post-processing...", 25);
            }

            // Priority: preset prompt > settings prompt > default
            let prompt = if let Some(ref p) = preset {
                p.prompt.as_str()
            } else {
                settings
                    .post_processing_prompt
                    .as_deref()
                    .unwrap_or(DEFAULT_POST_PROCESSING_PROMPT)
            };

            // For Ollama, use settings model if preset doesn't specify one
            let ollama_model = settings.get_ollama_model();
            let model = if let Some(ref p) = preset {
                p.model.as_deref()
            } else if post_processor == PostProcessor::Ollama {
                ollama_model.as_deref()
            } else {
                None
            };
            match runtime.block_on(post_process(
                &transcription,
                &post_processor,
                &key_or_url,
                prompt,
                model,
            )) {
                Ok(processed) => processed,
                Err(e) => {
                    eprintln!("Post-processing warning: {e}");
                    eprintln!("Falling back to raw transcript");
                    transcription
                }
            }
        } else {
            // Warn when post-processing was requested but we can't perform it
            // (Ollama case already warned above when auto-start failed)
            if post_processor != PostProcessor::None && post_processor.requires_api_key() {
                eprintln!(
                    "Warning: No API key for {} post-processor, skipping",
                    post_processor
                );
            }
            transcription
        }
    } else {
        transcription
    };

    // Output: --print sends to stdout, otherwise copy to clipboard (default)
    if print_flag {
        // Explicit stdout mode (for piping to other tools)
        // Use writeln! and ignore BrokenPipe (happens when piped to `head`, etc.)
        if let Err(e) = writeln!(io::stdout(), "{}", final_text)
            && e.kind() != io::ErrorKind::BrokenPipe
        {
            return Err(e.into());
        }
    } else {
        // Default: always copy to clipboard
        copy_to_clipboard(&final_text, settings.clipboard_method.clone())?;

        if whis_core::verbose::is_verbose() {
            println!("Done");
        } else {
            app::typewriter(" Done", 20);
            println!(); // End the status line
        }
        println!("Copied to clipboard");
    }

    Ok(())
}
