<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { relaunch } from '@tauri-apps/plugin-process'
import { computed, onMounted, ref } from 'vue'
import { settingsStore } from '../stores/settings'

const isRecording = ref(false)
const status = ref('')
const needsRestart = ref(false)
const toggleCommand = ref('whis-desktop --toggle')
const localShortcut = ref(settingsStore.state.ui.shortcut)

// Computed properties from store
const backendInfo = computed(() => settingsStore.state.backendInfo ?? null)
const portalShortcut = computed(() =>
  backendInfo.value?.backend === 'PortalGlobalShortcuts' ? settingsStore.state.portalShortcut : null,
)
const portalBindError = computed(() =>
  backendInfo.value?.backend === 'PortalGlobalShortcuts' ? settingsStore.state.portalBindError : null,
)
const currentShortcut = computed(() => localShortcut.value)

// Platform detection for macOS-friendly key display
const isMac = navigator.platform.toUpperCase().includes('MAC')

function displayKey(key: string): string {
  if (!isMac)
    return key
  switch (key.toLowerCase()) {
    case 'ctrl': return 'Control'
    case 'alt': return 'Option'
    case 'super': return 'Cmd'
    default: return key
  }
}

onMounted(async () => {
  localShortcut.value = settingsStore.state.ui.shortcut
  try {
    toggleCommand.value = await invoke<string>('get_toggle_command')
  }
  catch (e) {
    console.error('Failed to get toggle command:', e)
  }
})

// Parse portal shortcut format like "Press <Control><Alt>l" or "<Shift><Control>r"
function parsePortalShortcut(portalStr: string): string[] {
  const cleaned = portalStr.replace(/^Press\s+/i, '')
  const keys: string[] = []
  const matches = cleaned.matchAll(/<(\w+)>/g)
  for (const match of matches) {
    const mod = (match[1] ?? '').toLowerCase()
    if (mod === 'control')
      keys.push(displayKey('Ctrl'))
    else if (mod === 'shift')
      keys.push('Shift')
    else if (mod === 'alt')
      keys.push(displayKey('Alt'))
    else if (mod === 'super')
      keys.push(displayKey('Super'))
    else if (mod)
      keys.push(mod.charAt(0).toUpperCase() + mod.slice(1))
  }
  const finalKey = cleaned.replace(/<\w+>/g, '').trim()
  if (finalKey) {
    keys.push(finalKey.toUpperCase())
  }
  return keys
}

// Split shortcut into individual keys for display
const shortcutKeys = computed(() => {
  if (backendInfo.value?.backend === 'PortalGlobalShortcuts' && portalShortcut.value) {
    return parsePortalShortcut(portalShortcut.value)
  }
  if (currentShortcut.value === 'Press keys...') {
    return ['...']
  }
  // Apply platform-aware display for each key
  return currentShortcut.value.split('+').map(displayKey)
})

async function resetAndRestart() {
  try {
    status.value = 'Resetting...'
    await invoke('reset_shortcut')
    await relaunch()
  }
  catch (e) {
    status.value = `Failed: ${e}`
  }
}

async function saveShortcut() {
  try {
    settingsStore.setShortcut(localShortcut.value)
    const restartNeeded = await settingsStore.save()
    if (restartNeeded) {
      needsRestart.value = true
      status.value = ''
    }
    else {
      status.value = 'Saved'
      setTimeout(() => status.value = '', 2000)
    }
  }
  catch (e) {
    status.value = `Failed to save: ${e}`
  }
}

async function configureWithCapturedKey() {
  if (localShortcut.value === 'Press keys...' || !localShortcut.value) {
    status.value = 'Press a key combination first'
    return
  }

  try {
    status.value = 'Configuring...'
    const newBinding = await invoke<string | null>('configure_shortcut_with_trigger', {
      trigger: localShortcut.value,
    })
    if (newBinding) {
      settingsStore.setPortalShortcut(newBinding)
      status.value = 'Configured!'
    }
    else {
      status.value = 'Cancelled'
    }
    setTimeout(() => status.value = '', 2000)
  }
  catch (e) {
    status.value = `Failed: ${e}`
  }
}

async function restartApp() {
  await relaunch()
}

