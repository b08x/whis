<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface StatusResponse {
  state: 'Idle' | 'Recording' | 'Transcribing';
  config_valid: boolean;
}

interface Settings {
  shortcut: string;
  provider: 'openai' | 'mistral';
  language: string | null;
  openai_api_key: string | null;
  mistral_api_key: string | null;
}

// State
const status = ref<StatusResponse>({ state: 'Idle', config_valid: false });
const error = ref<string | null>(null);
const lastTranscription = ref<string | null>(null);
const showSettings = ref(false);
const showCopied = ref(false);

// Settings
const provider = ref<'openai' | 'mistral'>('openai');
const language = ref<string>('');
const openaiApiKey = ref('');
const mistralApiKey = ref('');
const saving = ref(false);

let pollInterval: number | null = null;

const buttonText = computed(() => {
  switch (status.value.state) {
    case 'Idle': return 'Tap to Record';
    case 'Recording': return 'Tap to Stop';
    case 'Transcribing': return 'Transcribing...';
  }
});

const canRecord = computed(() => {
  return status.value.config_valid && status.value.state !== 'Transcribing';
});

async function fetchStatus() {
  try {
    status.value = await invoke<StatusResponse>('get_status');
    error.value = null;
  } catch (e) {
    console.error('Failed to get status:', e);
  }
}

async function toggleRecording() {
  if (!canRecord.value) {
    if (!status.value.config_valid) {
      showSettings.value = true;
    }
    return;
  }

  try {
    error.value = null;

    if (status.value.state === 'Idle') {
      await invoke('start_recording');
    } else if (status.value.state === 'Recording') {
      const text = await invoke<string>('stop_recording');
      lastTranscription.value = text;
      showCopied.value = true;
      setTimeout(() => showCopied.value = false, 2000);
    }

    await fetchStatus();
  } catch (e) {
    error.value = String(e);
    await fetchStatus();
  }
}

async function loadSettings() {
  try {
    const settings = await invoke<Settings>('get_settings');
    provider.value = settings.provider || 'openai';
    language.value = settings.language || '';
    openaiApiKey.value = settings.openai_api_key || '';
    mistralApiKey.value = settings.mistral_api_key || '';
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

async function saveSettings() {
  saving.value = true;
  try {
    await invoke('save_settings', {
      settings: {
        shortcut: 'Ctrl+Shift+R', // Not used on mobile
        provider: provider.value,
        language: language.value || null,
        openai_api_key: openaiApiKey.value || null,
        mistral_api_key: mistralApiKey.value || null,
      }
    });
    await fetchStatus();
    showSettings.value = false;
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  fetchStatus();
  loadSettings();
  pollInterval = window.setInterval(fetchStatus, 500);
});

onUnmounted(() => {
  if (pollInterval) {
    clearInterval(pollInterval);
  }
});
</script>

<template>
  <div class="app">
    <!-- Main View -->
    <div v-if="!showSettings" class="main-view">
      <header class="header">
        <h1 class="logo">whis</h1>
        <button class="settings-btn" @click="showSettings = true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
      </header>

      <main class="content">
        <!-- Record Button -->
        <button
          class="record-btn"
          :class="{
            recording: status.state === 'Recording',
            transcribing: status.state === 'Transcribing',
            disabled: !canRecord && status.config_valid
          }"
          @click="toggleRecording"
        >
          <div class="record-circle">
            <div v-if="status.state === 'Recording'" class="pulse-ring"></div>
            <svg v-if="status.state === 'Idle'" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"/>
              <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"/>
            </svg>
            <svg v-else-if="status.state === 'Recording'" viewBox="0 0 24 24" fill="currentColor">
              <rect x="6" y="6" width="12" height="12" rx="2"/>
            </svg>
            <div v-else class="spinner"></div>
          </div>
        </button>

        <p class="status-text">{{ buttonText }}</p>

        <!-- Copied Toast -->
        <div v-if="showCopied" class="toast">Copied to clipboard!</div>

        <!-- Error -->
        <p v-if="error" class="error">{{ error }}</p>

        <!-- Setup Hint -->
        <p v-if="!status.config_valid" class="hint" @click="showSettings = true">
          Tap to configure API key
        </p>

        <!-- Last Transcription Preview -->
        <div v-if="lastTranscription && !error" class="preview">
          <p>{{ lastTranscription.substring(0, 100) }}{{ lastTranscription.length > 100 ? '...' : '' }}</p>
        </div>
      </main>
    </div>

    <!-- Settings View -->
    <div v-else class="settings-view">
      <header class="header">
        <button class="back-btn" @click="showSettings = false">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7"/>
          </svg>
        </button>
        <h1>Settings</h1>
      </header>

      <main class="settings-content">
        <div class="field">
          <label>Provider</label>
          <select v-model="provider">
            <option value="openai">OpenAI Whisper</option>
            <option value="mistral">Mistral Voxtral</option>
          </select>
        </div>

        <div class="field" v-if="provider === 'openai'">
          <label>OpenAI API Key</label>
          <input
            type="password"
            v-model="openaiApiKey"
            placeholder="sk-..."
            autocomplete="off"
          />
        </div>

        <div class="field" v-if="provider === 'mistral'">
          <label>Mistral API Key</label>
          <input
            type="password"
            v-model="mistralApiKey"
            placeholder="Enter API key"
            autocomplete="off"
          />
        </div>

        <div class="field">
          <label>Language (optional)</label>
          <input
            type="text"
            v-model="language"
            placeholder="auto-detect"
          />
          <span class="hint-text">ISO code like "en", "de", "fr"</span>
        </div>

        <button class="save-btn" @click="saveSettings" :disabled="saving">
          {{ saving ? 'Saving...' : 'Save Settings' }}
        </button>
      </main>
    </div>
  </div>
</template>

<style>
:root {
  --bg: #111;
  --bg-light: #1a1a1a;
  --text: #e0e0e0;
  --text-dim: #888;
  --accent: #ffd700;
  --recording: #ff4444;
  --radius: 12px;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  -webkit-font-smoothing: antialiased;
}

.app {
  height: 100vh;
  display: flex;
  flex-direction: column;
}

/* Header */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  padding-top: max(16px, env(safe-area-inset-top));
}

