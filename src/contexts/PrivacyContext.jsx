import { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'

const PrivacyContext = createContext()

export function PrivacyProvider({ children }) {
  const [privacyMode, setPrivacyModeState] = useState(true) // 默认开启隐私模式

  // 从后端加载设置
  useEffect(() => {
    invoke('get_app_settings').then(settings => {
      setPrivacyModeState(settings?.privacyMode ?? true) // 默认 true
    }).catch(() => {})
  }, [])

  // 保存设置到后端
  const setPrivacyMode = useCallback(async (enabled) => {
    setPrivacyModeState(enabled)
    try {
      await invoke('save_app_settings', { settings: { privacyMode: enabled } })
    } catch (err) {
      console.error('Failed to save privacy mode:', err)
    }
  }, [])

  // 邮箱脱敏: user12345@example.com -> us***45@***.com
  const maskEmail = useCallback((email) => {
    if (!privacyMode || !email) return email
    const [local, domain] = email.split('@')
    if (!domain) return email

    // 本地部分脱敏
    let maskedLocal
    if (local.length <= 2) {
      maskedLocal = '*'.repeat(local.length)
    } else if (local.length <= 4) {
      maskedLocal = local[0] + '***'
    } else {
      maskedLocal = local.slice(0, 2) + '***' + local.slice(-2)
    }

    // 域名部分脱敏
    const domainParts = domain.split('.')
    const tld = domainParts[domainParts.length - 1]
    const maskedDomain = '***.' + tld

    return `${maskedLocal}@${maskedDomain}`
  }, [privacyMode])

  // 昵称/标签脱敏: MyNickname -> My***me
  const maskNickname = useCallback((name) => {
    if (!privacyMode || !name) return name
    if (name.length <= 2) return '*'.repeat(name.length)
    if (name.length <= 4) return name[0] + '***'
    return name.slice(0, 2) + '***' + name.slice(-2)
  }, [privacyMode])

  const value = useMemo(() => ({
    privacyMode,
    setPrivacyMode,
    maskEmail,
    maskNickname,
  }), [privacyMode, maskEmail, maskNickname])

  return <PrivacyContext.Provider value={value}>{children}</PrivacyContext.Provider>
}

export function usePrivacy() {
  const context = useContext(PrivacyContext)
  if (!context) throw new Error('usePrivacy must be used within PrivacyProvider')
  return context
}
