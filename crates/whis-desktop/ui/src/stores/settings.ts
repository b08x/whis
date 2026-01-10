import type { BackendInfo, BubblePosition, PostProcessor, Provider, Settings } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { reactive, readonly, watch } from 'vue'

// Defaults fetched from backend (single source of truth: whis-core/src/defaults.rs)
interface Defaults {
  provider: Provider
  ollama_url: string
  ollama_model: string
  shortcut_key: string
  vad_enabled: boolean
  vad_threshold: number
}

// Debounce utility with cancel support
function debounce<T extends (...args: unknown[]) => unknown>(fn: T, ms: number) {
  let timeoutId: ReturnType<typeof setTimeout> | null = null

  const debounced = (...args: Parameters<T>) => {
    if (timeoutId)
      clearTimeout(timeoutId)
    timeoutId = setTimeout(() => {
      timeoutId = null
      fn(...args)
    }, ms)
  }

  debounced.cancel = () => {
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
  }

  return debounced
}

// Cached defaults from backend (populated on init)
// These are fallback values until get_defaults() is called
let defaults: Defaults = {
  provider: 'deepgram',
  ollama_url: 'http://localhost:11434',
  ollama_model: 'qwen2.5:1.5b',
  shortcut_key: 'Ctrl+Alt+W',
  vad_enabled: false,
  vad_threshold: 0.5,
}

// Get default settings using cached defaults
function getDefaultSettings(): Settings {
  return {
    transcription: {
      provider: defaults.provider,
      language: null,
      api_keys: {},
      local_models: {
        whisper_path: null,
        parakeet_path: null,
      },
    },
    post_processing: {
      processor: 'none',
      prompt: null,
    },
    services: {
      ollama: {
        url: defaults.ollama_url,
        model: defaults.ollama_model,
      },
    },
    ui: {
      shortcut_key: defaults.shortcut_key,
      clipboard_backend: 'auto',
      microphone_device: null,
      chunk_duration_secs: 90,
      vad: {
        enabled: defaults.vad_enabled,
        threshold: defaults.vad_threshold,
      },
      active_preset: null,
      bubble: {
        enabled: false,
        position: 'none' as BubblePosition,
      },
    },
  }
}

// Internal mutable state
const state = reactive({
  // Settings (initialized with defaults, updated from backend)
  ...getDefaultSettings(),

  // Backend info
  backendInfo: null as BackendInfo | null,
  portalShortcut: null as string | null,
  portalBindError: null as string | null,
  rdevGrabError: null as string | null,
  isInInputGroup: false,
  systemShortcut: null as string | null, // GNOME custom shortcut (RdevGrab backend)

  // Loading state
  loaded: false,

  // Window visibility state (for smooth show/hide transitions)
  windowVisible: false,

  // Download state (not persisted to disk)
  whisperDownload: {
    active: false,
    model: null as string | null,
    progress: null as { downloaded: number, total: number } | null,
    error: null as string | null,
  },
  parakeetDownload: {
    active: false,
    model: null as string | null,
    progress: null as { downloaded: number, total: number } | null,
    error: null as string | null,
  },
})

// Debounced auto-save (500ms delay)
const debouncedSave = debounce(async () => {
  try {
    await invoke<{ needs_restart: boolean }>('save_settings', {
      settings: {
        transcription: state.transcription,
        post_processing: state.post_processing,
        services: state.services,
        ui: state.ui,
      },
    })
  }
  catch (e) {
    console.error('Auto-save failed:', e)
  }
}, 500)

// Watch settings and auto-save on change
watch(
  () => [
    state.transcription,
    state.post_processing,
    state.services,
    state.ui,
  ],
  () => {
    if (state.loaded)
      debouncedSave()
  },
  { deep: true },
)

