<script setup lang="ts">
import { ref } from 'vue'

export interface CommandSegment {
  text: string
  highlight?: boolean
}

const props = defineProps<{
  command: string
  segments?: CommandSegment[]
  copyable?: boolean
}>()

const emit = defineEmits<{
  copied: []
}>()

const copied = ref(false)

async function copyCommand() {
  if (props.copyable === false)
    return

  try {
    await navigator.clipboard.writeText(props.command)
    copied.value = true
    emit('copied')
    setTimeout(() => {
      copied.value = false
    }, 2000)
  }
  catch (e) {
    console.error('Failed to copy:', e)
  }
}
</script>

<template>
  <div
    class="command"
    :class="{ copied, clickable: copyable !== false }"
    @click="copyCommand"
  >
    <code>
      <template v-if="segments && segments.length > 0">
        <span
          v-for="(seg, i) in segments"
          :key="i"
          :class="seg.highlight ? 'cmd-highlight' : 'cmd-dim'"
        >{{ seg.text }}</span>
      </template>
      <template v-else>
        {{ command }}
      </template>
    </code>
    <span v-if="copied" class="copied-indicator">copied</span>
  </div>
</template>

<style scoped>
.command {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  transition: border-color 0.15s ease;
}

.command.clickable {
  cursor: pointer;
}

.command.clickable:hover {
  border-color: var(--text-weak);
}

.command.copied {
  border-color: var(--accent);
}

.command code {
  flex: 1;
  min-width: 0;
  font-family: var(--font);
  font-size: 11px;
  color: var(--text);
  word-break: break-all;
  line-height: 1.5;
}

.copied-indicator {
  font-size: 10px;
  font-weight: 500;
  color: var(--accent);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.cmd-highlight {
  color: var(--text-strong);
  font-weight: 500;
  transition: color 0.15s ease;
}

.cmd-dim {
  color: var(--text-weak);
  transition: color 0.15s ease;
}

.command.clickable:hover .cmd-highlight,
.command.clickable:hover .cmd-dim {
  color: var(--accent);
}

.command.copied .cmd-highlight,
.command.copied .cmd-dim {
  color: var(--accent);
}
</style>
