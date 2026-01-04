<script setup lang="ts">
import type { TranscriptionMode } from '../components/settings/ModeCards.vue'
import type { PostProcessor, Provider, SelectOption, TranscriptionMethod } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'
import AppSelect from '../components/AppSelect.vue'
import CloudProviderConfig from '../components/settings/CloudProviderConfig.vue'
import LocalWhisperConfig from '../components/settings/LocalWhisperConfig.vue'
import ModeCards from '../components/settings/ModeCards.vue'
import PostProcessingConfig from '../components/settings/PostProcessingConfig.vue'
import ToggleSwitch from '../components/settings/ToggleSwitch.vue'
import { settingsStore } from '../stores/settings'
import { isLocalProvider } from '../types'

const helpOpen = ref(false)

// Settings from store
const provider = computed(() => settingsStore.state.transcription.provider)
const language = computed(() => settingsStore.state.transcription.language)
const apiKeys = computed(() => settingsStore.state.transcription.api_keys)
const postProcessor = computed(() => settingsStore.state.post_processing.processor)

// Transcription mode: cloud vs local
const transcriptionMode = ref<TranscriptionMode>(
  isLocalProvider(provider.value) ? 'local' : 'cloud',
)

// OpenAI transcription method: standard vs streaming
const openaiMethod = computed<TranscriptionMethod>(() => {
  if (provider.value === 'openai-realtime')
    return 'streaming'
  return 'standard'
})

// Whether to show OpenAI method toggle (cloud mode + OpenAI selected)
const showOpenAIMethod = computed(() =>
  transcriptionMode.value === 'cloud'
  && (provider.value === 'openai' || provider.value === 'openai-realtime'),
)

// Normalize provider for dropdown display (openai-realtime shows as openai)
const displayProvider = computed(() => {
  if (provider.value === 'openai-realtime')
    return 'openai'
  return provider.value
})

// Whisper model validation (for local provider)
const whisperModelValid = ref(false)

// Watch for provider changes to validate model
watch(provider, async () => {
  if (isLocalProvider(provider.value)) {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      // Validate the model for the current local provider
      if (provider.value === 'local-whisper') {
        whisperModelValid.value = await invoke<boolean>('is_whisper_model_valid')
      }
      else if (provider.value === 'local-parakeet') {
        whisperModelValid.value = await invoke<boolean>('is_parakeet_model_valid')
      }
    }
    catch {
      whisperModelValid.value = false
    }
  }
}, { immediate: true })

// Cloud provider options for dropdown
const cloudProviderOptions: SelectOption[] = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'mistral', label: 'Mistral' },
  { value: 'groq', label: 'Groq' },
  { value: 'deepgram', label: 'Deepgram' },
  { value: 'elevenlabs', label: 'ElevenLabs' },
]

// Filter providers based on streaming mode (only OpenAI supports streaming)
const filteredProviderOptions = computed(() => {
  if (openaiMethod.value === 'streaming') {
    return cloudProviderOptions.filter(p => p.value === 'openai')
  }
  return cloudProviderOptions
})

// Common language codes for the dropdown
const languageOptions: SelectOption[] = [
  { value: null, label: 'Auto-detect' },
  { value: 'en', label: 'English' },
  { value: 'de', label: 'German' },
  { value: 'fr', label: 'French' },
  { value: 'es', label: 'Spanish' },
  { value: 'it', label: 'Italian' },
  { value: 'pt', label: 'Portuguese' },
  { value: 'nl', label: 'Dutch' },
  { value: 'pl', label: 'Polish' },
  { value: 'ru', label: 'Russian' },
  { value: 'ja', label: 'Japanese' },
  { value: 'ko', label: 'Korean' },
  { value: 'zh', label: 'Chinese' },
]

function handleModeChange(mode: TranscriptionMode) {
  transcriptionMode.value = mode
  if (mode === 'cloud') {
    // Switch to default cloud provider if currently on local
    if (isLocalProvider(provider.value)) {
      settingsStore.setProvider('openai')
      // Auto-sync post-processor to match (if user has cloud post-processor enabled)
      if (postProcessor.value !== 'none' && postProcessor.value !== 'ollama') {
        settingsStore.setPostProcessor('openai')
      }
    }
  }
  else {
    // Switch to local-parakeet (recommended) if currently on cloud
    if (!isLocalProvider(provider.value)) {
      settingsStore.setProvider('local-parakeet')
    }
  }
}

