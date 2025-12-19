<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Provider, CloudProviderInfo } from '../../types'

const props = defineProps<{
  provider: Provider
  apiKeys: Record<string, string>
  showConfigCard?: boolean
}>()

const emit = defineEmits<{
  'update:apiKey': [provider: string, value: string]
}>()

const keyMasked = ref<Record<string, boolean>>({
  openai: true,
  mistral: true,
  groq: true,
  deepgram: true,
  elevenlabs: true,
})

// Cloud provider options with metadata
const cloudProviders: CloudProviderInfo[] = [
  { value: 'openai', label: 'OpenAI', desc: 'Industry standard, excellent accuracy', keyUrl: 'https://platform.openai.com/api-keys', placeholder: 'sk-...' },
  { value: 'mistral', label: 'Mistral', desc: 'European AI, multilingual support', keyUrl: 'https://console.mistral.ai/api-keys', placeholder: '...' },
  { value: 'groq', label: 'Groq', desc: 'Ultra-fast inference', keyUrl: 'https://console.groq.com/keys', placeholder: 'gsk_...' },
  { value: 'deepgram', label: 'Deepgram', desc: 'Real-time optimized', keyUrl: 'https://console.deepgram.com', placeholder: '...' },
  { value: 'elevenlabs', label: 'ElevenLabs', desc: 'Best for voice projects', keyUrl: 'https://elevenlabs.io/app/settings/api-keys', placeholder: '...' },
]

const currentProvider = computed((): CloudProviderInfo => {
  const found = cloudProviders.find(p => p.value === props.provider)
  return found ?? cloudProviders[0]!
})

const currentApiKey = computed(() => props.apiKeys[props.provider] || '')

function handleApiKeyChange(event: Event) {
  const value = (event.target as HTMLInputElement).value
  emit('update:apiKey', props.provider, value)
}
</script>

<template>
  <!-- API Key Configuration Card -->
  <div v-if="showConfigCard" class="config-card">
    <div class="api-key-input">
      <input
        :type="keyMasked[provider] ? 'password' : 'text'"
        :value="currentApiKey"
        @input="handleApiKeyChange"
        :placeholder="currentProvider.placeholder"
        spellcheck="false"
        autocomplete="off"
        aria-label="API Key"
      />
      <button
        @click="keyMasked[provider] = !keyMasked[provider]"
        class="toggle-btn"
        type="button"
        :aria-pressed="!keyMasked[provider]"
        aria-label="Toggle API key visibility"
      >
        {{ keyMasked[provider] ? 'show' : 'hide' }}
      </button>
    </div>
    <p class="hint">
      Get key at
      <a :href="currentProvider.keyUrl" target="_blank">{{ currentProvider.keyUrl.replace('https://', '') }}</a>
    </p>
  </div>
</template>

<style scoped>
.config-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.api-key-input {
  display: flex;
  gap: 8px;
}

.api-key-input input {
  flex: 1;
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
  transition: border-color 0.15s ease;
}

.api-key-input input::placeholder {
  color: var(--text-weak);
}

.api-key-input input:focus {
  outline: none;
  border-color: var(--accent);
}

.toggle-btn {
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 11px;
  color: var(--text-weak);
  cursor: pointer;
  transition: all 0.15s ease;
}

.toggle-btn:hover {
  border-color: var(--text-weak);
  color: var(--text);
}

.toggle-btn:focus-visible {
  outline: none;
  border-color: var(--accent);
}

.hint {
  font-size: 11px;
  color: var(--text-weak);
  margin: 0;
}

.hint a {
  color: var(--accent);
  text-decoration: none;
}

.hint a:hover {
  text-decoration: underline;
}
</style>
