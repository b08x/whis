<script setup lang="ts">
import { onMounted, ref } from 'vue'
import TerminalDemo from '@/components/TerminalDemo.vue'

const stars = ref<number | null>(null)
const downloads = ref<number | null>(null)
const contributors = ref<{ login: string, avatar_url: string }[]>([])

onMounted(async () => {
  // Fetch GitHub stars
  try {
    const gh = await fetch('https://api.github.com/repos/frankdierolf/whis')
    if (gh.ok) {
      const data = await gh.json()
      stars.value = data.stargazers_count
    }
  }
  catch {
    /* silent fail */
  }

  // Fetch cargo downloads
  try {
    const cargo = await fetch('https://crates.io/api/v1/crates/whis')
    if (cargo.ok) {
      const data = await cargo.json()
      downloads.value = data.crate.downloads
    }
  }
  catch {
    /* silent fail */
  }

  // Fetch contributors
  try {
    const contribs = await fetch('https://api.github.com/repos/frankdierolf/whis/contributors')
    if (contribs.ok) {
      contributors.value = await contribs.json()
    }
  }
  catch {
    /* silent fail */
  }
})

function formatNumber(n: number): string {
  if (n >= 1000)
    return `${(n / 1000).toFixed(1)}k`
  return n.toString()
}
</script>

<template>
  <div class="home">
    <div class="hero">
      <h1 class="tagline">
        <span>Speak.</span>
        <span>Paste.</span>
        <span>Ship.</span>
      </h1>
      <p class="subtitle">
        Voice → clipboard. Zero friction.
      </p>
      <p class="description">
        Record anywhere. Transcribe fast. Paste instantly.
      </p>
    </div>

    <div class="cta-group">
      <RouterLink to="/downloads" class="cta-primary">
        <span class="cta-icon">↓</span>
        Download
      </RouterLink>
      <a
        href="https://github.com/frankdierolf/whis"
        target="_blank"
        rel="noopener"
        class="cta-secondary"
      >
        View on GitHub
      </a>
    </div>

    <div class="proof">
      <span class="stat">★ {{ stars ?? '—' }} stars</span>
      <span class="divider">·</span>
      <span class="stat">{{ downloads ? formatNumber(downloads) : '—' }} installs</span>
      <span class="divider">·</span>
      <span v-if="contributors.length" class="stat contributors">
        <span class="avatars">
          <img
            v-for="c in contributors"
            :key="c.login"
            :src="c.avatar_url"
            :alt="c.login"
          >
        </span>
        {{ contributors.length }} contributors
      </span>
      <span v-if="contributors.length" class="divider">·</span>
      <span class="stat">MIT license</span>
    </div>

    <TerminalDemo />
  </div>
</template>

<style scoped>
.home {
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-height: 100%;
  padding: 3rem;
  gap: 2.5rem;
}

.hero {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.tagline {
  font-size: clamp(2rem, 5vw, 3rem);
  font-weight: 700;
  color: var(--accent);
  letter-spacing: -0.03em;
  line-height: 1.1;
  display: flex;
  gap: 0.4em;
}

.tagline span {
  display: inline-block;
}

@keyframes reveal {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.subtitle {
  font-size: 1.25rem;
  color: var(--text-strong);
  font-weight: 500;
}

.description {
  font-size: 0.95rem;
  color: var(--text);
  max-width: 28rem;
  line-height: 1.6;
}

.cta-group {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
}

.cta-primary {
  display: inline-flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1.5rem;
  background: var(--bg-strong);
  color: var(--text-inverted);
  border: none;
  border-radius: 4px;
  font-family: var(--font);
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  text-decoration: none;
  transition: all 0.15s ease;
}

.cta-primary:hover {
  background: var(--bg-strong-hover);
  transform: translateX(2px);
}

.cta-icon {
  font-size: 1.1rem;
}

.cta-secondary {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1.5rem;
  background: transparent;
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 0.9rem;
  font-weight: 500;
  text-decoration: none;
  transition: all 0.15s ease;
}

.cta-secondary:hover {
  background: var(--bg-weak);
  border-color: var(--text-weak);
  color: var(--text-strong);
}

.proof {
  display: flex;
  gap: 0.75rem;
  font-size: 0.8rem;
  color: var(--text-weak);
}

.stat {
  white-space: nowrap;
}

.divider {
  opacity: 0.4;
}

.contributors {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.avatars {
  display: inline-flex;
}

.avatars img {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--bg);
  margin-left: -8px;
}

.avatars img:first-child {
  margin-left: 0;
}

@media (max-width: 768px) {
  .home {
    padding: 2rem;
    gap: 2rem;
  }

  .tagline {
    font-size: 1.75rem;
    flex-wrap: wrap;
    gap: 0.2em;
  }

  .proof {
    flex-wrap: wrap;
  }
}
</style>
