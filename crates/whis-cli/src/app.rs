use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{IsTerminal, Write};
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
    let api_key = match &provider {
        TranscriptionProvider::LocalWhisper => {
            // Local whisper: use model path
            match settings.get_whisper_model_path() {
                Some(path) => path,
                None => {
                    eprintln!("Error: No whisper model path configured.");
                    eprintln!("(Required for local transcription)");
                    eprintln!("\nSet the model path with:");
                    eprintln!(
                        "  whis config --whisper-model-path ~/.local/share/whis/models/ggml-small.bin\n"
                    );
                    eprintln!("Or set the LOCAL_WHISPER_MODEL_PATH environment variable.");
                    eprintln!("\nTip: Run 'whis setup local' for guided setup.");
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
                    eprintln!("(Required for {} transcription)", provider.display_name());
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
        api_key, // For local-whisper this is model path
        language,
    })
}

/// Wait for user to stop recording via Enter key or global hotkey.
/// In TTY mode: accepts either Enter or hotkey.
/// In non-TTY mode (e.g., AI assistant shell): only hotkey works.
pub fn wait_for_stop(hotkey_str: &str) -> Result<()> {
    use crate::hotkey;

    std::io::stdout().flush()?;

    // Set up global hotkey listener
    let (hotkey_rx, _guard) = hotkey::setup(hotkey_str)?;

    if std::io::stdin().is_terminal() {
        // TTY mode: accept Enter OR hotkey
        enable_raw_mode()?;

        loop {
            // Check for hotkey (non-blocking)
            if hotkey_rx.try_recv().is_ok() {
                break;
            }

            // Check for Enter key with timeout (50ms polling)
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key_event) = event::read()?
                    && key_event.code == KeyCode::Enter
                {
                    break;
                }
            }
        }

        disable_raw_mode()?;
    } else {
        // Non-TTY mode: only hotkey works (blocks until pressed)
        hotkey_rx.recv()?;
    }

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
