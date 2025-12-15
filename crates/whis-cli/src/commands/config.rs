use anyhow::Result;
use whis_core::{OutputStyle, Polisher, Settings, TranscriptionProvider};

pub fn run(
    openai_api_key: Option<String>,
    mistral_api_key: Option<String>,
    provider: Option<String>,
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
                eprintln!("Invalid language code. Use ISO-639-1 format (e.g., 'en', 'de', 'fr') or 'auto'");
                std::process::exit(1);
            }
            settings.language = Some(lang_lower.clone());
            changed = true;
            println!("Language set to: {}", lang_lower);
        }
    }

    // Handle OpenAI API key
    if let Some(key) = openai_api_key {
        // Validate format for OpenAI
        if !key.starts_with("sk-") {
            eprintln!("Invalid key format. OpenAI keys start with 'sk-'");
            std::process::exit(1);
        }

        settings.openai_api_key = Some(key);
        changed = true;
        println!("OpenAI API key saved");
    }

    // Handle Mistral API key (basic validation)
    if let Some(key) = mistral_api_key {
        let key_trimmed = key.trim();
        if key_trimmed.is_empty() {
            eprintln!("Invalid Mistral API key: cannot be empty");
            std::process::exit(1);
        }
        if key_trimmed.len() < 20 {
            eprintln!("Invalid Mistral API key: key appears too short");
            std::process::exit(1);
        }
        settings.mistral_api_key = Some(key_trimmed.to_string());
        changed = true;
        println!("Mistral API key saved");
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

        // OpenAI API key
        if let Some(key) = &settings.openai_api_key {
            println!("OpenAI API key: {}", mask_key(key));
        } else {
            println!("OpenAI API key: (not set, using $OPENAI_API_KEY)");
        }

        // Mistral API key
        if let Some(key) = &settings.mistral_api_key {
            println!("Mistral API key: {}", mask_key(key));
        } else {
            println!("Mistral API key: (not set, using $MISTRAL_API_KEY)");
        }

        // Polisher settings
        println!("Polisher: {}", settings.polisher);
        if let Some(prompt) = &settings.polish_prompt {
            println!("Polish prompt: {}", truncate_prompt(prompt));
        } else {
            println!("Polish prompt: (default)");
        }
        println!("Available --as styles: {}", OutputStyle::all().join(", "));

        return Ok(());
    }

    // No flags - show help
    eprintln!("Usage:");
    eprintln!("  whis config --provider <openai|mistral>");
    eprintln!("  whis config --language <en|de|fr|...|auto>");
    eprintln!("  whis config --openai-api-key <KEY>");
    eprintln!("  whis config --mistral-api-key <KEY>");
    eprintln!("  whis config --polisher <none|openai|mistral>");
    eprintln!("  whis config --polish-prompt <PROMPT>");
    eprintln!("  whis config --show");
    std::process::exit(1);
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
