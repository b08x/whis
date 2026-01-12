import type { UnlistenFn } from '@tauri-apps/api/event'
import type { ParakeetModelInfo } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { settingsStore } from '../stores/settings'

/**
 * Composable for managing Parakeet model downloads and validation.
 * Used in LocalWhisperConfig for local transcription configuration.
 */
export function useParakeetModel() {
  const parakeetModelValid = ref(false)
  const availableModels = ref<ParakeetModelInfo[]>([])
  const selectedModel = ref('parakeet-v3')
  const downloadStatus = ref('')

  let downloadUnlisten: UnlistenFn | null = null

  // Computed properties from store
  const provider = computed(() => settingsStore.state.transcription.provider)
  const parakeetModelPath = computed(() => settingsStore.state.transcription.local_models.parakeet_path)

  // Download state from global store (persists across navigation)
  const downloadingModel = computed(() => settingsStore.state.parakeetDownload.active)
  const downloadProgress = computed(() => settingsStore.state.parakeetDownload.progress)

  // Check if parakeet model directory exists on disk
  async function checkParakeetModel() {
    if (provider.value === 'local-parakeet') {
      try {
        parakeetModelValid.value = await invoke<boolean>('is_parakeet_model_valid')
      }
      catch {
        parakeetModelValid.value = false
      }
    }
  }

  // Fetch available parakeet models
  async function loadParakeetModels() {
    try {
      availableModels.value = await invoke<ParakeetModelInfo[]>('get_parakeet_models')
      // If a model is already installed, select it by default
      const installed = availableModels.value.find(m => m.installed)
      if (installed) {
        selectedModel.value = installed.name

        // Auto-populate path if installed but not in settings
        if (!parakeetModelPath.value && installed.path) {
          settingsStore.setParakeetModelPath(installed.path)
        }
      }
    }
    catch (e) {
      console.error('Failed to load parakeet models:', e)
    }
  }

  // Download the selected model
  async function downloadModel() {
    // Prevent duplicate downloads
    if (downloadingModel.value) {
      return
    }

    settingsStore.startParakeetDownload(selectedModel.value)
    downloadStatus.value = ''

    // Clean up existing listener before registering new one
    if (downloadUnlisten) {
      downloadUnlisten()
      downloadUnlisten = null
    }

    try {
      // Listen for progress events
      downloadUnlisten = await listen<{ downloaded: number, total: number }>('download-progress', (event) => {
        settingsStore.updateParakeetDownloadProgress(event.payload.downloaded, event.payload.total)
      })

      const path = await invoke<string>('download_parakeet_model', { modelName: selectedModel.value })
      settingsStore.setParakeetModelPath(path)
      parakeetModelValid.value = true
      downloadStatus.value = 'Model downloaded successfully!'
      // Refresh model list BEFORE clearing download state
      // This ensures isParakeetInstalled becomes true before downloadingParakeet becomes false
      await loadParakeetModels()
      settingsStore.completeParakeetDownload()
      setTimeout(() => downloadStatus.value = '', 3000)
    }
    catch (e) {
      settingsStore.failParakeetDownload(String(e))
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

  // Get size for selected model (from backend data)
  const selectedModelSize = computed(() => {
    return availableModels.value.find(m => m.name === selectedModel.value)?.size ?? ''
  })

  // Setup watchers and lifecycle
  onMounted(async () => {
    checkParakeetModel()
    loadParakeetModels()

    // Resume monitoring if download is active (e.g., after navigation)
    if (downloadingModel.value) {
      downloadUnlisten = await listen<{ downloaded: number, total: number }>('download-progress', (event) => {
        settingsStore.updateParakeetDownloadProgress(event.payload.downloaded, event.payload.total)
      })
    }
  })

  watch(provider, checkParakeetModel)
  watch(parakeetModelPath, checkParakeetModel)

  // Cleanup download listener on unmount
  onUnmounted(() => {
    if (downloadUnlisten) {
      downloadUnlisten()
      downloadUnlisten = null
    }
  })

  return {
    // State
    parakeetModelValid,
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
    checkParakeetModel,
    loadParakeetModels,
    downloadModel,
  }
}
