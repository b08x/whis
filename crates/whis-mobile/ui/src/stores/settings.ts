import type { PostProcessor, Provider } from '../types'
import { Store } from '@tauri-apps/plugin-store'
import { reactive, readonly } from 'vue'

// Internal mutable state for reactive UI
const state = reactive({
  provider: 'deepgram' as Provider,
  language: null as string | null,
  openai_api_key: null as string | null,
  mistral_api_key: null as string | null,
  groq_api_key: null as string | null,
  deepgram_api_key: null as string | null,
  elevenlabs_api_key: null as string | null,
  post_processor: 'none' as PostProcessor,
  loaded: false,
})

// Tauri store instance (initialized lazily)
let store: Store | null = null

async function getStore(): Promise<Store> {
  if (!store) {
    store = await Store.load('settings.json', {
      autoSave: 500,
      defaults: {
        provider: 'deepgram',
        language: null,
        openai_api_key: null,
        mistral_api_key: null,
        groq_api_key: null,
        deepgram_api_key: null,
        elevenlabs_api_key: null,
        post_processor: 'none',
      },
    })
  }
  return store
}

// Actions
async function initialize() {
  try {
    const s = await getStore()

    // Load values from store
    state.provider = (await s.get<Provider>('provider')) || 'deepgram'
    state.language = (await s.get<string | null>('language')) ?? null
    state.openai_api_key = (await s.get<string | null>('openai_api_key')) ?? null
    state.mistral_api_key = (await s.get<string | null>('mistral_api_key')) ?? null
    state.groq_api_key = (await s.get<string | null>('groq_api_key')) ?? null
    state.deepgram_api_key = (await s.get<string | null>('deepgram_api_key')) ?? null
    state.elevenlabs_api_key = (await s.get<string | null>('elevenlabs_api_key')) ?? null
    state.post_processor = (await s.get<PostProcessor>('post_processor')) || 'none'
    state.loaded = true
  }
  catch (e) {
    console.error('Failed to load settings:', e)
    state.loaded = true // Mark as loaded even on error to unblock UI
  }
}

async function setProvider(value: Provider) {
  state.provider = value
  const s = await getStore()
  await s.set('provider', value)
}

async function setLanguage(value: string | null) {
  state.language = value
  const s = await getStore()
  await s.set('language', value)
}

async function setOpenaiApiKey(value: string | null) {
  state.openai_api_key = value
  const s = await getStore()
  await s.set('openai_api_key', value)
}

async function setMistralApiKey(value: string | null) {
  state.mistral_api_key = value
  const s = await getStore()
  await s.set('mistral_api_key', value)
}

async function setGroqApiKey(value: string | null) {
  state.groq_api_key = value
  const s = await getStore()
  await s.set('groq_api_key', value)
}

async function setDeepgramApiKey(value: string | null) {
  state.deepgram_api_key = value
  const s = await getStore()
  await s.set('deepgram_api_key', value)
}

async function setElevenlabsApiKey(value: string | null) {
  state.elevenlabs_api_key = value
  const s = await getStore()
  await s.set('elevenlabs_api_key', value)
}

async function setPostProcessor(value: PostProcessor) {
  state.post_processor = value
  const s = await getStore()
  await s.set('post_processor', value)
}

// Export reactive state and actions
export const settingsStore = {
  // Readonly state for reading
  state: readonly(state),

  // Actions
  initialize,

  // Setters
  setProvider,
  setLanguage,
  setOpenaiApiKey,
  setMistralApiKey,
  setGroqApiKey,
  setDeepgramApiKey,
  setElevenlabsApiKey,
  setPostProcessor,
}
