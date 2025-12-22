<script setup lang="ts">
import type { Platform } from '@/composables/usePlatformDetection'
import { computed, ref, watch } from 'vue'
import TabPanel from '@/components/TabPanel.vue'
import ViewHeader from '@/components/ViewHeader.vue'
import { useGitHubRelease } from '@/composables/useGitHubRelease'
import { usePlatformDetection } from '@/composables/usePlatformDetection'

const { platform, arch } = usePlatformDetection()
const { version, versionNum, findAsset } = useGitHubRelease()

const platformTab = ref('linux')

// Set initial tab when platform is detected
watch(platform, (p) => {
  if (p !== 'unknown') {
    platformTab.value = p
  }
}, { immediate: true })

// Download URLs
const downloads = computed(() => {
  const v = version.value
  const vn = versionNum.value
  const base = `https://github.com/frankdierolf/whis/releases/download/${v}`

  return {
    linux: {
      cli_x86: findAsset(/x86_64-unknown-linux-gnu\.tar\.gz$/)?.browser_download_url
        || `${base}/whis-${v}-x86_64-unknown-linux-gnu.tar.gz`,
      cli_arm: findAsset(/aarch64-unknown-linux-gnu\.tar\.gz$/)?.browser_download_url
        || `${base}/whis-${v}-aarch64-unknown-linux-gnu.tar.gz`,
      appimage_x86: findAsset(/Whis_.*_amd64\.AppImage$/)?.browser_download_url
        || `${base}/Whis_${vn}_amd64.AppImage`,
      appimage_arm: findAsset(/Whis_.*_arm64\.AppImage$/)?.browser_download_url
        || `${base}/Whis_${vn}_arm64.AppImage`,
      deb_x86: findAsset(/Whis_.*_amd64\.deb$/)?.browser_download_url
        || `${base}/Whis_${vn}_amd64.deb`,
      deb_arm: findAsset(/Whis_.*_arm64\.deb$/)?.browser_download_url
        || `${base}/Whis_${vn}_arm64.deb`,
      rpm_x86: findAsset(/Whis-.*\.x86_64\.rpm$/)?.browser_download_url
        || `${base}/Whis-${vn}-1.x86_64.rpm`,
      rpm_arm: findAsset(/Whis-.*\.aarch64\.rpm$/)?.browser_download_url
        || `${base}/Whis-${vn}-1.aarch64.rpm`,
    },
    macos: {
      cli_arm: findAsset(/aarch64-apple-darwin\.tar\.gz$/)?.browser_download_url
        || `${base}/whis-${v}-aarch64-apple-darwin.tar.gz`,
      cli_intel: findAsset(/x86_64-apple-darwin\.tar\.gz$/)?.browser_download_url
        || `${base}/whis-${v}-x86_64-apple-darwin.tar.gz`,
      dmg_arm: findAsset(/Whis_.*_aarch64\.dmg$/)?.browser_download_url
        || `${base}/Whis_${vn}_aarch64.dmg`,
      dmg_intel: findAsset(/Whis_.*_x64\.dmg$/)?.browser_download_url
        || `${base}/Whis_${vn}_x64.dmg`,
    },
    windows: {
      cli: findAsset(/x86_64-pc-windows-msvc\.zip$/)?.browser_download_url
        || `${base}/whis-${v}-x86_64-pc-windows-msvc.zip`,
      exe: findAsset(/Whis_.*_x64-setup\.exe$/)?.browser_download_url
        || `${base}/Whis_${vn}_x64-setup.exe`,
      msi: findAsset(/Whis_.*_x64_en-US\.msi$/)?.browser_download_url
        || `${base}/Whis_${vn}_x64_en-US.msi`,
    },
    android: {
      apk: findAsset(/app-universal-release.*\.apk$/)?.browser_download_url
        || `${base}/app-universal-release-unsigned.apk`,
    },
  }
})

const platformLabel = computed(() => {
  const labels: Record<Platform, string> = {
    linux: 'Linux',
    macos: 'macOS',
    windows: 'Windows',
    android: 'Android',
    unknown: 'your system',
  }
  return labels[platform.value]
})

