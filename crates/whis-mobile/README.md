<div align="center">
<img src="https://raw.githubusercontent.com/frankdierolf/whis/main/crates/whis-desktop/icons/128x128.png" alt="whis" width="80" height="80" />

<h3>whis-mobile</h3>
<p>
  Your voice, piped to clipboard. On the go.
  <br />
  <a href="https://whis.ink">Website</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-cli">CLI</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-desktop">Desktop</a>
  ·
  <a href="https://github.com/frankdierolf/whis/releases">Releases</a>
</p>
</div>

## Introduction

An Android app for voice-to-text on mobile. Lightweight, cloud-powered, and copies straight to clipboard. **(Alpha)**

## Screenshot

![Whis Mobile](https://raw.githubusercontent.com/frankdierolf/whis/main/crates/whis-mobile/screenshots/composite.png)

## Features

- **Voice-to-text** — tap to record, tap to transcribe
- **Multi-provider** — OpenAI Whisper, Mistral Voxtral
- **Clipboard** — transcriptions copied automatically

## Building

Requires Android SDK and NDK. See [Tauri mobile prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
# Install Android targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi

# Build
just setup-mobile     # First time setup
just dev-mobile       # Run on device
just build-mobile     # Build APK
```

## License

MIT
