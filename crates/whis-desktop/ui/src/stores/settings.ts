import type { BackendInfo, PostProcessor, Provider, Settings } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { reactive, readonly, watch } from 'vue'

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

// Default settings values (nested structure matching backend)
const defaultSettings: Settings = {
  transcription: {
    provider: 'openai',
    language: null,
    api_keys: {},
    local_models: {
      whisper_path: null,
      parakeet_path: null,
      parakeet_progressive_workers: 1,
    },
  },
  post_processing: {
    processor: 'none',
    prompt: null,
  },
  services: {
    ollama: {
      url: 'http://localhost:11434',
      model: null,
    },
  },
  ui: {
    shortcut: 'Ctrl+Alt+W',
    clipboard_method: 'auto',
    microphone_device: null,
    vad: {
      enabled: false,
      threshold: 0.5,
    },
    active_preset: null,
  },
}

// Internal mutable state
const state = reactive({
  // Settings
  ...defaultSettings,

  // Backend info
  backendInfo: null as BackendInfo | null,
  portalShortcut: null as string | null,
  portalBindError: null as string | null,

  // Loading state
  loaded: false,

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
    // Deep copy nested settings
    state.transcription = {
      provider: settings.transcription.provider || 'openai',
      language: settings.transcription.language,
      api_keys: settings.transcription.api_keys || {},
      local_models: {
        whisper_path: settings.transcription.local_models.whisper_path,
        parakeet_path: settings.transcription.local_models.parakeet_path,
        parakeet_progressive_workers: settings.transcription.local_models.parakeet_progressive_workers || 1,
      },
    }
    state.post_processing = {
      processor: settings.post_processing.processor || 'none',
      prompt: settings.post_processing.prompt,
    }
    state.services = {
      ollama: {
        url: settings.services.ollama.url || 'http://localhost:11434',
        model: settings.services.ollama.model,
      },
    }
    state.ui = {
      shortcut: settings.ui.shortcut,
      clipboard_method: settings.ui.clipboard_method,
      microphone_device: settings.ui.microphone_device,
      vad: {
        enabled: settings.ui.vad.enabled,
        threshold: settings.ui.vad.threshold,
      },
      active_preset: settings.ui.active_preset,
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
  }
  catch (e) {
    console.error('Failed to get backend info:', e)
  }
}

async function initialize() {
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
  catch (e) {
    // Command not yet implemented or error - ignore
    console.debug('No active download found:', e)
  }

  state.loaded = true
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

function setShortcut(value: string) {
  state.ui.shortcut = value
}

function setPortalShortcut(value: string | null) {
  state.portalShortcut = value
}

function setMicrophoneDevice(value: string | null) {
  state.ui.microphone_device = value
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
  setShortcut,
  setPortalShortcut,
  setMicrophoneDevice,

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
