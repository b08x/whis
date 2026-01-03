import { writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'

async function fetchStats() {
  const stats = {
    timestamp: new Date().toISOString(),
    crates: null,
    github: null,
    flathub: null,
  }

  // Fetch crates.io (requires User-Agent header)
  try {
    const res = await fetch('https://crates.io/api/v1/crates/whis', {
      headers: {
        'User-Agent': 'whis-website-stats (https://whis.ink)',
      },
    })
    if (res.ok) {
      const data = await res.json()
      stats.crates = data.crate.downloads
    }
  }
  catch (error) {
    console.warn('Failed to fetch crates.io stats:', error.message)
  }

  // Fetch GitHub releases
  try {
    const res = await fetch('https://api.github.com/repos/frankdierolf/whis/releases')
    if (res.ok) {
      const releases = await res.json()
      stats.github = releases.reduce((sum, release) => {
        const releaseTotal = release.assets.reduce(
          (assetSum, asset) => assetSum + asset.download_count,
          0,
        )
        return sum + releaseTotal
      }, 0)
    }
  }
  catch (error) {
    console.warn('Failed to fetch GitHub stats:', error.message)
  }

  // Fetch Flathub
  try {
    const res = await fetch('https://flathub.org/api/v2/stats/ink.whis.Whis')
    if (res.ok) {
      const data = await res.json()
      stats.flathub = data.installs_total
    }
  }
  catch (error) {
    console.warn('Failed to fetch Flathub stats:', error.message)
  }

  // Calculate total
  const validStats = [stats.crates, stats.github, stats.flathub].filter(v => v !== null)
  stats.total = validStats.length > 0 ? validStats.reduce((a, b) => a + b, 0) : null

  // Write to public directory
  const publicDir = join(process.cwd(), 'public')
  const statsPath = join(publicDir, 'stats.json')
  await writeFile(statsPath, JSON.stringify(stats, null, 2))

  console.log('✓ Stats fetched successfully:', stats)
  console.log(`  → Written to ${statsPath}`)
}

fetchStats().catch((error) => {
  console.error('Failed to fetch stats:', error)
  process.exit(1)
})
