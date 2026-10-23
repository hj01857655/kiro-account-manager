
import { useState, useEffect } from 'react'
import { getKiroLocalToken } from '../../../api/kiroApi'
import { getVersion } from '@tauri-apps/api/app'
import { User, Sun, Moon, Palette, Monitor, ChevronLeft, ChevronRight, LucideIcon } from 'lucide-react'
import { Button } from '../../ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip'
import { cn } from '../../../lib/utils'
import { useApp } from '../../../hooks/useApp'
import { routes } from '../../../routes'
import { THEME_KEYS, findTheme, type ThemeIconName } from '../../../lib/themeRegistry'

/** 主题图标：注册表只存图标名，组件映射留在视图层，避免 lib 依赖 React 组件 */
const THEME_ICONS: Record<ThemeIconName, LucideIcon> = { Monitor, Sun, Moon, Palette }

interface SidebarProps {
  activeMenu: string;
  onMenuChange: (id: string) => void;
  onLogout?: () => void;
}

interface LocalToken {
    provider?: string;
    expiresAt?: string | number;
}

function useMenuItems() {
  const { t } = useApp()
  return routes.map(r => ({
    id: r.id,
    icon: r.icon,
    label: t(r.nameKey),
    desc: r.descKey ? t(r.descKey) : undefined
  }))
}

