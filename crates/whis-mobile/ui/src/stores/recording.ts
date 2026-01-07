import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { reactive, readonly } from 'vue'
import { AudioStreamer } from '../utils/audioStreamer'
import { settingsStore } from './settings'

// Recording state
const state = reactive({
  isRecording: false,
  isTranscribing: false,
  isPostProcessing: false,
  isStreaming: false,
  isProgressiveMode: false,
  error: null as string | null,
  lastTranscription: null as string | null,
  configValid: false,
})

// Audio streamer instance
let audioStreamer: AudioStreamer | null = null

// Track recording start time to enforce minimum duration
let recordingStartTime: number | null = null
const MIN_RECORDING_DURATION_MS = 500

// Event listener cleanup functions
let cleanupListeners: (() => void)[] = []

// Guard against double initialization
let initialized = false

/**
 * Initialize the recording store - sets up event listeners.
 * Should be called once at app startup.
 */
async function initialize() {
  if (initialized)
    return
  initialized = true

  // Check initial config validity
  await checkConfig()

  // Listen for post-processing started event
  const unlistenPostProcess = await listen('post-processing-started', () => {
    state.isTranscribing = false
    state.isPostProcessing = true
  })

  // Listen for transcription complete event
  const unlistenComplete = await listen<string>('transcription-complete', (event) => {
    state.lastTranscription = event.payload
    resetState()
  })

  // Listen for transcription error event
  const unlistenError = await listen<string>('transcription-error', (event) => {
    state.error = event.payload
    resetState()
  })

  // Listen for post-processing warning
  const unlistenWarning = await listen<string>('post-process-warning', (event) => {
    state.error = `Post-processing failed: ${event.payload}. Raw transcript copied.`
  })

  cleanupListeners = [unlistenPostProcess, unlistenComplete, unlistenError, unlistenWarning]
}

/**
 * Cleanup event listeners - call when app is unmounting.
 */
function cleanup() {
  cleanupListeners.forEach(fn => fn())
  cleanupListeners = []
  initialized = false

  if (audioStreamer) {
    audioStreamer.stop()
    if (state.isProgressiveMode) {
      invoke('stop_recording').catch(console.error)
    }
    audioStreamer = null
  }
}

/**
 * Check if the current provider configuration is valid.
 */
async function checkConfig() {
  try {
    const status = await invoke<{ config_valid: boolean }>('get_status')
    state.configValid = status.config_valid
  }
  catch (e) {
    console.error('Failed to get status:', e)
    state.configValid = false
  }
}

/**
 * Reset all recording state to defaults.
 */
function resetState() {
  state.isRecording = false
  state.isTranscribing = false
  state.isPostProcessing = false
  state.isStreaming = false
  state.isProgressiveMode = false
  recordingStartTime = null
}

/**
 * Start recording audio.
 */
async function startRecording() {
  try {
    state.error = null
    state.isRecording = true
    recordingStartTime = Date.now()

    const storeProvider = settingsStore.state.provider
    const isRealtime = storeProvider === 'openai-realtime' || storeProvider === 'deepgram-realtime'

    // Map realtime providers to their base provider for API key lookup
    const apiKeyProvider = storeProvider === 'openai-realtime'
      ? 'openai'
      : storeProvider === 'deepgram-realtime'
        ? 'deepgram'
        : storeProvider
    const apiKey = settingsStore.state[`${apiKeyProvider}_api_key` as keyof typeof settingsStore.state] as string | null

    if (!apiKey) {
      throw new Error('No API key configured')
    }

    if (isRealtime) {
      // Start streaming transcription
      state.isStreaming = true

      // Start backend transcription
      await invoke('transcribe_streaming_start')

      // Start audio streamer
      audioStreamer = new AudioStreamer({
        onChunk: async (chunk) => {
          try {
            await invoke('transcribe_streaming_send_chunk', {
              chunk: Array.from(chunk),
            })
          }
          catch (e) {
            console.error('Failed to send chunk:', e)
          }
        },
        onError: (err) => {
          console.error('Audio streamer error:', err)
          state.error = err.message
          stopRecording()
        },
      })

      await audioStreamer.start()
    }
    else {
      // Standard providers: use progressive chunking
      state.isProgressiveMode = true

      // Start backend progressive recording pipeline
      await invoke('start_recording')

      // Start audio streamer (same as realtime, but calls progressive API)
      audioStreamer = new AudioStreamer({
        onChunk: async (chunk) => {
          try {
            await invoke('send_audio_chunk', {
              samples: Array.from(chunk),
            })
          }
          catch (e) {
            console.error('Failed to send chunk:', e)
          }
        },
        onError: (err) => {
          console.error('Audio streamer error:', err)
          state.error = err.message
          stopRecording()
        },
      })

      await audioStreamer.start()
    }
  }
  catch (e) {
    console.error('Failed to start recording:', e)
    if (e instanceof DOMException && e.name === 'NotAllowedError') {
      state.error = 'Microphone permission denied. Please allow microphone access.'
    }
    else {
      state.error = String(e)
    }
    resetState()
  }
}

/**
 * Stop recording and begin transcription.
 */
async function stopRecording() {
  // Enforce minimum recording duration to avoid empty/partial transcripts
  if (recordingStartTime) {
    const elapsed = Date.now() - recordingStartTime
    if (elapsed < MIN_RECORDING_DURATION_MS) {
      const remaining = MIN_RECORDING_DURATION_MS - elapsed
      await new Promise(resolve => setTimeout(resolve, remaining))
    }
    recordingStartTime = null
  }

  if (state.isProgressiveMode && audioStreamer) {
    // Stop progressive chunking mode
    audioStreamer.stop()
    audioStreamer = null
    state.isTranscribing = true
    state.isRecording = false

    // Signal backend to stop and get result
    try {
      await invoke('stop_recording')
    }
    catch (e) {
      console.error('Failed to stop recording:', e)
      state.error = String(e)
      resetState()
    }

    state.isProgressiveMode = false
  }
  else if (state.isStreaming && audioStreamer) {
    // Stop realtime streaming
    audioStreamer.stop()
    audioStreamer = null
    state.isRecording = false
    state.isTranscribing = true

    // Signal backend to stop
    try {
      await invoke('transcribe_streaming_stop')
    }
    catch (e) {
      console.error('Failed to stop streaming:', e)
    }

    state.isStreaming = false
  }
  else {
    state.isRecording = false
  }
}

/**
 * Toggle recording state - starts if not recording, stops if recording.
 * Returns true if recording was started, false if stopped or unable to record.
 */
async function toggleRecording(): Promise<boolean> {
  const canRecord = state.configValid && !state.isTranscribing && !state.isPostProcessing

  if (!canRecord) {
    return false
  }

  if (state.isRecording) {
    await stopRecording()
    return false
  }
  else {
    await startRecording()
    return true
  }
}

/**
 * Copy the last transcription to clipboard.
 */
async function copyLastTranscription() {
  if (state.lastTranscription) {
    await writeText(state.lastTranscription)
  }
}

/**
 * Clear current error message.
 */
function clearError() {
  state.error = null
}

// Export the store
export const recordingStore = {
  state: readonly(state),
  initialize,
  cleanup,
  checkConfig,
  startRecording,
  stopRecording,
  toggleRecording,
  copyLastTranscription,
  clearError,
}
