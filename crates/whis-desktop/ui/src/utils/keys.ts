// Platform detection for macOS-friendly key display
export const isMac = navigator.platform.toUpperCase().includes('MAC')

/**
 * Display a key name with platform-aware formatting.
 * On macOS, translates Ctrl -> Control, Alt -> Option, Super -> Cmd.
 */
export function displayKey(key: string): string {
  if (!isMac)
    return key
  switch (key.toLowerCase()) {
    case 'ctrl': return 'Control'
    case 'alt': return 'Option'
    case 'super': return 'Cmd'
    default: return key
  }
}

/**
 * Parse portal shortcut format like "Press <Control><Alt>l" into display keys.
 * Returns an array of platform-aware key names.
 */
export function parsePortalShortcut(portalStr: string): string[] {
  const cleaned = portalStr.replace(/^Press\s+/i, '')
  const keys: string[] = []
  const matches = cleaned.matchAll(/<(\w+)>/g)
  for (const match of matches) {
    const mod = (match[1] ?? '').toLowerCase()
    if (mod === 'control')
      keys.push(displayKey('Ctrl'))
    else if (mod === 'shift')
      keys.push('Shift')
    else if (mod === 'alt')
      keys.push(displayKey('Alt'))
    else if (mod === 'super')
      keys.push(displayKey('Super'))
    else if (mod)
      keys.push(mod.charAt(0).toUpperCase() + mod.slice(1))
  }
  const finalKey = cleaned.replace(/<\w+>/g, '').trim()
  if (finalKey) {
    keys.push(finalKey.toUpperCase())
  }
  return keys
}