const recommendedDownload = computed(() => {
  const d = downloads.value

  switch (platform.value) {
    case 'linux':
      return {
        label: 'AppImage',
        url: arch.value === 'arm64' ? d.linux.appimage_arm : d.linux.appimage_x86,
      }
    case 'macos':
      return {
        label: 'DMG',
        url: arch.value === 'arm64' ? d.macos.dmg_arm : d.macos.dmg_intel,
      }
    case 'windows':
      return {
        label: 'exe',
        url: d.windows.exe,
      }
    case 'android':
      return {
        label: 'APK',
        url: d.android.apk,
      }
    default:
      return null
  }
})
</script>

<template>
  <div class="downloads-content">
    <ViewHeader title="Downloads" subtitle="Get Whis for your platform" />

    <!-- Recommended Download -->
    <section v-if="recommendedDownload" class="recommended">
      <h2 class="recommended-title">
        Download for {{ platformLabel }}
      </h2>
      <a :href="recommendedDownload.url" class="download-button">
        <span class="download-icon">↓</span>
        <span class="download-label">{{ recommendedDownload.label }}</span>
        <span class="download-version">{{ version }}</span>
      </a>
    </section>

    <!-- All Downloads -->
    <section class="all-downloads">
      <TabPanel
        v-model:selected="platformTab"
        :tabs="[
          { value: 'linux', label: 'Linux' },
          { value: 'macos', label: 'macOS' },
          { value: 'windows', label: 'Windows' },
          { value: 'android', label: 'Android' },
        ]"
      >
        <!-- Linux -->
        <div v-if="platformTab === 'linux'" class="download-grid two-col">
          <div class="download-section">
            <h3>Desktop</h3>
            <div class="download-links">
              <a
                href="https://flathub.org/apps/ink.whis.Whis"
                target="_blank"
                rel="noopener"
                class="download-link"
              >
                <span class="link-label">Flatpak</span>
                <span class="link-arrow">→</span>
              </a>
              <div class="download-row">
                <span class="row-label">AppImage</span>
                <span class="arch-links">
                  <a :href="downloads.linux.appimage_x86">x86_64</a>
                  <span class="arch-sep">·</span>
                  <a :href="downloads.linux.appimage_arm">arm64</a>
                </span>
              </div>
              <div class="download-row">
                <span class="row-label">deb</span>
                <span class="arch-links">
                  <a :href="downloads.linux.deb_x86">x86_64</a>
                  <span class="arch-sep">·</span>
                  <a :href="downloads.linux.deb_arm">arm64</a>
                </span>
              </div>
              <div class="download-row">
                <span class="row-label">rpm</span>
                <span class="arch-links">
                  <a :href="downloads.linux.rpm_x86">x86_64</a>
                  <span class="arch-sep">·</span>
                  <a :href="downloads.linux.rpm_arm">arm64</a>
                </span>
              </div>
            </div>
          </div>
          <div class="download-section">
            <h3>CLI</h3>
            <div class="download-links">
              <a :href="downloads.linux.cli_x86" class="download-link">
                <span class="link-label">x86_64 (tar.gz)</span>
                <span class="link-arrow">→</span>
              </a>
              <a :href="downloads.linux.cli_arm" class="download-link">
                <span class="link-label">arm64 (tar.gz)</span>
                <span class="link-arrow">→</span>
              </a>
            </div>
          </div>
        </div>

        <!-- macOS -->
        <div v-else-if="platformTab === 'macos'" class="download-grid two-col">
          <div class="download-section">
            <h3>Desktop</h3>
            <div class="download-links">
              <div class="download-row">
                <span class="row-label">dmg</span>
                <span class="arch-links">
                  <a :href="downloads.macos.dmg_arm">Apple Silicon</a>
                  <span class="arch-sep">·</span>
                  <a :href="downloads.macos.dmg_intel">Intel</a>
                </span>
              </div>
            </div>
          </div>
          <div class="download-section">
            <h3>CLI</h3>
            <div class="download-links">
              <div class="download-row">
                <span class="row-label">tar.gz</span>
                <span class="arch-links">
                  <a :href="downloads.macos.cli_arm">Apple Silicon</a>
                  <span class="arch-sep">·</span>
                  <a :href="downloads.macos.cli_intel">Intel</a>
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Windows -->
        <div v-else-if="platformTab === 'windows'" class="download-grid two-col">
          <div class="download-section">
            <h3>Desktop</h3>
            <div class="download-links">
              <a :href="downloads.windows.exe" class="download-link">
                <span class="link-label">exe</span>
                <span class="link-arrow">→</span>
              </a>
              <a :href="downloads.windows.msi" class="download-link">
                <span class="link-label">msi</span>
                <span class="link-arrow">→</span>
              </a>
            </div>
          </div>
          <div class="download-section">
            <h3>CLI</h3>
            <div class="download-links">
              <a :href="downloads.windows.cli" class="download-link">
                <span class="link-label">zip</span>
                <span class="link-arrow">→</span>
              </a>
            </div>
          </div>
        </div>

        <!-- Android -->
        <div v-else-if="platformTab === 'android'" class="download-grid two-col">
          <div class="download-section">
            <h3>Mobile</h3>
            <div class="download-links">
              <a :href="downloads.android.apk" class="download-link">
                <span class="link-label">apk</span>
                <span class="link-arrow">→</span>
              </a>
            </div>
            <p class="section-note">
              Early development — feedback welcome!
            </p>
          </div>
        </div>
      </TabPanel>
    </section>
  </div>
