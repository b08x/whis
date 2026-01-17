// NOTE: Build shows "[lightningcss minify] 'deep' is not recognized" warning.
// This is a false positive - :deep() is valid Vue 3 scoped CSS syntax.
// The fix exists in vite-plugin-vue (PR #521) but doesn't work with rolldown-vite.
// Tracking: https://github.com/vitejs/rolldown-vite/issues/573

import process from 'node:process'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    host: host || '0.0.0.0',
    port: 5173,
    strictPort: true,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 5174,
        }
      : undefined,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2021',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
