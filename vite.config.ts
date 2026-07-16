import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import ui from '@nuxt/ui/vite'
import { resolve } from 'path'

export default defineConfig({
  plugins: [
    vue(),
    ui({
      ui: {
        colors: {
          primary: 'primary',
          secondary: 'info',
          neutral: 'neutral',
          success: 'success',
          warning: 'warning',
          error: 'error',
          info: 'info',
        },
      },
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  clearScreen: false,
  server: {
    port: 3000,
    strictPort: true,
    watch: {
      // Cargo 빌드 산출물(target/)을 Vite가 감시하면 Windows에서
      // 링커가 lock 중인 .exe 파일을 watch 시도해 EBUSY 크래시가 발생한다.
      ignored: ['**/target/**', '**/src-tauri/target/**', '**/dist/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2021',
    outDir: 'dist',
  },
})
