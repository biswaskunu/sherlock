import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      // Same-origin dev option: fetch('/api/metrics/live') proxies to Axum.
      '/api': {
        target: process.env.VITE_BACKEND_URL ?? 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
    },
  },
});
