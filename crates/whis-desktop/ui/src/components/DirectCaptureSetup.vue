<script setup lang="ts">
import type { CommandSegment } from './CommandBlock.vue'
import CommandBlock from './CommandBlock.vue'

defineProps<{
  environmentHint?: string | null
}>()

interface SetupCommand {
  segments: CommandSegment[]
  fullCmd: string
}

const setupCommands: SetupCommand[] = [
  {
    segments: [
      { text: 'sudo', highlight: true },
      { text: ' usermod -aG input $USER' },
    ],
    fullCmd: 'sudo usermod -aG input $USER',
  },
  {
    segments: [
      { text: 'echo', highlight: true },
      { text: ' \'...\' | ' },
      { text: 'sudo', highlight: true },
      { text: ' tee /etc/udev/rules.d/99-uinput.rules' },
    ],
    fullCmd: 'echo \'KERNEL=="uinput", GROUP="input", MODE="0660"\' | sudo tee /etc/udev/rules.d/99-uinput.rules',
  },
  {
    segments: [
      { text: 'sudo', highlight: true },
      { text: ' udevadm control --reload && ' },
      { text: 'sudo', highlight: true },
      { text: ' udevadm trigger' },
    ],
    fullCmd: 'sudo udevadm control --reload-rules && sudo udevadm trigger',
  },
]
</script>

<template>
  <div class="setup-container">
    <p class="setup-header">
      <span class="notice-marker">[!]</span>
      Setup required
    </p>

    <p class="hint">
      Whis needs permission to capture keyboard input.<br>
      Run these commands, then log out and back in.
    </p>

    <CommandBlock
      v-for="(step, index) in setupCommands"
      :key="index"
      :command="step.fullCmd"
      :segments="step.segments"
    />

    <p v-if="environmentHint" class="env-hint">
      <span class="hint-marker">[i]</span>
      Detected: {{ environmentHint }}
    </p>
  </div>
</template>

<style scoped>
.setup-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.setup-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 13px;
  color: var(--text);
}

.setup-header .notice-marker {
  color: var(--text-weak);
  font-weight: 400;
}

.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-weak);
  line-height: 1.5;
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
