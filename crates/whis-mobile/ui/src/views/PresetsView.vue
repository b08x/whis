<script setup lang="ts">
import type { CreatePresetInput, UpdatePresetInput } from '../types'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import AppInput from '../components/AppInput.vue'
import { headerStore } from '../stores/header'
import { presetsStore } from '../stores/presets'

// List state
const presets = computed(() => presetsStore.state.presets)
const loading = computed(() => presetsStore.state.loading)
const selectedPreset = computed(() => presetsStore.state.selectedPreset)
const loadingDetails = computed(() => presetsStore.state.loadingDetails)

// Panel state
const panelOpen = ref(false)
const panelMode = ref<'view' | 'edit' | 'create'>('view')

// Edit form state
const editName = ref('')
const editDescription = ref('')
const editPrompt = ref('')
const saving = ref(false)
const error = ref<string | null>(null)

// Delete confirmation
const confirmingDelete = ref(false)
const deleting = ref(false)

// Computed
const isEditing = computed(() => panelMode.value === 'edit' || panelMode.value === 'create')
const canEdit = computed(() => selectedPreset.value && !selectedPreset.value.is_builtin)

// Get current preset info for checking is_active
const currentPresetInfo = computed(() => {
  if (!selectedPreset.value)
    return null
  return presets.value.find(p => p.name === selectedPreset.value?.name)
})

// Open panel with preset details
async function openPreset(name: string) {
  panelOpen.value = true
  panelMode.value = 'view'
  error.value = null
  confirmingDelete.value = false
  await presetsStore.loadPresetDetails(name)
}

// Open panel for creating new preset
function openCreate() {
  presetsStore.clearSelectedPreset()
  panelOpen.value = true
  panelMode.value = 'create'
  error.value = null
  confirmingDelete.value = false

  // Reset form
  editName.value = ''
  editDescription.value = ''
  editPrompt.value = ''
}

// Close panel
function closePanel() {
  panelOpen.value = false
  confirmingDelete.value = false
}

// Start editing
function startEdit() {
  if (!selectedPreset.value)
    return

  panelMode.value = 'edit'
  editName.value = selectedPreset.value.name
  editDescription.value = selectedPreset.value.description
  editPrompt.value = selectedPreset.value.prompt
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
      const input: CreatePresetInput = {
        name: editName.value.trim(),
        description: editDescription.value.trim(),
        prompt: editPrompt.value,
      }
      await presetsStore.createPreset(input)

      // Open the new preset
      await openPreset(editName.value.trim())
    }
    else {
      const input: UpdatePresetInput = {
        description: editDescription.value.trim(),
        prompt: editPrompt.value,
      }
      await presetsStore.updatePreset(selectedPreset.value!.name, input)
      panelMode.value = 'view'
    }
  }
  catch (e) {
    console.error('Failed to save preset:', e)
    error.value = String(e)
  }
  finally {
    saving.value = false
  }
}

// Activate preset
async function activatePreset(name: string) {
  try {
    await presetsStore.setActivePreset(name)
  }
  catch (e) {
    console.error('Failed to activate preset:', e)
    error.value = String(e)
  }
}

// Deactivate preset
async function deactivatePreset() {
  try {
    await presetsStore.setActivePreset(null)
  }
  catch (e) {
    console.error('Failed to deactivate preset:', e)
    error.value = String(e)
  }
}

