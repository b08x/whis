<script setup lang="ts">
import type { StatusResponse } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { settingsStore } from '../stores/settings'
import { AudioStreamer } from '../utils/audioStreamer'

const router = useRouter()

// State
const configValid = ref(false)
const isRecording = ref(false)
const isTranscribing = ref(false)
const isPostProcessing = ref(false)
const error = ref<string | null>(null)
const lastTranscription = ref<string | null>(null)
const isStreaming = ref(false)
const isProgressiveMode = ref(false)

// MediaRecorder for fallback (kept but unused with progressive mode)
let mediaRecorder: MediaRecorder | null = null

// Audio streamer for Realtime
let audioStreamer: AudioStreamer | null = null

// Provider (to determine if using streaming)
const provider = computed(() => settingsStore.state.provider)

const buttonText = computed(() => {
  if (isPostProcessing.value)
    return 'Post-processing...'
  if (isTranscribing.value)
    return 'Transcribing...'
  if (isRecording.value)
    return 'Recording...'
  return 'Start Recording'
})

const canRecord = computed(() => {
  return configValid.value && !isTranscribing.value && !isPostProcessing.value
})

async function checkConfig() {
  try {
    const status = await invoke<StatusResponse>('get_status')
    configValid.value = status.config_valid
  }
  catch (e) {
    console.error('Failed to get status:', e)
  }
}

async function startRecording() {
  try {
    error.value = null
    isRecording.value = true

    const storeProvider = settingsStore.state.provider
    const isRealtime = storeProvider === 'openai-realtime'

    const apiKeyProvider = isRealtime ? 'openai' : storeProvider
    const apiKey = settingsStore.state[`${apiKeyProvider}_api_key` as keyof typeof settingsStore.state] as string | null

    if (!apiKey) {
      throw new Error('No API key configured')
    }

    if (isRealtime) {
      // Start streaming transcription
      isStreaming.value = true

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
          error.value = err.message
          stopRecording()
        },
      })

      await audioStreamer.start()
    }
    else {
      // Standard providers: use progressive chunking
      isProgressiveMode.value = true

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
          error.value = err.message
          stopRecording()
        },
      })

      await audioStreamer.start()
    }
  }
  catch (e) {
    console.error('Failed to start recording:', e)
    if (e instanceof DOMException && e.name === 'NotAllowedError') {
      error.value = 'Microphone permission denied. Please allow microphone access in your browser/app settings.'
    }
    else {
      error.value = String(e)
    }
    resetState()
  }
}

function stopRecording() {
  if (isProgressiveMode.value && audioStreamer) {
    // Stop progressive chunking mode
    audioStreamer.stop()
    audioStreamer = null
    isTranscribing.value = true

    // Signal backend to stop and get result
    invoke('stop_recording')
      .catch((e) => {
        console.error('Failed to stop recording:', e)
        error.value = String(e)
        resetState()
      })

    isProgressiveMode.value = false
  }
  else if (isStreaming.value && audioStreamer) {
    // Stop realtime streaming
    audioStreamer.stop()
    audioStreamer = null

    // Signal backend to stop
    invoke('transcribe_streaming_stop').catch(console.error)

    isStreaming.value = false
    isTranscribing.value = true
  }
  else if (mediaRecorder && mediaRecorder.state !== 'inactive') {
    // Stop MediaRecorder (fallback)
    mediaRecorder.stop()
  }

  isRecording.value = false
}

function resetState() {
  isRecording.value = false
  isTranscribing.value = false
  isPostProcessing.value = false
  isStreaming.value = false
  isProgressiveMode.value = false
  mediaRecorder = null
}

async function copyLastTranscription() {
  if (lastTranscription.value) {
    await writeText(lastTranscription.value)
  }
}

async function toggleRecording() {
  if (!canRecord.value) {
    if (!configValid.value) {
      router.push('/settings')
    }
    return
  }

  if (isRecording.value) {
    stopRecording()
  }
  else {
    await startRecording()
  }
}

