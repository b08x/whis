<script setup lang="ts">
import { ref, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { settingsStore } from '../../stores/settings'
import AppSelect from '../AppSelect.vue'
import type { SelectOption } from '../../types'

const ollamaUrl = computed(() => settingsStore.state.ollama_url)
const ollamaModel = computed(() => settingsStore.state.ollama_model)

// Platform detection using navigator
function detectPlatform(): 'linux' | 'macos' | 'windows' | 'unknown' {
  const ua = navigator.userAgent.toLowerCase()
  if (ua.includes('linux')) return 'linux'
  if (ua.includes('mac')) return 'macos'
  if (ua.includes('win')) return 'windows'
  return 'unknown'
}

const currentPlatform = ref<'linux' | 'macos' | 'windows' | 'unknown'>(detectPlatform())

// Connection state
const ollamaStatus = ref<'unknown' | 'connecting' | 'connected' | 'not-installed' | 'not-running' | 'error'>('unknown')
const ollamaStatusMessage = ref('')
const ollamaModels = ref<string[]>([])
const copied = ref(false)

// Status response type from backend
interface OllamaStatusResponse {
  installed: boolean
  running: boolean
  error: string | null
}

// Pull state
const pullModelName = ref('qwen2.5:1.5b')
const pullingModel = ref(false)
const pullStatus = ref('')
const pullProgress = ref<{ downloaded: number; total: number } | null>(null)
let unlistenPull: UnlistenFn | null = null

// Cleanup on unmount
onUnmounted(() => {
  if (unlistenPull) {
    unlistenPull()
    unlistenPull = null
  }
})

// Format pull progress
const pullProgressPercent = computed(() => {
  if (!pullProgress.value || pullProgress.value.total === 0) return 0
  return Math.round((pullProgress.value.downloaded / pullProgress.value.total) * 100)
})

const pullProgressText = computed(() => {
  if (!pullProgress.value) return ''
  const { downloaded, total } = pullProgress.value
  const downloadedMB = (downloaded / 1_000_000).toFixed(0)
  const totalMB = (total / 1_000_000).toFixed(0)
  return `${downloadedMB} MB / ${totalMB} MB`
})

async function testOllamaConnection() {
  const url = ollamaUrl.value || ''
  ollamaStatus.value = 'connecting'
  ollamaStatusMessage.value = 'Checking...'

  try {
    // First check the status (installed & running)
    const status = await invoke<OllamaStatusResponse>('check_ollama_status', { url })

    if (!status.installed) {
      ollamaStatus.value = 'not-installed'
      ollamaStatusMessage.value = 'Ollama not installed'
      return
    }

    if (status.running) {
      ollamaStatus.value = 'connected'
      ollamaStatusMessage.value = 'Connected'
      await loadOllamaModels()
      return
    }

    // Installed but not running - try to auto-start
    ollamaStatusMessage.value = 'Starting Ollama...'
    const result = await invoke<string>('start_ollama', { url })
    if (result === 'started' || result === 'running') {
      ollamaStatus.value = 'connected'
      ollamaStatusMessage.value = result === 'started' ? 'Started & connected' : 'Connected'
      await loadOllamaModels()
    } else {
      ollamaStatus.value = 'not-running'
      ollamaStatusMessage.value = 'Could not start Ollama'
    }
  } catch (e) {
    const error = String(e)
    if (error.toLowerCase().includes('not installed')) {
      ollamaStatus.value = 'not-installed'
      ollamaStatusMessage.value = 'Ollama not installed'
    } else if (error.toLowerCase().includes('not running') || error.toLowerCase().includes('connection refused')) {
      ollamaStatus.value = 'not-running'
      ollamaStatusMessage.value = 'Ollama not running'
    } else {
      ollamaStatus.value = 'error'
      ollamaStatusMessage.value = error
    }
  }
}

// Copy text to clipboard
async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
    setTimeout(() => copied.value = false, 2000)
  } catch {
    // Fallback
    const el = document.createElement('textarea')
    el.value = text
    document.body.appendChild(el)
    el.select()
    document.execCommand('copy')
    document.body.removeChild(el)
    copied.value = true
    setTimeout(() => copied.value = false, 2000)
  }
}

// Linux install command
const linuxInstallCommand = 'curl -fsSL https://ollama.com/install.sh | sh'

async function loadOllamaModels() {
  const url = ollamaUrl.value || ''
  try {
    ollamaModels.value = await invoke<string[]>('list_ollama_models', { url })
    // If no model selected but models exist, select first one
    const firstModel = ollamaModels.value[0]
    if (!ollamaModel.value && firstModel) {
      settingsStore.setOllamaModel(firstModel)
    }
  } catch (e) {
    console.error('Failed to load Ollama models:', e)
    ollamaModels.value = []
  }
}

