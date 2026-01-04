import { writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'

async function fetchStats() {
  const stats = {
    timestamp: new Date().toISOString(),
    crates: null,
    github: null,
    flathub: null,
    aurPopularity: null,
    githubStars: null,
    githubForks: null,
    githubWatchers: null,
    githubContributors: null,
    versionCrates: null,
    versionAur: null,
    versionFlathub: null,
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
      stats.versionCrates = data.crate.newest_version
    }
  }
  catch (error) {
    console.warn('Failed to fetch crates.io stats:', error.message)
  }

  // Fetch AUR stats
  try {
    const res = await fetch('https://aur.archlinux.org/rpc/v5/info?arg=whis')
    if (res.ok) {
      const data = await res.json()
      if (data.results && data.results.length > 0) {
        stats.aurPopularity = data.results[0].Popularity
        stats.versionAur = data.results[0].Version
      }
    }
  }
  catch (error) {
    console.warn('Failed to fetch AUR stats:', error.message)
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

  // Fetch GitHub repository stats
  try {
    const res = await fetch('https://api.github.com/repos/frankdierolf/whis')
    if (res.ok) {
      const data = await res.json()
      stats.githubStars = data.stargazers_count
      stats.githubForks = data.forks_count
      stats.githubWatchers = data.watchers_count
    }
  }
  catch (error) {
    console.warn('Failed to fetch GitHub repo stats:', error.message)
  }

  // Fetch GitHub contributors count
  try {
    const res = await fetch('https://api.github.com/repos/frankdierolf/whis/contributors')
    if (res.ok) {
      const contributors = await res.json()
      stats.githubContributors = Array.isArray(contributors) ? contributors.length : null
    }
  }
  catch (error) {
    console.warn('Failed to fetch GitHub contributors:', error.message)
  }

  // Fetch Flathub stats
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

  // Fetch Flathub version
  try {
    const res = await fetch('https://flathub.org/api/v2/appstream/ink.whis.Whis')
    if (res.ok) {
      const data = await res.json()
      if (data.releases && data.releases.length > 0) {
        stats.versionFlathub = data.releases[0].version
      }
    }
  }
  catch (error) {
    console.warn('Failed to fetch Flathub version:', error.message)
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