function handleProviderUpdate(value: string | null) {
  if (!value)
    return

  // If OpenAI selected and we're already in streaming mode, keep streaming
  if (value === 'openai' && openaiMethod.value === 'streaming') {
    // Already on openai-realtime, no change needed
    return
  }

  const newProvider = value as Provider
  settingsStore.setProvider(newProvider)
  // Auto-sync post-processor to match provider (if user has cloud post-processor enabled)
  if ((newProvider === 'openai' || newProvider === 'mistral')
    && postProcessor.value !== 'none' && postProcessor.value !== 'ollama') {
    settingsStore.setPostProcessor(newProvider as PostProcessor)
  }
}

function handleApiKeyUpdate(providerKey: string, value: string) {
  settingsStore.setApiKey(providerKey, value)
}

function handleOpenAIMethodChange(method: TranscriptionMethod) {
  const newProvider = method === 'standard' ? 'openai' : 'openai-realtime'
  settingsStore.setProvider(newProvider)
  // Keep post-processor as openai (both methods use same API)
  if (postProcessor.value !== 'none' && postProcessor.value !== 'ollama') {
    settingsStore.setPostProcessor('openai')
  }
}

function handleStreamingToggle(enabled: boolean) {
  handleOpenAIMethodChange(enabled ? 'streaming' : 'standard')
}

function handleLanguageChange(value: string | null) {
  settingsStore.setLanguage(value)
}

// Audio devices
interface AudioDevice {
  name: string
  is_default: boolean
}

const audioDevices = ref<AudioDevice[]>([])
const microphoneDevice = computed(() => settingsStore.state.ui.microphone_device)

// Load available audio devices
onMounted(async () => {
  try {
    audioDevices.value = await invoke<AudioDevice[]>('list_audio_devices')
  }
  catch (error) {
    console.error('Failed to load audio devices:', error)
  }
})

// Convert audio devices to select options
const microphoneOptions = computed<SelectOption[]>(() => {
  const options: SelectOption[] = [
    { value: null, label: 'System Default' },
  ]

  for (const device of audioDevices.value) {
    options.push({
      value: device.name,
      label: device.is_default ? `${device.name} (default)` : device.name,
    })
  }

  return options
})

function handleMicrophoneChange(value: string | null) {
  settingsStore.setMicrophoneDevice(value)
}
</script>

