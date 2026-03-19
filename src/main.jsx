import React from 'react'
import ReactDOM from 'react-dom/client'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.jsx'
import { ThemeProvider } from './contexts/ThemeContext.jsx'
import { DialogProvider } from './contexts/DialogContext.jsx'
import { AppSettingsProvider } from './contexts/AppSettingsContext.jsx'
import { I18nProvider } from './i18n.jsx'
import '@mantine/core/styles.css'
import './index.css'

// 生产环境禁用浏览器快捷键
if (import.meta.env.PROD) {
  document.addEventListener('keydown', (e) => {
    // F5 - 刷新
    // F12 - 开发者工具
    if (e.key === 'F5' || e.key === 'F12') {
      e.preventDefault()
      return false
    }
    
    // Ctrl 组合键
    if (e.ctrlKey) {
      const key = e.key.toLowerCase()
      // Ctrl+R - 刷新
      // Ctrl+U - 查看源码
      // Ctrl+P - 打印
      // Ctrl+S - 保存
      // Ctrl+G - 查找
      // Ctrl+F - 页面搜索
      if (['r', 'u', 'p', 's', 'g', 'f'].includes(key)) {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+I/J - 开发者工具
      if (e.shiftKey && ['i', 'j'].includes(key)) {
        e.preventDefault()
        return false
      }
    }
  })
  
  // 禁用右键菜单
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault()
    return false
  })
}

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <I18nProvider>
      <AppSettingsProvider>
        <ThemeProvider>
          <DialogProvider>
            <App />
          </DialogProvider>
        </ThemeProvider>
      </AppSettingsProvider>
    </I18nProvider>
  </React.StrictMode>,
)

// 页面加载完成后显示窗口
const hasCurrentTauriWindow = () => Boolean(window.__TAURI_INTERNALS__?.metadata?.currentWindow)

document.addEventListener('DOMContentLoaded', () => {
  if (!hasCurrentTauriWindow()) return

  setTimeout(() => {
    getCurrentWindow().show().catch?.(() => {})
  }, 100)
})
