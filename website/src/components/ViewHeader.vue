<script setup lang="ts">
defineProps<{
  title: string
  subtitle: string
}>()
</script>

<template>
  <header class="view-header">
    <h1>{{ title }}</h1>
    <p>{{ subtitle }}</p>
  </header>
</template>

<style scoped>
.view-header {
  margin-bottom: 2rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid var(--border-weak);
}

.view-header h1 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-strong);
  margin-bottom: 0.5rem;
}

.view-header p {
  font-size: 0.9rem;
  color: var(--text-weak);
}
</style>
