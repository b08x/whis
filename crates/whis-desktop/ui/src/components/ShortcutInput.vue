<script setup lang="ts">
import { useKeyboardCapture } from '../composables/useKeyboardCapture'

const props = defineProps<{
  modelValue: string
  disabled?: boolean
  readonly?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const { isRecording, shortcutKeys, handleKeyDown, startRecording, stopRecording, setShortcut }
  = useKeyboardCapture(props.modelValue)

// Sync external value changes
function syncFromProp() {
  if (props.modelValue !== shortcutKeys.value.join('+')) {
    setShortcut(props.modelValue)
  }
}

// Watch for recording changes to emit
function onKeyDown(e: KeyboardEvent) {
  handleKeyDown(e)
  // After handling, emit the new value
  const current = shortcutKeys.value.join('+')
  if (current !== '...' && current !== props.modelValue) {
    emit('update:modelValue', current)
  }
}

function onStartRecording() {
  if (props.disabled || props.readonly)
    return
  startRecording()
}

function onStopRecording() {
  stopRecording()
  // Emit final value
  const current = shortcutKeys.value.join('+')
  if (current !== '...' && current !== props.modelValue) {
    emit('update:modelValue', current)
  }
}

// Initialize with prop value
syncFromProp()
</script>

<template>
  <div
    class="shortcut-input"
    :class="{ recording: isRecording, disabled, readonly }"
    :tabindex="disabled || readonly ? -1 : 0"
    @click="onStartRecording"
    @blur="onStopRecording"
    @keydown="onKeyDown"
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
</template>

<style scoped>
.key.placeholder {
  color: var(--text-weak);
}

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

.shortcut-input:hover:not(.disabled):not(.readonly) {
  border-color: var(--text-weak);
}

.shortcut-input:focus {
  outline: none;
  border-color: var(--accent);
}

.shortcut-input.recording {
  border-color: var(--recording);
}

.shortcut-input.disabled,
.shortcut-input.readonly {
  cursor: default;
  opacity: 0.7;
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
</style>
