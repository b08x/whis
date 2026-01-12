import type { SelectOption } from '../types'

// UI timing constants (milliseconds)
export const STATUS_MESSAGE_DURATION = 2000
export const DOWNLOAD_STATUS_DURATION = 3000
export const STATUS_POLL_INTERVAL = 500

// Whisper model sizes for display
export const MODEL_SIZES: Record<string, string> = {
  tiny: '~75 MB',
  base: '~142 MB',
  small: '~466 MB',
  medium: '~1.5 GB',
}

// Post-processor options for dropdowns
export const POST_PROCESSOR_OPTIONS: SelectOption[] = [
  { value: 'openai', label: 'OpenAI (cloud)' },
  { value: 'mistral', label: 'Mistral (cloud)' },
  { value: 'ollama', label: 'Ollama (local)' },
]
