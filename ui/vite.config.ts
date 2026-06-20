import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/status': 'http://localhost:7700',
      '/transmitters': 'http://localhost:7700',
      '/viewers': 'http://localhost:7700',
      '/spout-senders': 'http://localhost:7700',
      '/logs': 'http://localhost:7700',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
