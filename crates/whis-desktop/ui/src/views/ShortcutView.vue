<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { relaunch } from '@tauri-apps/plugin-process'
import { computed, onMounted, ref, watch } from 'vue'
import DirectCaptureSetup from '../components/DirectCaptureSetup.vue'
import ManualShortcutSetup from '../components/ManualShortcutSetup.vue'

import PortalShortcutBind from '../components/PortalShortcutBind.vue'
// Components
import ShortcutInput from '../components/ShortcutInput.vue'
import ShortcutTabs from '../components/ShortcutTabs.vue'
import SystemShortcutConfig from '../components/SystemShortcutConfig.vue'
import { useKeyboardCapture } from '../composables/useKeyboardCapture'
import { settingsStore } from '../stores/settings'

// State
const status = ref('')
const needsRestart = ref(false)
const toggleCommand = ref('whis-desktop --toggle')
const activeTab = ref<'system' | 'direct'>('system')

// Use keyboard capture composable
const { capturedShortcut, setShortcut } = useKeyboardCapture(settingsStore.state.shortcuts.desktop_key)

// Computed properties from store
const backendInfo = computed(() => settingsStore.state.backendInfo ?? null)
const portalShortcut = computed(() =>
  backendInfo.value?.backend === 'PortalGlobalShortcuts' ? settingsStore.state.portalShortcut : null,
)
const portalBindError = computed(() =>
  backendInfo.value?.backend === 'PortalGlobalShortcuts' ? settingsStore.state.portalBindError : null,
)
const rdevGrabError = computed(() =>
  backendInfo.value?.backend === 'RdevGrab' ? settingsStore.state.rdevGrabError : null,
)
const systemShortcut = computed(() =>
  backendInfo.value?.backend === 'RdevGrab' ? settingsStore.state.systemShortcut : null,
)
const pathMismatch = computed(() =>
  backendInfo.value?.backend === 'RdevGrab' && settingsStore.state.shortcutPathMismatch != null,
)
const isInInputGroup = computed(() => settingsStore.state.isInInputGroup)
const currentShortcut = computed({
  get: () => capturedShortcut.value,
  set: (val: string) => setShortcut(val),
})

// Show tabs only for RdevGrab backend (Linux Wayland, non-Flatpak)
const showTabs = computed(() => backendInfo.value?.backend === 'RdevGrab')

// Flatpak detection
const isFlatpak = computed(() => backendInfo.value?.is_flatpak ?? false)

// Direct capture is working if user is in input group and no error
const directCaptureWorking = computed(() => isInInputGroup.value && !rdevGrabError.value)

// Compositor checks
const isGnome = computed(() =>
  backendInfo.value?.compositor?.toLowerCase().includes('gnome') ?? false,
)
const isKde = computed(() => {
  const c = backendInfo.value?.compositor?.toLowerCase() ?? ''
  return c.includes('kde') || c.includes('plasma')
})
const isSway = computed(() =>
  backendInfo.value?.compositor?.toLowerCase().includes('sway') ?? false,
)
const isHyprland = computed(() =>
  backendInfo.value?.compositor?.toLowerCase().includes('hyprland') ?? false,
)
const hasSettingsApp = computed(() => isGnome.value || isKde.value)

// Environment description for Wayland users
const environmentHint = computed(() => {
  if (!showTabs.value)
    return null
  const compositor = backendInfo.value?.compositor ?? 'Unknown'
  const flatpakSuffix = isFlatpak.value ? ' (Flatpak)' : ''
  return `${compositor} on Wayland${flatpakSuffix}`
})

// Tab definitions
const tabs = computed(() => [
  { id: 'system', title: 'System Shortcut', description: 'Configure in settings' },
  { id: 'direct', title: 'Direct Capture', description: 'Built-in capture' },
])

onMounted(async () => {
  setShortcut(settingsStore.state.shortcuts.desktop_key)
  try {
    toggleCommand.value = await invoke<string>('get_toggle_command')
  }
  catch (e) {
    console.error('Failed to get toggle command:', e)
  }
})