// Actions
async function load() {
  try {
    const settings = await invoke<Settings>('get_settings')
    // Deep copy nested settings, using cached defaults for fallbacks
    state.transcription = {
      provider: settings.transcription.provider || defaults.provider,
      language: settings.transcription.language,
      api_keys: settings.transcription.api_keys || {},
      local_models: {
        whisper_path: settings.transcription.local_models.whisper_path,
        parakeet_path: settings.transcription.local_models.parakeet_path,
      },
    }
    state.post_processing = {
      processor: settings.post_processing.processor || 'none',
      prompt: settings.post_processing.prompt,
    }
    state.services = {
      ollama: {
        url: settings.services.ollama.url || defaults.ollama_url,
        model: settings.services.ollama.model || defaults.ollama_model,
      },
    }
    state.ui = {
      shortcut_key: settings.ui.shortcut_key || defaults.shortcut_key,
      clipboard_backend: settings.ui.clipboard_backend,
      microphone_device: settings.ui.microphone_device,
      chunk_duration_secs: Math.max(10, Math.min(300, settings.ui.chunk_duration_secs ?? 90)),
      vad: {
        enabled: settings.ui.vad.enabled ?? defaults.vad_enabled,
        threshold: settings.ui.vad.threshold ?? defaults.vad_threshold,
      },
      active_preset: settings.ui.active_preset,
      bubble: {
        enabled: settings.ui.bubble?.enabled ?? false,
        position: settings.ui.bubble?.position ?? 'none',
      },
    }
  }
  catch (e) {
    console.error('Failed to load settings:', e)
  }
}

async function save(): Promise<boolean> {
  try {
    const result = await invoke<{ needs_restart: boolean }>('save_settings', {
      settings: {
        transcription: state.transcription,
        post_processing: state.post_processing,
        services: state.services,
        ui: state.ui,
      },
    })
    return result.needs_restart
  }
  catch (e) {
    console.error('Failed to save settings:', e)
    throw e
  }
}

async function loadBackendInfo() {
  try {
    state.backendInfo = await invoke<BackendInfo>('shortcut_backend')

    // For portal backend, fetch actual binding and any errors
    if (state.backendInfo?.backend === 'PortalGlobalShortcuts') {
      state.portalShortcut = await invoke<string | null>('portal_shortcut')
      state.portalBindError = await invoke<string | null>('portal_bind_error')
    }

    // For RdevGrab backend, fetch any grab errors and check input group
    if (state.backendInfo?.backend === 'RdevGrab') {
      state.rdevGrabError = await invoke<string | null>('rdev_grab_error')
      state.isInInputGroup = await invoke<boolean>('check_input_group_membership')

      // Try to read configured system shortcut from GNOME dconf
      state.systemShortcut = await invoke<string | null>('system_shortcut_from_dconf')
    }
  }
  catch (e) {
    console.error('Failed to get backend info:', e)
  }
}

async function loadDefaults() {
  try {
    defaults = await invoke<Defaults>('get_defaults')
  }
  catch (e) {
    console.error('Failed to load defaults from backend:', e)
    // Keep using fallback defaults
  }
}

async function initialize() {
  // Load canonical defaults from backend first (single source of truth)
  await loadDefaults()
  await loadBackendInfo()
  await load()

  // Query backend for active download state (survives window close/reopen)
  try {
    const activeDownload = await invoke<{
      model_name: string
      model_type: string
      downloaded: number
      total: number
    } | null>('get_active_download')

    if (activeDownload) {
      if (activeDownload.model_type === 'whisper') {
        state.whisperDownload.active = true
        state.whisperDownload.model = activeDownload.model_name
        state.whisperDownload.progress = {
          downloaded: activeDownload.downloaded,
          total: activeDownload.total,
        }
      }
      else if (activeDownload.model_type === 'parakeet') {
        state.parakeetDownload.active = true
        state.parakeetDownload.model = activeDownload.model_name
        state.parakeetDownload.progress = {
          downloaded: activeDownload.downloaded,
          total: activeDownload.total,
        }
      }
    }
  }
  catch {
    // Command not yet implemented or error - ignore
  }

  state.loaded = true

  // Sync detected system shortcut to settings.json
  // Must be after state.loaded = true so watcher triggers auto-save
  if (state.backendInfo?.backend === 'RdevGrab' && state.systemShortcut) {
    if (state.systemShortcut !== state.ui.shortcut_key) {
      state.ui.shortcut_key = state.systemShortcut
      // Watcher automatically saves since state.loaded is true
    }
  }
}

async function waitForLoaded(): Promise<void> {
  if (state.loaded)
    return

  return new Promise((resolve) => {
    const unwatch = watch(
      () => state.loaded,
      (loaded) => {
        if (loaded) {
          unwatch()
          resolve()
        }
      },
      { immediate: true },
    )
  })
}

