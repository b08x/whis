<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { recordingStore } from '../stores/recording'
import { settingsStore } from '../stores/settings'

const router = useRouter()

// Computed state from recording store
const isRecording = computed(() => recordingStore.state.isRecording)
const isTranscribing = computed(() => recordingStore.state.isTranscribing)
const isPostProcessing = computed(() => recordingStore.state.isPostProcessing)
const error = computed(() => recordingStore.state.error)
const lastTranscription = computed(() => recordingStore.state.lastTranscription)
const configValid = computed(() => recordingStore.state.configValid)

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

// Provider (to watch for changes)
const provider = computed(() => settingsStore.state.provider)

async function toggleRecording() {
  if (!canRecord.value) {
    if (!configValid.value) {
      router.push('/settings')
    }
    return
  }

  await recordingStore.toggleRecording()
}

async function copyLastTranscription() {
  await recordingStore.copyLastTranscription()
}

// Watch provider changes to update config validity
watch(provider, async () => {
  await recordingStore.checkConfig()
})

onMounted(async () => {
  // Re-check config when view mounts
  await recordingStore.checkConfig()
})

onUnmounted(() => {
  // Clear error when leaving the view
  recordingStore.clearError()
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
