import { invoke } from '@tauri-apps/api/core'

/**
 * Options for configuring the floating bubble.
 */
export interface BubbleOptions {
  /** Size of the bubble in dp. Default: 60 */
  size?: number
  /** Initial X position. Default: 0 */
  startX?: number
  /** Initial Y position. Default: 100 */
  startY?: number
}

/**
 * Response from visibility check.
 */
export interface VisibilityResponse {
  visible: boolean
}

/**
 * Response from permission check.
 */
export interface PermissionResponse {
  granted: boolean
}

/**
 * Show the floating bubble overlay.
 *
 * @param options - Configuration options for the bubble
 * @throws If overlay permission is not granted
 *
 * @example
 * ```typescript
 * import { showBubble } from '@frankdierolf/tauri-plugin-floating-bubble'
 * await showBubble({ size: 60, startX: 0, startY: 100 })
 * ```
 */
export async function showBubble(options?: BubbleOptions): Promise<void> {
  await invoke('plugin:floating-bubble|show_bubble', { options })
}

/**
 * Hide the floating bubble overlay.
 *
 * @example
 * ```typescript
 * import { hideBubble } from '@frankdierolf/tauri-plugin-floating-bubble'
 * await hideBubble()
 * ```
 */
export async function hideBubble(): Promise<void> {
  await invoke('plugin:floating-bubble|hide_bubble')
}

/**
 * Check if the floating bubble is currently visible.
 *
 * @returns Whether the bubble is visible
 *
 * @example
 * ```typescript
 * import { isBubbleVisible } from '@frankdierolf/tauri-plugin-floating-bubble'
 * const { visible } = await isBubbleVisible()
 * ```
 */
export async function isBubbleVisible(): Promise<VisibilityResponse> {
  return await invoke<VisibilityResponse>('plugin:floating-bubble|is_bubble_visible')
}

/**
 * Request the overlay permission (SYSTEM_ALERT_WINDOW).
 * Opens system settings if permission is not granted.
 *
 * @returns Whether permission was granted
 *
 * @example
 * ```typescript
 * import { requestOverlayPermission } from '@frankdierolf/tauri-plugin-floating-bubble'
 * const { granted } = await requestOverlayPermission()
 * if (granted) {
 *   await showBubble()
 * }
 * ```
 */
export async function requestOverlayPermission(): Promise<PermissionResponse> {
  return await invoke<PermissionResponse>('plugin:floating-bubble|request_overlay_permission')
}

/**
 * Check if the overlay permission (SYSTEM_ALERT_WINDOW) is granted.
 *
 * @returns Whether permission is granted
 *
 * @example
 * ```typescript
 * import { hasOverlayPermission } from '@frankdierolf/tauri-plugin-floating-bubble'
 * const { granted } = await hasOverlayPermission()
 * ```
 */
export async function hasOverlayPermission(): Promise<PermissionResponse> {
  return await invoke<PermissionResponse>('plugin:floating-bubble|has_overlay_permission')
}