<template>
  <section class="section settings-section">
    <header class="section-header">
      <div class="header-content">
        <h1>Settings</h1>
        <p>Configure transcription</p>
      </div>
      <button class="help-btn" :aria-label="helpOpen ? 'Close help' : 'Open help'" @click="helpOpen = !helpOpen">
        {{ helpOpen ? 'Close' : 'Help' }}
      </button>
    </header>

    <div class="settings-layout">
      <div class="section-content">
        <!-- Mode Cards (Cloud/Local) -->
        <ModeCards
          :model-value="transcriptionMode"
          @update:model-value="handleModeChange"
        />

        <!-- Cloud Provider Config (API key only) -->
        <CloudProviderConfig
          v-if="transcriptionMode === 'cloud'"
          :provider="provider"
          :api-keys="apiKeys"
          :show-config-card="true"
          @update:api-key="handleApiKeyUpdate"
        />

        <!-- Local Transcription Config (Whisper or Parakeet) -->
        <LocalWhisperConfig
          v-if="transcriptionMode === 'local'"
          :show-config-card="true"
          :provider="provider"
        />

        <!-- Transcription Section -->
        <div class="settings-section">
          <p class="section-label">
            transcription
          </p>

          <!-- Provider (only in cloud mode) -->
          <div v-if="transcriptionMode === 'cloud'" class="field-row">
            <label>Service</label>
            <AppSelect
              :model-value="displayProvider"
              :options="filteredProviderOptions"
              @update:model-value="handleProviderUpdate"
            />
          </div>

          <!-- Streaming toggle (only for OpenAI in cloud mode) -->
          <div v-if="showOpenAIMethod" class="field-row">
            <label>Streaming</label>
            <ToggleSwitch
              :model-value="openaiMethod === 'streaming'"
              @update:model-value="handleStreamingToggle"
            />
          </div>

          <!-- Language -->
          <div class="field-row">
            <label>Language</label>
            <AppSelect
              :key="`language-${settingsStore.state.loaded}`"
              :model-value="language"
              :options="languageOptions"
              @update:model-value="handleLanguageChange"
            />
          </div>

          <!-- Microphone Device -->
          <div class="field-row">
            <label>Microphone</label>
            <AppSelect
              :key="`microphone-${settingsStore.state.loaded}`"
              :model-value="microphoneDevice"
              :options="microphoneOptions"
              @update:model-value="handleMicrophoneChange"
            />
          </div>
        </div>

        <!-- Post-Processing Section -->
        <div class="settings-section">
          <p class="section-label">
            post-processing
          </p>
          <PostProcessingConfig />
        </div>
      </div>

      <!-- Help Panel -->
      <div class="help-panel" :class="{ open: helpOpen }">
        <div class="panel-content">
          <div class="panel-header">
            <h2>Help</h2>
          </div>

          <div class="help-section">
            <h3>transcription mode</h3>
            <p>Choose how to transcribe audio. Cloud is fast and easy (requires API key and internet). Local is private and free (requires model download, slower on older hardware).</p>
          </div>

          <div class="help-section">
            <h3>transcription service</h3>
            <p>Choose which cloud service performs speech-to-text. Each has different pricing, speed, and language support. Requires a separate API account.</p>
          </div>

          <div class="help-section">
            <h3>streaming</h3>
            <p>OpenAI-only feature. When enabled, audio streams to OpenAI during recording for lower latency. When disabled, audio is uploaded after recording (works with files too).</p>
          </div>

          <div class="help-section">
            <h3>api key</h3>
            <p>Your API key authenticates with the provider. Get it from your provider's website. Keys are stored locally on your device.</p>
          </div>

          <div class="help-section">
            <h3>local transcription</h3>
            <p><strong>Local Whisper:</strong> Run models on your device (75MB-3GB). Fully offline. Larger models = better accuracy, slower processing.</p>
          </div>

          <div class="help-section">
            <h3>language</h3>
            <p>Auto-detect works for most recordings. Set a specific language if you're getting poor results with accents, technical terms, or mixed languages.</p>
          </div>

          <div class="help-section">
            <h3>post-processing</h3>
            <p>Clean up transcripts with AI. Fixes grammar, punctuation, and can add structure. Works with cloud providers or local Ollama. Optional—leave off for verbatim transcripts.</p>
          </div>

          <div class="help-section">
            <h3>presets</h3>
            <p>Pre-configured post-processing instructions. Choose a style that matches your use case, or create custom presets in the Presets page.</p>
          </div>

          <div class="help-section">
            <h3>ollama</h3>
            <p>Use local AI models for post-processing without cloud APIs or costs. Requires Ollama running on your machine.</p>
            <div class="help-steps">
              <p><strong>Setup:</strong></p>
              <ol>
                <li>Install from <a href="https://ollama.com/download" target="_blank">ollama.com</a></li>
                <li>Download a model: <code>ollama pull llama3.2:3b</code></li>
                <li>Click "ping" below to test connection</li>
              </ol>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Settings Sections */
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-label {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-weak);
  letter-spacing: 0.05em;
  margin: 0;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

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

/* Settings layout for panel positioning */
.settings-section {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.settings-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
  position: relative;
}

.section-content {
  flex: 1;
  overflow-y: auto;
  padding-right: 16px;
}

/* Header with help button */
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.header-content {
  flex: 1;
}

.help-btn {
  background: none;
  border: none;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text-weak);
  cursor: pointer;
  padding: 4px 8px;
  transition: color 0.15s ease;
}

.help-btn:hover {
  color: var(--accent);
}

/* Help Panel */
.help-panel {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 320px;
  background: var(--bg);
  border-left: 1px solid var(--border);
  transform: translateX(100%);
  transition: transform 0.2s ease-out;
  overflow-y: auto;
}

.help-panel.open {
  transform: translateX(0);
}

.panel-content {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border);
}

.panel-header h2 {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-strong);
  margin: 0;
}

.help-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.help-section h3 {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-weak);
  margin: 0;
  text-transform: lowercase;
}

.help-section p {
  font-size: 12px;
  color: var(--text);
  line-height: 1.5;
  margin: 0;
}

.help-steps {
  margin-top: 8px;
}

.help-steps ol {
  margin: 4px 0 0 0;
  padding-left: 16px;
  font-size: 11px;
  color: var(--text);
  line-height: 1.6;
}

.help-steps li {
  margin-bottom: 2px;
}

.help-steps code {
  background: var(--bg-weak);
  padding: 1px 4px;
  border-radius: 2px;
  font-size: 10px;
}

.help-steps a {
  color: var(--accent);
  text-decoration: none;
}

.help-steps a:hover {
  text-decoration: underline;
}
</style>
