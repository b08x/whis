<script setup lang="ts">
import { computed } from 'vue'
import CommandBlock from './CommandBlock.vue'

const props = defineProps<{
  compositor: string
  toggleCommand: string
  currentShortcut: string
}>()

const compositorLower = computed(() => props.compositor.toLowerCase())
const isGnome = computed(() => compositorLower.value.includes('gnome'))
const isKde = computed(() =>
  compositorLower.value.includes('kde') || compositorLower.value.includes('plasma'),
)
const isSway = computed(() => compositorLower.value.includes('sway'))
const isHyprland = computed(() => compositorLower.value.includes('hyprland'))

const swayConfig = computed(() =>
  `bindsym ${props.currentShortcut.toLowerCase()} exec ${props.toggleCommand}`,
)

const hyprlandConfig = computed(() =>
  `bind = ${props.currentShortcut.replace(/\+/g, ', ')}, exec, ${props.toggleCommand}`,
)
</script>

<template>
  <div class="manual-shortcut-setup">
    <div class="notice warning">
      <span class="notice-marker">[!]</span>
      <p>Global shortcuts require manual configuration on {{ compositor }}.</p>
    </div>

    <div class="instructions">
      <label>setup instructions</label>

      <!-- GNOME -->
      <template v-if="isGnome">
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
      <template v-else-if="isKde">
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
      <template v-else-if="isSway">
        <p class="hint">
          Add to <code>~/.config/sway/config</code>:
        </p>
        <CommandBlock :command="swayConfig" />
      </template>

      <!-- Hyprland -->
      <template v-else-if="isHyprland">
        <p class="hint">
          Add to <code>~/.config/hypr/hyprland.conf</code>:
        </p>
        <CommandBlock :command="hyprlandConfig" />
      </template>

      <!-- Generic -->
      <template v-else>
        <p class="hint">
          Configure your compositor to run:
        </p>
        <CommandBlock :command="toggleCommand" />
      </template>
    </div>
  </div>
</template>

<style scoped>
.manual-shortcut-setup {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.notice.warning {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--warning, #f59e0b);
  border-radius: 4px;
}

.notice.warning .notice-marker {
  color: var(--warning, #f59e0b);
}

.notice p {
  margin: 0;
  font-size: 12px;
  color: var(--text);
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

.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-weak);
}

.hint code {
  background: var(--bg-weak);
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 11px;
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
</style>
