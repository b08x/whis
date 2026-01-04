<script setup lang="ts">
import type { Provider, SelectOption } from '../../types'
import { computed, ref } from 'vue'
import { useParakeetModel } from '../../composables/useParakeetModel'
import { useWhisperModel } from '../../composables/useWhisperModel'
import { settingsStore } from '../../stores/settings'
import AppSelect from '../AppSelect.vue'

const props = defineProps<{
  showConfigCard?: boolean
  provider?: Provider
}>()

const showPathInput = ref(false)

// Whisper model composable
const {
  availableModels: whisperModels,
  selectedModel: selectedWhisperModel,
  downloadingModel: downloadingWhisper,
  downloadStatus: whisperDownloadStatus,
  downloadProgress: whisperDownloadProgress,
  downloadProgressPercent: whisperProgressPercent,
  downloadProgressText: whisperProgressText,
  isSelectedModelInstalled: isWhisperInstalled,
  selectedModelSize: whisperModelSize,
  downloadModel: downloadWhisperModel,
  MODEL_SIZES,
} = useWhisperModel()

// Parakeet model composable
const {
  availableModels: parakeetModels,
  selectedModel: selectedParakeetModel,
  downloadingModel: downloadingParakeet,
  downloadStatus: parakeetDownloadStatus,
  downloadProgress: parakeetDownloadProgress,
  downloadProgressPercent: parakeetProgressPercent,
  downloadProgressText: parakeetProgressText,
  isSelectedModelInstalled: isParakeetInstalled,
  selectedModelSize: parakeetModelSize,
  downloadModel: downloadParakeetModel,
} = useParakeetModel()

const whisperModelPath = settingsStore.state.transcription.local_models.whisper_path
const parakeetModelPath = settingsStore.state.transcription.local_models.parakeet_path

// Local engine options
const localEngineOptions: SelectOption[] = [
  { value: 'local-parakeet', label: 'Parakeet V3 (Recommended)' },
  { value: 'local-whisper', label: 'Whisper' },
]

// Whether we're using Parakeet
const isParakeet = computed(() => props.provider === 'local-parakeet')

function handleEngineChange(value: string | null) {
  if (value === 'local-whisper' || value === 'local-parakeet') {
    settingsStore.setProvider(value as Provider)
  }
}

// Whisper model options
const whisperModelOptions = computed<SelectOption[]>(() =>
  whisperModels.value.map(model => ({
    value: model.name,
    label: `${model.name}${model.installed ? ' [installed]' : ''} - ${MODEL_SIZES[model.name]}`,
  })),
)

// Parakeet model options
const parakeetModelOptions = computed<SelectOption[]>(() =>
  parakeetModels.value.map(model => ({
    value: model.name,
    label: `${model.name}${model.installed ? ' [installed]' : ''} - ${model.size}`,
  })),
)

function handleWhisperModelChange(value: string | null) {
  if (value)
    selectedWhisperModel.value = value
}

function handleParakeetModelChange(value: string | null) {
  if (value)
    selectedParakeetModel.value = value
}
</script>

