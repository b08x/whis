<script setup lang="ts">
import { computed } from 'vue'
import { displayKey } from '../utils/keys.js'
import CommandBlock from './CommandBlock.vue'

const props = defineProps<{
  systemShortcut?: string | null
  hasSettingsApp: boolean
  isSway: boolean
  isHyprland: boolean
  toggleCommand: string
  currentShortcut: string
  compositor?: string
  isFlatpak: boolean
  environmentHint?: string | null
}>()

const emit = defineEmits<{
  openSettings: []
}>()

const displayedKeys = computed(() => {
  if (!props.systemShortcut)
    return []
  return props.systemShortcut.split('+').map(displayKey)
})

const configSnippet = computed(() => {
  if (props.isSway) {
    return `bindsym ${props.currentShortcut.toLowerCase()} exec ${props.toggleCommand}`
  }
  if (props.isHyprland) {
    return `bind = ${props.currentShortcut.replace(/\+/g, ', ')}, exec, ${props.toggleCommand}`
  }
  return null
})

const configPath = computed(() => {
  if (props.isSway)
    return '~/.config/sway/config'
  if (props.isHyprland)
    return '~/.config/hypr/hyprland.conf'
  return null
})

const reloadCommand = computed(() => {
  if (props.isSway)
    return 'swaymsg reload'
  if (props.isHyprland)
    return 'hyprctl reload'
  return null
})
</script>

<template>
  <div class="system-shortcut-config">
    <!-- Detected shortcut (when configured in GNOME) -->
    <template v-if="systemShortcut">
      <div class="shortcut-display">
        <div class="keys">
          <span v-for="(key, index) in displayedKeys" :key="index" class="key">
            {{ key }}
          </span>
        </div>
      </div>

      <p class="hint">
        Detected from GNOME custom shortcuts.
      </p>

      <button v-if="hasSettingsApp" class="btn btn-secondary" @click="emit('openSettings')">
        Open Keyboard Settings
      </button>
    </template>

    <!-- Not yet configured: show instructions -->
    <template v-else>
      <p class="tab-intro">
        Configure {{ compositor }} to trigger Whis.
      </p>

      <!-- GNOME / KDE: Settings app button -->
      <template v-if="hasSettingsApp">
        <button class="btn btn-secondary" @click="emit('openSettings')">
          Open Keyboard Settings
        </button>

        <p class="hint">
          Add a custom shortcut with this command:
        </p>

        <CommandBlock :command="toggleCommand" />
      </template>

      <!-- Sway / Hyprland: Config file -->
      <template v-else-if="isSway || isHyprland">
        <p class="hint">
          Add to your config file:
        </p>
        <p class="config-path">
          {{ configPath }}
        </p>

        <CommandBlock v-if="configSnippet" :command="configSnippet" />

        <p v-if="reloadCommand" class="hint reload-hint">
          Then reload: <code>{{ reloadCommand }}</code>
        </p>
      </template>

      <!-- Generic wlroots -->
      <template v-else>
        <p class="hint">
          Configure your compositor to run this command:
        </p>
        <CommandBlock :command="toggleCommand" />
      </template>
    </template>

    <!-- Environment detection hint -->
    <p v-if="environmentHint" class="env-hint">
      <span class="hint-marker">[i]</span>
      Detected: {{ environmentHint }}
    </p>
  </div>
</template>

<style scoped>
.system-shortcut-config {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.shortcut-display {
  display: flex;
  align-items: center;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
}

.tab-intro {
  font-size: 13px;
  color: var(--text);
  margin: 0;
}

.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-weak);
}

.config-path {
  margin: 0;
  padding: 6px 10px;
  background: var(--bg-weak);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 11px;
  color: var(--text-weak);
}

.reload-hint {
  font-size: 11px;
  color: var(--text-weak);
}

.reload-hint code {
  background: var(--bg-weak);
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 10px;
}

.env-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
  font-size: 10px;
  color: var(--text-weak);
}

.hint-marker {
  color: var(--text-weak);
  opacity: 0.7;
}
</style>
