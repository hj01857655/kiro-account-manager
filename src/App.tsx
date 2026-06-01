import { useState, useEffect, Suspense, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { Toaster } from 'react-hot-toast'
import Sidebar from './components/features/Layout'
import UpdateChecker from './components/shared/UpdateChecker'
import WelcomeModal from './components/shared/WelcomeModal'
import { dismissBootSplash } from './utils/bootSplash'

import { useApp } from './hooks/useApp'
import { useAppSettings } from './contexts/AppSettingsContext'
import { useDialog } from './contexts/DialogContext'
import { AccountProvider } from './contexts/AccountContext'
import { PrivacyProvider } from './contexts/PrivacyContext'
import { routes, internalRoutes } from './routes'
import { getMountedRouteIds, shouldPersistRoute } from './utils/routePersistence'

// Build route maps.
const routeMap = Object.fromEntries(routes.map(r => [r.id, r.component]))
const allRoutes = { ...routeMap, ...internalRoutes }

function PageLoading() {
  const { t } = useApp()
  return (
    <div className="h-full flex items-center justify-center glass-main">
      <div className="animate-pulse text-muted-foreground">{t('common.loading')}</div>
    </div>
  )
}

function App() {
  const [user, setUser] = useState<any>(null)
  const [activeMenu, setActiveMenu] = useState<string>(() => {
    return localStorage.getItem('activeMenu') || 'home'
  })
  const [mountedRouteIds, setMountedRouteIds] = useState<string[]>(() => 
    getMountedRouteIds([], localStorage.getItem('activeMenu') || 'home')
  )
  const { t } = useApp()
  const { settings: appSettings, loading: settingsLoading } = useAppSettings()
  const { showError, showInfo } = useDialog()

  // Persist the current page.
  useEffect(() => {
    if (activeMenu && activeMenu !== 'callback') {
      localStorage.setItem('activeMenu', activeMenu)
    }
  }, [activeMenu])

  useEffect(() => {
    setMountedRouteIds(prev => getMountedRouteIds(prev, activeMenu))
  }, [activeMenu])

  useEffect(() => {
    dismissBootSplash()
  }, [])

  useEffect(() => {
    checkAuth()

    const url = new URL(window.location.href)
    if (url.pathname === '/callback' && (url.searchParams.has('code') || url.searchParams.has('state'))) {
      setActiveMenu('callback')
      return
    }

    let unlisten: UnlistenFn | null = null
    let unlistenBanned: UnlistenFn | null = null
    let unlistenTokenInvalid: UnlistenFn | null = null
    let unlistenNetworkError: UnlistenFn | null = null
    let mounted = true

    const setupListeners = async () => {
      unlisten = await listen('login-success', () => {
        if (!mounted) return
        checkAuth()
        setActiveMenu('accounts')
      })
      
      // Backend tasks run automatically; no settings event listener is needed here.
      
      unlistenBanned = await listen<{ email: string }>('account-banned', (event) => {
        if (!mounted) return
        showError(
          t('app.accountBanned'),
          t('app.accountBannedMessage', { email: event.payload.email })
        )
      })

      unlistenTokenInvalid = await listen<{ email: string }>('account-token-invalid', (event) => {
        if (!mounted) return
        showInfo(
          t('app.tokenExpired'),
          t('app.tokenExpiredMessage', { email: event.payload.email })
        )
      })

      unlistenNetworkError = await listen<{ count: number, total: number }>('sync-network-error', (event) => {
        if (!mounted) return
        showError(
          t('app.networkError'),
          t('app.networkErrorMessage', { count: event.payload.count, total: event.payload.total })
        )
      })
    }

    setupListeners()
    
    return () => { 
      mounted = false
      if (unlisten) unlisten()
      if (unlistenBanned) unlistenBanned()
      if (unlistenTokenInvalid) unlistenTokenInvalid()
      if (unlistenNetworkError) unlistenNetworkError()
    }
  }, [showError, showInfo, t])

  const checkAuth = async () => {
    try {
      const currentUser = await invoke<any>('get_current_user')
      setUser(currentUser)
    } catch (e) {
      console.error('Auth check failed:', e)
    }
  }

  const handleLogin = () => {
    checkAuth()
  }

  const handleLogout = async () => {
    await invoke('logout')
    setUser(null)
  }

  const routeProps = useMemo<Record<string, any>>(() => ({
    home: { onNavigate: setActiveMenu },
    desktopOAuth: { onLogin: () => { handleLogin(); setActiveMenu('accounts') } },
    accounts: { onNavigate: setActiveMenu }
  }), [])

  const renderContent = () => {
    if (!shouldPersistRoute(activeMenu)) {
      const RouteComponent = allRoutes[activeMenu] || allRoutes.home
      return <RouteComponent {...(routeProps[activeMenu] || {})} />
    }

    return mountedRouteIds.map((routeId) => {
      const RouteComponent = allRoutes[routeId]
      if (!RouteComponent) return null

      return (
        <section
          key={routeId}
          className="h-full w-full"
          style={{ display: routeId === activeMenu ? 'block' : 'none' }}
          aria-hidden={routeId === activeMenu ? 'false' : 'true'}
        >
          <RouteComponent {...(routeProps[routeId] || {})} />
        </section>
      )
    })
  }

  return (
    <PrivacyProvider>
      <AccountProvider>
        <div className="flex h-screen w-full bg-transparent overflow-hidden">
          <Sidebar 
            activeMenu={activeMenu} 
            onMenuChange={setActiveMenu}
            onLogout={handleLogout}
          />
          <main className="flex-1 overflow-hidden glass-main">
            <div className="h-full w-full">
              <Suspense fallback={<PageLoading />}>
                {renderContent()}
              </Suspense>
            </div>
          </main>
          
          <UpdateChecker />
          <WelcomeModal />
          <Toaster 
            position="top-center"
            toastOptions={{
              style: {
                marginTop: '80px'
              }
            }}
          />
        </div>
      </AccountProvider>
    </PrivacyProvider>
  )
}

export default App
