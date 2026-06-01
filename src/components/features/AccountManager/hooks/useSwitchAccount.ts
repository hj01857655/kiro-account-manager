import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../../hooks/useApp'
import { useAppSettings } from '../../../../contexts/AppSettingsContext'
import { applyMachineGuid, buildSwitchParams } from '../../../../utils/kiroSwitch'
import { getAccountStatusMeta, isUnavailableStatus } from '../../../../utils/accountStatus'

/**
 * 账号切换逻辑 hook
 * @param {Function} onLocalTokenChange - 本地 token 变化回调
 * @returns {Object} 切换相关状态和方法
 */
interface InstallationInfo {
  cli_installed?: boolean
  cliInstalled?: boolean
  ide_installed?: boolean
  ideInstalled?: boolean
  installed?: boolean
  ide_executable_exists?: boolean
  config_dir_exists?: boolean
  error_message?: string
}

interface SyncResult {
  account: any
}

export function useSwitchAccount(onLocalTokenChange) {
  const { t } = useApp()
  const { settings: appSettings } = useAppSettings()
  const [switchingId, setSwitchingId] = useState(null)
  const [switchDialog, setSwitchDialog] = useState(null)

  // 显示切换确认弹窗
  const handleSwitchAccount = useCallback((account) => {
    if (isUnavailableStatus(account)) {
      const statusMeta = getAccountStatusMeta(account, t)
      setSwitchDialog({ type: 'error', title: t('switch.failed'), message: t('accounts.accountCannotSwitch', { status: statusMeta.label }), account: null })
      return
    }
    if (!account.accessToken || !account.refreshToken) {
      setSwitchDialog({ type: 'error', title: t('switch.failed'), message: t('switch.missingAuth'), account: null })
      return
    }
    setSwitchDialog({
      type: 'confirm',
      title: t('switch.title'),
      message: `${t('switch.confirmSwitch')} ${account.email}？`,
      account,
      switchTarget: 'ide'})
  }, [t])

  // 显示退出登录确认弹窗（登录的逆操作：清除当前登录态，账号仍保留在列表中）
  const handleLogoutAccount = useCallback((account) => {
    setSwitchDialog({
      type: 'confirm',
      mode: 'logout',
      title: t('switch.logoutTitle'),
      message: `${t('switch.confirmLogout')} ${account.email}？`,
      account,
      switchTarget: 'ide'})
  }, [t])

  // 确认切换 / 退出登录（共用一个弹窗，靠 mode 区分）
  const confirmSwitch = useCallback(async () => {
    const account = switchDialog?.account
    if (!account) return

    const mode = (switchDialog as any)?.mode
    const switchTarget = (switchDialog as any)?.switchTarget || (appSettings as any)?.switchTarget || 'ide'

    setSwitchDialog(null)
    setSwitchingId(account.id)

    // Logout is the inverse of login: login writes local auth state, logout removes it.
    // The backend command is idempotent when the IDE is already logged out.
    if (mode === 'logout') {
      try {
        if (switchTarget === 'ide' || switchTarget === 'both') {
          await invoke('logout_kiro_account')
        }
        if (switchTarget === 'cli' || switchTarget === 'both') {
          try {
            const cliPath = await invoke<string>('get_kiro_cli_default_path')
            if (cliPath) {
              await invoke('logout_cli_account', { dbPath: cliPath })
            }
          } catch (e) {
            console.warn('[Logout] CLI 退出登录失败:', e)
          }
        }

        // 刷新本地 token，使 LIVE 标识消失（账号记录仍留在列表中）
        invoke('get_kiro_local_token').then(onLocalTokenChange).catch(() => onLocalTokenChange(null))

        const targetLabel = switchTarget === 'both' ? 'IDE + CLI' : switchTarget === 'cli' ? 'CLI' : 'IDE'
        setSwitchDialog({
          type: 'success',
          title: t('switch.logoutSuccess'),
          message: `${account.email}\n\n🎯 ${t('switch.switchTarget')}: ${targetLabel}\n${t('switch.logoutDone')}`,
          account: null})
      } catch (e) {
        setSwitchDialog({
          type: 'error',
          title: t('switch.logoutFailed'),
          message: String(e),
          account: null})
      } finally {
        setSwitchingId(null)
      }
      return
    }

    try {
      // 检测 IDE 安装状态。
      // 切换的唯一前置条件是「Kiro IDE 可执行文件存在」，与是否已登录无关——
      // switch_kiro_account 后端会自动 create_dir_all 并写入 kiro-auth-token.json
      // 及 IdC 的 {clientIdHash}.json，切换本身就等同于首次登录。
      // 旧逻辑用 ide_installed（= 可执行文件存在「且」已有有效 token 文件）当门槛，
      // 因果倒置：要求文件先存在，却又让切换去创建它，导致未登录时被「请先首次登录」拦死。
      const ideInfo = await invoke<InstallationInfo>('check_ide_installation')
      const ideExecExists = ideInfo?.ide_executable_exists ?? ideInfo?.ide_installed ?? ideInfo?.ideInstalled ?? ideInfo?.installed
      if (!ideExecExists) {
        // 仅当可执行文件确实缺失时才阻断（IDE 未安装 / 自定义路径错误）。
        const errorMsg = ideInfo?.error_message || t('switch.ideNotInstalledMessage')
        setSwitchDialog({
          type: 'error',
          title: t('switch.ideNotInstalled'),
          message: errorMsg,
          account: null})
        setSwitchingId(null)
        return
      }

      // 检查 Token 是否过期或即将过期（1 小时内）
      let needsRefresh = false
      if (account.expiresAt) {
        try {
          const expiryDate = new Date(account.expiresAt.replace(/\//g, '-'))
          const now = new Date()
          const oneHourFromNow = new Date(now.getTime() + 60 * 60 * 1000)
          
          // 如果已过期或 1 小时内过期，需要刷新
          if (expiryDate <= oneHourFromNow) {
            needsRefresh = true
            console.log('[Switch] Token 即将过期，先刷新 Token:', account.email, 'expires:', account.expiresAt)
          }
        } catch (e) {
          console.warn('[Switch] 解析过期时间失败:', e)
        }
      }

      // 如果需要刷新，先刷新 Token（不刷新配额，节省时间）
      if (needsRefresh) {
        try {
          await invoke('refresh_account_token', { id: account.id })
          console.log('[Switch] Token 刷新成功')
        } catch (e) {
          console.error('[Switch] Token 刷新失败:', e)
          // 刷新失败不阻断切换，让后续流程处理
        }
      }

      // 同步账号（获取最新配额，如果 Token 仍然失效会再次刷新）
      const syncResult = await invoke<SyncResult>('sync_account', { id: account.id })
      let refreshedAccount = syncResult.account

      const settings = appSettings || {}
      refreshedAccount = await applyMachineGuid(refreshedAccount, settings)

      const switchTarget = (switchDialog as any)?.switchTarget || (settings as any).switchTarget || 'ide'

      // IDE 切号
      if (switchTarget === 'ide' || switchTarget === 'both') {
        const params = buildSwitchParams(refreshedAccount)
        await invoke('switch_kiro_account', { params })
      }

      // CLI 切号
      if (switchTarget === 'cli' || switchTarget === 'both') {
        try {
          const cliPath = await invoke<string>('get_kiro_cli_default_path')
          if (cliPath) {
            await invoke('switch_to_cli_account', { accountId: refreshedAccount.id, dbPath: cliPath })
          }
        } catch (e) {
          console.warn('[Switch] CLI 切号失败:', e)
        }
      }

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
      const provider = refreshedAccount.provider || 'Unknown'

      // 超额信息
      const overageConfig = usageData?.overageConfiguration
      const overageEnabled = overageConfig?.overageStatus === 'ENABLED'
      const currentOverages = breakdown?.currentOverages ?? 0
      const overageCharges = breakdown?.overageCharges ?? 0
      const overageCap = breakdown?.overageCap ?? 0
      const overageRate = breakdown?.overageRate ?? 0
      const subTitle = usageData?.subscriptionInfo?.subscriptionTitle || ''

      // 正确计算：剩余 = 限额 - min(已用, 限额)
      const baseUsed = Math.min(mainUsed, mainLimit)
      const remaining = Math.max(0, mainLimit - baseUsed)

      // 构建消息
      let message = `${refreshedAccount.email}\n\n`
      message += `📊 ${t('switch.quota')}: ${mainUsed}/${mainLimit}`
      if (remaining > 0) {
        message += ` (${t('switch.remaining')} ${remaining})`
      } else {
        message += ` (${t('accounts.capped')})`
      }
      message += `\n`
      message += `🏷️ ${t('switch.type')}: ${provider}`
      if (subTitle) message += ` (${subTitle})`
      message += `\n`
      message += `🎯 ${t('switch.target')}: ${switchTarget === 'both' ? 'IDE + CLI' : switchTarget === 'cli' ? 'CLI' : 'IDE'}`
      message += `\n`
      if (overageEnabled && currentOverages > 0) {
        message += `⚡ ${t('home.currentOverage')}: ${currentOverages} credits ($${overageCharges.toFixed(2)}) | ${t('home.overageRate')}: $${overageRate}/credit | ${t('home.overageCap')}: ${overageCap}`
      } else if (overageEnabled) {
        message += `⚡ ${t('home.overage')}: ${t('home.enabled')}, ${t('home.currentOverage')} 0`
      } else {
        message += `⚡ ${t('home.overage')}: ${t('home.disabled')}`
      }

      setSwitchDialog({
        type: 'success',
        title: t('switch.success'),
        message,
        account: null})
    } catch (e) {
      setSwitchDialog({
        type: 'error',
        title: t('switch.failed'),
        message: String(e),
        account: null})
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
    setSwitchDialog,
    handleSwitchAccount,
    handleLogoutAccount,
    confirmSwitch,
    closeSwitchDialog}
}
