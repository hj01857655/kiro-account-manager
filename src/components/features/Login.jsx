import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Loader, ArrowRight } from 'lucide-react'
import { useApp } from '../../hooks/useApp'
import { Button } from '../ui/button'
import { getThemeAccent } from './KiroConfig/themeAccent'

function Login({ onLogin }) {
  const { t, colors, theme } = useApp()
  const accent = getThemeAccent(theme)
  const [loadingProvider, setLoadingProvider] = useState(null)
  const [error, setError] = useState('')
  const [showEnterpriseModal, setShowEnterpriseModal] = useState(false)
  const [enterpriseStartUrl, setEnterpriseStartUrl] = useState('')
  const [enterpriseRegion, setEnterpriseRegion] = useState('us-east-1')
  const [showWaitingModal, setShowWaitingModal] = useState(false)
  const [waitingProviderName, setWaitingProviderName] = useState('')

  useEffect(() => {
    let unlistenSuccess

    const setupListener = async () => {
      unlistenSuccess = await listen('login-success', (event) => {
        setLoadingProvider(null)
        setShowWaitingModal(false)
        onLogin?.(event.payload)
      })
    }

    setupListener()

    return () => { 
      if (unlistenSuccess) unlistenSuccess()
    }
  }, [onLogin])

  // Provider 显示名称映射
  const providerNames = {
    'Google': 'Google',
    'Github': 'Github',
    'BuilderId': 'Builder ID',
    'Enterprise': 'IAM Identity Center'
  }

  const handleLogin = async (provider) => {
    // Enterprise 需要用户输入 start_url
    if (provider === 'Enterprise') {
      setShowEnterpriseModal(true)
      return
    }

    // 显示等待授权弹窗
    setWaitingProviderName(providerNames[provider] || provider)
    setShowWaitingModal(true)
    setLoadingProvider(provider)
    setError('')
    
    try {
      await invoke('kiro_login', { provider })
    } catch (e) {
      console.error('Login error:', e)
      setError(typeof e === 'string' ? e : e.message || t('login.failed'))
      setLoadingProvider(null)
      setShowWaitingModal(false)
    }
  }

  const handleCancelLogin = () => {
    setShowWaitingModal(false)
    setLoadingProvider(null)
    setError('')
  }

  const handleEnterpriseLogin = async () => {
    if (!enterpriseStartUrl.trim()) {
      setError('请输入 Start URL')
      return
    }

    setShowEnterpriseModal(false)
    setWaitingProviderName(providerNames['Enterprise'])
    setShowWaitingModal(true)
    setLoadingProvider('Enterprise')
    setError('')
    
    try {
      await invoke('kiro_login', { 
        provider: 'Enterprise',
        startUrl: enterpriseStartUrl.trim(),
        region: enterpriseRegion
      })
    } catch (e) {
      console.error('Login error:', e)
      setError(typeof e === 'string' ? e : e.message || t('login.failed'))
      setLoadingProvider(null)
      setShowWaitingModal(false)
    }
  }

  const providers = [
    {
      id: 'Google',
      name: 'Google',
      icon: (
        <svg width="28" height="28" viewBox="0 0 24 24">
          <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
          <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
          <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/>
          <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/>
        </svg>
      ),
    },
    {
      id: 'Github',
      name: 'Github',
      icon: (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor" className={colors.text}>
          <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
        </svg>
      ),
    },
    {
      id: 'BuilderId',
      name: 'Builder ID',
      icon: (
        <span className="text-[#ff9900] font-bold text-xl">aws</span>
      ),
    },
    {
      id: 'Enterprise',
      name: 'IAM Identity Center',
      icon: (
        <span className="text-[#ff9900] font-bold text-xl">aws</span>
      ),
    },
  ]

  return (
    <div className={`h-full flex flex-col items-center justify-center ${colors.main} relative overflow-hidden`}>
      <div className="relative z-10 w-full flex flex-col items-center px-8">
        {/* Logo */}
        <div className="flex items-center justify-center gap-3" style={{ marginBottom: '50px' }}>
          <div className={`w-12 h-12 rounded-2xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-lg ${accent.shadow}`}>
            <svg width="32" height="32" viewBox="0 0 40 40" fill="none">
              <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white"/>
            </svg>
          </div>
          <span
            className={`text-4xl font-bold ${colors.text} tracking-wide`}
            style={{
              fontFamily: 'system-ui, -apple-system, sans-serif',
            }}
          >
            KIRO
          </span>
        </div>

        {/* Title */}
        <h1 className={`text-xl font-normal ${colors.textMuted} text-center`} style={{ marginBottom: '30px' }}>
          Choose a way to sign in/sign up
        </h1>

        {/* Error */}
        {error && (
          <div className={`mb-6 p-4 ${colors.error} border ${colors.errorBorder} rounded-xl text-sm text-center max-w-[375px]`}>
            {error}
          </div>
        )}

        {/* Buttons */}
        <div className="flex flex-col gap-5 w-full max-w-[375px]">
          {providers.map((provider) => (
            <button
              key={provider.id}
              onClick={() => handleLogin(provider.id)}
              disabled={!!loadingProvider}
              className={`
                group w-full h-[68px] px-8 rounded-xl
                ${colors.card} border-2 ${colors.cardBorder}
                flex items-center justify-center
                transition-all duration-200 relative
                ${loadingProvider === provider.id
                  ? 'opacity-50 cursor-not-allowed'
                  : `${colors.cardHover} hover:shadow-lg ${accent.hoverBorder}`
                }
                ${loadingProvider && loadingProvider !== provider.id ? 'opacity-30' : ''}
              `}
            >
              <div className="flex items-center gap-4">
                {loadingProvider === provider.id ? (
                  <Loader size={28} className={accent.text} />
                ) : (
                  provider.icon
                )}
                <span className={`text-xl font-normal ${colors.text}`}>
                  {provider.name}
                </span>
              </div>
              {loadingProvider === provider.id ? (
                <span className={`text-sm ${colors.textMuted} absolute right-8`}>Loading...</span>
              ) : (
                <div className={`flex items-center gap-2 ${colors.textMuted} opacity-0 group-hover:opacity-100 transition-opacity duration-200 absolute right-8`}>
                  <span className="text-sm">Sign in</span>
                  <ArrowRight size={16} />
                </div>
              )}
            </button>
          ))}
        </div>

        {/* Footer */}
        <div className={`text-xs ${colors.textMuted} text-center leading-relaxed max-w-[500px]`} style={{ marginTop: '50px' }}>
          By signing in and using Kiro, you agree to the{' '}
          <a href="https://aws.amazon.com/agreement/" target="_blank" rel="noopener noreferrer" className={`${accent.text} hover:underline`}>
            AWS Customer Agreement
          </a>
          {' '}(or other agreement with us governing your use of AWS services),{' '}
          <a href="https://aws.amazon.com/service-terms/" target="_blank" rel="noopener noreferrer" className={`${accent.text} hover:underline`}>
            Service Terms
          </a>
          ,{' '}
          <a href="https://aws.amazon.com/privacy/" target="_blank" rel="noopener noreferrer" className={`${accent.text} hover:underline`}>
            Privacy Notice
          </a>
          , and{' '}
          <a href="https://aws.amazon.com/legal/aws-ip-license-terms/" target="_blank" rel="noopener noreferrer" className={`${accent.text} hover:underline`}>
            AWS Intellectual Property License
          </a>
          .
        </div>
      </div>

      {/* Enterprise Start URL 输入弹窗 */}
      {showEnterpriseModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4 animate-fade-in">
          <div 
            className={`${colors.card} rounded-2xl w-full max-w-[480px] shadow-2xl border ${colors.cardBorder}`}
            style={{ animation: 'dialogSlideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1)' }}
          >
            {/* Header */}
            <div className="px-6 pt-6 pb-2">
              <h2 className={`text-xl font-semibold ${colors.text}`}>IAM Identity Center</h2>
              <p className={`text-sm ${colors.textMuted} mt-2`}>请输入您企业的 AWS IAM Identity Center Start URL</p>
            </div>
            
            {/* Content */}
            <div className="px-6 py-4 space-y-4">
              <div>
                <label className={`block text-sm font-medium ${colors.text} mb-2`}>Start URL</label>
                <input
                  type="text"
                  value={enterpriseStartUrl}
                  onChange={(e) => setEnterpriseStartUrl(e.target.value)}
                  placeholder="https://d-1234567890.awsapps.com/start"
                  className={`w-full px-4 py-3 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2 transition-all`}
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleEnterpriseLogin()
                    if (e.key === 'Escape') setShowEnterpriseModal(false)
                  }}
                />
                <p className={`text-xs ${colors.textMuted} mt-1.5`}>
                  示例: https://d-90661d346f.awsapps.com/start
                </p>
              </div>

              <div>
                <label className={`block text-sm font-medium ${colors.text} mb-2`}>AWS Region</label>
                <input
                  type="text"
                  value={enterpriseRegion}
                  onChange={(e) => setEnterpriseRegion(e.target.value)}
                  placeholder="us-east-1"
                  className={`w-full px-4 py-3 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2 transition-all`}
                />
                <p className={`text-xs ${colors.textMuted} mt-1.5`}>
                  示例: us-east-1, ap-southeast-2, eu-west-1
                </p>
              </div>
            </div>
            
            {/* Footer */}
            <div className={`px-6 py-4 ${colors.dialogFooter} flex justify-end gap-3`}>
              <Button
                variant="secondary"
                onClick={() => setShowEnterpriseModal(false)}
              >
                取消
              </Button>
              <Button
                variant="primary"
                onClick={handleEnterpriseLogin}
              >
                继续
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* 等待授权弹窗 */}
      {showWaitingModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4 animate-fade-in">
          <div 
            className={`${colors.card} rounded-2xl w-full max-w-[400px] shadow-2xl border ${colors.cardBorder}`}
            style={{ animation: 'dialogSlideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1)' }}
          >
            {/* Header */}
            <div className="px-6 pt-6 pb-2">
              <div className="flex items-center gap-4">
                <div className={`w-12 h-12 rounded-2xl ${accent.iconBadgeBg} flex items-center justify-center`}>
                  <Loader size={24} className={`${accent.text} animate-spin`} />
                </div>
                <h2 className={`text-lg font-semibold ${colors.text}`}>等待授权</h2>
              </div>
            </div>
            
            {/* Content */}
            <div className="px-6 py-4">
              <p className={`text-sm ${colors.text} leading-relaxed`}>
                正在等待您在浏览器中完成 {waitingProviderName} 授权...
              </p>
              <p className={`text-xs ${colors.textMuted} mt-2`}>
                如果浏览器未自动打开，请手动打开授权页面
              </p>
            </div>
            
            {/* Footer */}
            <div className={`px-6 py-4 ${colors.dialogFooter} flex justify-end`}>
              <Button
                variant="secondary"
                onClick={handleCancelLogin}
              >
                取消
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default Login
