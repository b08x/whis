<script setup lang="ts" vapor>
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface SaveResult {
  needs_restart: boolean;
}

type Provider = 'openai' | 'mistral' | 'groq' | 'deepgram' | 'elevenlabs';

const props = defineProps<{
  currentShortcut: string;
  provider: Provider;
  language: string | null;
  apiKeys: Record<string, string>;
}>();

const emit = defineEmits<{
  'update:provider': [value: Provider];
  'update:language': [value: string | null];
  'update:apiKeys': [value: Record<string, string>];
}>();

const keyMasked = ref<Record<string, boolean>>({
  openai: true,
  mistral: true,
  groq: true,
  deepgram: true,
  elevenlabs: true,
});
const status = ref("");

// Common language codes for the dropdown
const languageOptions = [
  { value: null, label: 'Auto-detect' },
  { value: 'en', label: 'English (en)' },
  { value: 'de', label: 'German (de)' },
  { value: 'fr', label: 'French (fr)' },
  { value: 'es', label: 'Spanish (es)' },
  { value: 'it', label: 'Italian (it)' },
  { value: 'pt', label: 'Portuguese (pt)' },
  { value: 'nl', label: 'Dutch (nl)' },
  { value: 'pl', label: 'Polish (pl)' },
  { value: 'ru', label: 'Russian (ru)' },
  { value: 'ja', label: 'Japanese (ja)' },
  { value: 'ko', label: 'Korean (ko)' },
  { value: 'zh', label: 'Chinese (zh)' },
];

const currentApiKeyConfigured = computed(() => {
  const key = props.apiKeys[props.provider] || '';
  return key.length > 0;
});

function getApiKey(provider: Provider): string {
  return props.apiKeys[provider] || '';
}

function updateApiKey(provider: Provider, value: string) {
  const newKeys = { ...props.apiKeys, [provider]: value };
  emit('update:apiKeys', newKeys);
}

async function saveSettings() {
  try {
    // Validate OpenAI key format if provided
    const openaiKey = props.apiKeys.openai || '';
    if (openaiKey && !openaiKey.startsWith('sk-')) {
      status.value = "Invalid OpenAI key format. Keys start with 'sk-'";
      return;
    }

    // Validate Groq key format if provided
    const groqKey = props.apiKeys.groq || '';
    if (groqKey && !groqKey.startsWith('gsk_')) {
      status.value = "Invalid Groq key format. Keys start with 'gsk_'";
      return;
    }

    await invoke<SaveResult>('save_settings', {
      settings: {
        shortcut: props.currentShortcut,
        provider: props.provider,
        language: props.language,
        api_keys: props.apiKeys
      }
    });
    status.value = "Saved";
    setTimeout(() => status.value = "", 2000);
  } catch (e) {
    status.value = "Failed to save: " + e;
  }
}

function handleLanguageChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  emit('update:language', value === '' ? null : value);
}
</script>

