# Floating Bubble Android Demo

A self-contained Android example demonstrating `tauri-plugin-floating-bubble`.

> **Note:** This example is Android-only. The floating bubble feature requires Android's `SYSTEM_ALERT_WINDOW` permission and is not available on iOS or desktop platforms.

## Features

- Request overlay permission
- Show/hide floating bubble
- Change bubble state (idle, recording, processing)
- Listen for bubble click events

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)
- [Android SDK & NDK](https://developer.android.com/studio)
- [Tauri Android prerequisites](https://tauri.app/start/prerequisites/#android)

## Setup

```bash
# Install dependencies
npm install

# Initialize Android project (first time only)
npm run tauri android init
```

## Build & Install

```bash
# Build debug APK
npm run tauri android build -- --debug

# Install on connected device
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

## Development

```bash
# Run with hot-reload on connected device
npm run tauri android dev
```

## Usage

1. Tap **Request Permission** to grant overlay permission (required on Android)
2. Tap **Show Bubble** to display the floating bubble overlay
3. Use state buttons to change the bubble appearance
4. Tap the bubble to see click events in the log
5. Tap **Hide Bubble** to dismiss

## Project Structure

```
floating-bubble-android-demo/
├── src/                    # Frontend (TypeScript)
│   ├── main.ts             # App logic
│   └── styles.css          # Styling
├── src-tauri/              # Tauri backend (Rust)
│   ├── src/lib.rs          # Plugin registration
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri config (Android-only)
├── index.html              # Entry point
└── package.json            # Node dependencies
```
