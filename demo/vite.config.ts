import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  optimizeDeps: {
    // elk-api.js is CommonJS; pre-bundling converts it to ESM. Its engine
    // (elk-worker.min.js) is deliberately excluded — it is loaded as a worker
    // entry via `?worker`, not imported. See lib/layout.ts.
    include: ['elkjs/lib/elk-api.js'],
    exclude: ['elkjs/lib/elk-worker.min.js'],
  },
})
