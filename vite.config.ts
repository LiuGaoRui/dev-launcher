import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'
import { resolve } from 'node:path'

// Tauri 期望的固定端口
const host = process.env.TAURI_DEV_HOST

// https://vitejs.dev/config/
export default defineConfig(() => ({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
    }),
    Components({
      resolvers: [ElementPlusResolver()],
    }),
  ],

  // Tauri 使用固定端口；如需更改请同步改 tauri.conf.json
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 忽略 Rust 后端文件变更（由 Tauri 自己处理重编译）
      ignored: ['**/src-tauri/**'],
    },
  },

  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },

  // Tauri 用环境变量注入资源前缀
  envPrefix: ['VITE_', 'TAURI_ENV_*'],

  build: {
    // Tauri 在 debug build 下支持 inline sourcemap
    target:
      process.env.TAURI_ENV_PLATFORM === 'windows'
        ? 'chrome105'
        : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}))
