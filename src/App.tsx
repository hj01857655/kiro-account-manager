import { useState, useEffect, useRef, Suspense, useMemo } from 'react'
import { showMainWindow } from './api/systemApi'
import { getCurrentUser, logout as apiLogout } from './api/accountApi'
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

// 构建路由映射
const routeMap = Object.fromEntries(routes.map(r => [r.id, r.component]))
const allRoutes = { ...routeMap, ...internalRoutes }

// 页面加载骨架屏
function PageLoading() {
  const { t } = useApp()
  return (
    <div className="h-full flex items-center justify-center glass-main">
      <div className="animate-pulse text-muted-foreground">{t('common.loading')}</div>
    </div>
  )
}

// 路由 chunk 就绪信号。
//
// 挂在 Suspense 边界内部：chunk 传输期间 Suspense 只渲染 fallback，本组件连带被
// 替换掉；chunk 一到位它才挂载。所以 onReady 的调用时机就是「真正内容可读」的时机，
// 启动卡据此撤除、窗口据此点亮。
//
// 不能用 fallback 的 onMount 反过来推：fallback 挂载只说明 chunk 未到，而「卸载」
// 无法用 props 观测；更不能假设 fallback 一定出现过——chunk 命中缓存时 React 仍会
// 至少 suspend 一个微任务，但依赖这个假设会让逻辑随缓存状态漂移。
function RouteReady({ onReady }: { onReady: () => void }) {
  const reported = useRef(false)
  useEffect(() => {
    if (reported.current) return
    reported.current = true
    onReady()
  }, [onReady])
  return null
}

function App() {
  const [user, setUser] = useState<any>(null)
  const [activeMenu, setActiveMenu] = useState<string>(() => {
    return localStorage.getItem('activeMenu') || 'home'
  })
  const [mountedRouteIds, setMountedRouteIds] = useState<string[]>(() =>
    getMountedRouteIds([], localStorage.getItem('activeMenu') || 'home')
  )
  // 启动卡是否已撤由 splashDismissedRef 记录（见下方），不需要 state：
  // 它不参与渲染，用 ref 避免路由切换时重复 invoke。
  const { t } = useApp()
  const { settings: appSettings, loading: settingsLoading } = useAppSettings()
  const { showError, showInfo } = useDialog()

  // 保存当前页面到 localStorage
  useEffect(() => {
    if (activeMenu && activeMenu !== 'callback') {
      localStorage.setItem('activeMenu', activeMenu)
    }
  }, [activeMenu])

  useEffect(() => {
    setMountedRouteIds(prev => getMountedRouteIds(prev, activeMenu))
  }, [activeMenu])

  // 启动卡盖住「窗口隐藏 → 首屏可读」这段。窗口在 tauri.conf.json 里是 visible: false，
  // 由这里在首屏就绪时调 show_main_window 点亮；若窗口提前可见，用户看到的是 WebView
  // 尚未绘制时的默认白底——所以这里必须是「内容就绪」而不是固定帧数。
  //
  // 「内容就绪」的判据是路由 chunk 挂载：路由是 lazy() 的，React 首帧只渲染出
  // Sidebar + Suspense fallback，chunk 还在传输，那时撤卡等于亮骨架屏。
  // 判据取自 Suspense fallback（PageLoading）是否已让位，不是 mountedRouteIds——
  // 后者在首帧就非空，只代表 React 决定挂这个路由，不代表 chunk 已到位。
  const [routeContentMounted, setRouteContentMounted] = useState(false)
  const splashDismissedRef = useRef(false)
  useEffect(() => {
    if (splashDismissedRef.current) return
    if (!routeContentMounted) return
    // 再等一帧，让 chunk 挂载后的那一帧完成排版，避免露出半渲染界面
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        splashDismissedRef.current = true
        dismissBootSplash()
        showMainWindow().catch((e) => {
          console.error('Failed to show main window:', e)
        })
      })
    })
  }, [routeContentMounted])

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

      // 后端自动运行，前端无需监听 settings-changed 和 app-settings-changed

      unlistenBanned = await listen<{ email: string }>('account-banned', (event) => {
        if (!mounted) return
        showError(t('app.accountBanned'), t('app.accountBannedMessage', { email: event.payload.email }))
      })

      unlistenTokenInvalid = await listen<{ email: string }>('account-token-invalid', (event) => {
        if (!mounted) return
        showInfo(t('app.tokenExpired'), t('app.tokenExpiredMessage', { email: event.payload.email }))
      })

      unlistenNetworkError = await listen<{ count: number, total: number }>('sync-network-error', (event) => {
        if (!mounted) return
        showError(t('app.networkError'), t('app.networkErrorMessage', { count: event.payload.count, total: event.payload.total }))
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
  }, [])

  const checkAuth = async () => {
    try {
      const currentUser = await getCurrentUser<any>()
      setUser(currentUser)
    } catch (e) {
      console.error('Auth check failed:', e)
    }
  }

  const handleLogin = () => {
    checkAuth()
  }

  const handleLogout = async () => {
    await apiLogout()
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
                <RouteReady onReady={() => setRouteContentMounted(true)} />
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
