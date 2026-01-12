<script setup lang="ts">
import type { PresetDetails, PresetInfo, SelectOption } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import AppSelect from '../components/AppSelect.vue'
import PostProcessingToggle from '../components/settings/PostProcessingToggle.vue'
import { settingsStore } from '../stores/settings'
import { POST_PROCESSOR_OPTIONS } from '../utils/constants'

// List state
const presets = ref<PresetInfo[]>([])
const loading = ref(true)
const applyingPreset = ref<string | null>(null)

// Active preset from store (single source of truth)
const activePreset = computed(() => settingsStore.state.ui.active_preset)

// Panel state
const panelOpen = ref(false)
const panelMode = ref<'view' | 'edit' | 'create'>('view')
const selectedPreset = ref<PresetDetails | null>(null)
const loadingDetails = ref(false)

// Edit form state
const editName = ref('')
const editDescription = ref('')
const editPrompt = ref('')
const editPostProcessor = ref<string | null>(null)
const editModel = ref<string | null>(null)
const saving = ref(false)
const error = ref<string | null>(null)

// Delete confirmation
const confirmingDelete = ref(false)
const deleting = ref(false)

// Preset-specific processor options (extends base options with auto/none)
const presetProcessorOptions: SelectOption[] = [
  { value: null, label: 'Automatic (use settings)' },
  { value: 'none', label: 'Disabled (raw transcript)' },
  ...POST_PROCESSOR_OPTIONS,
]

// Computed
const isEditing = computed(() => panelMode.value === 'edit' || panelMode.value === 'create')
const canEdit = computed(() => selectedPreset.value && !selectedPreset.value.is_builtin)

// Load presets list
async function loadPresets() {
  try {
    presets.value = await invoke<PresetInfo[]>('list_presets')
    // activePreset is now a computed from settingsStore, no need to load separately
  }
  catch (e) {
    console.error('Failed to load presets:', e)
  }
  finally {
    loading.value = false
  }
}

// Open panel with preset details
async function openPreset(name: string) {
  loadingDetails.value = true
  panelOpen.value = true
  panelMode.value = 'view'
  error.value = null
  confirmingDelete.value = false

  try {
    selectedPreset.value = await invoke<PresetDetails>('get_preset_details', { name })
  }
  catch (e) {
    console.error('Failed to load preset details:', e)
    error.value = String(e)
  }
  finally {
    loadingDetails.value = false
  }
}

// Open panel for creating new preset
function openCreate() {
  selectedPreset.value = null
  panelOpen.value = true
  panelMode.value = 'create'
  error.value = null
  confirmingDelete.value = false

  // Reset form
  editName.value = ''
  editDescription.value = ''
  editPrompt.value = ''
  editPostProcessor.value = null
  editModel.value = null
}

// Close panel
function closePanel() {
  panelOpen.value = false
  confirmingDelete.value = false
}

// Toggle panel (for header button)
function togglePanel() {
  if (panelOpen.value) {
    closePanel()
  }
  else {
    openCreate()
  }
}

// Start editing
function startEdit() {
  if (!selectedPreset.value)
    return

  panelMode.value = 'edit'
  editName.value = selectedPreset.value.name
  editDescription.value = selectedPreset.value.description
  editPrompt.value = selectedPreset.value.prompt
  editPostProcessor.value = selectedPreset.value.post_processor
  editModel.value = selectedPreset.value.model
  error.value = null
}

// Cancel editing
function cancelEdit() {
  if (panelMode.value === 'create') {
    closePanel()
  }
  else {
    panelMode.value = 'view'
    error.value = null
  }
}

// Save preset (create or update)
async function savePreset() {
  saving.value = true
  error.value = null

  try {
    if (panelMode.value === 'create') {
      await invoke('create_preset', {
        input: {
          name: editName.value.trim(),
          description: editDescription.value.trim(),
          prompt: editPrompt.value,
          post_processor: editPostProcessor.value || null,
          model: editModel.value?.trim() || null,
        },
      })

      // Reload list and open the new preset
      await loadPresets()
      await openPreset(editName.value.trim())
    }
    else {
      await invoke('update_preset', {
        name: selectedPreset.value!.name,
        input: {
          description: editDescription.value.trim(),
          prompt: editPrompt.value,
          post_processor: editPostProcessor.value || null,
          model: editModel.value?.trim() || null,
        },
      })

      // Reload list and refresh details
      await loadPresets()
      await openPreset(selectedPreset.value!.name)
    }

    await settingsStore.load()
  }
  catch (e) {
    console.error('Failed to save preset:', e)
    error.value = String(e)
  }
  finally {
    saving.value = false
  }
}

// Apply preset (makes it active, applies settings, and enables post-processing)
async function applyPreset(name: string) {
  applyingPreset.value = name
  try {
    await settingsStore.applyPreset(name)
    // activePreset is a computed from settingsStore, automatically updated
  }
  catch (e) {
    console.error('Failed to apply preset:', e)
    error.value = String(e)
  }
  finally {
    applyingPreset.value = null
  }
}

