import { computed, ref } from 'vue'
import { displayKey, isMac } from '../utils/keys.js'

/**
 * Composable for capturing keyboard shortcuts.
 * Used in ShortcutView for recording global shortcuts.
 */
export function useKeyboardCapture(initialValue: string = '') {
  const isRecording = ref(false)
  const capturedShortcut = ref(initialValue)

  // Split shortcut into individual keys for display (with platform-aware names)
  const shortcutKeys = computed(() => {
    if (capturedShortcut.value === 'Press keys...') {
      return ['...']
    }
    return capturedShortcut.value.split('+').map(displayKey)
  })

  function handleKeyDown(e: KeyboardEvent) {
    if (!isRecording.value)
      return
    e.preventDefault()

    // Use platform-aware key names for display
    const keys: string[] = []
    if (e.ctrlKey)
      keys.push(isMac ? 'Control' : 'Ctrl')
    if (e.shiftKey)
      keys.push('Shift')
    if (e.altKey)
      keys.push(isMac ? 'Option' : 'Alt')
    if (e.metaKey)
      keys.push(isMac ? 'Cmd' : 'Super')

    const key = e.key.toUpperCase()
    if (!['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) {
      keys.push(key)
    }

    if (keys.length > 0) {
      capturedShortcut.value = keys.join('+')
    }
  }

  function startRecording() {
    isRecording.value = true
    capturedShortcut.value = 'Press keys...'
  }

  function stopRecording() {
    isRecording.value = false
  }

  function setShortcut(value: string) {
    capturedShortcut.value = value
  }

  return {
    isRecording,
    capturedShortcut,
    shortcutKeys,
    handleKeyDown,
    startRecording,
    stopRecording,
    setShortcut,
  }
}