function Sidebar({ activeMenu, onMenuChange }: SidebarProps) {
  const [localToken, setLocalToken] = useState<LocalToken | null>(null)
  const [version, setVersion] = useState('')
  // 默认折叠：窗口最小可缩到 1280×720（720p），展开时侧边栏占 192px，内容区剩 1088px，
  // 卡片布局已能正常展开。默认仍收成 64px 图标态（带 tooltip），折叠后内容区有 1216px。
  // 用户手动展开后记在 localStorage 里，下次保持自己的选择。
  //
  // 初值必须惰性读取：若先 useState(true) 再在 effect 里改回 false，
  // 会先渲染一帧 64px 再走 200ms 宽度动画——启动时能看到侧边栏“抽”一下。
  const [collapsed, setCollapsed] = useState(
    () => localStorage.getItem('sidebar-collapsed') !== 'false'
  )
  const { t, theme, setTheme } = useApp()
  const menuItems = useMenuItems()

  useEffect(() => {
    getKiroLocalToken<LocalToken>().then(setLocalToken).catch(() => {})
    getVersion().then(setVersion)
  }, [])

  const toggleCollapsed = () => {
    const newState = !collapsed
    setCollapsed(newState)
    localStorage.setItem('sidebar-collapsed', String(newState))
  }

  const currentTheme = findTheme(theme)
  const ThemeIcon = (currentTheme && THEME_ICONS[currentTheme.iconName]) || Sun
  const themeLabel = currentTheme ? (t(currentTheme.nameKey) || currentTheme.fallbackName) : String(theme ?? '')

  const handleThemeClick = () => {
    // 按注册表顺序轮转；当前主题不在表内（历史遗留 theme1/theme2）时从头开始，
    // 避免 indexOf 得 -1 导致 (0)%len 意外跳回第一个主题
    const currentIndex = THEME_KEYS.indexOf(theme)
    const nextIndex = (currentIndex + 1) % THEME_KEYS.length
    setTheme(THEME_KEYS[nextIndex])
  }

  return (
    <div
      className={cn("flex flex-col relative transition-[width] duration-200 ease-in-out glass-sidebar z-10 overflow-hidden sidebar-top-inset")}
      style={{ width: collapsed ? 64 : 192 }}
    >
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <div
              className={cn("cursor-pointer select-none", collapsed ? "p-2" : "p-3", "pb-2")}
              onClick={toggleCollapsed}
            >
              <div
                className={cn(
                  "flex items-center mb-2 animate-fade-in-up",
                  collapsed ? "justify-center gap-0" : "justify-start gap-2"
                )}
                style={{ animationDelay: '0.1s', animationFillMode: 'both' }}
              >
                <div className="w-10 h-10 rounded-xl flex items-center justify-center transition-transform hover:scale-105 flex-shrink-0 sidebar-card">
                  <svg width="24" height="24" viewBox="0 0 40 40" fill="none">
                    <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="currentColor" className="sidebar-foreground"/>
                  </svg>
                </div>
                {!collapsed && (
                  <div className="flex flex-col gap-0">
                    <span className="text-lg font-bold tracking-wider sidebar-foreground">KIRO</span>
                    <span className="text-xs sidebar-muted">Account Manager</span>
                  </div>
                )}
              </div>
            </div>
          </TooltipTrigger>
          <TooltipContent side="right">
            {collapsed ? t('nav.expandSidebar') : t('nav.collapseSidebar')}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      <div className={cn("flex flex-col gap-0.5 flex-1 overflow-auto no-scrollbar px-2")}>
        {menuItems.map((item, idx) => {
          const Icon = item.icon
          const isActive = activeMenu === item.id
          return (
            <TooltipProvider key={item.id}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => onMenuChange(item.id)}
                    className={cn(
                      "flex items-center gap-2.5 px-2.5 py-2.5 rounded-lg transition-all animate-slide-in-left",
                      !isActive && "sidebar-foreground sidebar-hover",
                      isActive && "sidebar-active font-semibold",
                      !isActive && "font-normal"
                    )}
                    style={{
                      animationDelay: `${0.15 + idx * 0.05}s`,
                      animationFillMode: 'both'
                    }}
                  >
                    {isActive && <div className="w-0.5 h-4 rounded-full bg-current opacity-80 -ml-0.5 mr-0.5 shrink-0" />}
                    <Icon size={18} strokeWidth={isActive ? 2.5 : 2} className="shrink-0" />
                    {!collapsed && (
                      <div className="flex-1 text-left whitespace-nowrap overflow-hidden">
                        <div className="text-sm">{item.label}</div>
                        {item.desc && <div className="text-[10px] sidebar-muted leading-tight">{item.desc}</div>}
                      </div>
                    )}
                  </button>
                </TooltipTrigger>
                {collapsed && <TooltipContent side="right">{item.label}</TooltipContent>}
              </Tooltip>
            </TooltipProvider>
          )
        })}
      </div>

      {localToken && (
        <div className={cn("mx-3 mb-3 p-3 rounded-xl sidebar-card", collapsed && "mx-2 px-0 flex justify-center")}>
          {!collapsed ? (
            <>
              <div className="flex items-center gap-2 mb-2">
                <div className="relative">
                  <div className="w-1.5 h-1.5 rounded-full bg-green-500" />
                  <div className="absolute inset-0 w-1.5 h-1.5 rounded-full bg-green-500 animate-ping" />
                </div>
                <span className="text-xs sidebar-foreground">{t('nav.kiroConnected')}</span>
              </div>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-green-500/25 flex items-center justify-center text-green-600 dark:text-green-400">
                  <User size={14} />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-bold truncate sidebar-foreground">
                    {localToken.provider || 'Local'}
                  </div>
                </div>
              </div>
            </>
          ) : (
            <div className="relative">
              <div className="w-2 h-2 rounded-full bg-green-500" />
              <div className="absolute inset-0 w-2 h-2 rounded-full bg-green-500 animate-ping" />
            </div>
          )}
        </div>
      )}

      <div className={cn("px-3 pb-3 flex items-center gap-2", collapsed ? "flex-col" : "justify-between")}>
        <div className="flex items-center gap-1">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleThemeClick}
                  className="sidebar-card sidebar-foreground sidebar-hover h-7 w-7"
                >
                  <ThemeIcon size={14} />
                </Button>
              </TooltipTrigger>
              <TooltipContent side={collapsed ? "right" : "top"}>
                {themeLabel}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={toggleCollapsed}
                  className="sidebar-foreground sidebar-hover h-7 w-7"
                >
                  {collapsed ? <ChevronRight size={14} /> : <ChevronLeft size={14} />}
                </Button>
              </TooltipTrigger>
              <TooltipContent side={collapsed ? "right" : "top"}>
                {collapsed ? t('nav.expandSidebar') : t('nav.collapseSidebar')}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>

        {!collapsed && (
          <span className="text-[10px] ml-auto sidebar-muted font-mono tracking-tighter">v{version || '...'}</span>
        )}
      </div>
    </div>
  )
}

export default Sidebar
