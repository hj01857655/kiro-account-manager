import { useState, useEffect, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { Loader } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { Button } from '../../shared/button'
import { Input } from '../../ui/input'
import { Label } from '../../ui/label'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '../../ui/dialog'

const DEFAULT_PROVIDER_ORDER = ['Google', 'Github', 'BuilderId', 'Enterprise']

interface ProviderMeta {
  name: string
  icon: React.ReactNode
}

function getProviderMeta(provider: string, t: any): ProviderMeta {
  const providers: Record<string, ProviderMeta> = {
    Google: {
      name: t('login.google'),
      icon: (
        <svg width="28" height="28" viewBox="0 0 24 24">
          <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4" />
          <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853" />
          <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05" />
          <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335" />
        </svg>
      ),
    },
    Github: {
      name: t('login.github'),
      icon: (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor" className="text-foreground">
          <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
        </svg>
      ),
    },
    BuilderId: {
      name: t('login.builderId'),
      icon: <span className="text-[#ff9900] font-bold text-lg">aws</span>,
    },
    Enterprise: {
      name: t('login.idc'),
      icon: <span className="text-[#ff9900] font-bold text-lg">aws</span>,
    },
  }

  return providers[provider] || {
    name: provider,
    icon: <span className="text-base font-bold text-foreground">{provider[0] || '?'}</span>,
  }
}

interface LoginProps {
  onLogin?: () => void
}

const AGREEMENT_LINKS = [
  { key: 'awsAgreement', href: 'https://aws.amazon.com/agreement/' },
  { key: 'serviceTerms', href: 'https://aws.amazon.com/service-terms/' },
  { key: 'privacy', href: 'https://aws.amazon.com/privacy/' },
  { key: 'ipLicense', href: 'https://aws.amazon.com/legal/aws-ip-license-terms/' },
] as const

function Login({ onLogin }: LoginProps) {
  const { t } = useApp()

  const [supportedProviders, setSupportedProviders] = useState<string[]>(DEFAULT_PROVIDER_ORDER)
  const [loadingProvider, setLoadingProvider] = useState<string | null>(null)
  const [loginPending, setLoginPending] = useState(false)
  const [canceling, setCanceling] = useState(false)
  const [error, setError] = useState('')
  const [showEnterpriseModal, setShowEnterpriseModal] = useState(false)
  const [enterpriseStartUrl, setEnterpriseStartUrl] = useState('')
  const [enterpriseRegion, setEnterpriseRegion] = useState('us-east-1')
  const [showWaitingModal, setShowWaitingModal] = useState(false)
  const [waitingProviderName, setWaitingProviderName] = useState('')

  useEffect(() => {
    let unlistenSuccess: UnlistenFn | undefined

    const setupListener = async () => {
      unlistenSuccess = await listen('login-success', () => {
        setLoginPending(false)
        setLoadingProvider(null)
        setShowWaitingModal(false)
        onLogin?.()
      })
    }

    setupListener()

    return () => {
      if (unlistenSuccess) unlistenSuccess()
    }
  }, [onLogin])

  useEffect(() => {
    let mounted = true

    const loadSupportedProviders = async () => {
      try {
        const providers = await invoke<string[]>('get_supported_providers')
        if (!mounted || !Array.isArray(providers) || providers.length === 0) return

        const normalizedProviders = [
          ...DEFAULT_PROVIDER_ORDER.filter((provider) => providers.includes(provider)),
          ...providers.filter((provider) => !DEFAULT_PROVIDER_ORDER.includes(provider)),
        ]
        setSupportedProviders(normalizedProviders)
      } catch (e) {
        console.error('Failed to load supported providers:', e)
      }
    }

    loadSupportedProviders()
    return () => { mounted = false }
  }, [])

  const getLoginErrorMessage = (e: any) => {
    const rawMessage = typeof e === 'string' ? e : e?.message || t('login.failed')
    if (rawMessage.includes('登录已取消') || rawMessage.toLowerCase().includes('cancel')) {
      return t('login.cancelled')
    }
    return rawMessage
  }

  const handleLogin = async (provider: string) => {
    if (loginPending) return

    if (provider === 'Enterprise') {
      setError('')
      setShowEnterpriseModal(true)
      return
    }

    const providerMeta = getProviderMeta(provider, t)
    setWaitingProviderName(providerMeta.name)
    setShowWaitingModal(true)
    setLoginPending(true)
    setLoadingProvider(provider)
    setError('')

    try {
      await invoke('kiro_login', { provider })
      onLogin?.()
    } catch (e) {
      console.error('Login error:', e)
      setError(getLoginErrorMessage(e))
    } finally {
      setCanceling(false)
      setLoginPending(false)
      setLoadingProvider(null)
      setShowWaitingModal(false)
    }
  }

  const handleCancelLogin = async () => {
    if (!loginPending || canceling) {
      setShowWaitingModal(false)
      return
    }
    setCanceling(true)
    try {
      await invoke('cancel_kiro_login')
    } catch (e) {
      console.error('Cancel login error:', e)
      setCanceling(false)
      setError(t('login.cancelFailed'))
    }
  }

  const handleEnterpriseLogin = async () => {
    const normalizedStartUrl = enterpriseStartUrl.trim()
    const normalizedRegion = enterpriseRegion.trim() || 'us-east-1'

    if (!normalizedStartUrl) {
      setError(t('login.startUrlRequired'))
      return
    }
    if (!/^https:\/\//i.test(normalizedStartUrl)) {
      setError(t('login.startUrlInvalid'))
      return
    }

    setShowEnterpriseModal(false)
    setWaitingProviderName(t('login.idc'))
    setShowWaitingModal(true)
    setLoginPending(true)
    setLoadingProvider('Enterprise')
    setError('')

    try {
      await invoke('kiro_login', {
        provider: 'Enterprise',
        startUrl: normalizedStartUrl,
        region: normalizedRegion,
      })
      onLogin?.()
    } catch (e) {
      console.error('Login error:', e)
      setError(getLoginErrorMessage(e))
    } finally {
      setCanceling(false)
      setLoginPending(false)
      setLoadingProvider(null)
      setShowWaitingModal(false)
    }
  }

  const providers = useMemo(
    () => supportedProviders.map((provider) => ({ id: provider, ...getProviderMeta(provider, t) })),
    [supportedProviders, t],
  )

  return (
    <div className="h-full flex flex-col items-center justify-center glass-main relative overflow-hidden p-6">
      <div className="relative z-10 w-full max-w-md flex flex-col items-center">
        {/* Logo + 标题 */}
        <div className="flex items-center gap-3 mb-8">
          <div className="w-14 h-14 rounded-xl bg-gradient-to-br from-primary/80 to-primary flex items-center justify-center shadow-md ring-1 ring-primary/20">
            <svg width="32" height="32" viewBox="0 0 40 40" fill="none">
              <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white" />
            </svg>
          </div>
          <span className="text-3xl font-bold text-foreground tracking-wide">KIRO</span>
        </div>

        {/* 副标题 */}
        <h1 className="text-base text-muted-foreground text-center mb-6">{t('login.subtitle')}</h1>

        {/* 错误条 */}
        {error && (
          <div className="w-full mb-5 p-4 rounded-lg text-sm text-center bg-destructive/10 text-destructive border border-destructive/20">
            {error}
          </div>
        )}

        {/* 登录按钮列表（居中独立按钮，不贴满容器）*/}
        <div className="flex flex-col items-center gap-3">
          {providers.map((provider) => {
            const isLoading = loadingProvider === provider.id
            const isDisabled = loadingProvider !== null && !isLoading
            return (
              <button
                key={provider.id}
                onClick={() => handleLogin(provider.id)}
                disabled={loginPending}
                className={`group relative h-14 px-8 min-w-[280px] rounded-xl glass-card border border-border flex items-center justify-center gap-3 transition-all duration-200 ${
                  isLoading
                    ? 'opacity-60 cursor-not-allowed'
                    : 'hover:bg-muted/50 hover:border-primary/40 hover:shadow-sm'
                } ${isDisabled ? 'opacity-30' : ''}`}
              >
                {isLoading ? <Loader size={22} className="text-primary animate-spin" /> : provider.icon}
                <span className="text-base font-medium text-foreground">
                  {isLoading ? t('login.logging') : provider.name}
                </span>
              </button>
            )
          })}
        </div>

        {/* 协议说明 */}
        <div className="text-xs text-muted-foreground text-center leading-relaxed mt-8 max-w-[480px]">
          {t('login.agreement')}{' '}
          {AGREEMENT_LINKS.map((link, i) => (
            <span key={link.key}>
              <a
                href={link.href}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                {t(`login.${link.key}`)}
              </a>
              {i < AGREEMENT_LINKS.length - 2 ? '、' : i === AGREEMENT_LINKS.length - 2 ? t('login.and') : ''}
            </span>
          ))}
          。
        </div>
      </div>

      {/* Enterprise Start URL 输入弹窗 */}
      <Dialog open={showEnterpriseModal} onOpenChange={setShowEnterpriseModal}>
        <DialogContent className="max-w-[460px]">
          <DialogHeader>
            <DialogTitle>{t('login.idc')}</DialogTitle>
            <DialogDescription>{t('login.enterprisePrompt')}</DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-2">
            <div>
              <Label className="text-xs text-muted-foreground mb-1 block">{t('login.startUrl')}</Label>
              <Input
                value={enterpriseStartUrl}
                onChange={(e) => setEnterpriseStartUrl(e.target.value)}
                placeholder="https://d-1234567890.awsapps.com/start"
                className="h-9 text-sm font-mono"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleEnterpriseLogin()
                  if (e.key === 'Escape') setShowEnterpriseModal(false)
                }}
              />
              <p className="text-[11px] text-muted-foreground mt-1">{t('login.startUrlExample')}</p>
            </div>
            <div>
              <Label className="text-xs text-muted-foreground mb-1 block">{t('login.region')}</Label>
              <Input
                value={enterpriseRegion}
                onChange={(e) => setEnterpriseRegion(e.target.value)}
                placeholder="us-east-1"
                className="h-9 text-sm font-mono"
              />
              <p className="text-[11px] text-muted-foreground mt-1">{t('login.regionExample')}</p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setShowEnterpriseModal(false)}>
              {t('login.cancel')}
            </Button>
            <Button onClick={handleEnterpriseLogin}>{t('common.continue')}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 等待授权弹窗 */}
      <Dialog open={showWaitingModal} onOpenChange={(open: boolean) => !open && handleCancelLogin()}>
        <DialogContent className="max-w-[400px]">
          <DialogHeader>
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center flex-shrink-0">
                <Loader size={20} className="text-primary animate-spin" />
              </div>
              <DialogTitle>{t('login.waitingTitle')}</DialogTitle>
            </div>
          </DialogHeader>
          <div className="space-y-2 py-2">
            <p className="text-sm text-foreground leading-relaxed">
              {t('login.waitingMessage', { provider: waitingProviderName })}
            </p>
            <p className="text-xs text-muted-foreground">{t('login.waitingHint')}</p>
            <p className="text-xs text-muted-foreground">{t('login.waitingCloseHint')}</p>
          </div>
          <DialogFooter>
            <Button variant="secondary" onClick={handleCancelLogin} disabled={canceling}>
              {canceling ? t('login.cancelling') : t('login.cancel')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default Login
