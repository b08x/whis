<script setup lang="ts">
import type { CloudProviderInfo, Provider } from '../../types'
import { computed, ref } from 'vue'

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

// Cloud provider options with metadata (ordered by recommendation)
const cloudProviders: CloudProviderInfo[] = [
  { value: 'deepgram', label: 'Deepgram', desc: 'Real-time optimized', keyUrl: 'https://console.deepgram.com', placeholder: '...' },
  { value: 'openai', label: 'OpenAI', desc: 'Industry standard, excellent accuracy', keyUrl: 'https://platform.openai.com/api-keys', placeholder: 'sk-...' },
  { value: 'mistral', label: 'Mistral', desc: 'European AI, multilingual support', keyUrl: 'https://console.mistral.ai/api-keys', placeholder: '...' },
  { value: 'groq', label: 'Groq', desc: 'Ultra-fast inference', keyUrl: 'https://console.groq.com/keys', placeholder: 'gsk_...' },
  { value: 'elevenlabs', label: 'ElevenLabs', desc: 'Best for voice projects', keyUrl: 'https://elevenlabs.io/app/settings/api-keys', placeholder: '...' },
]

// Normalize provider for API key lookup (openai-realtime uses openai key)
const normalizedProvider = computed(() =>
  props.provider === 'openai-realtime' ? 'openai' : props.provider,
)

const currentProvider = computed((): CloudProviderInfo => {
  const found = cloudProviders.find(p => p.value === normalizedProvider.value)
  return found ?? cloudProviders[0]!
})

const currentApiKey = computed(() => props.apiKeys[normalizedProvider.value] || '')

function handleApiKeyChange(event: Event) {
  const value = (event.target as HTMLInputElement).value
  // Always store under normalized provider name (openai for both methods)
  emit('update:apiKey', normalizedProvider.value, value)
}
</script>

<template>
  <!-- API Key Configuration Card -->
  <div v-if="showConfigCard" class="config-card">
    <div class="api-key-input">
      <input
        :type="keyMasked[normalizedProvider] ? 'password' : 'text'"
        :value="currentApiKey"
        :placeholder="currentProvider.placeholder"
        spellcheck="false"
        autocomplete="off"
        aria-label="API Key"
        @input="handleApiKeyChange"
      >
      <button
        class="toggle-btn"
        type="button"
        :aria-pressed="!keyMasked[normalizedProvider]"
        aria-label="Toggle API key visibility"
        @click="keyMasked[normalizedProvider] = !keyMasked[normalizedProvider]"
      >
        {{ keyMasked[normalizedProvider] ? 'show' : 'hide' }}
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
