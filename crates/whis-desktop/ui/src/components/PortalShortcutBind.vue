<script setup lang="ts">
import { computed } from 'vue'
import { displayKey, parsePortalShortcut } from '../utils/keys.js'
import ShortcutInput from './ShortcutInput.vue'

const props = defineProps<{
  portalShortcut?: string | null
  portalBindError?: string | null
  currentShortcut: string
}>()

const emit = defineEmits<{
  'update:currentShortcut': [value: string]
  'configure': []
  'reset': []
}>()

const shortcutKeys = computed(() => {
  if (props.portalShortcut) {
    return parsePortalShortcut(props.portalShortcut)
  }
  if (props.currentShortcut === 'Press keys...') {
    return ['...']
  }
  return props.currentShortcut.split('+').map(displayKey)
})

const canApply = computed(() =>
  props.currentShortcut !== 'Press keys...',
)
</script>

<template>
  <div class="portal-shortcut-bind">
    <!-- Warning if binding failed -->
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
        <ShortcutInput
          :model-value="currentShortcut"
          @update:model-value="emit('update:currentShortcut', $event)"
        />
      </div>

      <button class="btn btn-secondary" :disabled="!canApply" @click="emit('configure')">
        Apply
      </button>
    </template>

    <!-- Already bound: show current and reset option -->
    <template v-else-if="portalShortcut">
      <div class="shortcut-display">
        <div class="keys">
          <span v-for="(key, index) in shortcutKeys" :key="index" class="key">
            {{ key }}
          </span>
        </div>
      </div>

      <p class="hint">
        To change, reset the binding first.
      </p>
      <button class="btn btn-secondary" @click="emit('reset')">
        Reset & Restart
      </button>
    </template>
  </div>
</template>

<style scoped>
.portal-shortcut-bind {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.notice.warning {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--warning, #f59e0b);
  border-radius: 4px;
}

.notice.warning .notice-marker {
  color: var(--warning, #f59e0b);
}

.notice p {
  margin: 0 0 8px;
  font-size: 12px;
  color: var(--text);
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

.steps code {
  background: var(--bg-weak);
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 11px;
}

.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-weak);
}

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

.shortcut-display {
  display: flex;
  align-items: center;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
}
</style>
