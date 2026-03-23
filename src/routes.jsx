import { lazy } from 'react'
import { Home, Key, Settings2, LogIn, Settings, Info, Network } from 'lucide-react'

// 路由配置：菜单项 + 懒加载组件
// id 对应 nameKey 去掉 nav. 前缀
export const routes = [
  { id: 'home', icon: Home, nameKey: 'nav.home', component: lazy(() => import('./components/features/Home')) },
  { id: 'accounts', icon: Key, nameKey: 'nav.accounts', component: lazy(() => import('./components/features/AccountManager/index')) },
  { id: 'kiroConfig', icon: Settings2, nameKey: 'nav.kiroConfig', component: lazy(() => import('./components/features/KiroConfig/index')) },
  { id: 'desktopOAuth', icon: LogIn, nameKey: 'nav.desktopOAuth', descKey: 'nav.socialIdC', component: lazy(() => import('./components/features/Login')) },
  { id: 'gateway', icon: Network, nameKey: 'nav.gateway', component: lazy(() => import('./components/features/GatewayPage')) },
  { id: 'settings', icon: Settings, nameKey: 'nav.settings', component: lazy(() => import('./components/features/Settings')) },
  { id: 'about', icon: Info, nameKey: 'nav.about', component: lazy(() => import('./components/features/About')) },
]

// 内部路由（不在侧边栏显示）
export const internalRoutes = {
  callback: lazy(() => import('./components/shared/AuthCallback')),
}
