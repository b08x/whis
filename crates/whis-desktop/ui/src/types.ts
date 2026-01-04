// Generic select option for dropdowns
export interface SelectOption<T = string | null> {
  value: T
  label: string
  disabled?: boolean
}

// Transcription providers
export type Provider
  = | 'openai'
    | 'openai-realtime'
    | 'mistral'
    | 'groq'
    | 'deepgram'
    | 'elevenlabs'
    | 'local-whisper'
    | 'local-parakeet'

// OpenAI transcription method
export type TranscriptionMethod = 'standard' | 'streaming'

// Text post-processing providers
export type PostProcessor = 'none' | 'openai' | 'mistral' | 'ollama'

// All settings from the backend (nested structure)
export interface Settings {
  transcription: {
    provider: Provider
    language: string | null
    api_keys: Record<string, string>
    local_models: {
      whisper_path: string | null
      parakeet_path: string | null
      parakeet_progressive_workers: number
    }
  }
  post_processing: {
    processor: PostProcessor
    prompt: string | null
  }
  services: {
    ollama: {
      url: string | null
      model: string | null
    }
  }
  ui: {
    shortcut: string
    clipboard_method: string
    microphone_device: string | null
    vad: {
      enabled: boolean
      threshold: number
    }
    active_preset: string | null
  }
}

// Shortcut backend information
export interface BackendInfo {
  backend: string
  requires_restart: boolean
  compositor: string
  portal_version: number
}

// Status response from backend
export interface StatusResponse {
  state: 'Idle' | 'Recording' | 'Transcribing'
  config_valid: boolean
}

// Response when saving settings
export interface SaveSettingsResponse {
  needs_restart: boolean
}

// Whisper model info from backend
export interface WhisperModelInfo {
  name: string
  description: string
  installed: boolean
  path: string
}

// Parakeet model info from backend
export interface ParakeetModelInfo {
  name: string
  description: string
  size: string
  installed: boolean
  path: string
}

// Preset info from backend
export interface PresetInfo {
  name: string
  description: string
  is_builtin: boolean
}

// Full preset details
export interface PresetDetails {
  name: string
  description: string
  prompt: string
  post_processor: string | null
  model: string | null
  is_builtin: boolean
}

// Cloud provider configuration
export interface CloudProviderInfo {
  value: Provider
  label: string
  desc: string
  keyUrl: string
  placeholder: string
}

// Helper to check if provider is local
export function isLocalProvider(provider: Provider): boolean {
  return provider === 'local-whisper' || provider === 'local-parakeet'
}
