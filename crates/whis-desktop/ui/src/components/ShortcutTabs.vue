<script setup lang="ts">
export interface Tab {
  id: string
  title: string
  description: string
  badge?: string
}

defineProps<{
  tabs: Tab[]
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <div class="tabs-container">
    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="tab"
        :class="{ active: modelValue === tab.id }"
        @click="emit('update:modelValue', tab.id)"
      >
        <span class="tab-title">{{ tab.title }}</span>
        <span class="tab-desc">{{ tab.description }}</span>
        <span v-if="tab.badge" class="tab-badge">{{ tab.badge }}</span>
      </button>
    </div>

    <div class="tab-content">
      <slot :name="modelValue" />
    </div>
  </div>
</template>

<style scoped>
.tabs {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 16px;
}

.tab {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 12px 10px;
  background: var(--bg-weak);
  border: 2px solid var(--border);
  border-radius: 6px;
  font-family: var(--font);
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: center;
}

.tab-title {
  font-size: 13px;
  font-weight: 300;
  color: var(--text);
}

.tab-desc {
  font-size: 10px;
  color: var(--text-weak);
}

.tab:hover {
  border-color: var(--text-weak);
}

.tab:focus-visible {
  outline: none;
  border-color: var(--accent);
}

.tab.active {
  border-color: var(--accent);
  background: rgba(255, 213, 79, 0.08);
}

.tab.active .tab-title {
  font-weight: 500;
  color: var(--accent);
}

.tab-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  background: hsla(120, 60%, 45%, 0.2);
  border-radius: 50%;
  font-size: 9px;
  color: hsl(120, 60%, 55%);
}

.tab-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
</style>
