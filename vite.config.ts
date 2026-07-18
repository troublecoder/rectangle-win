import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import ui from '@nuxt/ui/vite'
import { resolve } from 'path'

export default defineConfig({
  plugins: [
    vue(),
    ui(),
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
    rollupOptions: {
      output: {
        manualChunks: {
          // konva는 무거운 캔버스 라이브러리 — SnapEditor lazy import와 분리해
          // 다른 페이지 진입 시 로드되지 않도록 독립 청크로 분리.
          // (@nuxt/ui는 vite 플러그인이 자체 처리하므로 manualChunks에서 제외 —
          //  넣으면 tailwindcss/oxide .node 바이너리까지 끌어와 빌드 에러)
          konva: ['konva', 'vue-konva'],
        },
      },
    },
  },
})
