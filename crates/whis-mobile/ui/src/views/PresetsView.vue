<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { presetsStore } from '../stores/presets'

const expandedPreset = ref<string | null>(null)

const presets = computed(() => presetsStore.state.presets)
const loading = computed(() => presetsStore.state.loading)
const selectedPreset = computed(() => presetsStore.state.selectedPreset)
const loadingDetails = computed(() => presetsStore.state.loadingDetails)

function togglePreset(name: string) {
  if (expandedPreset.value === name) {
    expandedPreset.value = null
    presetsStore.clearSelectedPreset()
  }
  else {
    expandedPreset.value = name
    presetsStore.loadPresetDetails(name)
  }
}

async function activatePreset(name: string) {
  await presetsStore.setActivePreset(name)
}

async function deactivatePreset() {
  await presetsStore.setActivePreset(null)
}

onMounted(() => {
  presetsStore.loadPresets()
})
</script>

<template>
  <div class="presets-view">
    <main class="presets-content">
      <!-- Loading State -->
      <div v-if="loading" class="loading">
        <div class="spinner" />
        <span>Loading presets...</span>
      </div>

      <!-- Presets List -->
      <div v-else class="presets-list">
        <div
          v-for="preset in presets"
          :key="preset.name"
          class="preset-card"
          :class="{
            active: preset.is_active,
            expanded: expandedPreset === preset.name,
          }"
        >
          <!-- Preset Header (always visible) -->
          <button
            class="preset-header"
            @click="togglePreset(preset.name)"
          >
            <div class="preset-info">
              <div class="preset-name">
                <span class="name-text">{{ preset.name }}</span>
                <span v-if="preset.is_builtin" class="builtin-badge">[built-in]</span>
                <span v-if="preset.is_active" class="active-badge">[active]</span>
              </div>
              <p class="preset-description">
                {{ preset.description }}
              </p>
            </div>
            <span class="expand-icon">{{ expandedPreset === preset.name ? '[-]' : '[+]' }}</span>
          </button>

          <!-- Expanded Details -->
          <div v-if="expandedPreset === preset.name" class="preset-details">
            <div v-if="loadingDetails" class="details-loading">
              <div class="spinner-small" />
            </div>
            <template v-else-if="selectedPreset">
              <!-- Prompt Preview -->
              <div class="prompt-section">
                <label>System Prompt</label>
                <div class="prompt-preview">
                  {{ selectedPreset.prompt }}
                </div>
              </div>

              <!-- Actions -->
              <div class="preset-actions">
                <button
                  v-if="!preset.is_active"
                  class="btn btn-primary"
                  @click="activatePreset(preset.name)"
                >
                  Activate Preset
                </button>
                <button
                  v-else
                  class="btn btn-secondary"
                  @click="deactivatePreset"
                >
                  Deactivate
                </button>
              </div>
            </template>
          </div>
        </div>
      </div>

      <!-- Empty State -->
      <div v-if="!loading && presets.length === 0" class="empty-state">
        <p>No presets available.</p>
      </div>
    </main>
  </div>
</template>

<style scoped>
.presets-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.presets-content {
  flex: 1;
  padding: 20px;
  padding-bottom: max(20px, env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

/* Loading */
.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 40px;
  color: var(--text-weak);
}

.spinner {
  width: 24px;
  height: 24px;
  border: 3px solid var(--bg-weak);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

.spinner-small {
  width: 16px;
  height: 16px;
  border: 2px solid var(--bg-weak);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

/* Presets List */
.presets-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* Preset Card */
.preset-card {
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  transition: border-color 0.15s ease;
}

.preset-card.active {
  border-color: var(--accent);
}

.preset-card.expanded {
  background: var(--bg-hover);
}

/* Preset Header */
.preset-header {
  all: unset;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 16px;
  cursor: pointer;
  min-height: var(--touch-target-min);
  box-sizing: border-box;
}

.preset-header:active {
  background: var(--bg-hover);
}

.preset-info {
  flex: 1;
  min-width: 0;
}

.preset-name {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.name-text {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-strong);
}

.builtin-badge {
  font-size: 11px;
  color: var(--text-weak);
}

.active-badge {
  font-size: 11px;
  color: var(--accent);
  font-weight: 500;
}

.preset-description {
  font-size: 13px;
  color: var(--text-weak);
  line-height: 1.4;
}

.expand-icon {
  flex-shrink: 0;
  font-size: 14px;
  color: var(--icon);
}

/* Preset Details */
.preset-details {
  padding: 0 16px 16px;
  border-top: 1px solid var(--border);
  margin-top: 0;
}

.details-loading {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}

.prompt-section {
  margin-top: 16px;
}

.prompt-section label {
  display: block;
  font-size: 12px;
  text-transform: lowercase;
  color: var(--text-weak);
  margin-bottom: 8px;
}

.prompt-preview {
  background: var(--bg);
  border: 1px solid var(--border-weak);
  border-radius: var(--radius);
  padding: 12px;
  font-size: 13px;
  color: var(--text);
  line-height: 1.5;
  max-height: 200px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

/* Actions */
.preset-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}

.preset-actions .btn {
  flex: 1;
}

/* Empty State */
.empty-state {
  text-align: center;
  padding: 40px;
  color: var(--text-weak);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
