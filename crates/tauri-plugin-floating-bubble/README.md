# Tauri Plugin: Floating Bubble

A Tauri plugin for displaying floating bubble overlays on Android. Perfect for voice input applications, chat heads, or any UI that needs to persist across apps.

| Platform | Supported |
| -------- | --------- |
| Android  | ✓         |
| iOS      | ✗         |
| Windows  | ✗         |
| macOS    | ✗         |
| Linux    | ✗         |

## Example

See the [floating-bubble-android-demo](./examples/floating-bubble-android-demo) for a complete, self-contained example that you can build and run on your Android device.

## Install

Install the Core plugin by adding the following to your `Cargo.toml` file:

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-floating-bubble = { path = "../tauri-plugin-floating-bubble" }
```

You can install the JavaScript Guest bindings using your preferred JavaScript package manager:

```sh
pnpm add tauri-plugin-floating-bubble
# or
npm add tauri-plugin-floating-bubble
# or
yarn add tauri-plugin-floating-bubble
```

For local development (monorepo):

```json
{
  "dependencies": {
    "tauri-plugin-floating-bubble": "file:../../tauri-plugin-floating-bubble"
  }
}
```

## Usage

First you need to register the core plugin with Tauri:

`src-tauri/src/lib.rs`

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_floating_bubble::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Afterwards all the plugin's APIs are available through the JavaScript guest bindings:

```typescript
import {
  showBubble,
  hideBubble,
  hasOverlayPermission,
  requestOverlayPermission,
} from 'tauri-plugin-floating-bubble'

// Check and request permission first
const { granted } = await hasOverlayPermission()
if (!granted) {
  await requestOverlayPermission()
  // User needs to grant permission in system settings
  // Check again after they return to the app
}

// Show the bubble
await showBubble({
  size: 60,      // Size in dp (default: 60)
  startX: 0,     // Initial X position (default: 0)
  startY: 100,   // Initial Y position (default: 100)
})

// Hide the bubble
await hideBubble()
```

### Listening for Bubble Click Events

The plugin emits events when the user taps the floating bubble. You can listen for these events using the `onBubbleClick` function:

```typescript
import { onBubbleClick } from 'tauri-plugin-floating-bubble'

// Register a listener for bubble clicks
const unlisten = await onBubbleClick((event) => {
  console.log('Bubble clicked!', event.action)
  // Start recording, toggle state, etc.
})

// Later, to stop listening:
unlisten()
```

You can also use the event name directly with Tauri's event system:

```typescript
import { listen } from '@tauri-apps/api/event'
import { BUBBLE_CLICK_EVENT } from 'tauri-plugin-floating-bubble'

const unlisten = await listen(BUBBLE_CLICK_EVENT, (event) => {
  console.log('Bubble clicked:', event.payload)
})
```

### Updating Bubble Appearance

You can change the bubble's visual state to indicate recording or other states:

```typescript
import { setBubbleRecording } from 'tauri-plugin-floating-bubble'

// Show recording state (red background, pause icon)
await setBubbleRecording(true)

// Show idle state (indigo background, microphone icon)
await setBubbleRecording(false)
```

### Checking Bubble Visibility

```typescript
import { isBubbleVisible } from 'tauri-plugin-floating-bubble'

const { visible } = await isBubbleVisible()
if (visible) {
  console.log('Bubble is currently shown')
}
```

## API Reference

### Functions

| Function | Description |
| -------- | ----------- |
| `showBubble(options?)` | Show the floating bubble overlay |
| `hideBubble()` | Hide the floating bubble |
| `isBubbleVisible()` | Check if the bubble is currently visible |
| `requestOverlayPermission()` | Request the overlay permission (opens system settings) |
| `hasOverlayPermission()` | Check if overlay permission is granted |
| `setBubbleRecording(recording)` | Update bubble appearance for recording state |
| `onBubbleClick(callback)` | Register a listener for bubble click events |

### Types

```typescript
interface BubbleOptions {
  size?: number    // Size in dp (default: 60)
  startX?: number  // Initial X position (default: 0)
  startY?: number  // Initial Y position (default: 100)
}

interface BubbleClickEvent {
  action: 'click'
}

interface VisibilityResponse {
  visible: boolean
}

interface PermissionResponse {
  granted: boolean
}
```

### Constants

| Constant | Value | Description |
| -------- | ----- | ----------- |
| `BUBBLE_CLICK_EVENT` | `'floating-bubble://click'` | Event name for bubble clicks |

## Permissions

This plugin requires the following Android permissions:

- `SYSTEM_ALERT_WINDOW` - Required to draw over other apps
- `FOREGROUND_SERVICE` - Required to keep the bubble alive in the background
- `FOREGROUND_SERVICE_SPECIAL_USE` - Required for Android 14+
- `POST_NOTIFICATIONS` - Required for the foreground service notification (Android 13+)

The plugin will guide the user to system settings to grant `SYSTEM_ALERT_WINDOW` permission. This cannot be granted programmatically.

## Capabilities

Add the plugin's default capabilities to your app:

`src-tauri/capabilities/default.json`

```json
{
  "permissions": [
    "floating-bubble:default"
  ]
}
```

## How It Works

The plugin creates a foreground service that manages a floating overlay using Android's `WindowManager`. The bubble:

- Persists across apps (draws over other applications)
- Can be dragged around the screen
- Snaps to the left or right edge when released
- Sends click events to your Tauri app via the global event system
- Runs as a foreground service with a notification (required for Android 8+)

## License

MIT
