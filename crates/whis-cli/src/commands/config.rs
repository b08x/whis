use anyhow::Result;
use whis_core::{Polisher, Preset, Settings, TranscriptionProvider};

#[allow(clippy::too_many_arguments)]
pub fn run(
    openai_api_key: Option<String>,
    mistral_api_key: Option<String>,
    groq_api_key: Option<String>,
    deepgram_api_key: Option<String>,
    elevenlabs_api_key: Option<String>,
    provider: Option<String>,
    whisper_model_path: Option<String>,
    remote_whisper_url: Option<String>,
    ollama_url: Option<String>,
    ollama_model: Option<String>,
    language: Option<String>,
    polisher: Option<String>,
    polish_prompt: Option<String>,
    show: bool,
) -> Result<()> {
    let mut settings = Settings::load();
    let mut changed = false;

    // Handle provider change
    if let Some(provider_str) = provider {
        match provider_str.parse::<TranscriptionProvider>() {
            Ok(p) => {
                settings.provider = p;
                changed = true;
                println!("Provider set to: {}", provider_str);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }

    // Handle whisper model path for local transcription
    if let Some(path) = whisper_model_path {
        let path_trimmed = path.trim();
        if path_trimmed.is_empty() {
            eprintln!("Invalid whisper model path: cannot be empty");
            std::process::exit(1);
        }
        // Expand ~ to home directory
        let expanded_path = if let Some(rest) = path_trimmed.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(rest).to_string_lossy().to_string()
            } else {
                path_trimmed.to_string()
            }
        } else {
            path_trimmed.to_string()
        };
        settings.whisper_model_path = Some(expanded_path.clone());
        changed = true;
        println!("Whisper model path set to: {}", expanded_path);
    }

    // Handle remote whisper server URL for self-hosted transcription
    if let Some(url) = remote_whisper_url {
        let url_trimmed = url.trim();
        if url_trimmed.is_empty() {
            eprintln!("Invalid remote whisper URL: cannot be empty");
            std::process::exit(1);
        }
        settings.remote_whisper_url = Some(url_trimmed.to_string());
        changed = true;
        println!("Remote whisper URL set to: {}", url_trimmed);
    }

    // Handle Ollama URL for local polishing
    if let Some(url) = ollama_url {
        let url_trimmed = url.trim();
        if url_trimmed.is_empty() {
            eprintln!("Invalid Ollama URL: cannot be empty");
            std::process::exit(1);
        }
        settings.ollama_url = Some(url_trimmed.to_string());
        changed = true;
        println!("Ollama URL set to: {}", url_trimmed);
    }

    // Handle Ollama model for local polishing
    if let Some(model) = ollama_model {
        let model_trimmed = model.trim();
        if model_trimmed.is_empty() {
            eprintln!("Invalid Ollama model: cannot be empty");
            std::process::exit(1);
        }
        settings.ollama_model = Some(model_trimmed.to_string());
        changed = true;
        println!("Ollama model set to: {}", model_trimmed);
    }

    // Handle language change
    if let Some(lang) = language {
        if lang.to_lowercase() == "auto" {
            settings.language = None;
            changed = true;
            println!("Language set to: auto-detect");
        } else {
            // Validate ISO-639-1 format: 2 lowercase alphabetic characters
            let lang_lower = lang.to_lowercase();
            if lang_lower.len() != 2 || !lang_lower.chars().all(|c| c.is_ascii_lowercase()) {
                eprintln!(
                    "Invalid language code. Use ISO-639-1 format (e.g., 'en', 'de', 'fr') or 'auto'"
                );
                std::process::exit(1);
            }
            settings.language = Some(lang_lower.clone());
            changed = true;
            println!("Language set to: {}", lang_lower);
        }
    }

    // Handle API keys for all providers
    if let Some(key) = openai_api_key {
        if !key.starts_with("sk-") {
            eprintln!("Invalid key format. OpenAI keys start with 'sk-'");
            std::process::exit(1);
        }
        settings.set_api_key(&TranscriptionProvider::OpenAI, key);
        changed = true;
        println!("OpenAI API key saved");
    }

    if let Some(key) = mistral_api_key {
        if let Err(msg) = validate_api_key(&key, "Mistral") {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
        settings.set_api_key(&TranscriptionProvider::Mistral, key);
        changed = true;
        println!("Mistral API key saved");
    }

    if let Some(key) = groq_api_key {
        if !key.starts_with("gsk_") {
            eprintln!("Invalid key format. Groq keys start with 'gsk_'");
            std::process::exit(1);
        }
        settings.set_api_key(&TranscriptionProvider::Groq, key);
        changed = true;
        println!("Groq API key saved");
    }

    if let Some(key) = deepgram_api_key {
        if let Err(msg) = validate_api_key(&key, "Deepgram") {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
        settings.set_api_key(&TranscriptionProvider::Deepgram, key);
        changed = true;
        println!("Deepgram API key saved");
    }

    if let Some(key) = elevenlabs_api_key {
        if let Err(msg) = validate_api_key(&key, "ElevenLabs") {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
        settings.set_api_key(&TranscriptionProvider::ElevenLabs, key);
        changed = true;
        println!("ElevenLabs API key saved");
    }

    // Handle polisher change
    if let Some(polisher_str) = polisher {
        match polisher_str.parse::<Polisher>() {
            Ok(p) => {
                settings.polisher = p;
                changed = true;
                println!("Polisher set to: {}", polisher_str);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }

    // Handle polish prompt change
    if let Some(prompt) = polish_prompt {
        let prompt_trimmed = prompt.trim();
        if prompt_trimmed.is_empty() {
            eprintln!("Invalid polish prompt: cannot be empty");
            std::process::exit(1);
        }
        settings.polish_prompt = Some(prompt_trimmed.to_string());
        changed = true;
        println!("Polish prompt saved");
    }

    // Save if anything changed
    if changed {
        settings.save()?;
        println!("Config saved to {}", Settings::path().display());
        return Ok(());
    }

    if show {
        println!("Config file: {}", Settings::path().display());
        println!("Provider: {}", settings.provider);
        println!(
            "Language: {}",
            settings.language.as_deref().unwrap_or("auto-detect")
        );
        println!("Shortcut: {}", settings.shortcut);

        // Show API keys for all providers
        for provider in TranscriptionProvider::all() {
            let key_status = if let Some(key) = settings.get_api_key_for(provider) {
                mask_key(&key)
            } else {
                format!("(not set, using ${})", provider.api_key_env_var())
            };
            println!("{} API key: {}", provider.display_name(), key_status);
        }

        // Polisher settings
        println!("Polisher: {}", settings.polisher);
        if let Some(prompt) = &settings.polish_prompt {
            println!("Polish prompt: {}", truncate_prompt(prompt));
        } else {
            println!("Polish prompt: (default)");
        }

        // Local transcription settings
        if let Some(path) = &settings.whisper_model_path {
            println!("Whisper model path: {}", path);
        } else {
            println!("Whisper model path: (not set, using $LOCAL_WHISPER_MODEL_PATH)");
        }

        // Remote whisper server settings
        if let Some(url) = &settings.remote_whisper_url {
            println!("Remote whisper URL: {}", url);
        } else {
            println!("Remote whisper URL: (not set, using $REMOTE_WHISPER_URL)");
        }

        // Ollama settings for local polishing
        if let Some(url) = &settings.ollama_url {
            println!("Ollama URL: {}", url);
        } else {
            println!("Ollama URL: (default: http://localhost:11434)");
        }
        if let Some(model) = &settings.ollama_model {
            println!("Ollama model: {}", model);
        } else {
            println!("Ollama model: (default: ministral-3:3b)");
        }

        println!("Available --as presets: {}", Preset::all_names().join(", "));

        return Ok(());
    }

    // No flags - show help
    eprintln!("Usage:");
    eprintln!(
        "  whis config --provider <openai|mistral|groq|deepgram|elevenlabs|local-whisper|remote-whisper>"
    );
    eprintln!("  whis config --language <en|de|fr|...|auto>");
    eprintln!("  whis config --openai-api-key <KEY>");
    eprintln!("  whis config --mistral-api-key <KEY>");
    eprintln!("  whis config --groq-api-key <KEY>");
    eprintln!("  whis config --deepgram-api-key <KEY>");
    eprintln!("  whis config --elevenlabs-api-key <KEY>");
    eprintln!("  whis config --whisper-model-path <PATH>       # For local-whisper provider");
    eprintln!("  whis config --remote-whisper-url <URL>        # For remote-whisper provider");
    eprintln!("  whis config --polisher <none|openai|mistral|ollama>");
    eprintln!(
        "  whis config --ollama-url <URL>                # For ollama polisher (default: http://localhost:11434)"
    );
    eprintln!(
        "  whis config --ollama-model <MODEL>            # For ollama polisher (default: ministral-3:3b)"
    );
    eprintln!("  whis config --polish-prompt <PROMPT>");
    eprintln!("  whis config --show");
    std::process::exit(1);
}

fn validate_api_key(key: &str, provider_name: &str) -> Result<(), String> {
    let key_trimmed = key.trim();
    if key_trimmed.is_empty() {
        return Err(format!(
            "Invalid {} API key: cannot be empty",
            provider_name
        ));
    }
    if key_trimmed.len() < 20 {
        return Err(format!(
            "Invalid {} API key: key appears too short",
            provider_name
        ));
    }
    Ok(())
}

fn mask_key(key: &str) -> String {
    if key.len() > 10 {
        format!("{}...{}", &key[..6], &key[key.len() - 4..])
    } else {
        "***".to_string()
    }
}

fn truncate_prompt(prompt: &str) -> String {
    if prompt.len() > 50 {
        format!("{}...", &prompt[..47])
    } else {
        prompt.to_string()
    }
}
