<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { settingsStore } from './stores/settings'

const route = useRoute()
const loaded = computed(() => settingsStore.state.loaded)
const sidebarOpen = ref(false)

const navItems = [
  { path: '/', name: 'home', label: 'home' },
  { path: '/presets', name: 'presets', label: 'presets' },
  { path: '/settings', name: 'settings', label: 'settings' },
  { path: '/about', name: 'about', label: 'about' },
]

const currentPageLabel = computed(() => {
  const item = navItems.find(i => i.name === route.name)
  return item?.label ?? 'whis'
})

function toggleSidebar() {
  sidebarOpen.value = !sidebarOpen.value
}

function closeSidebar() {
  sidebarOpen.value = false
}

// Close sidebar on route change
watch(() => route.path, () => {
  closeSidebar()
})

onMounted(() => {
  settingsStore.initialize()
})
</script>

<template>
  <div class="app" :class="{ loaded, 'sidebar-open': sidebarOpen }">
    <!-- Mobile Header -->
    <header class="mobile-header">
      <button
        class="menu-toggle"
        :aria-expanded="sidebarOpen"
        aria-label="Toggle menu"
        @click="toggleSidebar"
      >
        <span>{{ sidebarOpen ? '[x]' : '[=]' }}</span>
      </button>
      <span class="mobile-brand">{{ currentPageLabel }}</span>
    </header>

    <!-- Backdrop -->
    <div class="backdrop" @click="closeSidebar" />

    <!-- Sidebar Drawer -->
    <aside class="sidebar">
      <nav class="nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.name"
          :to="item.path"
          class="nav-item"
          :class="{ active: route.name === item.name }"
          @click="closeSidebar"
        >
          <span class="nav-marker" aria-hidden="true">{{
            route.name === item.name ? '>' : ' '
          }}</span>
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <div class="sidebar-footer">
        <span class="version-badge">v0.5.9</span>
      </div>
    </aside>

    <!-- Content -->
    <main class="content">
      <router-view />
    </main>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  background: var(--bg);
  opacity: 0;
  transition: opacity 0.15s ease;
}

.app.loaded {
  opacity: 1;
}

/* Mobile Header - always visible on mobile app */
.mobile-header {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: calc(48px + env(safe-area-inset-top, 0px));
  padding-top: env(safe-area-inset-top, 0px);
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding-left: 16px;
  padding-right: 16px;
  gap: 12px;
  z-index: 101;
}

.menu-toggle {
  all: unset;
  font-family: var(--font);
  font-size: 14px;
  color: var(--text);
  cursor: pointer;
  padding: 8px 4px;
  min-width: var(--touch-target-min);
  min-height: var(--touch-target-min);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s ease;
}

.menu-toggle:active {
  color: var(--accent);
}

.mobile-brand {
  font-family: var(--font);
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-strong);
  letter-spacing: -0.03em;
}

/* Backdrop */
.backdrop {
  position: fixed;
  inset: 0;
  top: calc(48px + env(safe-area-inset-top, 0px));
  background: rgba(0, 0, 0, 0.6);
  z-index: 99;
  opacity: 0;
  transition: opacity 0.2s ease;
  pointer-events: none;
}

.sidebar-open .backdrop {
  opacity: 1;
  pointer-events: auto;
}

/* Sidebar Drawer */
.sidebar {
  position: fixed;
  top: calc(48px + env(safe-area-inset-top, 0px));
  left: 0;
  bottom: 0;
  width: 220px;
  background: var(--bg);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  transform: translateX(-100%);
  transition: transform 0.25s ease;
  z-index: 100;
  padding-bottom: env(safe-area-inset-bottom, 0px);
}

.sidebar-open .sidebar {
  transform: translateX(0);
}

.nav {
  display: flex;
  flex-direction: column;
  flex: 1;
  padding-top: 8px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 20px;
  border-left: 2px solid transparent;
  color: var(--text-weak);
  font-family: var(--font);
  font-size: 14px;
  min-height: var(--touch-target-min);
  cursor: pointer;
  text-decoration: none;
  transition: all 0.15s ease;
}

.nav-item:active {
  color: var(--text-strong);
  background: var(--bg-weak);
}

.nav-item.active {
  color: var(--accent);
  border-left-color: var(--accent);
}

.nav-marker {
  color: var(--icon);
  font-weight: 400;
  width: 0.75em;
}

.nav-item.active .nav-marker {
  color: var(--accent);
}

.sidebar-footer {
  padding: 16px 20px;
  border-top: 1px solid var(--border);
  margin-top: auto;
}

.version-badge {
  font-size: 12px;
  color: var(--text-weak);
  opacity: 0.6;
}

/* Content */
.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  margin-top: calc(48px + env(safe-area-inset-top, 0px));
  min-height: calc(100vh - 48px - env(safe-area-inset-top, 0px));
  overflow-y: auto;
}
</style>