// Set default tab based on whether direct capture is working
watch([backendInfo, directCaptureWorking], () => {
  if (backendInfo.value?.backend === 'RdevGrab') {
    activeTab.value = directCaptureWorking.value ? 'direct' : 'system'
  }
}, { immediate: true })

async function saveShortcut() {
  try {
    settingsStore.setDesktopKey(currentShortcut.value)
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
  if (currentShortcut.value === 'Press keys...' || !currentShortcut.value) {
    status.value = 'Press a key combination first'
    return
  }
  try {
    status.value = 'Configuring...'
    const newBinding = await invoke<string | null>('configure_shortcut_with_trigger', {
      trigger: currentShortcut.value,
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

async function restartApp() {
  await relaunch()
}

async function openKeyboardSettings() {
  try {
    await invoke('open_keyboard_settings', {
      compositor: backendInfo.value?.compositor ?? '',
    })
  }
  catch (e) {
    console.error('Failed to open settings:', e)
    status.value = `Failed to open settings: ${e}`
  }
}
</script>

<template>
  <section class="section">
    <header class="section-header">
      <h1>Global Shortcut</h1>
      <p>Toggle recording from anywhere</p>
    </header>

    <div class="section-content">
      <!-- RdevGrab backend (Linux Wayland) - Tabbed Interface -->
      <template v-if="showTabs">
        <ShortcutTabs v-model="activeTab" :tabs="tabs">
          <template #system>
            <SystemShortcutConfig
              :system-shortcut="systemShortcut"
              :has-settings-app="hasSettingsApp"
              :is-sway="isSway"
              :is-hyprland="isHyprland"
              :toggle-command="toggleCommand"
              :current-shortcut="currentShortcut"
              :compositor="backendInfo?.compositor"
              :is-flatpak="isFlatpak"
              :environment-hint="environmentHint"
              :path-mismatch="pathMismatch"
              @open-settings="openKeyboardSettings"
            />
          </template>

          <template #direct>
            <template v-if="directCaptureWorking">
              <div class="field">
                <label>press to record</label>
                <ShortcutInput v-model="currentShortcut" />
              </div>

              <button class="btn btn-secondary" @click="saveShortcut">
                Save
              </button>
            </template>

            <template v-else>
              <DirectCaptureSetup :environment-hint="environmentHint" />
            </template>
          </template>
        </ShortcutTabs>
      </template>

      <!-- Portal backend (Wayland) -->
      <template v-else-if="backendInfo?.backend === 'PortalGlobalShortcuts'">
        <PortalShortcutBind
          :portal-shortcut="portalShortcut"
          :portal-bind-error="portalBindError"
          :current-shortcut="currentShortcut"
          @update:current-shortcut="currentShortcut = $event"
          @configure="configureWithCapturedKey"
          @reset="resetAndRestart"
        />
      </template>

      <!-- Manual Setup (Wayland without portal support) -->
      <template v-else-if="backendInfo?.backend === 'ManualSetup'">
        <ManualShortcutSetup
          :compositor="backendInfo.compositor ?? 'Unknown'"
          :toggle-command="toggleCommand"
          :current-shortcut="currentShortcut"
        />
      </template>

      <!-- Tauri plugin (X11/macOS/Windows) - Simple recorder -->
      <template v-else>
        <div class="field">
          <label>press to record</label>
          <ShortcutInput v-model="currentShortcut" />
        </div>

        <button class="btn btn-secondary" @click="saveShortcut">
          Save
        </button>
      </template>

      <!-- Restart banner (shown for any backend when shortcut change requires restart) -->
      <div v-if="needsRestart" class="restart-banner">
        <span>[*] Restart required</span>
        <button class="btn-link" @click="restartApp">
          Restart now
        </button>
      </div>

      <!-- Status -->
      <div class="status" :class="{ visible: status }">
        {{ status }}
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Field */
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field label {
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
</style>