// Clear active preset (also disables post-processing)
async function clearPreset() {
  try {
    await settingsStore.clearPreset()
    // activePreset is a computed from settingsStore, automatically updated
  }
  catch (e) {
    console.error('Failed to clear preset:', e)
  }
}

// Delete preset
async function deletePreset() {
  if (!selectedPreset.value)
    return

  deleting.value = true
  error.value = null

  try {
    await invoke('delete_preset', { name: selectedPreset.value.name })
    await loadPresets()
    closePanel()
    await settingsStore.load()
  }
  catch (e) {
    console.error('Failed to delete preset:', e)
    error.value = String(e)
  }
  finally {
    deleting.value = false
    confirmingDelete.value = false
  }
}

onMounted(loadPresets)
</script>

<template>
  <section class="section presets-section">
    <header class="section-header">
      <div class="header-title">
        <h1>Presets</h1>
        <p>One-click configurations for different use cases</p>
      </div>
      <button class="panel-toggle-btn" :aria-label="panelOpen ? 'Close panel' : 'New preset'" @click="togglePanel">
        {{ panelOpen ? '[x]' : '[+]' }}
      </button>
    </header>

    <!-- Post-processing toggle (synced with Settings page) -->
    <PostProcessingToggle class="presets-toggle" />

    <div class="presets-layout">
      <!-- Presets list -->
      <div class="presets-list-container">
        <!-- Loading state -->
        <div v-if="loading" class="loading">
          Loading presets...
        </div>

        <!-- Presets list -->
        <div v-else class="presets-list">
          <button
            v-for="preset in presets"
            :key="preset.name"
            class="preset-card"
            :class="{
              active: activePreset === preset.name,
              selected: selectedPreset?.name === preset.name && panelOpen,
            }"
            @click="openPreset(preset.name)"
          >
            <span class="preset-marker" aria-hidden="true">{{ activePreset === preset.name ? '[*]' : '   ' }}</span>
            <div class="preset-content">
              <span class="preset-name">
                {{ preset.name }}
                <span v-if="preset.is_builtin" class="builtin-badge">(built-in)</span>
              </span>
              <span class="preset-description">{{ preset.description }}</span>
            </div>
          </button>
        </div>

        <!-- Clear preset button -->
        <button
          v-if="activePreset && !loading"
          class="clear-btn"
          @click="clearPreset"
        >
          Clear active preset
        </button>
      </div>

      <!-- Sliding detail panel -->
      <div class="slide-panel" :class="{ open: panelOpen }">
        <div class="slide-panel-content">
          <!-- Panel header -->
          <div class="slide-panel-header">
            <h2 v-if="panelMode === 'create'">
              New Preset
            </h2>
            <h2 v-else-if="isEditing">
              Edit Preset
            </h2>
            <h2 v-else>
              {{ selectedPreset?.name }}
            </h2>
          </div>

          <!-- Loading state -->
          <div v-if="loadingDetails" class="panel-loading">
            Loading...
          </div>

          <!-- Error message -->
          <div v-if="error" class="panel-error">
            {{ error }}
          </div>

          <!-- View mode -->
          <template v-if="!isEditing && selectedPreset">
            <div class="panel-field">
              <label>Description</label>
              <p>{{ selectedPreset.description }}</p>
            </div>

            <div class="panel-field">
              <label>Prompt</label>
              <p class="prompt-text">
                {{ selectedPreset.prompt || '(empty)' }}
              </p>
            </div>

            <div v-if="selectedPreset.post_processor" class="panel-field">
              <label>Post-processor override</label>
              <p>{{ selectedPreset.post_processor }}</p>
            </div>

            <div v-if="selectedPreset.model" class="panel-field">
              <label>Model override</label>
              <p>{{ selectedPreset.model }}</p>
            </div>

            <!-- Actions -->
            <div class="panel-actions">
              <button
                class="btn-accent"
                :disabled="applyingPreset !== null"
                @click="applyPreset(selectedPreset.name)"
              >
                {{ applyingPreset === selectedPreset.name ? 'Applying...' : 'Apply' }}
              </button>

              <button
                v-if="canEdit"
                class="btn-secondary"
                @click="startEdit"
              >
                Edit
              </button>

              <template v-if="canEdit">
                <button
                  v-if="!confirmingDelete"
                  class="btn-danger"
                  @click="confirmingDelete = true"
                >
                  Delete
                </button>
                <div v-else class="delete-confirm">
                  <span>Delete?</span>
                  <button class="btn-danger btn-sm" :disabled="deleting" @click="deletePreset">
                    {{ deleting ? '...' : 'Yes' }}
                  </button>
                  <button class="btn-secondary btn-sm" @click="confirmingDelete = false">
                    No
                  </button>
                </div>
              </template>
            </div>
          </template>

          <!-- Edit/Create mode -->
          <template v-if="isEditing">
            <div class="panel-field">
              <label for="edit-name">Name</label>
              <input
                id="edit-name"
                v-model="editName"
                :disabled="panelMode === 'edit'"
                :class="{ disabled: panelMode === 'edit' }"
                placeholder="my-preset"
              >
            </div>

            <div class="panel-field">
              <label for="edit-description">Description</label>
              <input
                id="edit-description"
                v-model="editDescription"
                placeholder="Brief description of this preset"
              >
            </div>

            <div class="panel-field">
              <label for="edit-prompt">Prompt</label>
              <textarea
                id="edit-prompt"
                v-model="editPrompt"
                placeholder="System prompt for post-processing transcripts..."
                rows="6"
              />
            </div>

            <details class="advanced-section">
              <summary>Advanced options</summary>

              <div class="panel-field">
                <label>Post-processor override</label>
                <AppSelect
                  :model-value="editPostProcessor"
                  :options="presetProcessorOptions"
                  aria-label="Post-processor override"
                  @update:model-value="editPostProcessor = $event"
                />
              </div>

              <div class="panel-field">
                <label for="edit-model">Model override</label>
                <input
                  id="edit-model"
                  v-model="editModel"
                  placeholder="e.g., gpt-4o-mini"
                >
              </div>
            </details>

            <!-- Edit actions -->
            <div class="panel-actions">
              <button
                class="btn-accent"
                :disabled="saving || !editName.trim() || !editDescription.trim()"
                @click="savePreset"
              >
                {{ saving ? 'Saving...' : 'Save' }}
              </button>
              <button
                class="btn-secondary"
                :disabled="saving"
                @click="cancelEdit"
              >
                Cancel
              </button>
            </div>
          </template>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.presets-section {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.presets-toggle {
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  margin-bottom: 12px;
}

.presets-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
  position: relative;
}

