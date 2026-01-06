# Tauri Plugin: Floating Bubble

A Tauri plugin for displaying floating bubble overlays on Android.

## Platform Support

| Platform | Supported |
|----------|-----------|
| Android  | Yes       |
| iOS      | No        |
| Windows  | No        |
| macOS    | No        |
| Linux    | No        |

## Installation

### Rust

Add the plugin to your `Cargo.toml`:

```toml
[dependencies]
tauri-plugin-floating-bubble = { path = "../tauri-plugin-floating-bubble" }
```

### JavaScript

```bash
npm install @frankdierolf/tauri-plugin-floating-bubble
```

## Usage

### Rust Setup

Register the plugin in your Tauri app:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_floating_bubble::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### JavaScript API

```typescript
import {
  showBubble,
  hideBubble,
  isBubbleVisible,
  requestOverlayPermission,
  hasOverlayPermission,
} from '@frankdierolf/tauri-plugin-floating-bubble'

// Check and request permission
const { granted } = await hasOverlayPermission()
if (!granted) {
  await requestOverlayPermission()
  // User needs to grant permission in system settings
  // Check again after they return to the app
}

// Show the bubble
await showBubble({
  size: 60,      // Size in dp
  startX: 0,     // Initial X position
  startY: 100,   // Initial Y position
})

// Check visibility
const { visible } = await isBubbleVisible()

// Hide the bubble
await hideBubble()
```

## Permissions

This plugin requires the following Android permissions:

- `SYSTEM_ALERT_WINDOW` - Required to draw over other apps
- `FOREGROUND_SERVICE` - Required to keep the bubble alive
- `POST_NOTIFICATIONS` - Required for the foreground service notification (Android 13+)

The plugin will automatically request these permissions, but the user must manually grant `SYSTEM_ALERT_WINDOW` in system settings.

## License

MIT