<template>
  <section class="section">
    <header class="section-header">
      <h1>Settings</h1>
      <p>Configure transcription provider and API keys</p>
    </header>

    <div class="section-content">
      <!-- Provider Selection -->
      <div class="field">
        <label>Transcription Provider</label>
        <div class="provider-options">
          <button
            class="provider-btn"
            :class="{ active: provider === 'openai' }"
            @click="emit('update:provider', 'openai')"
          >
            OpenAI
          </button>
          <button
            class="provider-btn"
            :class="{ active: provider === 'mistral' }"
            @click="emit('update:provider', 'mistral')"
          >
            Mistral
          </button>
          <button
            class="provider-btn"
            :class="{ active: provider === 'groq' }"
            @click="emit('update:provider', 'groq')"
          >
            Groq
          </button>
        </div>
        <div class="provider-options" style="margin-top: 8px;">
          <button
            class="provider-btn"
            :class="{ active: provider === 'deepgram' }"
            @click="emit('update:provider', 'deepgram')"
          >
            Deepgram
          </button>
          <button
            class="provider-btn"
            :class="{ active: provider === 'elevenlabs' }"
            @click="emit('update:provider', 'elevenlabs')"
          >
            ElevenLabs
          </button>
        </div>
        <p class="hint">
          <template v-if="provider === 'openai'">~$0.006/min · whisper-1</template>
          <template v-else-if="provider === 'mistral'">~$0.02/min · voxtral-mini</template>
          <template v-else-if="provider === 'groq'">~$0.0007/min · whisper-large-v3-turbo</template>
          <template v-else-if="provider === 'deepgram'">~$0.0043/min · nova-2</template>
          <template v-else-if="provider === 'elevenlabs'">~$0.0067/min · scribe_v1</template>
        </p>
      </div>

      <!-- Language Hint -->
      <div class="field">
        <label>Language Hint</label>
        <select
          class="select-input"
          :value="language ?? ''"
          @change="handleLanguageChange"
        >
          <option v-for="opt in languageOptions" :key="opt.value ?? 'auto'" :value="opt.value ?? ''">
            {{ opt.label }}
          </option>
        </select>
        <p class="hint">
          Helps improve accuracy. Leave on auto-detect if you speak multiple languages.
        </p>
      </div>

      <div class="divider"></div>

      <!-- OpenAI API Key -->
      <div class="field">
        <label>
          OpenAI API Key
          <span v-if="provider === 'openai'" class="active-badge">active</span>
        </label>
        <div class="api-key-input">
          <input
            :type="keyMasked.openai ? 'password' : 'text'"
            :value="getApiKey('openai')"
            @input="updateApiKey('openai', ($event.target as HTMLInputElement).value)"
            placeholder="sk-..."
            spellcheck="false"
            autocomplete="off"
          />
          <button @click="keyMasked.openai = !keyMasked.openai" class="toggle-btn" type="button">
            {{ keyMasked.openai ? 'show' : 'hide' }}
          </button>
        </div>
        <p class="hint">
          Get your key from
          <a href="https://platform.openai.com/api-keys" target="_blank">platform.openai.com</a>
        </p>
      </div>

      <!-- Mistral API Key -->
      <div class="field">
        <label>
          Mistral API Key
          <span v-if="provider === 'mistral'" class="active-badge">active</span>
        </label>
        <div class="api-key-input">
          <input
            :type="keyMasked.mistral ? 'password' : 'text'"
            :value="getApiKey('mistral')"
            @input="updateApiKey('mistral', ($event.target as HTMLInputElement).value)"
            placeholder="..."
            spellcheck="false"
            autocomplete="off"
          />
          <button @click="keyMasked.mistral = !keyMasked.mistral" class="toggle-btn" type="button">
            {{ keyMasked.mistral ? 'show' : 'hide' }}
          </button>
        </div>
        <p class="hint">
          Get your key from
          <a href="https://console.mistral.ai/api-keys" target="_blank">console.mistral.ai</a>
        </p>
      </div>

      <!-- Groq API Key -->
      <div class="field">
        <label>
          Groq API Key
          <span v-if="provider === 'groq'" class="active-badge">active</span>
        </label>
        <div class="api-key-input">
          <input
            :type="keyMasked.groq ? 'password' : 'text'"
            :value="getApiKey('groq')"
            @input="updateApiKey('groq', ($event.target as HTMLInputElement).value)"
            placeholder="gsk_..."
            spellcheck="false"
            autocomplete="off"
          />
          <button @click="keyMasked.groq = !keyMasked.groq" class="toggle-btn" type="button">
            {{ keyMasked.groq ? 'show' : 'hide' }}
          </button>
        </div>
        <p class="hint">
          Get your key from
          <a href="https://console.groq.com/keys" target="_blank">console.groq.com</a>
        </p>
      </div>

      <!-- Deepgram API Key -->
      <div class="field">
        <label>
          Deepgram API Key
          <span v-if="provider === 'deepgram'" class="active-badge">active</span>
        </label>
        <div class="api-key-input">
          <input
            :type="keyMasked.deepgram ? 'password' : 'text'"
            :value="getApiKey('deepgram')"
            @input="updateApiKey('deepgram', ($event.target as HTMLInputElement).value)"
            placeholder="..."
            spellcheck="false"
            autocomplete="off"
          />
          <button @click="keyMasked.deepgram = !keyMasked.deepgram" class="toggle-btn" type="button">
            {{ keyMasked.deepgram ? 'show' : 'hide' }}
          </button>
        </div>
        <p class="hint">
          Get your key from
          <a href="https://console.deepgram.com/" target="_blank">console.deepgram.com</a>
        </p>
      </div>

      <!-- ElevenLabs API Key -->
      <div class="field">
        <label>
          ElevenLabs API Key
          <span v-if="provider === 'elevenlabs'" class="active-badge">active</span>
        </label>
        <div class="api-key-input">
          <input
            :type="keyMasked.elevenlabs ? 'password' : 'text'"
            :value="getApiKey('elevenlabs')"
            @input="updateApiKey('elevenlabs', ($event.target as HTMLInputElement).value)"
            placeholder="..."
            spellcheck="false"
            autocomplete="off"
          />
          <button @click="keyMasked.elevenlabs = !keyMasked.elevenlabs" class="toggle-btn" type="button">
            {{ keyMasked.elevenlabs ? 'show' : 'hide' }}
          </button>
        </div>
        <p class="hint">
          Get your key from
          <a href="https://elevenlabs.io/app/settings/api-keys" target="_blank">elevenlabs.io</a>
        </p>
      </div>

      <button @click="saveSettings" class="btn btn-secondary">Save</button>

      <div class="status" :class="{ visible: status }">{{ status }}</div>

      <div v-if="!currentApiKeyConfigured" class="notice">
        <span class="notice-marker">[!]</span>
        <p>Add your {{ provider.charAt(0).toUpperCase() + provider.slice(1) }} API key to start transcribing.</p>
      </div>

      <div class="notice">
        <span class="notice-marker">[i]</span>
        <p>Settings stored locally in ~/.config/whis/settings.json</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Provider selection buttons */
.provider-options {
  display: flex;
  gap: 8px;
}

.provider-btn {
  flex: 1;
  padding: 10px 16px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text-weak);
  cursor: pointer;
  transition: all 0.15s ease;
}

.provider-btn:hover {
  border-color: var(--text-weak);
  color: var(--text);
}

.provider-btn.active {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(255, 213, 79, 0.1);
}

/* Select input */
.select-input {
  padding: 10px 12px;
  background: var(--bg-weak);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font);
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
  transition: border-color 0.15s ease;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23808080' d='M3 4.5L6 7.5L9 4.5'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
}

.select-input:focus {
  outline: none;
  border-color: var(--accent);
}

.select-input option {
  background: var(--bg);
  color: var(--text);
}

/* Divider */
.divider {
  height: 1px;
  background: var(--border);
  margin: 8px 0;
}

/* Active badge */
.active-badge {
  display: inline-block;
  padding: 2px 6px;
  margin-left: 8px;
  font-size: 9px;
  text-transform: uppercase;
  color: var(--accent);
  background: rgba(255, 213, 79, 0.15);
  border-radius: 3px;
  vertical-align: middle;
}

/* API key input */
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
</style>
