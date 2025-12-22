import { onMounted, ref } from 'vue'

export type Platform = 'linux' | 'macos' | 'windows' | 'android' | 'unknown'
export type Arch = 'x86_64' | 'arm64' | 'unknown'

export function usePlatformDetection() {
  const platform = ref<Platform>('unknown')
  const arch = ref<Arch>('unknown')

  function detect() {
    const ua = navigator.userAgent.toLowerCase()
    const navPlatform = navigator.platform?.toLowerCase() || ''

    // Mobile first
    if (/android/.test(ua)) {
      platform.value = 'android'
    }
    else if (/iphone|ipad|ipod/.test(ua)) {
      platform.value = 'unknown' // iOS not supported
    }
    else if (/mac/.test(navPlatform) || /mac/.test(ua)) {
      platform.value = 'macos'
    }
    else if (/win/.test(navPlatform) || /win/.test(ua)) {
      platform.value = 'windows'
    }
    else if (/linux/.test(navPlatform) || /linux/.test(ua)) {
      platform.value = 'linux'
    }

    // Detect architecture
    // @ts-expect-error userAgentData is experimental
    const uaData = navigator.userAgentData
    if (uaData?.platform === 'macOS') {
      const canvas = document.createElement('canvas')
      const gl = canvas.getContext('webgl')
      const renderer = gl?.getParameter(gl.RENDERER) || ''
      if (/apple m/i.test(renderer) || /apple gpu/i.test(renderer)) {
        arch.value = 'arm64'
      }
      else {
        arch.value = 'x86_64'
      }
    }
    else if (/arm|aarch64/.test(ua)) {
      arch.value = 'arm64'
    }
    else {
      arch.value = 'x86_64'
    }
  }

  onMounted(detect)

  return { platform, arch }
}
