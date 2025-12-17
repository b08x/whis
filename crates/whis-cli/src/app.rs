use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::Write;
use std::thread;
use std::time::Duration;
use whis_core::{Settings, TranscriptionProvider};

/// Configuration for transcription, including provider, API key, and language
pub struct TranscriptionConfig {
    pub provider: TranscriptionProvider,
    pub api_key: String,
    pub language: Option<String>,
}

pub fn ensure_ffmpeg_installed() -> Result<()> {
    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_err()
    {
        eprintln!("Error: FFmpeg is not installed or not in PATH.");
        eprintln!("\nwhis requires FFmpeg for audio compression.");
        eprintln!("Please install FFmpeg:");
        eprintln!("  - Ubuntu/Debian: sudo apt install ffmpeg");
        eprintln!("  - macOS: brew install ffmpeg");
        eprintln!("  - Windows: choco install ffmpeg or download from ffmpeg.org");
        eprintln!("  - Or visit: https://ffmpeg.org/download.html\n");
        std::process::exit(1);
    }
    Ok(())
}

pub fn load_transcription_config() -> Result<TranscriptionConfig> {
    let settings = Settings::load();
    let provider = settings.provider.clone();
    let language = settings.language.clone();

    // Handle different provider types:
    // - Cloud providers: require API key
    // - LocalWhisper: requires model path
    // - RemoteWhisper: requires server URL
    let api_key = match &provider {
        TranscriptionProvider::LocalWhisper => {
            // Local whisper: use model path
            match settings.get_whisper_model_path() {
                Some(path) => path,
                None => {
                    eprintln!("Error: No whisper model path configured.");
                    eprintln!("\nSet the model path with:");
                    eprintln!(
                        "  whis config --whisper-model-path ~/.local/share/whis/models/ggml-small.bin\n"
                    );
                    eprintln!("Or set the LOCAL_WHISPER_MODEL_PATH environment variable.");
                    std::process::exit(1);
                }
            }
        }
        TranscriptionProvider::RemoteWhisper => {
            // Remote whisper: use server URL
            match settings.get_remote_whisper_url() {
                Some(url) => url,
                None => {
                    eprintln!("Error: No remote whisper server URL configured.");
                    eprintln!("\nSet the server URL with:");
                    eprintln!("  whis config --remote-whisper-url http://localhost:8765\n");
                    eprintln!("Or set the REMOTE_WHISPER_URL environment variable.");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            // Cloud providers: require API key
            match settings.get_api_key_for(&provider) {
                Some(key) => key,
                None => {
                    eprintln!("Error: No {} API key configured.", provider.display_name());
                    eprintln!("\nSet your key with:");
                    eprintln!("  whis config --{}-api-key YOUR_KEY\n", provider.as_str());
                    eprintln!(
                        "Or set the {} environment variable.",
                        provider.api_key_env_var()
                    );
                    std::process::exit(1);
                }
            }
        }
    };

    Ok(TranscriptionConfig {
        provider,
        api_key, // For local-whisper this is model path, for remote-whisper this is server URL
        language,
    })
}

pub fn wait_for_enter() -> Result<()> {
    std::io::stdout().flush()?;

    // Enable raw mode to read keypresses without echoing
    enable_raw_mode()?;

    // Wait for Enter key
    loop {
        if let Event::Key(key_event) = event::read()? {
            if key_event.code == KeyCode::Enter {
                break;
            }
        }
    }

    // Restore normal mode
    disable_raw_mode()?;

    Ok(())
}

/// Print text with a typewriter effect
pub fn typewriter(text: &str, delay_ms: u64) {
    for c in text.chars() {
        print!("{}", c);
        std::io::stdout().flush().ok();
        thread::sleep(Duration::from_millis(delay_ms));
    }
}
