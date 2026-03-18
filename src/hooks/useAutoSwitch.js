import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { getQuota, getUsed } from '../utils/accountStats'
import { isBannedStatus } from '../utils/accountStatus'
import { applyMachineGuid, buildSwitchParams } from '../utils/kiroSwitch'

// 默认值
const DEFAULT_THRESHOLD = 1 // 余额阈值
const DEFAULT_INTERVAL = 5  // 检查间隔（分钟）

/**
 * 自动换号 Hook
 * 当当前账号余额低于阈值时，自动切换到其他可用账号
 * @param {Object} appSettings - 应用设置
 * @param {boolean} settingsLoading - 设置是否加载中
 */
export function useAutoSwitch(appSettings, settingsLoading) {
  const timerRef = useRef(null)
  const appSettingsRef = useRef(appSettings)

  // 同步 appSettings 到 ref
  useEffect(() => {
    appSettingsRef.current = appSettings
  }, [appSettings])

  // 检查并自动切换账号
  const checkAndAutoSwitch = async () => {
    const settings = appSettingsRef.current || {}
    
    // 未启用自动换号
    if (!settings.autoSwitchEnabled) return

    const threshold = settings.autoSwitchThreshold ?? DEFAULT_THRESHOLD

    try {
      // 获取所有账号
      const accounts = await invoke('get_accounts')
      if (!accounts?.length) return

      // 获取当前使用的账号（从本地 Kiro 凭证）
      let currentAccount = null
      try {
        const localToken = await invoke('get_kiro_local_token')
        if (localToken?.refreshToken) {
          currentAccount = accounts.find(acc => acc.refreshToken === localToken.refreshToken)
        }
      } catch (e) {
        return
      }

      if (!currentAccount) {
        return
      }

      // 先刷新当前账号状态获取最新余额
      try {
        const syncResult = await invoke('sync_account', { id: currentAccount.id })
        currentAccount = syncResult.account
      } catch (e) {
        const errorMsg = String(e)
        if (errorMsg.includes('BANNED')) {
          // 更新账号状态为封禁
          try {
            await invoke('update_account', {
              id: currentAccount.id,
              status: 'banned'
            })
            emit('accounts-updated')
          } catch (updateErr) {
            // 静默处理
          }
          return
        }
        // 其他错误静默处理
      }

      // 计算剩余额度
      const quota = getQuota(currentAccount)
      const used = getUsed(currentAccount)
      const remaining = quota - used

      // 检查是否需要切换
      if (remaining > threshold) {
        return
      }

      // 查找可用账号
      const availableAccount = accounts.find(acc => {
        // 排除当前账号
        if (acc.id === currentAccount.id) return false
        // 排除被封禁的账号
        if (isBannedStatus(acc.status)) return false
        // 排除余额不足的账号
        const accQuota = getQuota(acc)
        const accUsed = getUsed(acc)
        const accRemaining = accQuota - accUsed
        if (accRemaining <= threshold) return false
        return true
      })

      if (!availableAccount) {
        return
      }

      // 执行切换
      const switchableAccount = await applyMachineGuid(availableAccount, settings)
      const params = buildSwitchParams(switchableAccount)
      await invoke('switch_kiro_account', { params })

      emit('accounts-updated')
      emit('account-switched', { email: availableAccount.email })

    } catch (e) {
      // 静默处理
    }
  }

  // 启动定时器
  const startAutoSwitchTimer = () => {
    // 先清理旧定时器
    if (timerRef.current) {
      clearInterval(timerRef.current)
      timerRef.current = null
    }

    const settings = appSettingsRef.current || {}
    
    // 未启用则不启动
    if (!settings.autoSwitchEnabled) {
      return
    }

    const interval = settings.autoSwitchInterval ?? DEFAULT_INTERVAL
    const intervalMs = interval * 60 * 1000

    // 立即检查一次
    checkAndAutoSwitch()

    // 设置定时检查
    timerRef.current = setInterval(checkAndAutoSwitch, intervalMs)
  }

  // 停止定时器
  const stopAutoSwitchTimer = () => {
    if (timerRef.current) {
      clearInterval(timerRef.current)
      timerRef.current = null
    }
  }

  // 设置加载完成后启动定时器
  useEffect(() => {
    if (settingsLoading) return

    const settings = appSettingsRef.current || {}
    if (settings.autoSwitchEnabled) {
      startAutoSwitchTimer()
    }

    return () => {
      stopAutoSwitchTimer()
    }
  }, [settingsLoading])

  // 监听设置变化
  useEffect(() => {
    if (settingsLoading) return
    
    const settings = appSettingsRef.current || {}
    if (settings.autoSwitchEnabled) {
      startAutoSwitchTimer()
    } else {
      stopAutoSwitchTimer()
    }
  }, [appSettings?.autoSwitchEnabled, appSettings?.autoSwitchInterval, settingsLoading])

  return { startAutoSwitchTimer, stopAutoSwitchTimer, checkAndAutoSwitch }
}
