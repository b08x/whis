<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { StatusResponse } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { settingsStore } from '../stores/settings'

const status = ref<StatusResponse>({ state: 'Idle', config_valid: false })
const error = ref<string | null>(null)
const postProcessWarning = ref<string | null>(null)
const isPostProcessing = ref(false)
let pollInterval: number | null = null
let unlistenPostProcessWarning: UnlistenFn | null = null
let unlistenPostProcessStarted: UnlistenFn | null = null
let unlistenTranscriptionComplete: UnlistenFn | null = null

// Configuration readiness state (proactive checks)
const configReadiness = ref<{
  transcriptionReady: boolean
  transcriptionError: string | null
  postProcessingReady: boolean
  postProcessingError: string | null
  checking: boolean
}>({
  transcriptionReady: true,
  transcriptionError: null,
  postProcessingReady: true,
  postProcessingError: null,
  checking: false,
})

const buttonText = computed(() => {
  switch (status.value.state) {
    case 'Idle': return 'Start Recording'
    case 'Recording': return 'Stop Recording'
    case 'Transcribing': return 'Transcribing...'
    default: return 'Start Recording'
  }
})

// Configuration summary for status display (compact single line)
const configSummary = computed(() => {
  const { transcription, post_processing, ui } = settingsStore.state
  const provider = transcription.provider
  const language = transcription.language
  const postProcessor = post_processing.processor
  const activePreset = ui.active_preset

  // Mode + Provider
  let mode = 'Cloud'
  let providerName: string = provider
  if (provider === 'local-whisper') {
    mode = 'Local'
    providerName = 'Whisper'
  }
  else if (provider === 'local-parakeet') {
    mode = 'Local'
    providerName = 'Parakeet'
  }
  else {
    // Capitalize cloud provider names
    providerName = provider.charAt(0).toUpperCase() + provider.slice(1)
  }

  // Language: show code or omit if auto-detect
  const lang = language ? language.toUpperCase() : null

  // Post-processing status: show preset name if active, "Post-processing" if enabled but no preset, omit if off
  let postProcessStatus: string | null = null
  if (postProcessor !== 'none') {
    postProcessStatus = activePreset || 'Post-processing'
  }

  return { mode, provider: providerName, lang, postProcessStatus }
})

const canRecord = computed(() => {
  return status.value.config_valid
    && status.value.state !== 'Transcribing'
    && configReadiness.value.transcriptionReady
})

// Check configuration readiness (proactive check for better UX)
async function checkConfigReadiness() {
  const { transcription, post_processing, services } = settingsStore.state

  configReadiness.value.checking = true
  try {
    const result = await invoke<{
      transcription_ready: boolean
      transcription_error: string | null
      post_processing_ready: boolean
      post_processing_error: string | null
    }>('check_config_readiness', {
      provider: transcription.provider,
      postProcessor: post_processing.processor,
      apiKeys: transcription.api_keys,
      whisperModelPath: transcription.local_models.whisper_path,
      parakeetModelPath: transcription.local_models.parakeet_path,
      ollamaUrl: services.ollama.url,
    })
    configReadiness.value = {
      transcriptionReady: result.transcription_ready,
      transcriptionError: result.transcription_error,
      postProcessingReady: result.post_processing_ready,
      postProcessingError: result.post_processing_error,
      checking: false,
    }
  }
  catch (e) {
    console.error('Failed to check config readiness:', e)
    configReadiness.value.checking = false
  }
}

// Watch for settings changes to re-check readiness
watch(
  () => [
    settingsStore.state.transcription.provider,
    settingsStore.state.transcription.api_keys,
    settingsStore.state.transcription.local_models.whisper_path,
    settingsStore.state.transcription.local_models.parakeet_path,
    settingsStore.state.post_processing.processor,
    settingsStore.state.services.ollama.url,
  ],
  () => checkConfigReadiness(),
  { deep: true },
)

// Re-check readiness when settings finish loading
watch(
  () => settingsStore.state.loaded,
  (loaded) => {
    if (loaded)
      checkConfigReadiness()
  },
  { immediate: false },
)

// Platform detection for macOS-friendly key display
const isMac = navigator.platform.toUpperCase().includes('MAC')