.presets-list-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  padding-right: 16px;
}

/* Header with toggle button */
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.header-title {
  display: flex;
  flex-direction: column;
}

.panel-toggle-btn {
  background: none;
  border: none;
  font-family: var(--font);
  font-size: 14px;
  color: var(--text-weak);
  cursor: pointer;
  padding: 4px 8px;
  transition: color 0.15s ease;
}

.panel-toggle-btn:hover {
  color: var(--text);
}

.loading {
  color: var(--text-weak);
  font-size: 12px;
}

.presets-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.preset-card {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: left;
  font-family: var(--font);
}

.preset-card:hover {
  background: var(--bg-hover);
  border-color: var(--text-weak);
}

.preset-card:focus-visible {
  outline: none;
  border-color: var(--accent);
}

.preset-card.active {
  border-color: var(--accent);
}

.preset-card.selected {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.preset-marker {
  color: var(--accent);
  font-size: 12px;
  flex-shrink: 0;
  font-family: var(--font);
}

.preset-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.preset-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-strong);
}

.builtin-badge {
  font-weight: 400;
  font-size: 10px;
  color: var(--text-weak);
  margin-left: 6px;
}

.preset-description {
  font-size: 11px;
  color: var(--text-weak);
}

.clear-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 16px;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text-weak);
  cursor: pointer;
  transition: all 0.15s ease;
  align-self: flex-start;
}

.clear-btn:hover {
  background: var(--bg-weak);
  border-color: var(--text-weak);
  color: var(--text);
}

/* Panel state messages */
.panel-loading,
.panel-error {
  font-size: 12px;
}

.panel-loading {
  color: var(--text-weak);
}

.panel-error {
  color: var(--danger, #e74c3c);
  background: rgba(231, 76, 60, 0.1);
  padding: 8px 12px;
  border-radius: 4px;
}

/* Prompt text display */
.prompt-text {
  background: var(--bg-weak);
  padding: 8px;
  border-radius: 4px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 150px;
  overflow-y: auto;
}

/* Panel form inputs */
.panel-field input,
.panel-field textarea,
.panel-field select {
  padding: 8px 10px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
}

.panel-field input:focus,
.panel-field textarea:focus,
.panel-field select:focus {
  outline: none;
  border-color: var(--accent);
}

.panel-field input.disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.panel-field textarea {
  resize: vertical;
  min-height: 100px;
}

/* Advanced options */
.advanced-section {
  margin-top: 8px;
}

.advanced-section summary {
  font-size: 11px;
  color: var(--text-weak);
  cursor: pointer;
  padding: 4px 0;
}

.advanced-section summary:hover {
  color: var(--text);
}

.advanced-section[open] {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* Actions bar */
.panel-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 8px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

/* Delete confirmation */
.delete-confirm {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-weak);
}

/* Small button modifier */
.btn-sm {
  padding: 4px 10px;
  font-size: 11px;
}
</style>
