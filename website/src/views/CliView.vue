<script setup lang="ts">
import { ref } from 'vue'
import CommandCopy from '@/components/CommandCopy.vue'
import Lightbox from '@/components/Lightbox.vue'
import TabPanel from '@/components/TabPanel.vue'
import ViewHeader from '@/components/ViewHeader.vue'

const installTab = ref('cargo')
const lightboxOpen = ref(false)

const demoImage = [
  { src: '/demo.gif', alt: 'whis CLI demo', caption: 'Record → Transcribe → Paste' },
]
</script>

<template>
  <div class="cli-content">
    <ViewHeader title="CLI" subtitle="Voice-to-text for terminal workflows" />

    <!-- Install -->
    <section id="install" class="install">
      <TabPanel
        v-model:selected="installTab"
        :tabs="[
          { value: 'cargo', label: 'cargo' },
          { value: 'aur', label: 'aur' },
          { value: 'source', label: 'source' },
        ]"
      >
        <div v-if="installTab === 'cargo'" class="panel">
          <CommandCopy
            :segments="[
              { text: 'cargo install ' },
              { text: 'whis', highlight: true },
            ]"
          />
        </div>
        <div v-else-if="installTab === 'aur'" class="panel">
          <CommandCopy
            :segments="[
              { text: 'yay -S ' },
              { text: 'whis', highlight: true },
            ]"
          />
        </div>
        <div v-else class="panel">
          <CommandCopy
            :segments="[
              { text: 'git clone ' },
              { text: 'https://github.com/frankdierolf/whis', highlight: true },
              { text: ' && cd whis && ' },
              { text: 'cargo build --release', highlight: true },
            ]"
          />
        </div>
      </TabPanel>
      <p class="install-note">
        <RouterLink to="/downloads">
          More options →
        </RouterLink>
      </p>
    </section>

    <!-- Features -->
    <section class="features">
      <div class="section-header">
        <h2>What is whis?</h2>
        <p>A minimal CLI that pipes your voice straight to the clipboard.</p>
      </div>
      <ul>
        <li>
          <span class="marker">[*]</span>
          <div><strong>One command</strong> Run whis. Speak. Done.</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>6 providers</strong> OpenAI, Groq, Deepgram, Mistral, ElevenLabs, or local whisper.cpp</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>Run locally</strong> Fully offline with whisper.cpp — private and free</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>AI post-processing</strong> Clean up filler words and grammar with LLMs</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>Presets</strong> ai-prompt, email, default — or create your own</div>
        </li>
        <li>
          <span class="marker">[*]</span>
          <div><strong>Hotkey mode</strong> Ctrl+Alt+W toggles recording from anywhere</div>
        </li>
      </ul>
    </section>

    <!-- Demo -->
    <section class="demo">
      <figure>
        <img
          src="/demo.gif"
          alt="whis demo: run whis command, speak, press Enter, text is copied to clipboard"
          loading="lazy"
          class="clickable"
          @click="lightboxOpen = true"
        >
        <figcaption>Record &rarr; Transcribe &rarr; Paste</figcaption>
      </figure>
    </section>

    <!-- Lightbox -->
    <Lightbox v-model:open="lightboxOpen" :images="demoImage" :initial-index="0" />

    <!-- Quick Start -->
    <section class="quickstart">
      <h2>Quick Start</h2>
      <pre><code><span class="comment"># Cloud setup (guided wizard)</span>
<span class="highlight">whis setup cloud</span>

<span class="comment"># Or go fully local (private, free)</span>
<span class="highlight">whis setup local</span>

<span class="comment"># Then just run:</span>
<span class="highlight">whis</span>
<span class="comment"># Press Enter to stop — text is copied!</span>

<span class="comment"># Post-process your transcript with AI:</span>
<span class="highlight">whis --post-process</span></code></pre>
    </section>
  </div>
</template>

<style scoped>
.cli-content {
  padding: 2rem;
}

.install {
  padding: 2rem 0;
}

.panel {
  display: block;
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
