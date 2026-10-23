import ReactDOM from 'react-dom/client'
import './index.css'
import App from './App'
import { ThemeProvider } from './components/theme-provider'
import { DialogProvider } from './contexts/DialogContext'
import { AppSettingsProvider } from './contexts/AppSettingsContext'
import { I18nProvider } from './i18n'
import { TooltipProvider } from '@/components/ui/tooltip'
import { THEME_KEYS } from './lib/themeRegistry'
import { scheduleBootSplashFallback } from './utils/bootSplash'
import { applyOsAttribute } from './utils/os'
import { showMainWindow } from './api/systemApi'

// 撤卡兜底：React 若始终没挂载，启动卡会永久盖住窗口。必须在 render 之前注册，
// 这样渲染抛错时也能撤。不能放进 index.html 的内联 script——生产 CSP 会拦掉。
// 兜底触发时同步点亮窗口：否则卡片撤了、窗口仍 visible:false，用户什么也看不到。
scheduleBootSplashFallback(document, undefined, () => {
  void showMainWindow().catch((e) => console.error('boot fallback show window failed:', e))
})

// 首屏 chunk 预热：路由是 lazy() 的，dev 模式下 Vite 要现场转译整条依赖链，
// 首次冷启动可能等几十秒。这里在 render 前就发起请求，与 React 启动并行，
// 让编译/下载提前开始（生产构建里只是命中预加载，代价可忽略）。
void import('./components/features/Home/index').catch(() => {
  /* 预热失败不影响正常路由加载：lazy() 会自己重试并走 Suspense fallback */
})

// 平台标记：给 <html> 打 data-os，供 CSS 区分 macOS 交通灯避让与滚动条样式。
// 同样必须早于 render——晚一步会在 macOS 上闪一帧无避让的布局。
// CSS 层无法胜任：Tauri 三端里 macOS 与 Linux 同为 WebKit 系，
// 任何特性查询都会把 Linux 误判成 macOS（详见 src/utils/os.ts 注释）。
applyOsAttribute()

// 生产环境禁用浏览器快捷键
if (import.meta.env.PROD) {
  document.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'F5' || e.key === 'F12') {
      e.preventDefault()
    }
    if (e.ctrlKey) {
      const key = e.key.toLowerCase()
      if (['r', 'u', 'p', 's', 'g', 'f'].includes(key)) {
        e.preventDefault()
      }
      if (e.shiftKey && ['i', 'j'].includes(key)) {
        e.preventDefault()
      }
    }
  })

  document.addEventListener('contextmenu', (e: MouseEvent) => {
    e.preventDefault()
  })
}

const rootElement = document.getElementById('root')
if (!rootElement) throw new Error('Failed to find the root element')

ReactDOM.createRoot(rootElement).render(
  <I18nProvider>
    <AppSettingsProvider>
      <ThemeProvider
        attribute="data-theme"
        defaultTheme="dark"
        // 开启系统跟随：theme='system' 时 next-themes 会按 prefers-color-scheme
        // 把 data-theme 解析成 'light' 或 'dark'（见外观设置里的「跟随系统」选项）。
        enableSystem
        disableTransitionOnChange
        // 主题清单来自统一注册表；system 由 enableSystem 处理，不放进 themes
        themes={THEME_KEYS.filter((k) => k !== 'system')}
      >
        <TooltipProvider>
          <DialogProvider>
            <App />
          </DialogProvider>
        </TooltipProvider>
      </ThemeProvider>
    </AppSettingsProvider>
  </I18nProvider>,
)
