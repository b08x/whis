<div align="center">
<img src="https://raw.githubusercontent.com/frankdierolf/whis/main/crates/whis-desktop/icons/128x128.png" alt="whis" width="80" height="80" />

<h3>whis</h3>
<p>
  Your voice, piped to clipboard.
  <br />
  <a href="https://whis.ink">Website</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-desktop">Desktop</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-mobile">Mobile</a>
  ·
  <a href="https://github.com/frankdierolf/whis/releases">Releases</a>
</p>
</div>

## Why?

- **Built for AI workflows** — speak your prompt, paste to OpenCode, Claude, Codex, ...
- **Cloud or local** — OpenAI, Mistral, Groq, Deepgram, ElevenLabs, or free with local Whisper
- **Simple** — record → transcribe → clipboard

## Quick Start

```bash
# Install
cargo install whis

# Setup (pick one)
whis setup cloud   # Cloud providers (guided wizard)
whis setup local   # Fully local (private, free)

# Run
whis               # Press Enter to stop — text copied!
```

## Screenshot

![whis Demo](https://raw.githubusercontent.com/frankdierolf/whis/main/demo.gif)

## Usage

```bash
# Record once
whis

# Background hotkey mode
whis listen        # Ctrl+Alt+W toggles recording

# Post-process with AI (cleanup grammar/filler)
whis --post-process

# Use with terminal AI assistants
claude "$(whis --as ai-prompt)"   # Start session with voice prompt
!whis --as ai-prompt              # Or use shell mode inside session

# Presets
whis --as email          # Use preset
whis presets             # List all
whis presets new xyz     # Create new preset
whis presets edit xyz    # Edit in $EDITOR

# Transcribe existing audio 
whis -f audio.mp3

# Help - for you or your helper
whis --help 
```

## Installation

```bash
cargo install whis
```

Or download binaries from [GitHub Releases](https://github.com/frankdierolf/whis/releases).

## Requirements

- API key from [OpenAI](https://platform.openai.com/api-keys), [Mistral](https://console.mistral.ai/api-keys), [Groq](https://console.groq.com/keys), [Deepgram](https://deepgram.com), or [ElevenLabs](https://elevenlabs.io) — or use local Whisper/Parakeet (no API key needed)
- FFmpeg (`sudo apt install ffmpeg`, `brew install ffmpeg`, or `scoop install ffmpeg`)
- **Linux only**: Vulkan SDK for local transcription
  ```bash
  sudo apt install libvulkan-dev vulkan-tools glslc libshaderc-dev  # Ubuntu/Debian
  sudo pacman -S vulkan-headers vulkan-tools shaderc                # Arch
  ```
- Linux (X11/Wayland), macOS, or Windows

**For hotkey mode** (one-time setup on Linux):
```bash
sudo usermod -aG input $USER
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-uinput.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
# Logout and login again
```

## Desktop & Mobile

- **[Desktop](https://github.com/frankdierolf/whis/tree/main/crates/whis-desktop)** — GUI with system tray
- **[Mobile](https://github.com/frankdierolf/whis/tree/main/crates/whis-mobile)** — Android app (Alpha)

## Development

```bash
git clone https://github.com/frankdierolf/whis.git
cd whis
just                # List all commands
just install-cli    # Build and install CLI
```

See [CONTRIBUTING.md](https://github.com/frankdierolf/whis/blob/main/CONTRIBUTING.md) for full setup instructions.

## Architecture

![Whis Architecture](https://raw.githubusercontent.com/frankdierolf/whis/main/website/public/architecture.jpg)

## License

MIT
