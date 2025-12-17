# Chapter 23: Vue Frontend & Tauri Integration

The Whis desktop UI is built with Vue 3, using a proper SPA architecture with routing, centralized state management, and reusable composables. This chapter explores how the frontend integrates with the Rust backend through Tauri's IPC system.

## Architecture Overview

```
whis-desktop/ui/src/
├── main.ts                    # Entry point with router setup
├── App.vue                    # Root component with sidebar/shell
├── types.ts                   # TypeScript type definitions
├── router/
│   └── index.ts               # Vue Router configuration
├── stores/
│   └── settings.ts            # Centralized settings state
├── composables/
│   ├── useKeyboardCapture.ts  # Keyboard shortcut recording
│   ├── useAsyncOperation.ts   # Async state management
│   └── useWhisperModel.ts     # Model download management
├── components/
│   ├── AppButton.vue          # Reusable button
│   ├── AppInput.vue           # Reusable input
│   ├── AppSelect.vue          # Reusable dropdown
│   └── settings/              # Settings-specific components
│       ├── ModeCards.vue
│       ├── CloudProviderConfig.vue
│       ├── LocalWhisperConfig.vue
│       └── ...
└── views/
    ├── HomeView.vue           # Recording controls
    ├── ShortcutView.vue       # Global shortcut config
    ├── SettingsView.vue       # Provider & API settings
    ├── PresetsView.vue        # Preset management
    └── AboutView.vue          # Version info
```

This structure separates concerns:
- **Router**: Navigation between views
- **Stores**: Centralized state management
- **Composables**: Reusable logic extracted from components
- **Components**: Reusable UI elements
- **Views**: Page-level components rendered by router

## Entry Point: main.ts

**From `whis-desktop/ui/src/main.ts`**:

```typescript
import { createApp } from 'vue'
import App from './App.vue'
import router from './router'

const app = createApp(App)
app.use(router)
app.mount('#app')
```

Simple and standard Vue 3 setup:
1. Create app instance
2. Install router plugin
3. Mount to DOM

No Pinia or other plugins—the settings store is a lightweight custom implementation.

## Vue Router Configuration

**From `whis-desktop/ui/src/router/index.ts`**:

```typescript
import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'home',
    component: () => import('../views/HomeView.vue'),
    meta: { title: 'Whis' },
  },
  {
    path: '/shortcut',
    name: 'shortcut',
    component: () => import('../views/ShortcutView.vue'),
    meta: { title: 'Global Shortcut' },
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('../views/SettingsView.vue'),
    meta: { title: 'Settings' },
  },
  {
    path: '/presets',
    name: 'presets',
    component: () => import('../views/PresetsView.vue'),
    meta: { title: 'Presets' },
  },
  {
    path: '/about',
    name: 'about',
    component: () => import('../views/AboutView.vue'),
    meta: { title: 'About' },
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    component: () => import('../views/NotFoundView.vue'),
    meta: { title: 'Not Found' },
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

// Update document title on navigation
router.afterEach((to) => {
  const title = to.meta.title as string | undefined
  document.title = title ? `${title} - Whis` : 'Whis'
})

export default router
```

**Key design decisions**:

### Hash Mode for Tauri

```typescript
history: createWebHashHistory()
```

Tauri loads the frontend from local files, not a web server. Hash mode (`/#/settings`) works without server-side routing:
- URLs like `index.html#/settings` work offline
- No 404s when navigating directly to a route
- Bookmarks work correctly

### Lazy-Loaded Components

```typescript
component: () => import('../views/HomeView.vue'),
```

