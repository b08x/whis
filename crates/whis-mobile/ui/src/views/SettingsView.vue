<script setup lang="ts">
import type { PostProcessor, Provider, SelectOption, TranscriptionMethod } from '../types'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onMounted } from 'vue'
import AppInput from '../components/AppInput.vue'
import AppSelect from '../components/AppSelect.vue'
import ToggleSwitch from '../components/ToggleSwitch.vue'
import { presetsStore } from '../stores/presets'
import { settingsStore } from '../stores/settings'

// Provider options (ordered by recommendation)
const providerOptions: SelectOption[] = [
  { value: 'deepgram', label: 'Deepgram' },
  { value: 'openai', label: 'OpenAI Whisper' },
  { value: 'mistral', label: 'Mistral Voxtral' },
  { value: 'groq', label: 'Groq' },
  { value: 'elevenlabs', label: 'ElevenLabs' },
]

// Language options
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
  { value: 'ja', label: 'Japanese' },
  { value: 'zh', label: 'Chinese' },
  { value: 'ko', label: 'Korean' },
]

// Post-processor options
const postProcessorOptions: SelectOption[] = [
  { value: 'none', label: 'Disabled' },
  { value: 'openai', label: 'OpenAI' },
  { value: 'mistral', label: 'Mistral' },
]

// Computed bindings to store
const provider = computed({
  get: () => settingsStore.state.provider,
  set: val => settingsStore.setProvider(val),
})

const language = computed({
  get: () => settingsStore.state.language,
  set: val => settingsStore.setLanguage(val),
})

const openaiApiKey = computed({
  get: () => settingsStore.state.openai_api_key ?? '',
  set: val => settingsStore.setOpenaiApiKey(val || null),
})

const mistralApiKey = computed({
  get: () => settingsStore.state.mistral_api_key ?? '',
  set: val => settingsStore.setMistralApiKey(val || null),
})

const groqApiKey = computed({
  get: () => settingsStore.state.groq_api_key ?? '',
  set: val => settingsStore.setGroqApiKey(val || null),
})

const deepgramApiKey = computed({
  get: () => settingsStore.state.deepgram_api_key ?? '',
  set: val => settingsStore.setDeepgramApiKey(val || null),
})

const elevenlabsApiKey = computed({
  get: () => settingsStore.state.elevenlabs_api_key ?? '',
  set: val => settingsStore.setElevenlabsApiKey(val || null),
})

// OpenAI streaming method
const openaiMethod = computed<TranscriptionMethod>({
  get: () => provider.value === 'openai-realtime' ? 'streaming' : 'standard',
  set: (val) => {
    const newProvider = val === 'streaming' ? 'openai-realtime' : 'openai'
    settingsStore.setProvider(newProvider)
  },
})

// Show streaming toggle only for OpenAI
const showStreamingToggle = computed(() => {
  return provider.value === 'openai' || provider.value === 'openai-realtime'
})

// Streaming toggle state (convert TranscriptionMethod to boolean)
const isStreamingEnabled = computed({
  get: () => openaiMethod.value === 'streaming',
  set: (value: boolean) => {
    openaiMethod.value = value ? 'streaming' : 'standard'
  },
})

// Deepgram streaming method
const deepgramMethod = computed<TranscriptionMethod>({
  get: () => provider.value === 'deepgram-realtime' ? 'streaming' : 'standard',
  set: (val) => {
    const newProvider = val === 'streaming' ? 'deepgram-realtime' : 'deepgram'
    settingsStore.setProvider(newProvider)
  },
})

// Show streaming toggle for Deepgram
const showDeepgramStreamingToggle = computed(() => {
  return provider.value === 'deepgram' || provider.value === 'deepgram-realtime'
})

// Deepgram streaming toggle state
const isDeepgramStreamingEnabled = computed({
  get: () => deepgramMethod.value === 'streaming',
  set: (value: boolean) => {
    deepgramMethod.value = value ? 'streaming' : 'standard'
  },
})

// Normalize provider for display (realtime variants show as base provider)
const displayProvider = computed<Provider>(() => {
  if (provider.value === 'openai-realtime')
    return 'openai'
  if (provider.value === 'deepgram-realtime')
    return 'deepgram'
  return provider.value
})

// Post-processor binding
const postProcessor = computed({
  get: () => settingsStore.state.post_processor,
  set: val => settingsStore.setPostProcessor(val as PostProcessor),
})

// Active preset (read-only display)
const activePresetName = computed(() => {
  const active = presetsStore.state.presets.find(p => p.is_active)
  return active?.name ?? 'None'
})

// Load presets on mount to get active preset
onMounted(() => {
  presetsStore.loadPresets()
})
</script>