<template>
  <!-- Model Configuration Card -->
  <div v-if="showConfigCard" class="config-card">
    <!-- Engine selector -->
    <div class="engine-selector">
      <AppSelect
        :model-value="provider ?? 'local-parakeet'"
        :options="localEngineOptions"
        @update:model-value="handleEngineChange"
      />
    </div>

    <!-- Parakeet config with download -->
    <template v-if="isParakeet">
      <div class="model-selector">
        <AppSelect
          :model-value="selectedParakeetModel"
          :options="parakeetModelOptions"
          :disabled="downloadingParakeet"
          @update:model-value="handleParakeetModelChange"
        />
        <button
          class="btn-primary"
          :disabled="downloadingParakeet || isParakeetInstalled"
          @click="downloadParakeetModel"
        >
          {{ downloadingParakeet ? `${parakeetProgressPercent}%` : isParakeetInstalled ? 'Installed' : 'Download' }}
        </button>
      </div>
      <p v-if="parakeetDownloadProgress" class="hint">
        {{ parakeetProgressText }}
      </p>
      <p v-else-if="parakeetDownloadStatus" class="hint" :class="{ error: parakeetDownloadStatus.includes('failed'), success: parakeetDownloadStatus.includes('successfully') }">
        {{ parakeetDownloadStatus }}
      </p>
      <p v-else class="hint">
        {{ parakeetModelSize || 'Fast and accurate local transcription' }}
      </p>
      <button class="path-toggle" type="button" @click="showPathInput = !showPathInput">
        <span class="toggle-indicator">{{ showPathInput ? 'v' : '>' }}</span>
        <span>or specify path</span>
      </button>
      <input
        v-show="showPathInput"
        type="text"
        class="text-input"
        :value="parakeetModelPath || ''"
        placeholder="/path/to/parakeet-model-dir"
        spellcheck="false"
        aria-label="Custom Parakeet model path"
        @input="settingsStore.setParakeetModelPath(($event.target as HTMLInputElement).value || null)"
      >
    </template>

    <!-- Whisper config with download -->
    <template v-else>
      <div class="model-selector">
        <AppSelect
          :model-value="selectedWhisperModel"
          :options="whisperModelOptions"
          :disabled="downloadingWhisper"
          @update:model-value="handleWhisperModelChange"
        />
        <button
          class="btn-primary"
          :disabled="downloadingWhisper || isWhisperInstalled"
          @click="downloadWhisperModel"
        >
          {{ downloadingWhisper ? `${whisperProgressPercent}%` : isWhisperInstalled ? 'Installed' : 'Download' }}
        </button>
      </div>
      <p v-if="whisperDownloadProgress" class="hint">
        {{ whisperProgressText }}
      </p>
      <p v-else-if="whisperDownloadStatus" class="hint" :class="{ error: whisperDownloadStatus.includes('failed'), success: whisperDownloadStatus.includes('successfully') }">
        {{ whisperDownloadStatus }}
      </p>
      <p v-else class="hint">
        {{ selectedWhisperModel === 'small' ? 'Recommended for most users' : whisperModelSize }}
      </p>
      <button class="path-toggle" type="button" @click="showPathInput = !showPathInput">
        <span class="toggle-indicator">{{ showPathInput ? 'v' : '>' }}</span>
        <span>or specify path</span>
      </button>
      <input
        v-show="showPathInput"
        type="text"
        class="text-input"
        :value="whisperModelPath || ''"
        placeholder="/path/to/model.bin"
        spellcheck="false"
        aria-label="Custom Whisper model path"
        @input="settingsStore.setWhisperModelPath(($event.target as HTMLInputElement).value || null)"
      >
    </template>
  </div>
</template>

<style scoped>
.config-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.engine-selector {
  margin-bottom: 4px;
}

.model-selector {
  display: flex;
  gap: 8px;
}

.model-selector :deep(.custom-select) {
  flex: 1;
  min-width: 0;
}

.model-selector .btn-primary {
  flex-shrink: 0;
  min-width: 80px;
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

.hint {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0;
}

.hint.success {
  color: #4ade80;
}

.hint.error {
  color: #f87171;
}

.path-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  background: none;
  border: none;
  color: var(--text-weak);
  cursor: pointer;
  font-family: var(--font);
  font-size: 11px;
  margin-top: 4px;
}

.path-toggle:hover {
  color: var(--text);
}

.path-toggle .toggle-indicator {
  font-size: 10px;
  width: 10px;
}

.text-input {
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
  transition: border-color 0.15s ease;
  min-width: 0;
  width: 100%;
}

.text-input:focus {
  outline: none;
  border-color: var(--accent);
}

.text-input::placeholder {
  color: var(--text-weak);
}
</style>
