import { createContext, useContext, useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const AppSettingsContext = createContext(null)

// 默认设置
const DEFAULT_SETTINGS = {
  lockModel: true,
  lockedModel: 'claude-opus-4.5',
  autoRefresh: true,
  autoRefreshInterval: 50,
  autoChangeMachineId: true,
  bindMachineIdToAccount: true,
  browserPath: '',
  privacyMode: true,
  autoSwitchEnabled: false,
  autoSwitchThreshold: 1,
  autoSwitchInterval: 5,
  enableCodebaseIndexing: true,
  enableTabAutocomplete: true,
  usageSummary: true,
  codeReferences: true,
  enableDebugLogs: false,
  notifyActionRequired: true,
  notifyFailure: true,
  notifySuccess: true,
  notifyBilling: true,
  trustedTools: [],
  referenceTracker: false,
  configureMcp: 'Enabled',
  telemetryContentCollection: false,
  telemetryUsageAnalytics: false,
  telemetryEditStats: false,
  telemetryFeedback: false
}

export function AppSettingsProvider({ children }) {
  const [settings, setSettings] = useState(null)
  const [loading, setLoading] = useState(true)

  // 加载设置
  const loadSettings = async () => {
    try {
      const appSettings = await invoke('get_app_settings')
      setSettings(appSettings || DEFAULT_SETTINGS)
    } catch (err) {
      console.error('[AppSettings] 加载失败:', err)
      setSettings(DEFAULT_SETTINGS)
    } finally {
      setLoading(false)
    }
  }

  // 更新设置（同时更新缓存和后端）
  const updateSettings = async (updates) => {
    try {
      await invoke('save_app_settings', { settings: updates })
      let nextSettings = null
      setSettings(prev => {
        nextSettings = { ...(prev || DEFAULT_SETTINGS), ...updates }
        return nextSettings
      })
      return nextSettings
    } catch (err) {
      console.error('[AppSettings] 保存失败:', err)
      return null
    }
  }

  useEffect(() => {
    loadSettings()

    let unlisten

    const setupListener = async () => {
      // 监听设置变更事件
      unlisten = await listen('app-settings-changed', (event) => {
        if (event.payload) {
          setSettings(event.payload)
        } else {
          // 如果没有payload，重新加载设置
          loadSettings()
        }
      })
    }

    setupListener()

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  return (
    <AppSettingsContext.Provider value={{ settings, loading, updateSettings, reload: loadSettings }}>
      {children}
    </AppSettingsContext.Provider>
  )
}

export function useAppSettings() {
  const context = useContext(AppSettingsContext)
  if (context === null) {
    throw new Error('useAppSettings must be used within AppSettingsProvider')
  }
  return context
}