async function pullOllamaModel() {
  if (!pullModelName.value.trim()) {
    pullStatus.value = 'Enter a model name'
    return
  }

  const url = ollamaUrl.value || ''
  pullingModel.value = true
  pullProgress.value = null
  pullStatus.value = ''

  // Listen for progress events
  unlistenPull = await listen<{ downloaded: number; total: number }>('ollama-pull-progress', (event) => {
    pullProgress.value = event.payload
  })

  try {
    await invoke('pull_ollama_model', { url, model: pullModelName.value.trim() })
    await loadOllamaModels()
    // Auto-select the pulled model
    settingsStore.setOllamaModel(pullModelName.value.trim())
    pullStatus.value = 'Model ready!'
    setTimeout(() => pullStatus.value = '', 3000)
  } catch (e) {
    pullStatus.value = String(e)
  } finally {
    pullingModel.value = false
    pullProgress.value = null
    if (unlistenPull) {
      unlistenPull()
      unlistenPull = null
    }
  }
}

// Convert models to SelectOption format
const modelOptions = computed<SelectOption[]>(() => [
  { value: null, label: 'Select a model...' },
  ...ollamaModels.value.map(model => ({ value: model, label: model }))
])

function handleOllamaModelChange(value: string | null) {
  settingsStore.setOllamaModel(value)
}
</script>

<template>
  <!-- URL and Connection Test -->
  <div class="field-row">
    <label>URL</label>
    <div class="url-config">
      <input
        type="text"
        :value="ollamaUrl || 'http://localhost:11434'"
        @input="settingsStore.setOllamaUrl(($event.target as HTMLInputElement).value || null)"
        spellcheck="false"
        aria-label="Ollama server URL"
      />
      <button
        class="ollama-ping-btn"
        @click="testOllamaConnection"
        :disabled="ollamaStatus === 'connecting'"
      >
        {{ ollamaStatus === 'connecting' ? '...' : 'ping' }}
      </button>
    </div>
  </div>
  <!-- Simple status message for connected/connecting/error -->
  <p v-if="ollamaStatusMessage && ollamaStatus !== 'not-installed' && ollamaStatus !== 'not-running'"
     class="hint ollama-hint"
     :class="{ success: ollamaStatus === 'connected', error: ollamaStatus === 'error' }"
     role="status"
     aria-live="polite">
    {{ ollamaStatusMessage }}
  </p>

  <!-- Install Helper Panel - Not Installed -->
  <div v-if="ollamaStatus === 'not-installed'" class="install-panel ollama-notice">
    <div class="install-header">
      <span class="install-icon">[!]</span>
      <span class="install-title">Ollama not installed</span>
    </div>
    <p class="install-desc">Ollama is required for local AI text polishing.</p>

    <!-- Linux instructions -->
    <div v-if="currentPlatform === 'linux'" class="install-section">
      <p class="install-label">Install on Linux:</p>
      <div class="code-block">
        <code>{{ linuxInstallCommand }}</code>
        <button class="copy-btn" @click="copyToClipboard(linuxInstallCommand)" :title="copied ? 'Copied!' : 'Copy'">
          {{ copied ? 'ok' : 'copy' }}
        </button>
      </div>
    </div>

    <!-- macOS/Windows instructions -->
    <div v-if="currentPlatform === 'macos' || currentPlatform === 'windows'" class="install-section">
      <a href="https://ollama.com/download" target="_blank" class="install-link">
        Download from ollama.com
      </a>
    </div>

    <!-- Unknown platform - show both -->
    <div v-if="currentPlatform === 'unknown'" class="install-section">
      <p class="install-label">Linux:</p>
      <div class="code-block">
        <code>{{ linuxInstallCommand }}</code>
        <button class="copy-btn" @click="copyToClipboard(linuxInstallCommand)" :title="copied ? 'Copied!' : 'Copy'">
          {{ copied ? 'ok' : 'copy' }}
        </button>
      </div>
      <p class="install-label" style="margin-top: 8px;">macOS / Windows:</p>
      <a href="https://ollama.com/download" target="_blank" class="install-link">
        Download from ollama.com
      </a>
    </div>

    <p class="install-footer">After installing, click "ping" to connect.</p>
  </div>

  <!-- Not Running Panel -->
  <div v-if="ollamaStatus === 'not-running'" class="install-panel ollama-notice not-running">
    <div class="install-header">
      <span class="install-title">Ollama installed but not running</span>
    </div>
    <div class="not-running-actions">
      <button class="btn-secondary" @click="testOllamaConnection">
        Start Ollama
      </button>
      <span class="or-text">or run:</span>
      <code class="inline-code">ollama serve</code>
    </div>
  </div>

  <!-- Model Selection (only if connected) -->
  <div v-if="ollamaStatus === 'connected'" class="field-row">
    <label>Model</label>
    <AppSelect
      :model-value="ollamaModel"
      :options="modelOptions"
      aria-label="Select Ollama model"
      @update:model-value="handleOllamaModelChange"
    />
  </div>
  <p v-if="ollamaStatus === 'connected' && ollamaModels.length > 0" class="hint ollama-hint">
    First polish may be slow while model loads into memory.
  </p>

  <!-- No models warning -->
  <div v-if="ollamaStatus === 'connected' && ollamaModels.length === 0" class="notice ollama-notice">
    <span class="notice-marker">[!]</span>
    <p>No models installed. Pull a model below to get started.</p>
  </div>

  <!-- Pull Model UI (only if connected) -->
  <div v-if="ollamaStatus === 'connected'" class="field-row">
    <label>Pull model</label>
    <div class="pull-model-input">
      <input
        type="text"
        v-model="pullModelName"
        placeholder="qwen2.5:1.5b"
        spellcheck="false"
        :disabled="pullingModel"
        aria-label="Model name to pull"
      />
      <button
        class="btn-primary"
        @click="pullOllamaModel"
        :disabled="pullingModel || !pullModelName.trim()"
      >
        {{ pullingModel ? `${pullProgressPercent}%` : 'Pull' }}
      </button>
    </div>
  </div>
  <p v-if="pullProgress" class="hint ollama-hint">
    {{ pullProgressText }}
  </p>
  <p v-else-if="pullStatus" class="hint ollama-hint" :class="{ success: pullStatus.includes('ready'), error: !pullStatus.includes('ready') }">
    {{ pullStatus }}
  </p>
