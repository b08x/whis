// Generic select option for dropdowns
export interface SelectOption<T = string | null> {
  value: T
  label: string
  disabled?: boolean
}

// Mobile transcription providers
export type Provider = 'openai' | 'openai-realtime' | 'mistral' | 'groq' | 'deepgram' | 'deepgram-realtime' | 'elevenlabs'

// OpenAI transcription method
export type TranscriptionMethod = 'standard' | 'streaming'

// Post-processing provider (LLM for transcript cleanup)
export type PostProcessor = 'none' | 'openai' | 'mistral'

// Settings keys used by Tauri Store plugin
export interface SettingsKeys {
  provider: Provider
  language: string | null
  openai_api_key: string | null
  mistral_api_key: string | null
  groq_api_key: string | null
  deepgram_api_key: string | null
  elevenlabs_api_key: string | null
}

// Status response from backend
export interface StatusResponse {
  state: 'Idle' | 'Recording' | 'Transcribing'
  config_valid: boolean
}

// Preset info from backend
export interface PresetInfo {
  name: string
  description: string
  is_builtin: boolean
  is_active: boolean
}

// Full preset details from backend
export interface PresetDetails {
  name: string
  description: string
  prompt: string
  is_builtin: boolean
}

// Audio chunk for streaming
export interface AudioChunk {
  samples: Float32Array
  timestamp: number
}

// Preset creation input
export interface CreatePresetInput {
  name: string
  description: string
  prompt: string
}

// Preset update input
export interface UpdatePresetInput {
  description: string
  prompt: string
}