function displayKey(key: string): string {
  if (!isMac)
    return key
  switch (key.toLowerCase()) {
    case 'ctrl': return 'Control'
    case 'alt': return 'Option'
    case 'super': return 'Cmd'
    default: return key
  }
}

const displayShortcut = computed(() => {
  const portalShortcut = settingsStore.state.portalShortcut
  const currentShortcut = settingsStore.state.ui.shortcut

  if (portalShortcut) {
    let shortcut = portalShortcut
    // Use platform-aware key names
    shortcut = shortcut
      .replace(/<Control>/gi, `${displayKey('Ctrl')}+`)
      .replace(/<Shift>/gi, 'Shift+')
      .replace(/<Alt>/gi, `${displayKey('Alt')}+`)
      .replace(/<Super>/gi, `${displayKey('Super')}+`)
    shortcut = shortcut.replace(/\+$/, '')
    const parts = shortcut.split('+')
    if (parts.length > 0 && parts[parts.length - 1]) {
      parts[parts.length - 1] = parts[parts.length - 1]!.toUpperCase()
    }
    return parts.join('+')
  }
  // Apply platform-aware display for stored shortcut
  if (currentShortcut) {
    return currentShortcut.split('+').map(displayKey).join('+')
  }
  return null
})

async function fetchStatus() {
  try {
    status.value = await invoke<StatusResponse>('get_status')
    error.value = null
  }
  catch (e) {
    console.error('Failed to get status:', e)
  }
}

async function toggleRecording() {
  if (!canRecord.value)
    return

  try {
    error.value = null
    await invoke('toggle_recording')
    await fetchStatus()
  }
  catch (e) {
    error.value = String(e)
  }
}

onMounted(async () => {
  fetchStatus()
  // Wait for settings to fully load before checking config
  await settingsStore.waitForLoaded()
  checkConfigReadiness()
  pollInterval = window.setInterval(fetchStatus, 500)

  // Listen for post-processing events
  unlistenPostProcessWarning = await listen<string>('post-process-warning', (event) => {
    postProcessWarning.value = event.payload
    // Auto-dismiss after 8 seconds
    setTimeout(() => {
      postProcessWarning.value = null
    }, 8000)
  })

  unlistenPostProcessStarted = await listen('post-process-started', () => {
    isPostProcessing.value = true
  })

  unlistenTranscriptionComplete = await listen('transcription-complete', () => {
    isPostProcessing.value = false
  })
})

onUnmounted(() => {
  if (pollInterval) {
    clearInterval(pollInterval)
  }
  unlistenPostProcessWarning?.()
  unlistenPostProcessStarted?.()
  unlistenTranscriptionComplete?.()
})
</script>