// Listen for transcription events
onMounted(async () => {
  await settingsStore.initialize()
  await checkConfig()

  // Listen for post-processing started event
  const unlistenPostProcess = await listen('post-processing-started', () => {
    isTranscribing.value = false
    isPostProcessing.value = true
  })

  // Listen for transcription complete event
  const unlistenComplete = await listen<string>('transcription-complete', (event) => {
    lastTranscription.value = event.payload
    resetState()
  })

  // Listen for transcription error event
  const unlistenError = await listen<string>('transcription-error', (event) => {
    error.value = event.payload
    resetState()
  })

  // Cleanup on unmount
  onUnmounted(() => {
    unlistenPostProcess()
    unlistenComplete()
    unlistenError()

    if (audioStreamer) {
      audioStreamer.stop()
      // If progressive mode was active, also stop the backend
      if (isProgressiveMode.value) {
        invoke('stop_recording').catch(console.error)
      }
    }

    if (mediaRecorder && mediaRecorder.state !== 'inactive') {
      mediaRecorder.stop()
    }
  })
})

// Watch provider changes
watch(provider, async () => {
  // Update config validity when provider changes
  await checkConfig()
})
</script>

<template>
  <div class="home-view">
    <main class="content">
      <!-- Record Button -->
      <button
        class="btn btn-secondary"
        :class="{
          'recording': isRecording,
          'transcribing': isTranscribing,
          'post-processing': isPostProcessing,
        }"
        :disabled="!canRecord"
        @click="toggleRecording"
      >
        <span class="record-indicator" />
        <span>{{ buttonText }}</span>
      </button>

      <!-- Error -->
      <p v-if="error" class="error">
        {{ error }}
      </p>

      <!-- Setup Hint -->
      <p v-if="!configValid" class="setup-hint" @click="router.push('/settings')">
        Tap to configure API key
      </p>

      <!-- Last Transcription Preview -->
      <div v-if="lastTranscription && !error" class="preview">
        <p>{{ lastTranscription.substring(0, 20) }}{{ lastTranscription.length > 20 ? '...' : '' }}</p>
        <button class="copy-btn" @click="copyLastTranscription">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
        </button>
      </div>
    </main>
  </div>
</template>

<style scoped>
.home-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  padding: 20px;
  padding-bottom: max(80px, calc(env(safe-area-inset-bottom) + 60px));
  gap: 24px;
}

/* Button needs inline-flex for indicator */
.btn.btn-secondary {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

/* Recording state - red */
.btn.btn-secondary.recording {
  background: var(--recording);
  border-color: var(--recording);
  color: white;
}

/* Transcribing and Post-processing states - same muted style */
.btn.btn-secondary.transcribing,
.btn.btn-secondary.post-processing {
  background: var(--bg-weak);
  border-color: var(--border);
  color: var(--text-weak);
  cursor: wait;
}

/* Status indicator dot inside button */
.record-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.5;
}

.btn.btn-secondary.recording .record-indicator {
  opacity: 1;
  animation: pulse 1s ease-in-out infinite;
}

.btn.btn-secondary.transcribing .record-indicator,
.btn.btn-secondary.post-processing .record-indicator {
  opacity: 1;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* Error */
.error {
  color: var(--recording);
  text-align: center;
  padding: 12px;
  background: rgba(255, 68, 68, 0.1);
  border-radius: var(--radius);
  max-width: 300px;
}

/* Setup Hint */
.setup-hint {
  color: var(--accent);
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 2px;
}

/* Preview */
.preview {
  max-width: 300px;
  padding: 8px 12px;
  background: var(--bg-weak);
  border-radius: var(--radius);
  font-size: 14px;
  color: var(--text-weak);
  display: flex;
  align-items: center;
  gap: 8px;
}

.copy-btn {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-weak);
  padding: 6px;
  border-radius: var(--radius);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
