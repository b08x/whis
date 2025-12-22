import { computed, onMounted, ref } from 'vue'
import { version as fallbackVersion } from '../../package.json'

export interface ReleaseAsset {
  name: string
  browser_download_url: string
}

export interface Release {
  tag_name: string
  assets: ReleaseAsset[]
}

export function useGitHubRelease() {
  const release = ref<Release | null>(null)
  const loading = ref(true)

  // Use package.json version as fallback (prefixed with v)
  const version = computed(() => release.value?.tag_name || `v${fallbackVersion}`)
  const versionNum = computed(() => version.value.replace(/^v/, ''))

  function findAsset(pattern: RegExp): ReleaseAsset | undefined {
    return release.value?.assets.find(a => pattern.test(a.name))
  }

  function getDownloadUrl(pattern: RegExp, fallback: string): string {
    return findAsset(pattern)?.browser_download_url || fallback
  }

  onMounted(async () => {
    try {
      const res = await fetch('https://api.github.com/repos/frankdierolf/whis/releases/latest')
      if (res.ok)
        release.value = await res.json()
    }
    catch {
      // Silent fail - version will show as 'latest'
    }
    finally {
      loading.value = false
    }
  })

  return {
    release,
    loading,
    version,
    versionNum,
    findAsset,
    getDownloadUrl,
  }
}