<template>
  <section class="section">
    <header class="section-header">
      <h1>Home</h1>
      <p>Your voice, piped to clipboard</p>
    </header>

    <div class="section-content">
      <!-- Recording action -->
      <div class="record-action">
        <button
          class="btn btn-secondary"
          :class="{ recording: status.state === 'Recording', transcribing: status.state === 'Transcribing' }"
          :disabled="!canRecord"
          @click="toggleRecording"
        >
          <span class="record-indicator" />
          <span>{{ buttonText }}</span>
        </button>

        <!-- Shortcut hint - shown inline when available -->
        <span v-if="displayShortcut && status.state === 'Idle'" class="shortcut-hint">
          or press <kbd>{{ displayShortcut }}</kbd>
        </span>

        <!-- Config summary - compact single line status -->
        <span v-if="status.config_valid && status.state === 'Idle'" class="config-summary">
          <span :class="{ 'config-error': !configReadiness.transcriptionReady }">{{ configSummary.mode }} · {{ configSummary.provider }}</span><span v-if="configSummary.lang"> · {{ configSummary.lang }}</span><span v-if="configSummary.postProcessStatus" :class="{ 'config-warning': !configReadiness.postProcessingReady }"> · {{ configSummary.postProcessStatus }}</span>
        </span>

        <!-- State hints (announced to screen readers) -->
        <span role="status" aria-live="polite" class="state-hints">
          <span v-if="status.state === 'Recording'" class="state-hint recording">
            speak now...
          </span>
          <span v-else-if="isPostProcessing" class="state-hint post-processing">
            post-processing...
          </span>
          <span v-else-if="status.state === 'Transcribing'" class="state-hint">
            processing audio...
          </span>
        </span>
      </div>

      <!-- Error message -->
      <p v-if="error" class="error-msg">
        {{ error }}
      </p>

      <!-- Post-processing warning (runtime) -->
      <div v-if="postProcessWarning" class="warning-msg">
        <strong>Post-processing skipped:</strong> {{ postProcessWarning }}
      </div>

      <!-- Transcription not ready (blocking) -->
      <div v-if="!configReadiness.transcriptionReady && status.config_valid" class="config-notice error">
        <span class="notice-marker">[!]</span>
        <div>
          <p>{{ configReadiness.transcriptionError }}</p>
          <router-link to="/settings">
            Configure →
          </router-link>
        </div>
      </div>

      <!-- Post-processing not ready (non-blocking warning) -->
      <div v-else-if="!configReadiness.postProcessingReady && settingsStore.state.post_processing.processor !== 'none' && status.config_valid" class="config-notice warning">
        <span class="notice-marker">[!]</span>
        <div>
          <p>Post-processing unavailable: {{ configReadiness.postProcessingError }}</p>
          <router-link to="/settings">
            Configure →
          </router-link>
        </div>
      </div>

      <!-- Only show notice when something needs attention -->
      <div v-if="!status.config_valid" class="notice">
        <span class="notice-marker">[!]</span>
        <p>Configure your provider in <strong>settings</strong> to start transcribing.</p>
      </div>

      <div v-else-if="!displayShortcut" class="notice">
        <span class="notice-marker">[*]</span>
        <p>Configure a global <strong>shortcut</strong> to record hands-free.</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Recording action group */
.record-action {
  display: flex;
  flex-direction: column;
  gap: 12px;
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

.btn.btn-secondary.recording:hover:not(:disabled) {
  background: #ff6666;
  border-color: #ff6666;
}

/* Transcribing state */
.btn.btn-secondary.transcribing {
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

.btn.btn-secondary.transcribing .record-indicator {
  background: var(--accent);
  opacity: 1;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* Shortcut hint */
.shortcut-hint {
  font-size: 11px;
  color: var(--text-weak);
}

/* Config summary */
.config-summary {
  font-size: 10px;
  color: var(--text-weak);
  opacity: 0.7;
}

.shortcut-hint kbd {
  display: inline-block;
  padding: 2px 6px;
  font-family: var(--font);
  font-size: 10px;
  color: var(--accent);
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 3px;
}

/* State hints */
.state-hint {
  font-size: 11px;
  color: var(--text-weak);
  font-style: italic;
}

.state-hint.recording {
  color: var(--recording);
}

.state-hint.post-processing {
  color: var(--accent);
}

/* Error message */
.error-msg {
  font-size: 12px;
  color: var(--recording);
  padding: 8px 12px;
  background: rgba(255, 68, 68, 0.1);
  border: 1px solid rgba(255, 68, 68, 0.3);
  border-radius: 4px;
}

/* Warning message (for post-processing warnings) */
.warning-msg {
  font-size: 12px;
  color: var(--text);
  padding: 8px 12px;
  background: rgba(255, 180, 68, 0.1);
  border: 1px solid rgba(255, 180, 68, 0.3);
  border-radius: 4px;
}

.warning-msg strong {
  color: #ffb444;
}

/* Notice overrides */
.notice strong {
  color: var(--text-strong);
}

/* Config readiness warnings in summary */
.config-error {
  color: #f87171;
}

.config-warning {
  opacity: 0.5;
}

/* Config readiness notice cards */
.config-notice {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 12px;
}

.config-notice p {
  margin: 0 0 4px 0;
  color: var(--text);
}

.config-notice a {
  color: var(--accent);
  text-decoration: none;
  font-size: 11px;
}

.config-notice a:hover {
  text-decoration: underline;
}

.config-notice.error .notice-marker {
  color: #f87171;
}

.config-notice.warning .notice-marker {
  color: var(--accent);
}
</style>
