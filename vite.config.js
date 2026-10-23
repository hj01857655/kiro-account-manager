import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'
import pkg from './package.json'

export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  plugins: [
    react(),
    tailwindcss(),
  ],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // 优化开发服务器性能
    hmr: {
      overlay: false, // 禁用错误覆盖层，减少渲染开销
    },
    // 预热常用文件，加快首次加载
    //
    // 注意：不要在这里列 App.tsx。它会 import 全部路由（懒加载 chunk 的
    // import 语句仍然会被解析），warmup 会把整条依赖链在 dev server
    // 就绪前同步预转换，导致 index.html 首次响应等 14-50 秒（实测）。
    // 只预热入口本身，其余交给按需编译。
    warmup: {
      clientFiles: ['./src/main.tsx'],
    },
  },
  // 优化依赖预构建
  optimizeDeps: {
    include: [
      'react',
      'react-dom',
      'lucide-react',
      '@tauri-apps/api',
      'i18next',
      'react-i18next',
    ],
    // 强制预构建，避免首次启动慢
    force: false,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'terser' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    terserOptions: {
      compress: {
        drop_console: true,      // 移除 console.log
        drop_debugger: true,     // 移除 debugger
        pure_funcs: ['console.info', 'console.debug', 'console.warn'],
      },
      mangle: {
        toplevel: true,          // 混淆顶级变量名
        safari10: true,
      },
      format: {
        comments: false,         // 移除所有注释
      },
    },
    // 代码分割优化
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor': ['react', 'react-dom'],
          'icons': ['lucide-react'],
          'tauri': [
            '@tauri-apps/api',
            '@tauri-apps/plugin-dialog',
            '@tauri-apps/plugin-fs',
            '@tauri-apps/plugin-opener',
            '@tauri-apps/plugin-process',
            '@tauri-apps/plugin-shell',
            '@tauri-apps/plugin-updater'
          ],
          'i18n': ['i18next', 'react-i18next'],
        }
      }
    },
    // 大项目可关闭压缩报告加速构建
    reportCompressedSize: false,
  },
})
