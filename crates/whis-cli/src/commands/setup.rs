//! Setup wizard for different usage modes
//!
//! Provides a streamlined setup experience for:
//! - Cloud users (API key setup)
//! - Local users (on-device transcription)

use anyhow::{Result, anyhow};
use std::io::{self, Write};
use whis_core::{PostProcessor, Settings, TranscriptionProvider, model, ollama};

use crate::args::SetupMode;

pub fn run(mode: SetupMode) -> Result<()> {
    match mode {
        SetupMode::Cloud => setup_cloud(),
        SetupMode::Local => setup_local(),
    }
}

/// Setup for cloud providers
fn setup_cloud() -> Result<()> {
    println!("Cloud Setup");
    println!("===========");
    println!();
    println!("Available providers:");
    println!("  1. OpenAI     - High quality, most popular");
    println!("  2. Mistral    - European provider, good quality");
    println!("  3. Groq       - Very fast, good for real-time");
    println!("  4. Deepgram   - Fast, good for conversations");
    println!("  5. ElevenLabs - Good multilingual support");
    println!();

    let provider = prompt_choice("Select provider (1-5)", 1, 5)?;
    let provider = match provider {
        1 => TranscriptionProvider::OpenAI,
        2 => TranscriptionProvider::Mistral,
        3 => TranscriptionProvider::Groq,
        4 => TranscriptionProvider::Deepgram,
        5 => TranscriptionProvider::ElevenLabs,
        _ => unreachable!(),
    };

    println!();
    println!("Get your API key from:");
    match provider {
        TranscriptionProvider::OpenAI => println!("  https://platform.openai.com/api-keys"),
        TranscriptionProvider::Mistral => println!("  https://console.mistral.ai/api-keys"),
        TranscriptionProvider::Groq => println!("  https://console.groq.com/keys"),
        TranscriptionProvider::Deepgram => println!("  https://console.deepgram.com"),
        TranscriptionProvider::ElevenLabs => {
            println!("  https://elevenlabs.io/app/settings/api-keys")
        }
        _ => {}
    }
    println!();

    let api_key = prompt_secret("Enter API key")?;

    // Validate key format
    match provider {
        TranscriptionProvider::OpenAI => {
            if !api_key.starts_with("sk-") {
                return Err(anyhow!("Invalid OpenAI key format. Keys start with 'sk-'"));
            }
        }
        TranscriptionProvider::Groq => {
            if !api_key.starts_with("gsk_") {
                return Err(anyhow!("Invalid Groq key format. Keys start with 'gsk_'"));
            }
        }
        _ => {
            if api_key.len() < 20 {
                return Err(anyhow!("API key seems too short"));
            }
        }
    }

    // Save settings
    let mut settings = Settings::load();
    settings.provider = provider.clone();
    settings.set_api_key(&provider, api_key);

    // Set post-processor based on provider
    settings.post_processor = match provider {
        TranscriptionProvider::OpenAI => PostProcessor::OpenAI,
        TranscriptionProvider::Mistral => PostProcessor::Mistral,
        _ => {
            // For other providers, use OpenAI for post-processing if they have an OpenAI key
            if settings
                .get_api_key_for(&TranscriptionProvider::OpenAI)
                .is_some()
            {
                PostProcessor::OpenAI
            } else {
                PostProcessor::None
            }
        }
    };

    settings.save()?;

    println!();
    println!("Setup complete!");
    println!();
    println!("Transcription: {}", provider.display_name());
    if settings.post_processor != PostProcessor::None {
        if provider == TranscriptionProvider::OpenAI || provider == TranscriptionProvider::Mistral {
            println!("Post-processing: {} (same API key)", settings.post_processor);
        } else {
            println!("Post-processing: {}", settings.post_processor);
        }
    }
    println!();
    println!("Try it out:");
    println!("  whis                # Record and transcribe");
    println!("  whis --post-process # Record, transcribe, and post-process");
    println!();

    Ok(())
}

/// Setup for fully local (on-device) transcription
fn setup_local() -> Result<()> {
    println!("Local Setup");
    println!("===========");
    println!();
    println!("This will set up fully local transcription:");
    println!("  - Whisper model for transcription (runs on CPU)");
    println!("  - Ollama for transcript post-processing (runs locally)");
    println!();

    // Step 1: Download whisper model
    println!("Step 1: Whisper Model");
    println!("---------------------");
    model::list_models();
    println!();

    let model_path = model::default_model_path(model::DEFAULT_MODEL);
    if model::model_exists(&model_path) {
        println!("Model '{}' already installed at:", model::DEFAULT_MODEL);
        println!("  {}", model_path.display());
    } else {
        println!(
            "Downloading '{}' model (recommended)...",
            model::DEFAULT_MODEL
        );
        model::download_model(model::DEFAULT_MODEL, &model_path)?;
    }
    println!();

    // Step 2: Setup Ollama
    println!("Step 2: Ollama (for post-processing)");
    println!("------------------------------------");

    let ollama_url = ollama::DEFAULT_OLLAMA_URL;
    let ollama_model = ollama::DEFAULT_OLLAMA_MODEL;

    // Check if Ollama is installed
    if !ollama::is_ollama_installed() {
        println!("Ollama is not installed.");
        println!();
        println!("Install Ollama:");
        println!("  Linux:  curl -fsSL https://ollama.com/install.sh | sh");
        println!("  macOS:  brew install ollama");
        println!("  Website: https://ollama.com/download");
        println!();
        return Err(anyhow!("Please install Ollama and run setup again"));
    }

    // Start Ollama if not running
    ollama::ensure_ollama_running(ollama_url)?;

    // Pull the model if needed
    if !ollama::has_model(ollama_url, ollama_model)? {
        println!("Pulling Ollama model '{}'...", ollama_model);
        ollama::pull_model(ollama_url, ollama_model)?;
    } else {
        println!("Ollama model '{}' is ready.", ollama_model);
    }
    println!();

    // Step 3: Save configuration
    println!("Step 3: Saving Configuration");
    println!("----------------------------");

    let mut settings = Settings::load();
    settings.provider = TranscriptionProvider::LocalWhisper;
    settings.whisper_model_path = Some(model_path.to_string_lossy().to_string());
    settings.post_processor = PostProcessor::Ollama;
    settings.ollama_url = Some(ollama_url.to_string());
    settings.ollama_model = Some(ollama_model.to_string());
    settings.save()?;

    println!("Configuration saved to: {}", Settings::path().display());
    println!();
    println!("Setup complete!");
    println!();
    println!("Your setup:");
    println!("  Transcription:    Local Whisper ({})", model::DEFAULT_MODEL);
    println!("  Post-processing:  Ollama ({})", ollama_model);
    println!();
    println!("Try it out:");
    println!("  whis                # Record and transcribe locally");
    println!("  whis --post-process # Record, transcribe, and post-process locally");
    println!();
    println!("Note: Ollama will auto-start when needed.");

    Ok(())
}

// --- Helper functions ---

fn prompt_choice(prompt: &str, min: usize, max: usize) -> Result<usize> {
    loop {
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().parse::<usize>() {
            Ok(n) if n >= min && n <= max => return Ok(n),
            _ => println!("Please enter a number between {} and {}", min, max),
        }
    }
}

fn prompt_secret(prompt: &str) -> Result<String> {
    // Note: Input will be visible. For true hidden input, use rpassword crate.
    print!("{}: ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
