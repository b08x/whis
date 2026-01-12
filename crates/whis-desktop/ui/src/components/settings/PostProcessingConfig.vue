<!-- PostProcessingConfig: LLM post-processor settings (processor type, shared toggle) -->
<script setup lang="ts">
import type { PostProcessor } from '../../types'
import { computed } from 'vue'
import { settingsStore } from '../../stores/settings'
import { POST_PROCESSOR_OPTIONS } from '../../utils/constants'
import AppSelect from '../AppSelect.vue'
import OllamaConfig from './OllamaConfig.vue'
import PostProcessingToggle from './PostProcessingToggle.vue'

const postProcessingEnabled = computed(() => settingsStore.state.post_processing.enabled)
const postProcessor = computed(() => settingsStore.state.post_processing.processor)

function handlePostProcessorChange(value: string | null) {
  if (value)
    settingsStore.setPostProcessor(value as PostProcessor)
}
</script>

<template>
  <div class="post-processing-section">
    <!-- Shared toggle component (shows enabled + active preset) -->
    <PostProcessingToggle :show-manage-link="true" />

    <!-- Config (shown when post-processing ON) -->
    <div v-if="postProcessingEnabled" class="post-process-config">
      <div class="field-row">
        <label>LLM</label>
        <AppSelect
          :model-value="postProcessor"
          :options="POST_PROCESSOR_OPTIONS"
          @update:model-value="handlePostProcessorChange"
        />
      </div>

      <!-- Cloud post-processor hint -->
      <p v-if="postProcessor === 'openai' || postProcessor === 'mistral'" class="cloud-hint">
        Uses the same {{ postProcessor === 'openai' ? 'OpenAI' : 'Mistral' }} API key as transcription.
      </p>
    </div>

    <!-- Ollama Config (shown when Ollama selected) -->
    <OllamaConfig v-if="postProcessingEnabled && postProcessor === 'ollama'" />
  </div>
</template>

<style scoped>
.post-processing-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.post-process-config {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.cloud-hint {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0;
  padding-top: 4px;
  padding-left: calc(var(--field-label-width) + 12px);
}
</style>
