import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../hooks/useApp'

export default function AuthCallback() {
  const { t} = useApp()
  const [status, setStatus] = useState('loading')
  const [message, setMessage] = useState('')

  const closeCurrentPage = async () => {
    // 优先关闭 Tauri 窗口；若当前在外部浏览器，window.close 可能被浏览器拦截
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().close()
      return
    } catch (_) {
      // ignore, fallback below
    }

    window.close()
    setTimeout(() => {
      if (!document.hidden) {
        setMessage('如果窗口未自动关闭，请手动关闭此页并返回应用。')
      }
    }, 500)
  }

  useEffect(() => {
    setMessage(t('callback.processing'))
    
    const handleCallback = async () => {
      try {
        const url = new URL(window.location.href)
        const code = url.searchParams.get('code')
        const state = url.searchParams.get('state')

        if (!code || !state) {
          setStatus('error')
          setMessage(t('callback.missingParams'))
          return
        }

        setStatus('processing')
        setMessage(t('callback.exchangingToken'))

        await invoke('handle_kiro_social_callback', { code, callbackState: state })

        setStatus('success')
        setMessage(t('callback.success'))

        setTimeout(() => {
          closeCurrentPage()
        }, 3000)

      } catch (error) {
        console.error('Callback error:', error)
        setStatus('error')
        setMessage(error.message || t('callback.failed'))
      }
    }

    handleCallback()
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
