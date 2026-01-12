<!-- ToggleSwitch: On/off toggle switch. Props: modelValue (boolean), disabled (boolean) -->
<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue: boolean
  disabled?: boolean
}>(), {
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

function handleClick() {
  if (!props.disabled) {
    emit('update:modelValue', !props.modelValue)
  }
}
</script>

<template>
  <button
    class="toggle-switch"
    :class="{ active: modelValue, disabled }"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    type="button"
    @click="handleClick"
  >
    <span class="toggle-knob" />
  </button>
</template>

<style scoped>
.toggle-switch {
  position: relative;
  width: 28px;
  height: 16px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 3px;
  cursor: pointer;
  transition: background-color 150ms, border-color 150ms;
  padding: 0;
  flex-shrink: 0;
}

.toggle-switch:hover {
  border-color: var(--text-weak);
  background: var(--bg);
}

.toggle-switch.active {
  background: var(--accent);
  border-color: var(--accent);
}

.toggle-switch.active:hover {
  background: var(--accent);
  border-color: var(--accent);
}

.toggle-knob {
  position: absolute;
  top: 0;
  left: 0;
  width: 14px;
  height: 14px;
  background: var(--text-strong);
  border-radius: 2px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  transition: transform 150ms;
}

.toggle-switch.active .toggle-knob {
  transform: translateX(12px);
  background: var(--bg);
}

.toggle-switch.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle-switch.disabled:hover {
  border-color: var(--border);
  background: var(--bg-weak);
}
</style>