<template>
  <div class="settings-view">
    <main class="settings-content">
      <!-- Provider -->
      <div class="field">
        <label>provider</label>
        <AppSelect
          :model-value="displayProvider"
          :options="providerOptions"
          aria-label="Select provider"
          @update:model-value="(val) => provider = val as Provider"
        />
      </div>

      <!-- OpenAI API Key -->
      <div v-if="provider === 'openai' || provider === 'openai-realtime'" class="field">
        <label>openai api key</label>
        <AppInput
          v-model="openaiApiKey"
          type="password"
          placeholder="sk-..."
        />
        <span class="hint">
          Get your key at <span class="link" @click="openUrl('https://platform.openai.com/api-keys')">platform.openai.com</span>
        </span>
      </div>

      <!-- Streaming Toggle (OpenAI only) -->
      <div v-if="showStreamingToggle" class="field streaming-field">
        <label>streaming mode</label>
        <div class="field-row">
          <ToggleSwitch v-model="isStreamingEnabled" />
          <span class="method-description">
            {{ openaiMethod === 'streaming' ? 'Real-time' : 'Standard' }}
          </span>
        </div>
        <span v-if="openaiMethod === 'streaming'" class="hint">
          Lower latency via WebSocket. Transcription begins as you speak.
        </span>
        <span v-else class="hint">
          Upload audio after recording. More reliable for longer recordings.
        </span>
      </div>

      <!-- Mistral API Key -->
      <div v-if="provider === 'mistral'" class="field">
        <label>mistral api key</label>
        <AppInput
          v-model="mistralApiKey"
          type="password"
          placeholder="Enter API key"
        />
        <span class="hint">
          Get your key at <span class="link" @click="openUrl('https://console.mistral.ai/api-keys')">console.mistral.ai</span>
        </span>
      </div>

      <!-- Groq API Key -->
      <div v-if="provider === 'groq'" class="field">
        <label>groq api key</label>
        <AppInput
          v-model="groqApiKey"
          type="password"
          placeholder="gsk_..."
        />
        <span class="hint">
          Get your key at <span class="link" @click="openUrl('https://console.groq.com/keys')">console.groq.com</span>
        </span>
      </div>

      <!-- Deepgram API Key -->
      <div v-if="provider === 'deepgram' || provider === 'deepgram-realtime'" class="field">
        <label>deepgram api key</label>
        <AppInput
          v-model="deepgramApiKey"
          type="password"
          placeholder="Enter API key"
        />
        <span class="hint">
          Get your key at <span class="link" @click="openUrl('https://console.deepgram.com')">console.deepgram.com</span>
        </span>
      </div>

      <!-- Streaming Toggle (Deepgram) -->
      <div v-if="showDeepgramStreamingToggle" class="field streaming-field">
        <label>streaming mode</label>
        <div class="field-row">
          <ToggleSwitch v-model="isDeepgramStreamingEnabled" />
          <span class="method-description">
            {{ deepgramMethod === 'streaming' ? 'Real-time' : 'Standard' }}
          </span>
        </div>
        <span v-if="deepgramMethod === 'streaming'" class="hint">
          Lower latency via WebSocket. Transcription begins as you speak.
        </span>
        <span v-else class="hint">
          Upload audio after recording. More reliable for longer recordings.
        </span>
      </div>

      <!-- ElevenLabs API Key -->
      <div v-if="provider === 'elevenlabs'" class="field">
        <label>elevenlabs api key</label>
        <AppInput
          v-model="elevenlabsApiKey"
          type="password"
          placeholder="Enter API key"
        />
        <span class="hint">
          Get your key at <span class="link" @click="openUrl('https://elevenlabs.io/app/settings/api-keys')">elevenlabs.io</span>
        </span>
      </div>

      <!-- Language -->
      <div class="field">
        <label>language</label>
        <AppSelect
          v-model="language"
          :options="languageOptions"
          aria-label="Select language"
        />
        <span class="hint">
          Language of audio being transcribed
        </span>
      </div>

      <!-- Active Preset (read-only) -->
      <div class="field">
        <label>active preset</label>
        <div class="field-value">
          {{ activePresetName }}
        </div>
      </div>

      <!-- Post-Processing -->
      <div class="field">
        <label>post-processing</label>
        <AppSelect
          v-model="postProcessor"
          :options="postProcessorOptions"
          aria-label="Select post-processor"
        />
      </div>

      <!-- Auto-save notice -->
      <div class="auto-save-notice">
        <span class="notice-marker">[*]</span>
        <span>Settings are saved automatically</span>
      </div>
    </main>
  </div>
</template>

<style scoped>
.settings-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.settings-content {
  flex: 1;
  padding: 20px;
  padding-bottom: max(20px, env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 24px;
}

/* Streaming field styling */
.streaming-field .field-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.method-description {
  font-size: 14px;
  color: var(--text);
}

/* Read-only field value (for preset display) */
.field-value {
  padding: 12px 14px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 14px;
  color: var(--text);
}

.auto-save-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: auto;
  padding-top: 24px;
  font-size: 13px;
  color: var(--text-weak);
}

.notice-marker {
  color: var(--accent);
}

.link {
  color: var(--text-strong);
  text-decoration: underline;
  text-underline-offset: 2px;
  cursor: pointer;
}

.link:active {
  color: var(--accent);
}

.hint {
  display: block;
  font-size: 13px;
  color: var(--text-weak);
  margin-top: 8px;
  line-height: 1.4;
}
</style>