// Flush pending settings immediately (no debounce delay)
// Called before window close to ensure settings are persisted
async function flush(): Promise<void> {
  if (!state.loaded)
    return

  // Cancel any pending debounced save to avoid race condition
  debouncedSave.cancel()

  try {
    await invoke<{ needs_restart: boolean }>('save_settings', {
      settings: {
        transcription: state.transcription,
        post_processing: state.post_processing,
        services: state.services,
        ui: state.ui,
      },
    })
  }
  catch (e) {
    console.error('Failed to flush settings:', e)
  }
}

// Setters for individual fields (for v-model binding)
function setProvider(value: Provider) {
  state.transcription.provider = value
}

function setLanguage(value: string | null) {
  state.transcription.language = value
}

function setApiKey(provider: string, key: string) {
  state.transcription.api_keys = { ...state.transcription.api_keys, [provider]: key }
}

function setWhisperModelPath(value: string | null) {
  state.transcription.local_models.whisper_path = value
}

function setParakeetModelPath(value: string | null) {
  state.transcription.local_models.parakeet_path = value
}

function setPostProcessor(value: PostProcessor) {
  state.post_processing.processor = value
}

function setOllamaUrl(value: string | null) {
  state.services.ollama.url = value
}

function setOllamaModel(value: string | null) {
  state.services.ollama.model = value
}

function setPostProcessingPrompt(value: string | null) {
  state.post_processing.prompt = value
}

function setShortcutKey(value: string) {
  state.ui.shortcut_key = value
}

function setPortalShortcut(value: string | null) {
  state.portalShortcut = value
}

function setMicrophoneDevice(value: string | null) {
  state.ui.microphone_device = value
}

function setBubbleEnabled(value: boolean) {
  state.ui.bubble.enabled = value
}

function setBubblePosition(value: BubblePosition) {
  state.ui.bubble.position = value
}

function setChunkDuration(value: number) {
  // Clamp to valid range (10-300 seconds)
  state.ui.chunk_duration_secs = Math.max(10, Math.min(300, value))
}

// Window visibility state (for smooth show/hide transitions on Wayland)
function setWindowVisible(visible: boolean) {
  state.windowVisible = visible
}

// Download state management - Whisper
function startWhisperDownload(model: string) {
  state.whisperDownload.active = true
  state.whisperDownload.model = model
  state.whisperDownload.progress = null
  state.whisperDownload.error = null
}

function updateWhisperDownloadProgress(downloaded: number, total: number) {
  if (state.whisperDownload.active) {
    state.whisperDownload.progress = { downloaded, total }
  }
}

function completeWhisperDownload() {
  state.whisperDownload.active = false
  state.whisperDownload.progress = null
  state.whisperDownload.model = null
}

function failWhisperDownload(error: string) {
  state.whisperDownload.active = false
  state.whisperDownload.error = error
}

// Download state management - Parakeet
function startParakeetDownload(model: string) {
  state.parakeetDownload.active = true
  state.parakeetDownload.model = model
  state.parakeetDownload.progress = null
  state.parakeetDownload.error = null
}

function updateParakeetDownloadProgress(downloaded: number, total: number) {
  if (state.parakeetDownload.active) {
    state.parakeetDownload.progress = { downloaded, total }
  }
}

function completeParakeetDownload() {
  state.parakeetDownload.active = false
  state.parakeetDownload.progress = null
  state.parakeetDownload.model = null
}

function failParakeetDownload(error: string) {
  state.parakeetDownload.active = false
  state.parakeetDownload.error = error
}

// Getter for default provider (used by SettingsView when switching modes)
function getDefaultProvider(): Provider {
  return defaults.provider
}

// Export reactive state and actions
export const settingsStore = {
  // Readonly state for reading (prevents accidental mutation)
  state: readonly(state),

  // Mutable state for v-model (use sparingly)
  mutableState: state,

  // Actions
  load,
  save,
  initialize,
  waitForLoaded,
  flush,
  getDefaultProvider,

  // Setters
  setProvider,
  setLanguage,
  setApiKey,
  setWhisperModelPath,
  setParakeetModelPath,
  setPostProcessor,
  setOllamaUrl,
  setOllamaModel,
  setPostProcessingPrompt,
  setShortcutKey,
  setPortalShortcut,
  setMicrophoneDevice,
  setBubbleEnabled,
  setBubblePosition,
  setChunkDuration,
  setWindowVisible,

  // Download state management
  startWhisperDownload,
  updateWhisperDownloadProgress,
  completeWhisperDownload,
  failWhisperDownload,
  startParakeetDownload,
  updateParakeetDownloadProgress,
  completeParakeetDownload,
  failParakeetDownload,
}