</template>

<style scoped>
.field-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.field-row > label {
  width: var(--field-label-width);
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-weak);
}

.field-row > input {
  flex: 1;
}

.field-row :deep(.custom-select) {
  flex: 1;
}

.url-config {
  display: flex;
  gap: 8px;
  flex: 1;
}

.url-config input {
  flex: 1;
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
  transition: border-color 0.15s ease;
}

.url-config input:focus {
  outline: none;
  border-color: var(--accent);
}

.url-config input::placeholder {
  color: var(--text-weak);
}

.ollama-ping-btn {
  min-width: 56px;
  padding: 10px 16px;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text-weak);
  cursor: pointer;
  transition: all 0.15s ease;
}

.ollama-ping-btn:hover:not(:disabled) {
  border-color: var(--text-weak);
  color: var(--text);
}

.ollama-ping-btn:disabled {
  cursor: wait;
  color: var(--accent);
  border-color: var(--accent);
}

.hint {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0;
}

.hint.success {
  color: var(--text-weak);
}

.hint.error {
  color: #f87171;
}

.ollama-hint {
  margin-left: calc(var(--field-label-width) + 12px);
}

.ollama-hint a {
  color: var(--accent);
  text-decoration: none;
}

.ollama-hint a:hover {
  text-decoration: underline;
}

.notice {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
}

.notice-marker {
  color: var(--accent);
  flex-shrink: 0;
}

.notice p {
  font-size: 12px;
  color: var(--text);
  line-height: 1.5;
  margin: 0;
}

.ollama-notice {
  margin-left: calc(var(--field-label-width) + 12px);
  margin-bottom: 0;
}

.pull-model-input {
  display: flex;
  gap: 8px;
  flex: 1;
}

.pull-model-input input {
  flex: 1;
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
  transition: border-color 0.15s ease;
}

.pull-model-input input:focus {
  outline: none;
  border-color: var(--accent);
}

.pull-model-input input::placeholder {
  color: var(--text-weak);
}

.pull-model-input input:disabled {
  opacity: 0.6;
}

.btn-primary {
  padding: 10px 20px;
  background: var(--accent);
  border: none;
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--bg);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn-primary:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Install Panel Styles */
.install-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  border-left: 3px solid var(--accent);
}

.install-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.install-icon {
  color: var(--accent);
  font-weight: bold;
}

.install-title {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
}

.install-desc {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0;
}

.install-section {
  margin-top: 4px;
}

.install-label {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0 0 4px 0;
}

.code-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 4px;
}

.code-block code {
  flex: 1;
  font-family: monospace;
  font-size: 11px;
  color: var(--text);
  word-break: break-all;
}

.copy-btn {
  padding: 4px 8px;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font);
  font-size: 10px;
  color: var(--text-weak);
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.copy-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.install-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  background: var(--accent);
  border-radius: 4px;
  font-size: 12px;
  color: var(--bg);
  text-decoration: none;
  transition: filter 0.15s ease;
}

.install-link:hover {
  filter: brightness(1.1);
}

.install-footer {
  font-size: 11px;
  color: var(--text-weak);
  margin: 4px 0 0 0;
}

/* Not Running Panel */
.install-panel.not-running {
  border-left-color: #fbbf24;
}

.not-running-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 4px;
}

.btn-secondary {
  padding: 8px 14px;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-secondary:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.or-text {
  font-size: 11px;
  color: var(--text-weak);
}

.inline-code {
  padding: 4px 8px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: monospace;
  font-size: 11px;
  color: var(--text);
}
</style>
