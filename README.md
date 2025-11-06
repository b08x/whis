# Whispo

Voice-to-text for terminal users. Record your voice, get instant transcription to clipboard.

Built for Linux terminal workflows with Claude Code, Cursor, Gemini CLI, and other AI coding tools. Think whisperflow.ai, but minimal and CLI-native.

## Why Whispo?

OpenAI's Whisper API delivers the most accurate transcriptions compared to local models or other providers. Whispo makes it dead simple to use from the terminal.

## Quick Start

```bash
# Clone and enter directory
git clone https://github.com/frankdierolf/whispo
cd whispo

# Set your OpenAI API key
cp .env.example .env
# Edit .env and add: OPENAI_API_KEY=sk-your-key-here

# Build
cargo build --release

# Run
./target/release/whispo
```

## Usage

```bash
./target/release/whispo
```

1. Recording starts automatically
2. Press Enter to stop
3. Transcription copies to clipboard

That's it. Paste into your AI coding tool.

## Requirements

- Rust (latest stable)
- OpenAI API key ([get one here](https://platform.openai.com/api-keys))
- Linux with working microphone
- ALSA or PulseAudio

## Building from Source

```bash
cargo build --release
```

Binary will be at `./target/release/whispo`

## License

MIT