// Delete preset
async function deletePreset() {
  if (!selectedPreset.value)
    return

  deleting.value = true
  error.value = null

  try {
    await presetsStore.deletePreset(selectedPreset.value.name)
    closePanel()
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

// Update header action based on panel state
function updateHeaderAction() {
  if (panelOpen.value) {
    headerStore.setAction({
      label: '[x]',
      ariaLabel: 'Close panel',
      onClick: closePanel,
    })
  }
  else {
    headerStore.setAction({
      label: '[+]',
      ariaLabel: 'New preset',
      onClick: openCreate,
    })
  }
}

// Watch panel state and update header
watch(panelOpen, updateHeaderAction)

onMounted(() => {
  presetsStore.loadPresets()
  updateHeaderAction()
})

onUnmounted(() => {
  headerStore.clearAction()
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
        <button
          v-for="preset in presets"
          :key="preset.name"
          class="preset-card"
          :class="{
            active: preset.is_active,
            selected: selectedPreset?.name === preset.name && panelOpen,
          }"
          @click="openPreset(preset.name)"
        >
          <span class="preset-marker" aria-hidden="true">{{ preset.is_active ? '[*]' : '   ' }}</span>
          <div class="preset-content">
            <span class="preset-name">
              {{ preset.name }}
              <span v-if="preset.is_builtin" class="builtin-badge">(built-in)</span>
            </span>
            <span class="preset-description">{{ preset.description }}</span>
          </div>
        </button>
      </div>

      <!-- Empty State -->
      <div v-if="!loading && presets.length === 0" class="empty-state">
        <p>No presets available.</p>
        <button class="btn btn-primary" @click="openCreate">
          Create Preset
        </button>
      </div>
    </main>

    <!-- Panel Backdrop -->
    <div class="panel-backdrop" :class="{ open: panelOpen }" @click="closePanel" />

    <!-- Sliding Detail Panel -->
    <div class="detail-panel" :class="{ open: panelOpen }">
      <div class="panel-content">
        <!-- Panel Header -->
        <div class="panel-header">
          <h2 v-if="panelMode === 'create'">
            New Preset
          </h2>
          <h2 v-else-if="panelMode === 'edit'">
            Edit Preset
          </h2>
          <h2 v-else>
            {{ selectedPreset?.name }}
          </h2>
        </div>

        <!-- Loading state -->
        <div v-if="loadingDetails && panelMode === 'view'" class="panel-loading">
          <div class="spinner-small" />
        </div>

        <!-- Error message -->
        <div v-if="error" class="panel-error">
          {{ error }}
        </div>

        <!-- View mode -->
        <template v-if="!isEditing && selectedPreset">
          <div class="detail-field">
            <label>description</label>
            <p>{{ selectedPreset.description }}</p>
          </div>

          <div class="detail-field">
            <label>prompt</label>
            <p class="prompt-text">
              {{ selectedPreset.prompt || '(empty)' }}
            </p>
          </div>

          <!-- Actions -->
          <div class="panel-actions">
            <button
              v-if="!currentPresetInfo?.is_active"
              class="btn btn-primary"
              @click="activatePreset(selectedPreset.name)"
            >
              Activate
            </button>
            <button
              v-else
              class="btn btn-secondary"
              @click="deactivatePreset"
            >
              Deactivate
            </button>

            <button
              v-if="canEdit"
              class="btn btn-secondary"
              @click="startEdit"
            >
              Edit
            </button>

            <template v-if="canEdit">
              <button
                v-if="!confirmingDelete"
                class="btn btn-danger"
                @click="confirmingDelete = true"
              >
                Delete
              </button>
              <div v-else class="delete-confirm">
                <span>Delete?</span>
                <button class="btn-danger-sm" :disabled="deleting" @click="deletePreset">
                  {{ deleting ? '...' : 'Yes' }}
                </button>
                <button class="btn-secondary-sm" @click="confirmingDelete = false">
                  No
                </button>
              </div>
            </template>
          </div>
        </template>

        <!-- Edit/Create mode -->
        <template v-if="isEditing">
          <div class="edit-field">
            <label for="edit-name">name</label>
            <AppInput
              id="edit-name"
              v-model="editName"
              :disabled="panelMode === 'edit'"
              placeholder="my-preset"
            />
          </div>

          <div class="edit-field">
            <label for="edit-description">description</label>
            <AppInput
              id="edit-description"
              v-model="editDescription"
              placeholder="Brief description of this preset"
            />
          </div>

          <div class="edit-field">
            <label for="edit-prompt">prompt</label>
            <textarea
              id="edit-prompt"
              v-model="editPrompt"
              placeholder="System prompt for post-processing transcripts..."
              rows="6"
            />
          </div>

          <!-- Edit actions -->
          <div class="panel-actions">
            <button
              class="btn btn-primary"
              :disabled="saving || !editName.trim() || !editDescription.trim()"
              @click="savePreset"
            >
              {{ saving ? 'Saving...' : 'Save' }}
            </button>
            <button
              class="btn btn-secondary"
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
</template>

<style scoped>
.presets-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 100%;
  position: relative;
}

.presets-content {
  flex: 1;
  padding: 20px;
  padding-bottom: max(20px, env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 12px;
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
  gap: 8px;
}

.preset-card {
  all: unset;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 14px 16px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  cursor: pointer;
  transition: all 0.15s ease;
  min-height: var(--touch-target-min);
  box-sizing: border-box;
}

.preset-card:active {
  background: var(--bg-hover);
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
  font-size: 14px;
  font-weight: 600;
  color: var(--text-strong);
}

.builtin-badge {
  font-weight: 400;
  font-size: 11px;
  color: var(--text-weak);
  margin-left: 6px;
}

.preset-description {
  font-size: 12px;
  color: var(--text-weak);
  line-height: 1.4;
}

/* Empty State */
.empty-state {
  text-align: center;
  padding: 40px;
  color: var(--text-weak);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

/* Panel Backdrop */
.panel-backdrop {
  position: fixed;
  inset: 0;
  top: calc(48px + env(safe-area-inset-top));
  background: rgba(0, 0, 0, 0.5);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.25s ease;
  z-index: 49;
}

.panel-backdrop.open {
  opacity: 1;
  pointer-events: auto;
}

/* Detail Panel */
.detail-panel {
  position: fixed;
  top: calc(48px + env(safe-area-inset-top));
  right: 0;
  bottom: 0;
  width: 85%;
  max-width: 320px;
  background: var(--bg);
  border-left: 1px solid var(--border);
  transform: translateX(100%);
  transition: transform 0.25s ease;
  z-index: 50;
  overflow-y: auto;
}

.detail-panel.open {
  transform: translateX(0);
}

.panel-content {
  padding: 20px;
  padding-bottom: max(20px, env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.panel-header h2 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-strong);
  margin: 0;
}

.panel-loading {
  display: flex;
  justify-content: center;
  padding: 20px 0;
}

.panel-error {
  font-size: 13px;
  color: var(--recording);
  background: rgba(255, 68, 68, 0.1);
  padding: 10px 12px;
  border-radius: var(--radius);
}

/* Detail fields (view mode) */
.detail-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-field label {
  font-size: 12px;
  color: var(--text-weak);
  text-transform: lowercase;
}

.detail-field p {
  font-size: 14px;
  color: var(--text);
  margin: 0;
  line-height: 1.4;
}

.prompt-text {
  background: var(--bg-weak);
  padding: 12px;
  border-radius: var(--radius);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow-y: auto;
}

/* Edit form */
.edit-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.edit-field label {
  font-size: 12px;
  color: var(--text-weak);
  text-transform: lowercase;
}

.edit-field textarea {
  padding: 12px 14px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-family: var(--font);
  font-size: 14px;
  color: var(--text);
  resize: vertical;
  min-height: 120px;
}

.edit-field textarea:focus {
  outline: none;
  border-color: var(--accent);
}

/* Actions */
.panel-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 8px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.btn {
  padding: 12px 20px;
  border-radius: var(--radius);
  font-family: var(--font);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s ease;
  min-height: var(--touch-target-min);
}

.btn-primary {
  background: var(--accent);
  border: 1px solid var(--accent);
  color: var(--bg);
}

.btn-primary:active:not(:disabled) {
  opacity: 0.8;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text);
}

.btn-secondary:active:not(:disabled) {
  background: var(--bg-weak);
}

.btn-danger {
  background: transparent;
  border: 1px solid var(--recording);
  color: var(--recording);
}

.btn-danger:active:not(:disabled) {
  background: rgba(255, 68, 68, 0.1);
}

.delete-confirm {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  color: var(--text-weak);
}

.btn-danger-sm,
.btn-secondary-sm {
  padding: 8px 14px;
  border-radius: var(--radius);
  font-family: var(--font);
  font-size: 13px;
  cursor: pointer;
  min-height: var(--touch-target-min);
}

.btn-danger-sm {
  background: var(--recording);
  border: none;
  color: white;
}

.btn-secondary-sm {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
