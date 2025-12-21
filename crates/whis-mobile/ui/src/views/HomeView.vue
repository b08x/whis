<script setup lang="ts">
import type { StatusResponse } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { settingsStore } from '../stores/settings'

const router = useRouter()

// State
const configValid = ref(false)
const isRecording = ref(false)
const isTranscribing = ref(false)
const error = ref<string | null>(null)
const lastTranscription = ref<string | null>(null)
const showCopied = ref(false)

// MediaRecorder
let mediaRecorder: MediaRecorder | null = null
let audioChunks: Blob[] = []

const buttonText = computed(() => {
  if (isTranscribing.value)
    return 'Transcribing...'
  if (isRecording.value)
    return 'Stop Recording'
  return 'Start Recording'
})

const canRecord = computed(() => {
  return configValid.value && !isTranscribing.value
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
    audioChunks = []

    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })

    // Try to use webm/opus, fall back to whatever is available
    const mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
      ? 'audio/webm;codecs=opus'
      : MediaRecorder.isTypeSupported('audio/webm')
        ? 'audio/webm'
        : MediaRecorder.isTypeSupported('audio/ogg;codecs=opus')
          ? 'audio/ogg;codecs=opus'
          : ''

    mediaRecorder = new MediaRecorder(stream, mimeType ? { mimeType } : undefined)

    mediaRecorder.ondataavailable = (event) => {
      if (event.data.size > 0) {
        audioChunks.push(event.data)
      }
    }

    mediaRecorder.onstop = async () => {
      // Stop all tracks to release the microphone
      stream.getTracks().forEach(track => track.stop())

      if (audioChunks.length === 0) {
        error.value = 'No audio recorded'
        isRecording.value = false
        return
      }

      const audioBlob = new Blob(audioChunks, { type: mediaRecorder?.mimeType || 'audio/webm' })
      await transcribeAudio(audioBlob)
    }

    mediaRecorder.onerror = (event) => {
      console.error('MediaRecorder error:', event)
      error.value = 'Recording error occurred'
      isRecording.value = false
      stream.getTracks().forEach(track => track.stop())
    }

    mediaRecorder.start()
    isRecording.value = true
  }
  catch (e) {
    console.error('Failed to start recording:', e)
    if (e instanceof DOMException && e.name === 'NotAllowedError') {
      error.value = 'Microphone permission denied. Please allow microphone access in your browser/app settings.'
    }
    else {
      error.value = String(e)
    }
  }
}

function stopRecording() {
  if (mediaRecorder && mediaRecorder.state !== 'inactive') {
    mediaRecorder.stop()
  }
  isRecording.value = false
}

async function transcribeAudio(audioBlob: Blob) {
  isTranscribing.value = true
  error.value = null

  try {
    // Convert blob to Uint8Array
    const arrayBuffer = await audioBlob.arrayBuffer()
    const audioData = Array.from(new Uint8Array(arrayBuffer))

    const text = await invoke<string>('transcribe_audio', {
      audioData,
      mimeType: audioBlob.type || 'audio/webm',
    })

    lastTranscription.value = text
    showCopied.value = true
    setTimeout(() => showCopied.value = false, 2000)
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    isTranscribing.value = false
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

onMounted(async () => {
  await settingsStore.initialize()
  await checkConfig()
})

onUnmounted(() => {
  // Clean up recording if component unmounts
  if (mediaRecorder && mediaRecorder.state !== 'inactive') {
    mediaRecorder.stop()
  }
})
</script>

<template>
  <div class="home-view">
    <main class="content">
      <!-- Record Button -->
      <button
        class="btn btn-secondary"
        :class="{ recording: isRecording, transcribing: isTranscribing }"
        :disabled="!canRecord"
        @click="toggleRecording"
      >
        <span class="record-indicator" />
        <span>{{ buttonText }}</span>
      </button>

      <!-- Copied Toast -->
      <div v-if="showCopied" class="toast">
        Copied to clipboard!
      </div>

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
        <p>{{ lastTranscription.substring(0, 100) }}{{ lastTranscription.length > 100 ? '...' : '' }}</p>
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

/* Content */
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

/* Record Button - aligned with desktop */
.btn.btn-secondary {
  gap: 10px;
}

/* Toast */
.toast {
  position: fixed;
  bottom: 100px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--accent);
  color: var(--text-inverted);
  padding: 12px 24px;
  border-radius: var(--radius);
  font-weight: 500;
  animation: fadeIn 0.2s ease;
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
  padding: 16px;
  background: var(--bg-weak);
  border-radius: var(--radius);
  font-size: 14px;
  color: var(--text-weak);
  text-align: center;
}
</style>
