<script setup lang="ts">
import type { CardOption } from './CardSelector.vue'
import CardSelector from './CardSelector.vue'

export type TranscriptionMode = 'cloud' | 'local'

defineProps<{
  modelValue: TranscriptionMode
}>()

const emit = defineEmits<{
  'update:modelValue': [value: TranscriptionMode]
}>()

const modeOptions: CardOption<TranscriptionMode>[] = [
  { value: 'cloud', title: 'Cloud', description: 'Online' },
  { value: 'local', title: 'Local', description: 'Offline' },
]
</script>

<template>
  <CardSelector
    :model-value="modelValue"
    :options="modeOptions"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
