import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../../hooks/useApp'
import { useAppSettings } from '../../../../contexts/AppSettingsContext'
import { applyMachineGuid, buildSwitchParams } from '../../../../utils/kiroSwitch'

/**
 * 账号切换逻辑 hook
 * @param {Function} onLocalTokenChange - 本地 token 变化回调
 * @returns {Object} 切换相关状态和方法
 */
export function useSwitchAccount(onLocalTokenChange) {
  const { t } = useApp()
  const { settings: appSettings } = useAppSettings()
  const [switchingId, setSwitchingId] = useState(null)
  const [switchDialog, setSwitchDialog] = useState(null)

  // 显示切换确认弹窗
  const handleSwitchAccount = useCallback((account) => {
    if (!account.accessToken || !account.refreshToken) {
      setSwitchDialog({ type: 'error', title: t('switch.failed'), message: t('switch.missingAuth'), account: null })
      return
    }
    setSwitchDialog({
      type: 'confirm',
      title: t('switch.title'),
      message: `${t('switch.confirmSwitch')} ${account.email}？`,
      account,
    })
  }, [t])

  // 确认切换
  const confirmSwitch = useCallback(async () => {
    const account = switchDialog?.account
    if (!account) return
    
    setSwitchDialog(null)
    setSwitchingId(account.id)
    
    try {
      // 先同步账号（刷新 token + 获取最新配额）
      const syncResult = await invoke('sync_account', { id: account.id })
      let refreshedAccount = syncResult.account  // ✅ 修复：从 syncResult 中提取 account
      
      const settings = appSettings || {}
      refreshedAccount = await applyMachineGuid(refreshedAccount, settings)
      
      const params = buildSwitchParams(refreshedAccount)
      await invoke('switch_kiro_account', { params })
      
      // 更新当前账号标识
      invoke('get_kiro_local_token').then(onLocalTokenChange).catch(() => onLocalTokenChange(null))
      
      // 从 usageData 获取配额信息（API 原始响应）
      const usageData = refreshedAccount.usageData
      const breakdown = usageData?.usageBreakdownList?.[0]
      const now = Date.now()
      
      // 主配额（永不过期）
      const mainUsed = breakdown?.currentUsage ?? 0
      const mainLimit = breakdown?.usageLimit ?? 0
      
      // 试用配额（检查过期）
      const trialInfo = breakdown?.freeTrialInfo
      const trialExpiry = trialInfo?.freeTrialExpiry ? trialInfo.freeTrialExpiry * 1000 : 0
      const trialValid = trialExpiry > now
      const trialUsed = trialValid ? (trialInfo?.currentUsage ?? 0) : 0
      const trialLimit = trialValid ? (trialInfo?.usageLimit ?? 0) : 0
      
      // 奖励配额（检查每个奖励的过期时间）
      const bonuses = breakdown?.bonuses ?? []
      let bonusUsed = 0, bonusLimit = 0
      bonuses.forEach(b => {
        const expiry = b.expiresAt ? b.expiresAt * 1000 : Infinity
        if (expiry > now) {
          bonusUsed += b.currentUsage ?? 0
          bonusLimit += b.usageLimit ?? 0
        }
      })
      
      // 总计
      const totalUsed = mainUsed + trialUsed + bonusUsed
      const totalLimit = mainLimit + trialLimit + bonusLimit
      const remaining = totalLimit - totalUsed
      const provider = refreshedAccount.provider || 'Unknown'
      
      setSwitchDialog({
        type: 'success',
        title: t('switch.success'),
        message: `${refreshedAccount.email}\n\n📊 ${t('switch.quota')}: ${totalUsed}/${totalLimit} (${t('switch.remaining')} ${remaining})\n🏷️ ${t('switch.type')}: ${provider}`,
        account: null,
      })
    } catch (e) {
      setSwitchDialog({
        type: 'error',
        title: t('switch.failed'),
        message: String(e),
        account: null,
      })
    } finally {
      setSwitchingId(null)
    }
  }, [switchDialog, appSettings, onLocalTokenChange, t])

  // 关闭弹窗
  const closeSwitchDialog = useCallback(() => setSwitchDialog(null), [])

  return {
    switchingId,
    setSwitchingId,
    switchDialog,
    handleSwitchAccount,
    confirmSwitch,
    closeSwitchDialog,
  }
}
