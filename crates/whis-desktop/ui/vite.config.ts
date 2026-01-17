// NOTE: Build shows "[lightningcss minify] 'deep' is not recognized" warning.
// This is a false positive - :deep() is valid Vue 3 scoped CSS syntax.
// The fix exists in vite-plugin-vue (PR #521) but doesn't work with rolldown-vite.
// Tracking: https://github.com/vitejs/rolldown-vite/issues/573

import { resolve } from 'node:path'
import process from 'node:process'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        bubble: resolve(__dirname, 'src/bubble/index.html'),
      },
    },
  },
})
