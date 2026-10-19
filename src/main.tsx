import ReactDOM from 'react-dom/client'
import './index.css'
import App from './App'
import { ThemeProvider } from './components/theme-provider'
import { DialogProvider } from './contexts/DialogContext'
import { AppSettingsProvider } from './contexts/AppSettingsContext'
import { I18nProvider } from './i18n'
import { TooltipProvider } from '@/components/ui/tooltip'
import { THEME_KEYS } from './lib/themeRegistry'
import { applyOsAttribute } from './utils/os'

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