Each view is lazy-loaded. Benefits:
- Faster initial load (only load what's needed)
- Better code splitting
- Smaller initial bundle

### Route Meta for Titles

```typescript
meta: { title: 'Global Shortcut' }

router.afterEach((to) => {
  document.title = title ? `${title} - Whis` : 'Whis'
})
```

The `afterEach` guard updates the document title on every navigation. This shows the current section in the window title bar.

## Type Definitions

**From `whis-desktop/ui/src/types.ts`**:

```typescript
// Transcription providers
export type Provider =
  | 'openai'
  | 'mistral'
  | 'groq'
  | 'deepgram'
  | 'elevenlabs'
  | 'local-whisper'
  | 'remote-whisper'

// Text polishing providers
export type Polisher = 'none' | 'openai' | 'mistral' | 'ollama'

// All settings from the backend
export interface Settings {
  shortcut: string
  provider: Provider
  language: string | null
  api_keys: Record<string, string>
  whisper_model_path: string | null
  remote_whisper_url: string | null
  polisher: Polisher
  ollama_url: string | null
  ollama_model: string | null
  polish_prompt: string | null
  active_preset: string | null
}

// Shortcut backend information
export interface BackendInfo {
  backend: string
  requires_restart: boolean
  compositor: string
  portal_version: number
}

// Status response from backend
export interface StatusResponse {
  state: 'Idle' | 'Recording' | 'Transcribing'
  config_valid: boolean
}
```

These types mirror the Rust structs from the backend. TypeScript ensures the frontend and backend stay in sync—mismatches cause compile-time errors.

## The Settings Store

Instead of Pinia, Whis uses a lightweight custom store pattern with Vue's reactivity system.

**From `whis-desktop/ui/src/stores/settings.ts`**:

```typescript
import { reactive, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Settings, BackendInfo, Provider, Polisher } from '../types'

// Default settings values
const defaultSettings: Settings = {
  shortcut: 'Ctrl+Shift+R',
  provider: 'openai',
  language: null,
  api_keys: {},
  // ... more defaults
}

// Internal mutable state
const state = reactive({
  ...defaultSettings,
  backendInfo: null as BackendInfo | null,
  portalShortcut: null as string | null,
  portalBindError: null as string | null,
  loaded: false,
})

// Actions
async function load() {
  try {
    const settings = await invoke<Settings>('get_settings')
    state.shortcut = settings.shortcut
    state.provider = settings.provider || 'openai'
    // ... copy all fields
  } catch (e) {
    console.error('Failed to load settings:', e)
  }
}

async function save(): Promise<boolean> {
  try {
    const result = await invoke<{ needs_restart: boolean }>('save_settings', {
      settings: {
        shortcut: state.shortcut,
        provider: state.provider,
        // ... all settings fields
      },
    })
    return result.needs_restart
  } catch (e) {
    console.error('Failed to save settings:', e)
    throw e
  }
}

// Setters for individual fields
function setProvider(value: Provider) {
  state.provider = value
}

function setApiKey(provider: string, key: string) {
  state.api_keys = { ...state.api_keys, [provider]: key }
}

// Export reactive state and actions
export const settingsStore = {
  state: readonly(state),      // Read-only for most consumers
  mutableState: state,          // Mutable for v-model binding
  load,
  save,
  setProvider,
  setApiKey,
  // ... more setters
}
```

### Why Not Pinia?

1. **Small scope**: Single store for settings is all we need
2. **Rust is source of truth**: Backend owns the data, frontend just mirrors it
3. **Simpler**: No plugin setup, no devtools dependency
4. **Type inference**: Custom store has full TypeScript support

### Readonly vs Mutable State

```typescript
state: readonly(state),    // For reading
mutableState: state,       // For v-model
```

Two access patterns:
- `settingsStore.state.provider` — Read-only, prevents accidental mutations
- `settingsStore.mutableState.shortcut` — Mutable for `v-model` binding

## Composables: Reusable Logic

Composables extract reusable logic from components. Vue 3's Composition API makes this pattern elegant.

### useKeyboardCapture

**From `whis-desktop/ui/src/composables/useKeyboardCapture.ts`**:

```typescript
import { ref, computed } from 'vue'

export function useKeyboardCapture(initialValue: string = '') {
  const isRecording = ref(false)
  const capturedShortcut = ref(initialValue)

  const shortcutKeys = computed(() => {
    if (capturedShortcut.value === 'Press keys...') return ['...']
    return capturedShortcut.value.split('+')
  })

  function handleKeyDown(e: KeyboardEvent) {
    if (!isRecording.value) return
    e.preventDefault()

    const keys: string[] = []
    if (e.ctrlKey) keys.push('Ctrl')
    if (e.shiftKey) keys.push('Shift')
    if (e.altKey) keys.push('Alt')
    if (e.metaKey) keys.push('Super')

    const key = e.key.toUpperCase()
    if (!['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) {
      keys.push(key)
    }

    if (keys.length > 0) {
      capturedShortcut.value = keys.join('+')
    }
  }

  function startRecording() {
    isRecording.value = true
    capturedShortcut.value = 'Press keys...'
  }

  function stopRecording() {
    isRecording.value = false
  }

  return {
    isRecording,
    capturedShortcut,
    shortcutKeys,
    handleKeyDown,
    startRecording,
    stopRecording,
  }
}
```

**Usage in ShortcutView**:

```vue
<script setup lang="ts">
import { useKeyboardCapture } from '@/composables/useKeyboardCapture'

const {
  isRecording,
  shortcutKeys,
  handleKeyDown,
  startRecording,
  stopRecording,
} = useKeyboardCapture('Ctrl+Shift+R')
</script>

<template>
  <div
    class="shortcut-input"
    :class="{ recording: isRecording }"
    @click="startRecording"
    @blur="stopRecording"
    @keydown="handleKeyDown"
    tabindex="0"
  >
    <span v-for="key in shortcutKeys" class="key">{{ key }}</span>
  </div>
</template>
```

### useAsyncOperation

**From `whis-desktop/ui/src/composables/useAsyncOperation.ts`**:

```typescript
export function useAsyncOperation<T, Args extends unknown[] = unknown[]>(
  operation: (...args: Args) => Promise<T>,
  options: {
    errorTimeout?: number
    successTimeout?: number
    onSuccess?: (data: T) => void
    onError?: (error: string) => void
  } = {}
): AsyncOperationReturn<T, Args> {
  const data = ref<T | null>(null)
  const error = ref<string | null>(null)
  const isLoading = ref(false)

  const isSuccess = computed(() => data.value !== null && !error.value && !isLoading.value)
  const isError = computed(() => error.value !== null)

  async function execute(...args: Args): Promise<T | null> {
    isLoading.value = true
    error.value = null

    try {
      const result = await operation(...args)
      data.value = result
      options.onSuccess?.(result)
      return result
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      options.onError?.(error.value)
      return null
    } finally {
      isLoading.value = false
    }
  }

  return { data, error, isLoading, isSuccess, isError, execute }
}
```

This composable standardizes async state management:

```typescript
const { data, isLoading, error, execute } = useAsyncOperation(
  async (id: string) => await invoke<User>('get_user', { id })
)

// In template: v-if="isLoading" / v-if="error" / {{ data }}
await execute('user-123')
```

## Root Component: App.vue

**From `whis-desktop/ui/src/App.vue`**:

```vue
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { settingsStore } from './stores/settings'

const route = useRoute()
const loaded = computed(() => settingsStore.state.loaded)
const currentRoute = computed(() => route.name as string)

const navItems = [
  { name: 'home', label: 'home', path: '/' },
  { name: 'shortcut', label: 'shortcut', path: '/shortcut' },
  { name: 'settings', label: 'settings', path: '/settings' },
  { name: 'presets', label: 'presets', path: '/presets' },
  { name: 'about', label: 'about', path: '/about' },
]

async function closeWindow() {
  try {
    const canReopen = await invoke<boolean>('can_reopen_window')
    if (canReopen) {
      await getCurrentWindow().hide()
    } else {
      await getCurrentWindow().close()
    }
  } catch {
    await getCurrentWindow().close()
  }
}

onMounted(async () => {
  await settingsStore.initialize()
})
</script>

<template>
  <div class="app" :class="{ loaded }">
    <div class="window">
      <aside class="sidebar" data-tauri-drag-region>
        <div class="brand">
          <span class="wordmark">whis</span>
        </div>

        <nav class="nav">
          <router-link
            v-for="item in navItems"
            :key="item.name"
            :to="item.path"
            class="nav-item"
            :class="{ active: currentRoute === item.name }"
          >
            {{ item.label }}
          </router-link>
        </nav>
      </aside>

      <main class="content">
        <div class="titlebar" data-tauri-drag-region>
          <div class="window-controls">
            <button @click="minimizeWindow">-</button>
            <button @click="closeWindow">×</button>
          </div>
        </div>

        <router-view />
      </main>
    </div>
  </div>
</template>
```

**Key patterns**:

### Router-Link Navigation

```vue
<router-link :to="item.path" class="nav-item">
  {{ item.label }}
</router-link>
```

Vue Router's `<router-link>` handles navigation. The `active` class is applied based on current route name.

### Router-View Slot

```vue
<router-view />
```

This renders the current route's component (HomeView, SettingsView, etc.).

### Window Controls with Tauri Drag Region

```vue
<div class="titlebar" data-tauri-drag-region>
```

The `data-tauri-drag-region` attribute tells Tauri this area can be used to drag the window. Essential for custom window decorations.

### Smart Close Behavior

```typescript
async function closeWindow() {
  const canReopen = await invoke<boolean>('can_reopen_window')
  if (canReopen) {
    await getCurrentWindow().hide()
  } else {
    await getCurrentWindow().close()
  }
}
```

If the user has a global shortcut configured, "close" just hides the window (can reopen with shortcut). Otherwise, it actually closes.

## Calling Rust Commands

The `@tauri-apps/api` package provides the IPC bridge to Rust.

### Basic Invocation

```typescript
import { invoke } from '@tauri-apps/api/core'

const settings = await invoke<Settings>('get_settings')
```

The generic `<Settings>` ensures type safety on the returned value.

### With Parameters

```typescript
await invoke<SaveResult>('save_settings', {
  settings: {
    shortcut: 'Ctrl+Shift+R',
    provider: 'openai',
    // ...
  }
})
```

Parameters are passed as a single object. Keys must match Rust command parameter names.

### Error Handling

```typescript
try {
  await invoke('toggle_recording')
} catch (e) {
  error.value = String(e)
}
```

If the Rust command returns `Err()`, `invoke()` throws. The error message is the string from `Err(String)`.

## Real-World Flow: Saving Settings

Let's trace what happens when the user saves settings:

**1. User changes a setting**

In SettingsView, the user selects a different provider:

```vue
<AppSelect
  :model-value="settingsStore.state.provider"
  @update:model-value="settingsStore.setProvider"
  :options="providerOptions"
/>
```

The `setProvider` action updates the reactive state:

```typescript
function setProvider(value: Provider) {
  state.provider = value
}
```

**2. User clicks Save**

```vue
<button @click="handleSave" class="btn">Save</button>
```

```typescript
async function handleSave() {
  try {
    const needsRestart = await settingsStore.save()
    if (needsRestart) {
      showRestartNotice.value = true
    } else {
      status.value = 'Saved'
    }
  } catch (e) {
    status.value = 'Failed to save'
  }
}
```

**3. Store saves to backend**

```typescript
async function save(): Promise<boolean> {
  const result = await invoke<{ needs_restart: boolean }>('save_settings', {
    settings: {
      shortcut: state.shortcut,
      provider: state.provider,
      // ... all fields
    },
  })
  return result.needs_restart
}
```

**4. Rust command writes to disk**

The `save_settings` command serializes to JSON and writes to `~/.config/whis/settings.json`.

**5. UI shows feedback**

```vue
<div class="status" :class="{ visible: status }">{{ status }}</div>
```

## Component Library

Whis has a small library of reusable components for consistent UI:

### AppButton

```vue
<AppButton @click="save" :loading="isSaving">
  Save Settings
</AppButton>
```

### AppInput

```vue
<AppInput
  v-model="apiKey"
  type="password"
  placeholder="sk-..."
/>
```

### AppSelect

```vue
<AppSelect
  v-model="provider"
  :options="[
    { value: 'openai', label: 'OpenAI' },
    { value: 'groq', label: 'Groq' },
  ]"
/>
```

### Settings Components

The `components/settings/` folder has domain-specific components:

- **ModeCards.vue**: Cloud vs Local toggle cards
- **CloudProviderConfig.vue**: Provider selection + API key input
- **LocalWhisperConfig.vue**: Model download and path configuration
- **RemoteWhisperConfig.vue**: Server URL configuration
- **OllamaConfig.vue**: Ollama server and model settings
- **PolishingConfig.vue**: Polisher selection and prompt customization

These are composed in SettingsView:

```vue
<template>
  <div class="section">
    <ModeCards v-model="mode" />

    <CloudProviderConfig
      v-if="mode === 'cloud'"
      v-model:provider="provider"
      v-model:api-key="apiKey"
    />

    <LocalWhisperConfig
      v-else-if="mode === 'local'"
      v-model:model-path="modelPath"
    />
  </div>
</template>
```

## Styling: Design System

Whis uses CSS custom properties for consistent theming:

```css
:root {
  /* Background */
  --bg: hsl(0, 0%, 7%);
  --bg-weak: hsl(0, 0%, 11%);
  --bg-hover: hsl(0, 0%, 16%);

  /* Text */
  --text: hsl(0, 0%, 80%);
  --text-weak: hsl(0, 0%, 62%);
  --text-strong: hsl(0, 0%, 100%);

  /* Accent - gold */
  --accent: hsl(48, 100%, 60%);

  /* Typography */
  --font: "JetBrains Mono", "Fira Code", monospace;
}
```

**From `whis-desktop/ui/src/App.vue:112-140`**

Design matches the whis.ink website:
- **Dark theme**: Almost black background
- **Gold accent**: For highlights and active states
- **Monospace font**: Terminal aesthetic

## Build Process: Vite

**From `whis-desktop/ui/package.json`**:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc -b && vite build",
    "preview": "vite preview"
  }
}
```

During development:

```bash
cd crates/whis-desktop/ui
npm run dev
```

Vite starts at `http://localhost:5173`. Tauri's dev server loads this URL.

