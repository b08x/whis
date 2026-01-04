import type { UnlistenFn } from '@tauri-apps/api/event'
import type { WhisperModelInfo } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { settingsStore } from '../stores/settings'

// Model sizes for display
const MODEL_SIZES: Record<string, string> = {
  tiny: '~75 MB',
  base: '~142 MB',
  small: '~466 MB',
  medium: '~1.5 GB',
}

/**
 * Composable for managing Whisper model downloads and validation.
 * Used in ApiKeyView for local transcription configuration.
 */
export function useWhisperModel() {
  const whisperModelValid = ref(false)
  const availableModels = ref<WhisperModelInfo[]>([])
  const selectedModel = ref('small')
  const downloadStatus = ref('')

  let downloadUnlisten: UnlistenFn | null = null

  // Computed properties from store
  const provider = computed(() => settingsStore.state.transcription.provider)
  const whisperModelPath = computed(() => settingsStore.state.transcription.local_models.whisper_path)

  // Download state from global store (persists across navigation)
  const downloadingModel = computed(() => settingsStore.state.whisperDownload.active)
  const downloadProgress = computed(() => settingsStore.state.whisperDownload.progress)

  // Check if whisper model file exists on disk
  async function checkWhisperModel() {
    if (provider.value === 'local-whisper') {
      try {
        whisperModelValid.value = await invoke<boolean>('is_whisper_model_valid')
      }
      catch {
        whisperModelValid.value = false
      }
    }
  }

  // Fetch available whisper models
  async function loadWhisperModels() {
    try {
      availableModels.value = await invoke<WhisperModelInfo[]>('get_whisper_models')
      // If a model is already installed, select it by default
      const installed = availableModels.value.find(m => m.installed)
      if (installed) {
        selectedModel.value = installed.name
      }
    }
    catch (e) {
      console.error('Failed to load whisper models:', e)
    }
  }

  // Download the selected model
  async function downloadModel() {
    // Prevent duplicate downloads
    if (downloadingModel.value) {
      return
    }

    settingsStore.startWhisperDownload(selectedModel.value)
    downloadStatus.value = ''

    // Clean up existing listener before registering new one
    if (downloadUnlisten) {
      downloadUnlisten()
      downloadUnlisten = null
    }

    try {
      // Listen for progress events
      downloadUnlisten = await listen<{ downloaded: number, total: number }>('download-progress', (event) => {
        settingsStore.updateWhisperDownloadProgress(event.payload.downloaded, event.payload.total)
      })

      const path = await invoke<string>('download_whisper_model', { modelName: selectedModel.value })
      settingsStore.setWhisperModelPath(path)
      whisperModelValid.value = true
      downloadStatus.value = 'Model downloaded successfully!'
      // Refresh model list BEFORE clearing download state
      // This ensures isWhisperInstalled becomes true before downloadingWhisper becomes false
      await loadWhisperModels()
      settingsStore.completeWhisperDownload()
      setTimeout(() => downloadStatus.value = '', 3000)
    }
    catch (e) {
      settingsStore.failWhisperDownload(String(e))
      downloadStatus.value = `Download failed: ${e}`
    }
    finally {
      if (downloadUnlisten) {
        downloadUnlisten()
        downloadUnlisten = null
      }
    }
  }

  // Format download progress for display
  const downloadProgressPercent = computed(() => {
    if (!downloadProgress.value || downloadProgress.value.total === 0)
      return 0
    return Math.round((downloadProgress.value.downloaded / downloadProgress.value.total) * 100)
  })

  const downloadProgressText = computed(() => {
    if (!downloadProgress.value)
      return ''
    const { downloaded, total } = downloadProgress.value
    const downloadedMB = (downloaded / 1_000_000).toFixed(0)
    const totalMB = (total / 1_000_000).toFixed(0)
    return `${downloadedMB} MB / ${totalMB} MB`
  })

  // Check if selected model is installed
  const isSelectedModelInstalled = computed(() => {
    return availableModels.value.find(m => m.name === selectedModel.value)?.installed ?? false
  })

  // Get size for selected model
  const selectedModelSize = computed(() => {
    return MODEL_SIZES[selectedModel.value] ?? ''
  })

  // Setup watchers and lifecycle
  onMounted(async () => {
    checkWhisperModel()
    loadWhisperModels()

    // Resume monitoring if download is active (e.g., after navigation)
    if (downloadingModel.value) {
      downloadUnlisten = await listen<{ downloaded: number, total: number }>('download-progress', (event) => {
        settingsStore.updateWhisperDownloadProgress(event.payload.downloaded, event.payload.total)
      })
    }
  })

  watch(provider, checkWhisperModel)
  watch(whisperModelPath, checkWhisperModel)

  // Cleanup download listener on unmount
  onUnmounted(() => {
    if (downloadUnlisten) {
      downloadUnlisten()
      downloadUnlisten = null
    }
  })

  return {
    // State
    whisperModelValid,
    availableModels,
    selectedModel,
    downloadingModel,
    downloadStatus,
    downloadProgress,

    // Computed
    downloadProgressPercent,
    downloadProgressText,
    isSelectedModelInstalled,
    selectedModelSize,

    // Actions
    checkWhisperModel,
    loadWhisperModels,
    downloadModel,

    // Constants
    MODEL_SIZES,
  }
}
