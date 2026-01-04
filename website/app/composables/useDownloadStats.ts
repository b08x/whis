import { computed, onMounted, ref } from 'vue'

interface DownloadStats {
  timestamp: string
  crates: number | null
  github: number | null
  flathub: number | null
  total: number | null
  aurPopularity: number | null
  githubStars: number | null
  githubForks: number | null
  githubWatchers: number | null
  githubContributors: number | null
  versionCrates: string | null
  versionAur: string | null
  versionFlathub: string | null
}

export function useDownloadStats() {
  const stats = ref<DownloadStats | null>(null)
  const error = ref<Error | null>(null)
  const loading = ref(true)

  onMounted(async () => {
    try {
      const response = await fetch('/stats.json')
      if (response.ok) {
        stats.value = await response.json()
      }
    }
    catch (e) {
      error.value = e as Error
      // Silent fail - stats remain null
    }
    finally {
      loading.value = false
    }
  })

  const total = computed(() => stats.value?.total ?? null)
  const breakdown = computed(() => ({
    crates: stats.value?.crates ?? null,
    github: stats.value?.github ?? null,
    flathub: stats.value?.flathub ?? null,
  }))

  return {
    total,
    breakdown,
    loading,
    error,
  }
}
