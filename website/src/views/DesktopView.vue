<script setup lang="ts">
import type { Platform } from '@/composables/usePlatformDetection'
import { computed, ref } from 'vue'
import ImageCarousel from '@/components/ImageCarousel.vue'
import Lightbox from '@/components/Lightbox.vue'
import ViewHeader from '@/components/ViewHeader.vue'
import { useGitHubRelease } from '@/composables/useGitHubRelease'
import { usePlatformDetection } from '@/composables/usePlatformDetection'

const { platform, arch } = usePlatformDetection()
const { version, versionNum, findAsset } = useGitHubRelease()

const lightboxOpen = ref(false)
const lightboxIndex = ref(0)

const screenshots = [
  { src: '/screenshot-01-about.png', alt: 'About page showing Whis version and description', caption: 'About' },
  { src: '/screenshot-02-tray.png', alt: 'Whis running in system tray', caption: 'System tray' },
  { src: '/screenshot-03-home.png', alt: 'Whis home screen with Start Recording button', caption: 'Home' },
  { src: '/screenshot-04-shortcuts.png', alt: 'Global shortcut configuration', caption: 'Shortcuts' },
  { src: '/screenshot-05-settings-cloud.png', alt: 'Cloud provider settings with OpenAI, Groq, Deepgram options', caption: 'Cloud mode' },
  { src: '/screenshot-06-settings-local.png', alt: 'Local whisper model selection for offline transcription', caption: 'Local mode' },
  { src: '/screenshot-07-presets.png', alt: 'Preset configurations for AI prompts, email, and notes', caption: 'Presets' },
]

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
  const v = version.value
  const vn = versionNum.value
  const base = `https://github.com/frankdierolf/whis/releases/download/${v}`

  switch (platform.value) {
    case 'linux':
      return {
        label: 'AppImage',
        url: findAsset(/Whis_.*_amd64\.AppImage$/)?.browser_download_url || `${base}/Whis_${vn}_amd64.AppImage`,
      }
    case 'macos':
      return {
        label: 'DMG',
        url: arch.value === 'arm64'
          ? (findAsset(/Whis_.*_aarch64\.dmg$/)?.browser_download_url || `${base}/Whis_${vn}_aarch64.dmg`)
          : (findAsset(/Whis_.*_x64\.dmg$/)?.browser_download_url || `${base}/Whis_${vn}_x64.dmg`),
      }
    case 'windows':
      return {
        label: 'exe',
        url: findAsset(/Whis_.*_x64-setup\.exe$/)?.browser_download_url || `${base}/Whis_${vn}_x64-setup.exe`,
      }
    default:
      return {
        label: 'AppImage',
        url: findAsset(/Whis_.*_amd64\.AppImage$/)?.browser_download_url || `${base}/Whis_${vn}_amd64.AppImage`,
      }
  }
})

function openLightbox(index: number) {
  lightboxIndex.value = index
  lightboxOpen.value = true
}
</script>

<template>
  <div class="desktop-content">
    <ViewHeader title="Desktop" subtitle="System tray app for voice-to-text anywhere" />

    <!-- Install -->
    <section class="install">
      <h2 class="install-title">
        Download for {{ platformLabel }}
      </h2>
      <a :href="recommendedDownload.url" class="download-button">
        <span class="download-icon">↓</span>
        <span class="download-label">{{ recommendedDownload.label }}</span>
        <span class="download-version">{{ version }}</span>
      </a>
      <p class="install-note">
        <RouterLink to="/downloads">
          More options →
        </RouterLink>
      </p>
    </section>

    <!-- Features -->
    <section class="features">
      <div class="section-header">
        <h2>What is Whis Desktop?</h2>
        <p>A system tray app that brings voice-to-text to your entire desktop.</p>
      </div>
      <ul>
        <li>
          <span class="marker">[*]</span>
          <div><strong>System tray</strong> Lives in your panel, always ready</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>Global hotkey</strong> Ctrl+Alt+W from anywhere (configurable)</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>6 providers</strong> OpenAI, Groq, Deepgram, Mistral, ElevenLabs, or local</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>Run locally</strong> Download whisper models in-app — private and free</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>AI post-processing</strong> Clean up transcripts with presets</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>Cross-platform</strong> Linux, macOS, Windows (experimental)</div>
        </li>
      </ul>
    </section>

    <!-- Screenshots -->
    <section class="demo">
      <figure>
        <ImageCarousel :images="screenshots" @select="openLightbox" />
      </figure>
    </section>

    <!-- Quick Start -->
    <section class="quickstart">
      <h2>Quick Start</h2>
      <pre><code><span class="comment"># 1. Download and install using the button above</span>

<span class="comment"># 2. Launch "Whis" from your app menu</span>

<span class="comment"># 3. Tray icon → Settings → Configure provider</span>

<span class="comment"># 4. Use Ctrl+Alt+W or tray menu to record!</span></code></pre>
    </section>

    <!-- Lightbox -->
    <Lightbox
      v-model:open="lightboxOpen"
      :images="screenshots"
      :initial-index="lightboxIndex"
    />
  </div>
</template>

<style scoped>
.desktop-content {
  padding: 2rem;
}

.install {
  padding: var(--vertical-padding) var(--padding);
}

.install-title {
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

.install-note {
  margin-top: 0.75rem;
  font-size: 0.75rem;
  color: var(--text-weak);
}

.install-note a {
  color: var(--text);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.install-note a:hover {
  color: var(--accent);
}

.features {
  border-top: 1px solid var(--border-weak);
  padding: var(--vertical-padding) var(--padding);
}

.section-header {
  margin-bottom: 2rem;
}

.section-header h2 {
  font-size: 1.1rem;
  font-weight: 500;
  color: var(--text-strong);
  margin-bottom: 0.5rem;
}

.section-header p {
  color: var(--text);
}

.features ul {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.features li {
  display: flex;
  gap: 0.75rem;
  line-height: 1.6;
}

.marker {
  color: var(--icon);
  flex-shrink: 0;
}

.features li strong {
  color: var(--text-strong);
  font-weight: 500;
  margin-right: 0.5rem;
}

.demo {
  border-top: 1px solid var(--border-weak);
  padding: var(--vertical-padding) var(--padding);
}

.demo figure {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.quickstart {
  border-top: 1px solid var(--border-weak);
  padding: var(--vertical-padding) var(--padding);
}

.quickstart h2 {
  font-size: 1.1rem;
  font-weight: 500;
  color: var(--text-strong);
  margin-bottom: 1.5rem;
}

.quickstart pre {
  background: var(--bg-weak);
  border: 1px solid var(--border-weak);
  border-radius: 6px;
  padding: 1.5rem;
  overflow-x: auto;
}

.quickstart code {
  font-family: var(--font);
  font-size: 0.9rem;
  line-height: 1.8;
}

.comment {
  color: var(--text-weak);
}

.highlight {
  color: var(--text-strong);
  font-weight: 500;
}
</style>