function handleKeyDown(e: KeyboardEvent) {
  if (!isRecording.value)
    return
  e.preventDefault()

  const keys = []
  // Use platform-aware key names for display
  if (e.ctrlKey)
    keys.push(isMac ? 'Control' : 'Ctrl')
  if (e.shiftKey)
    keys.push('Shift')
  if (e.altKey)
    keys.push(isMac ? 'Option' : 'Alt')
  if (e.metaKey)
    keys.push(isMac ? 'Cmd' : 'Super')

  const key = e.key.toUpperCase()
  if (!['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) {
    keys.push(key)
  }

  if (keys.length > 0) {
    localShortcut.value = keys.join('+')
  }
}

function startRecording() {
  isRecording.value = true
  localShortcut.value = 'Press keys...'
}

function stopRecording() {
  isRecording.value = false
}
</script>

<template>
  <section class="section">
    <header class="section-header">
      <h1>Global Shortcut</h1>
      <p>Toggle recording from anywhere</p>
    </header>

    <div v-if="!settingsStore.state.loaded" class="section-content">
      <div class="loading-container">
        <div class="loading-message">
          Loading shortcut settings...
        </div>
      </div>
    </div>

    <div v-else class="section-content">
      <!-- Portal backend (Wayland) -->
      <template v-if="backendInfo?.backend === 'PortalGlobalShortcuts'">
        <!-- Warning if binding failed (e.g., launched from terminal) -->
        <div v-if="portalBindError" class="notice warning">
          <span class="notice-marker">[!]</span>
          <div>
            <p>Shortcut binding failed. For global shortcuts on Wayland:</p>
            <ol class="steps">
              <li>Run <code>./Whis.AppImage --install</code> in terminal</li>
              <li>Launch Whis from your app menu</li>
            </ol>
          </div>
        </div>

        <!-- Not yet bound: allow configuration -->
        <template v-if="!portalShortcut && !portalBindError">
          <p class="hint">
            Press keys below, then click Apply to bind the shortcut.
          </p>

          <div class="field">
            <label>press to record</label>
            <div
              class="shortcut-input"
              :class="{ recording: isRecording }"
              tabindex="0"
              @click="startRecording"
              @blur="stopRecording"
              @keydown="handleKeyDown"
            >
              <div class="keys">
                <span
                  v-for="(key, index) in shortcutKeys"
                  :key="index"
                  class="key"
                  :class="{ placeholder: key === '...' }"
                >{{ key }}</span>
              </div>
              <span v-show="isRecording" class="recording-dot" aria-hidden="true" />
            </div>
          </div>

          <button class="btn btn-secondary" :disabled="isRecording || currentShortcut === 'Press keys...'" @click="configureWithCapturedKey">
            Apply
          </button>
        </template>

        <!-- Already bound: show current and reset option -->
        <template v-else>
          <div class="shortcut-display">
            <div class="keys">
              <span
                v-for="(key, index) in shortcutKeys"
                :key="index"
                class="key"
              >{{ key }}</span>
            </div>
          </div>

          <p class="hint">
            To change, reset the binding first.
          </p>
          <button class="btn btn-secondary" @click="resetAndRestart">
            Reset & Restart
          </button>
        </template>
      </template>

      <!-- Manual Setup (Wayland without portal support) -->
      <template v-else-if="backendInfo?.backend === 'ManualSetup'">
        <div class="notice warning">
          <span class="notice-marker">[!]</span>
          <p>Global shortcuts require manual configuration on {{ backendInfo.compositor }}.</p>
        </div>

        <div class="instructions">
          <label>setup instructions</label>

          <!-- GNOME -->
          <template v-if="backendInfo.compositor.toLowerCase().includes('gnome')">
            <ol class="steps">
              <li>Open <strong>Settings</strong> → <strong>Keyboard</strong> → <strong>Custom Shortcuts</strong></li>
              <li>Add a new shortcut with these values:</li>
            </ol>
            <div class="command-block">
              <div class="command-row">
                <span class="command-label">Name:</span>
                <code>Whis Toggle Recording</code>
              </div>
              <div class="command-row">
                <span class="command-label">Command:</span>
                <code>{{ toggleCommand }}</code>
              </div>
              <div class="command-row">
                <span class="command-label">Shortcut:</span>
                <code>{{ currentShortcut }}</code>
              </div>
            </div>
          </template>

          <!-- KDE/Plasma -->
          <template v-else-if="backendInfo.compositor.toLowerCase().includes('kde') || backendInfo.compositor.toLowerCase().includes('plasma')">
            <ol class="steps">
              <li>Open <strong>System Settings</strong> → <strong>Shortcuts</strong> → <strong>Custom Shortcuts</strong></li>
              <li>Add a new shortcut:</li>
            </ol>
            <div class="command-block">
              <div class="command-row">
                <span class="command-label">Command:</span>
                <code>{{ toggleCommand }}</code>
              </div>
            </div>
          </template>

          <!-- Sway -->
          <template v-else-if="backendInfo.compositor.toLowerCase().includes('sway')">
            <p class="hint">
              Add to <code>~/.config/sway/config</code>:
            </p>
            <div class="command">
              <code>bindsym {{ currentShortcut.toLowerCase() }} exec {{ toggleCommand }}</code>
            </div>
          </template>

          <!-- Hyprland -->
          <template v-else-if="backendInfo.compositor.toLowerCase().includes('hyprland')">
            <p class="hint">
              Add to <code>~/.config/hypr/hyprland.conf</code>:
            </p>
            <div class="command">
              <code>bind = {{ currentShortcut.replace(/\+/g, ', ') }}, exec, {{ toggleCommand }}</code>
            </div>
          </template>

          <!-- Generic -->
          <template v-else>
            <p class="hint">
              Configure your compositor to run:
            </p>
            <div class="command">
              <code>{{ toggleCommand }}</code>
            </div>
          </template>
        </div>
      </template>

      <!-- Tauri plugin (X11/macOS/Windows) -->
      <template v-else>
        <div class="field">
          <label>press to record</label>
          <div
            class="shortcut-input"
            :class="{ recording: isRecording }"
            tabindex="0"
            @click="startRecording"
            @blur="stopRecording"
            @keydown="handleKeyDown"
          >
            <div class="keys">
              <span
                v-for="(key, index) in shortcutKeys"
                :key="index"
                class="key"
                :class="{ placeholder: key === '...' }"
              >{{ key }}</span>
            </div>
            <span v-show="isRecording" class="recording-dot" aria-hidden="true" />
          </div>
        </div>

        <button class="btn btn-secondary" :disabled="isRecording" @click="saveShortcut">
          Save
        </button>

        <!-- Restart banner -->
        <div v-if="needsRestart" class="restart-banner">
          <span>[*] Restart required</span>
          <button class="btn-link" @click="restartApp">
            Restart now
          </button>
        </div>
      </template>

      <!-- Status -->
      <div class="status" :class="{ visible: status }">
        {{ status }}
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Keys display */
.keys {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  height: 26px;
  padding: 0 8px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 11px;
  font-weight: 500;
  color: var(--accent);
}

.key.placeholder {
  color: var(--text-weak);
}

/* Shortcut input */
.shortcut-input {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.shortcut-input:hover {
  border-color: var(--text-weak);
}

.shortcut-input:focus {
  outline: none;
  border-color: var(--accent);
}

.shortcut-input.recording {
  border-color: var(--recording);
}

/* Read-only shortcut display */
.shortcut-display {
  display: flex;
  align-items: center;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
}

.recording-dot {
  width: 6px;
  height: 6px;
  background: var(--recording);
  border-radius: 50%;
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* Command block */
.command {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.command:hover {
  border-color: var(--text-weak);
}

.command.copied {
  border-color: var(--accent);
}

.command code {
  flex: 1;
  min-width: 0;
  font-family: var(--font);
  font-size: 11px;
  color: var(--text);
  word-break: break-all;
  line-height: 1.5;
}

.copy-btn {
  all: unset;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: var(--icon);
  cursor: pointer;
  transition: color 0.15s ease;
}

.copy-btn:hover {
  color: var(--text-strong);
}

.command.copied .copy-btn {
  color: var(--accent);
}

.copy-btn svg {
  width: 14px;
  height: 14px;
}

/* Reset info */
.reset-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.reset-info label {
  font-size: 11px;
  text-transform: lowercase;
  color: var(--text-weak);
}

/* Restart banner */
.restart-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 12px;
  color: var(--text);
}

/* Manual Setup instructions */
.notice.warning {
  border-color: var(--warning, #f59e0b);
}

.notice.warning .notice-marker {
  color: var(--warning, #f59e0b);
}

.instructions {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.instructions label {
  font-size: 11px;
  text-transform: lowercase;
  color: var(--text-weak);
}

.steps {
  margin: 0;
  padding-left: 20px;
  font-size: 12px;
  color: var(--text);
  line-height: 1.6;
}

.steps li {
  margin-bottom: 4px;
}

.steps strong {
  color: var(--text-strong);
}

.command-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
}

.command-row {
  display: flex;
  gap: 8px;
  align-items: baseline;
}

.command-label {
  font-size: 11px;
  color: var(--text-weak);
  min-width: 70px;
}

.command-block code {
  font-family: var(--font);
  font-size: 11px;
  color: var(--accent);
}

/* Loading state */
.loading-container {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 400px;
}

.loading-message {
  color: var(--text-weak);
  font-size: 14px;
}
</style>