For production:

```bash
npm run build
```

Vite compiles to `ui/dist/`. Tauri's build process embeds this in the final binary.

## Summary

**Key Takeaways:**

1. **Vue Router**: Hash mode for Tauri, lazy-loaded routes
2. **Custom store**: Lightweight reactive state with `reactive()` and `readonly()`
3. **Composables**: Reusable logic (`useKeyboardCapture`, `useAsyncOperation`)
4. **Tauri API**: `invoke()` for type-safe Rust command calls
5. **Component library**: Consistent UI with AppButton, AppInput, etc.
6. **CSS custom properties**: Design system with dark theme and gold accent

**Where This Matters in Whis:**

- Router config: `ui/src/router/index.ts`
- Settings state: `ui/src/stores/settings.ts`
- Composables: `ui/src/composables/`
- Reusable components: `ui/src/components/`
- Views: `ui/src/views/`

**Patterns Used:**

- **Centralized store**: Single source of truth for settings
- **Composables**: Extract and reuse reactive logic
- **Props down, events up**: Data flow for component communication
- **Lazy loading**: Route-based code splitting

**Design Decisions:**

1. **Why not Pinia?** Small app, Rust is source of truth, custom store is simpler
2. **Why hash mode?** Tauri loads from files, no server routing
3. **Why composables?** Extract keyboard capture, async state—reusable across views
4. **Why custom components?** Consistent styling, reduced duplication

---

Next: [Chapter 24: Common Patterns in Whis](../part8-patterns/ch24-patterns.md)
