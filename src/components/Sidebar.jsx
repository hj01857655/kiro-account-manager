import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { Home, Key, Settings, Info, User, LogIn, Globe, Sun, Moon, Palette, Settings2, Languages, Server } from 'lucide-react'
import { useTheme, themes } from '../contexts/ThemeContext'
import { useI18n, locales } from '../i18n.jsx'

function useMenuItems() {
  const { t } = useI18n()
  return [
    { id: 'home', label: t('nav.home'), icon: Home },
    { id: 'token', label: t('nav.accounts'), icon: Key },
    { id: 'kiro-config', label: t('nav.kiroConfig'), icon: Settings2 },
    { id: 'local-server', label: 'Local Server', icon: Server },
    { id: 'login', label: t('nav.desktopOAuth'), icon: LogIn, desc: t('nav.socialIdC') },
    { id: 'web-oauth', label: t('nav.webOAuth'), icon: Globe, desc: t('nav.webviewLogin') },
    { id: 'settings', label: t('nav.settings'), icon: Settings },
    { id: 'about', label: t('nav.about'), icon: Info },
  ]
}

function Sidebar({ activeMenu, onMenuChange }) {
  const [localToken, setLocalToken] = useState(null)
  const [showThemeMenu, setShowThemeMenu] = useState(false)
  const [showLangMenu, setShowLangMenu] = useState(false)
  const [version, setVersion] = useState('')
  const { theme, setTheme, colors } = useTheme()
  const { locale, setLocale, loading: langLoading } = useI18n()
  const menuItems = useMenuItems()

  useEffect(() => {
    invoke('get_kiro_local_token').then(setLocalToken).catch(() => {})
    getVersion().then(setVersion)
  }, [])

  const themeIcons = { light: Sun, dark: Moon, purple: Palette, green: Palette }
  const ThemeIcon = themeIcons[theme] || Sun

  return (
    <div className={`w-56 ${colors.sidebar} ${colors.sidebarText} flex flex-col relative`}>
      {/* Logo */}
      <div className="p-5 pb-4">
        <div className="flex items-center gap-2.5 mb-1 animate-fade-in-up" style={{ animationDelay: '0.1s' }}>
          <div className="w-10 h-10 bg-white/20 rounded-xl flex items-center justify-center backdrop-blur-sm transition-transform hover:scale-110 hover:rotate-3">
            <svg width="24" height="24" viewBox="0 0 40 40" fill="none">
              <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white"/>
            </svg>
          </div>
          <div>
            <span className="font-bold text-lg tracking-wide">KIRO</span>
            <p className={`text-xs ${colors.sidebarMuted}`}>Account Manager</p>
          </div>
        </div>
      </div>

      {/* Menu */}
      <nav className="flex-1 px-3 space-y-1">
        {menuItems.map((item, index) => {
          const Icon = item.icon
          const isActive = activeMenu === item.id
          return (
            <button
              key={item.id}
              onClick={() => onMenuChange(item.id)}
              className={`w-full flex items-center gap-3 px-4 py-2.5 text-left transition-all rounded-xl group animate-slide-in-left ${
                isActive ? `${colors.sidebarActive} font-medium shadow-sm` : `${colors.sidebarText} ${colors.sidebarHover}`
              }`}
              style={{ animationDelay: `${0.15 + index * 0.05}s` }}
            >
              <div className={`transition-transform ${isActive ? '' : 'group-hover:scale-110'}`}>
                <Icon size={18} strokeWidth={isActive ? 2.5 : 2} />
              </div>
              <div className="flex-1 min-w-0">
                <span className="text-sm">{item.label}</span>
                {item.desc && <p className={`text-xs ${colors.sidebarMuted} truncate`}>{item.desc}</p>}
              </div>
              {isActive && (
                <div className="w-1.5 h-1.5 rounded-full bg-white/80 animate-pulse" />
              )}
            </button>
          )
        })}
      </nav>

      {/* Kiro IDE 本地连接状态 */}
      {localToken && (
        <div className={`mx-3 mb-3 ${colors.sidebarCard} rounded-xl p-3 animate-fade-in-up card-glow`} style={{ animationDelay: '0.5s' }}>
          <div className={`text-xs ${colors.sidebarMuted} mb-2 flex items-center gap-1.5`}>
            <div className="w-1.5 h-1.5 bg-green-400 rounded-full animate-pulse"></div>
            Kiro IDE 已连接
          </div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center text-sm font-medium text-green-300 transition-transform hover:scale-110">
              <User size={14} />
            </div>
            <div className="flex-1 min-w-0">
              <div className="text-xs font-medium truncate">{localToken.provider || 'Local'}</div>
              <div className={`text-xs ${colors.sidebarMuted}`}>
                {localToken.expiresAt ? new Date(localToken.expiresAt).toLocaleTimeString() : ''}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Theme & Language & Version */}
      <div className={`px-3 pb-3 flex items-center justify-between gap-2`}>
        {/* 主题切换 */}
        <div className="relative">
          <button
            onClick={() => setShowThemeMenu(!showThemeMenu)}
            className={`flex items-center gap-1.5 px-2 py-1.5 ${colors.sidebarCard} rounded-lg text-xs ${colors.sidebarMuted} hover:text-white transition-all hover:scale-105`}
          >
            <ThemeIcon size={14} />
          </button>
          
          {showThemeMenu && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setShowThemeMenu(false)} />
              <div className="absolute bottom-full left-0 mb-2 bg-white dark:bg-gray-800 rounded-xl shadow-xl border border-gray-200 dark:border-gray-700 py-1 min-w-[100px] z-50 animate-scale-in">
                {Object.entries(themes).map(([key, t]) => {
                  const TIcon = themeIcons[key] || Sun
                  return (
                    <button
                      key={key}
                      onClick={() => { setTheme(key); setShowThemeMenu(false) }}
                      className={`w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                        theme === key ? 'text-blue-600 font-medium' : 'text-gray-700 dark:text-gray-300'
                      }`}
                    >
                      <TIcon size={14} />
                      {t.name}
                    </button>
                  )
                })}
              </div>
            </>
          )}
        </div>

        {/* 语言切换 */}
        <div className="relative">
          <button
            onClick={() => setShowLangMenu(!showLangMenu)}
            disabled={langLoading}
            className={`flex items-center gap-1.5 px-2 py-1.5 ${colors.sidebarCard} rounded-lg text-xs ${colors.sidebarMuted} hover:text-white transition-all hover:scale-105 disabled:opacity-50`}
          >
            <Languages size={14} />
            <span>{locales[locale]?.substring(0, 2) || '中'}</span>
          </button>
          
          {showLangMenu && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setShowLangMenu(false)} />
              <div className="absolute bottom-full left-0 mb-2 bg-white dark:bg-gray-800 rounded-xl shadow-xl border border-gray-200 dark:border-gray-700 py-1 min-w-[120px] z-50 animate-scale-in">
                {Object.entries(locales).map(([key, name]) => (
                  <button
                    key={key}
                    onClick={() => { setLocale(key); setShowLangMenu(false) }}
                    className={`w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                      locale === key ? 'text-blue-600 font-medium' : 'text-gray-700 dark:text-gray-300'
                    }`}
                  >
                    {name}
                  </button>
                ))}
              </div>
            </>
          )}
        </div>
        
        <span className={`text-xs ${colors.sidebarMuted} ml-auto`}>v{version || '...'}</span>
      </div>
    </div>
  )
}

export default Sidebar
