<!-- PostProcessingToggle: Reusable toggle for post-processing with preset display -->
<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { settingsStore } from '../../stores/settings'
import ToggleSwitch from './ToggleSwitch.vue'

interface Props {
  showManageLink?: boolean
}

defineProps<Props>()

const router = useRouter()

const postProcessingEnabled = computed(() => settingsStore.state.post_processing.enabled)
const activePreset = computed(() => settingsStore.state.ui.active_preset)

function togglePostProcessing(enable: boolean) {
  if (enable) {
    settingsStore.enablePostProcessing()
  }
  else {
    settingsStore.disablePostProcessing()
  }
}

function goToPresets() {
  router.push('/presets')
}
</script>

<template>
  <div class="post-processing-toggle">
    <div class="field-row">
      <label>Post-processing</label>
      <ToggleSwitch
        :model-value="postProcessingEnabled"
        @update:model-value="togglePostProcessing"
      />
    </div>

    <!-- Show active preset when enabled -->
    <div v-if="postProcessingEnabled && activePreset" class="active-preset-info">
      <span class="preset-label">Preset:</span>
      <span class="preset-name">{{ activePreset }}</span>
      <button v-if="showManageLink" class="btn-link btn-link--sm" @click="goToPresets">
        manage
      </button>
    </div>
  </div>
</template>

<style scoped>
.post-processing-toggle {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.active-preset-info {
  display: flex;
  align-items: center;
  gap: 6px;
  padding-left: calc(var(--field-label-width) + 12px);
  font-size: 11px;
}

.preset-label {
  color: var(--text-weak);
}

.preset-name {
  color: var(--text);
}

.btn-link--sm {
  font-size: 11px;
  color: var(--accent);
  text-decoration: none;
}

.btn-link--sm:hover {
  text-decoration: underline;
}
</style>