</template>

<style scoped>
.downloads-content {
  padding: 2rem;
}

/* Recommended section */
.recommended {
  padding: var(--vertical-padding) var(--padding);
}

.recommended-title {
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--text-weak);
  margin-bottom: 0.75rem;
}

.download-button {
  display: inline-flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.875rem 1.5rem;
  background: var(--bg-strong);
  color: var(--text-inverted);
  border-radius: 4px;
  font-size: 0.95rem;
  font-weight: 600;
  text-decoration: none;
  transition: all 0.15s ease;
}

.download-button:hover {
  background: var(--bg-strong-hover);
  transform: translateX(2px);
}

.download-icon {
  font-size: 1.1rem;
}

.download-version {
  font-size: 0.8rem;
  font-weight: 400;
  opacity: 0.7;
}

/* All Downloads */
.all-downloads {
  border-top: 1px solid var(--border-weak);
  padding: var(--vertical-padding) var(--padding);
}

.all-downloads h2 {
  font-size: 1rem;
  font-weight: 500;
  color: var(--text-strong);
  margin-bottom: 1rem;
}

.download-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1.5rem;
}

.download-grid.two-col {
  grid-template-columns: 1fr 1fr;
}

.download-section h3 {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 0.75rem;
}

.download-links {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.download-link {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--bg);
  border: 1px solid var(--border-weak);
  border-radius: 4px;
  font-size: 0.85rem;
  color: var(--text);
  text-decoration: none;
  transition: all 0.15s ease;
}

.download-link:hover {
  background: var(--bg-hover);
  border-color: var(--border);
  color: var(--text-strong);
}

.link-arrow {
  color: var(--icon);
  transition: transform 0.15s ease;
}

.download-link:hover .link-arrow {
  transform: translateX(2px);
  color: var(--accent);
}

.section-note {
  margin-top: 0.75rem;
  font-size: 0.75rem;
  color: var(--text-weak);
}

/* Inline arch links */
.download-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--bg);
  border: 1px solid var(--border-weak);
  border-radius: 4px;
  font-size: 0.85rem;
}

.row-label {
  color: var(--text);
}

.arch-links {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.arch-links a {
  color: var(--text-weak);
  text-decoration: none;
  transition: color 0.15s ease;
}

.arch-links a:hover {
  color: var(--accent);
}

.arch-sep {
  color: var(--text-weak);
  opacity: 0.5;
}

@media (max-width: 768px) {
  .download-grid,
  .download-grid.two-col {
    grid-template-columns: 1fr;
  }
}
</style>
