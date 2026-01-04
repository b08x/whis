<script setup lang="ts">
import type { PostProcessor, SelectOption } from '../../types'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { settingsStore } from '../../stores/settings'
import AppSelect from '../AppSelect.vue'
import OllamaConfig from './OllamaConfig.vue'
import ToggleSwitch from './ToggleSwitch.vue'

const router = useRouter()

const postProcessor = computed(() => settingsStore.state.post_processing.processor)
const activePreset = computed(() => settingsStore.state.ui.active_preset)

// Preset list for inline dropdown
const presets = ref<string[]>([])
const loadingPresets = ref(false)

// Computed: is post-processing enabled?
const postProcessingEnabled = computed(() => postProcessor.value !== 'none')

// Options for dropdowns
const postProcessorOptions: SelectOption[] = [
  { value: 'openai', label: 'OpenAI (cloud)' },
  { value: 'mistral', label: 'Mistral (cloud)' },
  { value: 'ollama', label: 'Ollama (local)' },
]

const presetOptions = computed<SelectOption[]>(() => [
  { value: null, label: 'Default' },
  ...presets.value.map(name => ({ value: name, label: name })),
])

// Load presets on mount
onMounted(async () => {
  await loadPresets()
})

async function loadPresets() {
  loadingPresets.value = true
  try {
    const result = await invoke<{ name: string }[]>('list_presets')
    presets.value = result.map(p => p.name)
  }
  catch (e) {
    console.error('Failed to load presets:', e)
    presets.value = []
  }
  finally {
    loadingPresets.value = false
  }
}

// Get default post-processor based on transcription provider
function getDefaultPostProcessor(): PostProcessor {
  const provider = settingsStore.state.transcription.provider
  if (provider === 'openai')
    return 'openai'
  if (provider === 'mistral')
    return 'mistral'
  if (provider === 'local-whisper' || provider === 'local-parakeet')
    return 'ollama'
  // Default fallback
  return 'openai'
}

function togglePostProcessing(enable: boolean) {
  if (enable) {
    settingsStore.setPostProcessor(getDefaultPostProcessor())
  }
  else {
    settingsStore.setPostProcessor('none')
  }
}

function handlePostProcessorChange(value: string | null) {
  if (value)
    settingsStore.setPostProcessor(value as PostProcessor)
}

function handlePresetChange(value: string | null) {
  settingsStore.mutableState.ui.active_preset = value
}

function goToPresets() {
  router.push('/presets')
}
</script>

<template>
  <div class="post-processing-section">
    <!-- Toggle Row with Description -->
    <div class="toggle-row">
      <div class="toggle-info">
        <label>Post-processing</label>
        <span class="toggle-desc">Clean up with AI</span>
      </div>
      <ToggleSwitch
        :model-value="postProcessingEnabled"
        @update:model-value="togglePostProcessing"
      />
    </div>

    <!-- Config (shown when post-processing ON) -->
    <div v-if="postProcessingEnabled" class="post-process-config">
      <div class="field-row">
        <label>LLM</label>
        <AppSelect
          :model-value="postProcessor"
          :options="postProcessorOptions"
          @update:model-value="handlePostProcessorChange"
        />
      </div>

      <div class="field-row">
        <label>Preset</label>
        <div class="preset-row">
          <AppSelect
            :model-value="activePreset"
            :options="presetOptions"
            :disabled="loadingPresets"
            @update:model-value="handlePresetChange"
          />
          <button class="btn-link" @click="goToPresets">
            manage
          </button>
        </div>
      </div>

      <!-- Cloud post-processor hint -->
      <p v-if="postProcessor === 'openai' || postProcessor === 'mistral'" class="cloud-hint">
        Uses the same {{ postProcessor === 'openai' ? 'OpenAI' : 'Mistral' }} API key as transcription.
      </p>
    </div>

    <!-- Ollama Config (shown when Ollama selected) -->
    <div v-if="postProcessingEnabled && postProcessor === 'ollama'" class="ollama-section">
      <p class="section-label">
        ollama
      </p>
      <OllamaConfig />
    </div>
  </div>
</template>

<style scoped>
.post-processing-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* Section Label */
.section-label {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-weak);
  letter-spacing: 0.05em;
  margin: 0;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

/* Ollama Section */
.ollama-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 4px;
}

/* Toggle Row */
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.toggle-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toggle-info > label {
  font-size: 12px;
  color: var(--text-weak);
}

.toggle-desc {
  font-size: 11px;
  color: var(--text-weak);
  opacity: 0.7;
}

/* Config section */
.post-process-config {
  display: flex;
  flex-direction: column;
  gap: 10px;
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

.preset-row {
  flex: 1;
  display: flex;
  gap: 8px;
  align-items: center;
}

.preset-row :deep(.custom-select) {
  flex: 1;
}

.field-row :deep(.custom-select) {
  flex: 1;
}

.btn-link {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  font-family: var(--font);
  font-size: 11px;
  padding: 0;
  white-space: nowrap;
}

.btn-link:hover {
  text-decoration: underline;
}

.cloud-hint {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0;
  padding-top: 4px;
}
</style>
