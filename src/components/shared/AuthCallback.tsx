import { useEffect, useState, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../hooks/useApp'

export default function AuthCallback() {
  const { t} = useApp()
  const [status, setStatus] = useState('loading')
  const [message, setMessage] = useState('')
  
  const colors = useMemo(() => ({
    statusLoadingBorder: 'border-primary/40',
    statusSuccessBg: 'bg-emerald-500',
    statusErrorBg: 'bg-destructive'
  }), [])

  const closeCurrentPage = async () => {
    const currentUrl = new URL(window.location.href)
    if (currentUrl.pathname === '/callback') {
      // 如果在 /callback 路径，跳转到账号管理页面而不是关闭
      localStorage.setItem('activeMenu', 'accounts')
      currentUrl.pathname = '/'
      currentUrl.search = ''
      currentUrl.hash = ''
      window.location.replace(currentUrl.toString())
      return
    }
    
    // 优先尝试关闭 Tauri 窗口
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().close()
      return
    } catch (_) {
      // 如果不是 Tauri 环境，fallback 到普通关闭
    }

    // 尝试关闭浏览器窗口
    window.close()
    
    // 如果窗口未关闭，显示提示
    setTimeout(() => {
      if (!document.hidden) {
        setMessage(t('callback.autoCloseHint'))
      }
    }, 500)
  }

  useEffect(() => {
    // 后端 login_social 已完成 OAuth 流程（token 交换 + 保存账号）
    // 前端只需显示成功并跳转
    setStatus('success')
    setMessage(t('callback.success'))

    setTimeout(() => {
      closeCurrentPage()
    }, 1500)
  }, [t])

  const getStatusIcon = () => {
    switch (status) {
      case 'loading':
      case 'processing':
        return (
          <div className={`w-16 h-16 border-4 ${colors.statusLoadingBorder} border-t-transparent rounded-full animate-spin mx-auto mb-4`}></div>
        )
      case 'success':
        return (
          <div className={`w-16 h-16 ${colors.statusSuccessBg} rounded-full flex items-center justify-center mx-auto mb-4`}>
            <svg className="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          </div>
        )
      case 'error':
        return (
          <div className={`w-16 h-16 ${colors.statusErrorBg} rounded-full flex items-center justify-center mx-auto mb-4`}>
            <svg className="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </div>
        )
      default:
        return null
    }
  }

  const getStatusColor = () => {
    switch (status) {
      case 'success':
        return "text-foreground"
      case 'error':
        return "text-foreground"
      default:
        return "text-foreground"
    }
  }

  return (
    <div className={`min-h-screen flex items-center justify-center p-4 glass-main`}>
      <div className={`glass-card rounded-2xl shadow-xl p-8 max-w-md w-full border border-border`}>
        {getStatusIcon()}
        
        <h1 className={`text-2xl font-bold text-center mb-4 ${getStatusColor()}`}>
          {status === 'success' && t('callback.loginSuccess')}
          {status === 'error' && t('callback.loginFailed')}
          {(status === 'loading' || status === 'processing') && t('callback.processingTitle')}
        </h1>
        
        <p className={`text-muted-foreground text-center mb-6 leading-relaxed`}>
          {message}
        </p>

        {status === 'success' && (
          <div className="text-center">
            <p className={`text-sm text-muted-foreground mb-4`}>
              {t('callback.autoCloseHint')}
            </p>
            <button
              onClick={closeCurrentPage}
              className={`px-6 py-2 bg-primary text-primary-foreground rounded-lg transition-colors`}
            >
              {t('callback.closeWindow')}
            </button>
          </div>
        )}

        {status === 'error' && (
          <div className="text-center">
            <button
              onClick={closeCurrentPage}
              className={`px-6 py-2 bg-secondary text-secondary-foreground rounded-lg transition-colors`}
            >
              {t('callback.closeWindow')}
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
