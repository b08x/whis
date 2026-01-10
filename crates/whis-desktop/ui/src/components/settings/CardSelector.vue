<script setup lang="ts" generic="T extends string">
export interface CardOption<T extends string = string> {
  value: T
  title: string
  description?: string
}

defineProps<{
  modelValue: T
  options: CardOption<T>[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: T]
}>()
</script>

<template>
  <div class="card-selector">
    <button
      v-for="option in options"
      :key="option.value"
      class="selector-card"
      :class="{ active: modelValue === option.value }"
      :aria-pressed="modelValue === option.value"
      @click="emit('update:modelValue', option.value)"
    >
      <div class="card-title">
        {{ option.title }}
      </div>
      <div v-if="option.description" class="card-desc">
        {{ option.description }}
      </div>
    </button>
  </div>
</template>

<style scoped>
.card-selector {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 16px;
}

.selector-card {
  padding: 12px 10px;
  background: var(--bg-weak);
  border: 2px solid var(--border);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: center;
  font-family: var(--font);
}

.selector-card:hover {
  border-color: var(--text-weak);
}

.selector-card:focus-visible {
  outline: none;
  border-color: var(--accent);
}

.selector-card.active {
  border-color: var(--accent);
  background: rgba(255, 213, 79, 0.08);
}

.card-title {
  font-size: 13px;
  font-weight: 300;
  color: var(--text);
  margin-bottom: 2px;
}

.selector-card.active .card-title {
  font-weight: 500;
  color: var(--accent);
}

.card-desc {
  font-size: 10px;
  color: var(--text-weak);
}
</style>
