<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { settingsStore } from '../../stores/settings'
import AppSelect from '../AppSelect.vue'
import OllamaConfig from './OllamaConfig.vue'
import type { Polisher, SelectOption } from '../../types'

const router = useRouter()

const polisher = computed(() => settingsStore.state.polisher)
const activePreset = computed(() => settingsStore.state.active_preset)

// Preset list for inline dropdown
const presets = ref<string[]>([])
const loadingPresets = ref(false)

// Computed: is polishing enabled?
const polishingEnabled = computed(() => polisher.value !== 'none')

// Options for dropdowns
const polisherOptions: SelectOption[] = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'mistral', label: 'Mistral' },
  { value: 'ollama', label: 'Ollama (local)' },
]

const presetOptions = computed<SelectOption[]>(() => [
  { value: null, label: 'Default' },
  ...presets.value.map(name => ({ value: name, label: name }))
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
  } catch (e) {
    console.error('Failed to load presets:', e)
    presets.value = []
  } finally {
    loadingPresets.value = false
  }
}

// Get default polisher based on transcription provider
function getDefaultPolisher(): Polisher {
  const provider = settingsStore.state.provider
  if (provider === 'openai') return 'openai'
  if (provider === 'mistral') return 'mistral'
  if (provider === 'local-whisper') return 'ollama'
  // Default fallback
  return 'openai'
}

function togglePolishing(enable: boolean) {
  if (enable) {
    settingsStore.setPolisher(getDefaultPolisher())
  } else {
    settingsStore.setPolisher('none')
  }
}

function handlePolisherChange(value: string | null) {
  if (value) settingsStore.setPolisher(value as Polisher)
}

function handlePresetChange(value: string | null) {
  settingsStore.mutableState.active_preset = value
}

function goToPresets() {
  router.push('/presets')
}
</script>

<template>
  <div class="polishing-section">
    <!-- Toggle Row with Description -->
    <div class="toggle-row">
      <div class="toggle-info">
        <label>Polishing</label>
        <span class="toggle-desc">Clean up with AI</span>
      </div>
      <button
        class="toggle-switch"
        :class="{ active: polishingEnabled }"
        :aria-pressed="polishingEnabled"
        @click="togglePolishing(!polishingEnabled)"
        type="button"
      >
        <span class="toggle-knob"></span>
      </button>
    </div>

    <!-- Config (shown when polishing ON) -->
    <div v-if="polishingEnabled" class="polish-config">
      <div class="field-row">
        <label>Provider</label>
        <AppSelect
          :model-value="polisher"
          :options="polisherOptions"
          @update:model-value="handlePolisherChange"
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
          <button class="btn-link" @click="goToPresets">manage</button>
        </div>
      </div>

      <!-- Cloud polisher hint -->
      <p v-if="polisher === 'openai' || polisher === 'mistral'" class="cloud-hint">
        Uses your {{ polisher === 'openai' ? 'OpenAI' : 'Mistral' }} API key from transcription settings.
      </p>
    </div>

    <!-- Ollama Config (shown when Ollama selected) -->
    <div v-if="polishingEnabled && polisher === 'ollama'" class="ollama-section">
      <p class="section-label">ollama</p>
      <OllamaConfig />
    </div>
  </div>
</template>

<style scoped>
.polishing-section {
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

/* Toggle Switch */
.toggle-switch {
  position: relative;
  width: 36px;
  height: 20px;
  background: var(--border);
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: background 0.15s ease;
  padding: 0;
}

.toggle-switch:hover {
  background: var(--text-weak);
}

.toggle-switch.active {
  background: var(--accent);
}

.toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  background: var(--bg);
  border-radius: 50%;
  transition: transform 0.15s ease;
}

.toggle-switch.active .toggle-knob {
  transform: translateX(16px);
}

/* Config section */
.polish-config {
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
