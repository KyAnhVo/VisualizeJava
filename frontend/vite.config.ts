import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import path from "path"

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    // Build to <repo>/doc rather than demo/dist so the bundle sits at the
    // project root, where a static host can serve it directly.
    outDir: fileURLToPath(new URL('../docs', import.meta.url)),
    // outDir is outside Vite's root (demo/), and Vite refuses to clear such a
    // directory unless told to explicitly. Without this, stale hashed assets
    // from previous builds accumulate forever.
    emptyOutDir: true,
  },
})