.logo {
  font-size: 24px;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.settings-btn, .back-btn {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-light);
  border: none;
  border-radius: 50%;
  color: var(--text);
  cursor: pointer;
}

.settings-btn svg, .back-btn svg {
  width: 24px;
  height: 24px;
}

/* Main View */
.main-view {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  gap: 24px;
}

/* Record Button */
.record-btn {
  width: 160px;
  height: 160px;
  border-radius: 50%;
  border: none;
  background: var(--bg-light);
  cursor: pointer;
  position: relative;
  transition: transform 0.2s, background 0.2s;
}

.record-btn:active {
  transform: scale(0.95);
}

.record-btn.recording {
  background: var(--recording);
}

.record-btn.transcribing {
  background: var(--bg-light);
  cursor: wait;
}

.record-btn.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.record-circle {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.record-circle svg {
  width: 64px;
  height: 64px;
  color: var(--text);
}

.recording .record-circle svg {
  color: white;
}

/* Pulse animation */
.pulse-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  border: 3px solid white;
  animation: pulse 1.5s ease-out infinite;
}

@keyframes pulse {
  0% {
    transform: scale(1);
    opacity: 0.8;
  }
  100% {
    transform: scale(1.4);
    opacity: 0;
  }
}

/* Spinner */
.spinner {
  width: 48px;
  height: 48px;
  border: 4px solid var(--bg);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Status */
.status-text {
  font-size: 18px;
  color: var(--text-dim);
}

/* Toast */
.toast {
  position: fixed;
  bottom: 100px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--accent);
  color: #111;
  padding: 12px 24px;
  border-radius: var(--radius);
  font-weight: 500;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateX(-50%) translateY(10px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}

/* Error */
.error {
  color: var(--recording);
  text-align: center;
  padding: 12px;
  background: rgba(255, 68, 68, 0.1);
  border-radius: var(--radius);
  max-width: 300px;
}

/* Hint */
.hint {
  color: var(--accent);
  cursor: pointer;
  text-decoration: underline;
}

/* Preview */
.preview {
  max-width: 300px;
  padding: 16px;
  background: var(--bg-light);
  border-radius: var(--radius);
  font-size: 14px;
  color: var(--text-dim);
  text-align: center;
}

/* Settings View */
.settings-view {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.settings-view .header h1 {
  font-size: 18px;
  font-weight: 600;
}

.settings-content {
  flex: 1;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field label {
  font-size: 14px;
  color: var(--text-dim);
}

.field input, .field select {
  padding: 14px 16px;
  background: var(--bg-light);
  border: 1px solid #333;
  border-radius: var(--radius);
  color: var(--text);
  font-size: 16px;
  -webkit-appearance: none;
}

.field input:focus, .field select:focus {
  outline: none;
  border-color: var(--accent);
}

.hint-text {
  font-size: 12px;
  color: var(--text-dim);
}

.save-btn {
  margin-top: auto;
  padding: 16px;
  background: var(--accent);
  border: none;
  border-radius: var(--radius);
  color: #111;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  margin-bottom: max(20px, env(safe-area-inset-bottom));
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
