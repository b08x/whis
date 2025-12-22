<script setup lang="ts">
import { computed, ref } from 'vue'
import Lightbox from '@/components/Lightbox.vue'
import ViewHeader from '@/components/ViewHeader.vue'
import { useGitHubRelease } from '@/composables/useGitHubRelease'

const { version, findAsset } = useGitHubRelease()

const lightboxOpen = ref(false)

const demoImage = [
  { src: '/mobile-demo.png', alt: 'Whis mobile app', caption: 'Home · Presets · Settings · About' },
]

const apkUrl = computed(() => {
  const asset = findAsset(/\.apk$/)
  if (asset)
    return asset.browser_download_url
  const v = version.value
  return `https://github.com/frankdierolf/whis/releases/download/${v}/app-universal-release-unsigned.apk`
})
</script>

<template>
  <div class="mobile-content">
    <ViewHeader title="Mobile" subtitle="Voice-to-text on the go" />

    <!-- Install -->
    <section class="install">
      <h2 class="install-title">
        Download for Android <span class="status-badge">[experimental]</span>
      </h2>
      <a :href="apkUrl" class="download-button">
        <span class="download-icon">↓</span>
        <span class="download-label">APK</span>
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
        <h2>What is Whis Mobile?</h2>
        <p>Same whis experience, from your phone.</p>
      </div>
      <ul>
        <li>
          <span class="marker">[*]</span>
          <div>Voice-to-text from your phone</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div>Cloud transcription (OpenAI, Mistral)</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div>Text straight to clipboard</div>
        </li>
      </ul>
    </section>

    <!-- Demo -->
    <section class="demo">
      <figure>
        <img
          src="/mobile-demo.png"
          alt="Whis mobile app screenshots showing home, presets, settings, and about screens"
          loading="lazy"
          class="clickable"
          @click="lightboxOpen = true"
        >
        <figcaption>Home · Presets · Settings · About</figcaption>
      </figure>
    </section>

    <!-- Lightbox -->
    <Lightbox v-model:open="lightboxOpen" :images="demoImage" :initial-index="0" />
  </div>
</template>

<style scoped>
.mobile-content {
  padding: 2rem;
}

/* Install section */
.install {
  padding: var(--vertical-padding) var(--padding);
}

.install-title {
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--text-weak);
  margin-bottom: 0.75rem;
}

.status-badge {
  font-size: 0.75rem;
  color: var(--accent);
  font-weight: 400;
  margin-left: 0.5rem;
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

/* Features - match CLI/Desktop */
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

/* Demo */
.demo {
  border-top: 1px solid var(--border-weak);
  padding: var(--vertical-padding) var(--padding);
}

.demo figure {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.demo img {
  width: 100%;
  height: auto;
  border-radius: 6px;
  border: 1px solid var(--border-weak);
}

.demo img.clickable {
  cursor: zoom-in;
  transition: border-color 0.15s ease;
}

.demo img.clickable:hover {
  border-color: var(--border);
}

.demo figcaption {
  font-size: 0.85rem;
  color: var(--text-weak);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
</style>
