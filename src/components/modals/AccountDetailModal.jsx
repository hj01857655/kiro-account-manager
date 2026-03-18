import { useState, useRef, useEffect, memo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Copy, Check, RefreshCw, User, CreditCard, Shield } from 'lucide-react'
import { TextInput } from '@mantine/core'
import { useApp } from '../../hooks/useApp'
import { useDialog } from '../../contexts/DialogContext'
import { formatUsage, getAccountDisplayName } from '../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus } from '../../utils/accountStatus'
import { getProviderDisplayName, isGitHubProvider } from '../../utils/accountProvider'
import { TokenJsonView } from '../features/AccountManager/TokenJsonView'
import {
  DialogRoot,
  DialogContent,
  DialogBody,
  DialogFooter,
} from '../ui/dialog'
import { Button } from '../ui/button'

// 配额卡片组件（优化性能）
const QuotaCard = memo(({ title, used, quota, icon, status, expiry, colors, t }) => {
  const isActive = status === 'ACTIVE'
  const hasQuota = quota > 0
  
  return (
    <div className={`rounded-lg p-3 border transition-colors duration-200 hover:shadow-md ${
      hasQuota && isActive
        ? 'border-blue-500/30 bg-blue-500/5 shadow-blue-500/10'
        : `${colors.cardBorder} ${colors.cardSecondary}`
    }`}>
      <div className="flex items-center gap-2 mb-3">
        <div className={`w-2.5 h-2.5 rounded-full ${
          hasQuota && isActive
            ? title.includes('试用')
              ? 'bg-cyan-500 shadow-lg shadow-cyan-500/50'
              : title.includes('奖励')
                ? 'bg-purple-500 shadow-lg shadow-purple-500/50'
                : 'bg-blue-500 shadow-lg shadow-blue-500/50'
            : 'bg-gray-400'
        }`}></div>
        <span className={`text-xs font-medium uppercase tracking-wide ${
          hasQuota && isActive
            ? title.includes('试用')
              ? 'text-cyan-500'
              : title.includes('奖励')
                ? 'text-purple-500'
                : colors.textMuted
            : colors.textMuted
        }`}>{title}</span>
        {status && status !== 'ACTIVE' && (
          <span className={`text-xs px-2 py-0.5 rounded-md font-medium ${colors.cardSecondary} ${colors.textMuted}`}>
            {status}
          </span>
        )}
      </div>
      <div className={`text-2xl font-semibold ${colors.text} mb-1`}>
        {hasQuota ? (
          <>{formatUsage(used)} <span className={`text-base ${colors.textMuted} font-normal`}>/ {formatUsage(quota)}</span></>
        ) : (
          <span className={colors.textMuted}>-</span>
        )}
      </div>
      {expiry && (
        <div className={`text-xs ${colors.textMuted} mt-2 flex items-center gap-1`}>
          <span>{icon}</span>
          <span>{expiry}</span>
        </div>
      )}
    </div>
  )
})

QuotaCard.displayName = 'QuotaCard'

function AccountDetailModal({ account, onClose }) {
  const { t, colors } = useApp()
  const { showError } = useDialog()
  const initQuota = account.usageData?.usageBreakdownList?.[0]?.usageLimit ?? account.quota ?? 0
  const initUsed = account.usageData?.usageBreakdownList?.[0]?.currentUsage ?? account.used ?? 0
  const [form, setForm] = useState({
    email: account.email || getAccountDisplayName(account),
    label: account.label || '',
    quota: initQuota,
    used: initUsed,
    status: account.status,
    accessToken: account.accessToken || '',
    refreshToken: account.refreshToken || '',
  })

  const [refreshing, setRefreshing] = useState(false)
  const [copied, setCopied] = useState(null)
  const copiedTimerRef = useRef(null)

  // 清理timer
  useEffect(() => {
    return () => {
      if (copiedTimerRef.current) {
        clearTimeout(copiedTimerRef.current)
      }
    }
  }, [])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const result = await invoke('sync_account', { id: account.id })
      const updated = result.account
      
      // 如果有警告，显示提示
      if (result.warning) {
        await showError('同步警告', result.warning)
      }
      
      // 封禁账号额度为 0
      const isBanned = isBannedStatus(updated.status)
      const quota = isBanned ? 0 : (updated.usageData?.usageBreakdownList?.[0]?.usageLimit ?? 0)
      const used = updated.usageData?.usageBreakdownList?.[0]?.currentUsage ?? 0
      setForm(prev => ({ ...prev, quota, used, status: updated.status }))
    } catch (e) {
      const errorMsg = String(e)
      let status = '刷新失败'
      if (errorMsg.includes('BANNED')) {
        status = 'banned'
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid')) {
        status = 'invalid'
      }
      setForm(prev => ({ ...prev, status }))
      await showError(t('detail.refreshFailed'), errorMsg)
    } finally {
      setRefreshing(false)
    }
  }

  const handleCopy = (text, field) => {
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopied(field)
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current)
    }
    copiedTimerRef.current = setTimeout(() => setCopied(null), 1500)
  }

  // 从 usageData 读取免费试用和奖励信息
  const breakdown = account.usageData?.usageBreakdownList?.[0]
  const freeTrialInfo = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []
  const now = Date.now()
  
  // 检查试用是否过期
  const trialExpiry = freeTrialInfo?.freeTrialExpiry ? freeTrialInfo.freeTrialExpiry * 1000 : 0
  const trialActive = freeTrialInfo?.freeTrialStatus === 'ACTIVE' || (trialExpiry > now)
  const freeTrialQuota = trialActive ? (freeTrialInfo?.usageLimit || 0) : 0
  const freeTrialUsed = trialActive ? (freeTrialInfo?.currentUsage || 0) : 0
  
  // 检查每个奖励是否过期（只计入未过期且状态为 ACTIVE 的奖励）
  let bonusQuota = 0, bonusUsed = 0
  bonuses.forEach(b => {
    const expiry = b.expiresAt ? b.expiresAt * 1000 : Infinity
    if (expiry > now && b.status === 'ACTIVE') {
      bonusQuota += b.usageLimit || 0
      bonusUsed += b.currentUsage || 0
    }
  })
  
  const totalQuota = form.quota + freeTrialQuota + bonusQuota
  const totalUsed = form.used + freeTrialUsed + bonusUsed
  const totalPercent = totalQuota > 0 ? Math.min(100, (totalUsed / totalQuota) * 100) : 0
  const statusMeta = getAccountStatusMeta(form.status, t)

  return (
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="800px">
        {/* 顶部渐变背景 */}
        <div className="absolute top-0 left-0 right-0 h-40 bg-gradient-to-br from-blue-500/5 via-purple-500/3 to-transparent pointer-events-none rounded-t-2xl" />
        
        {/* Header - 自定义复杂头部 */}
        <div className={`relative border-b ${colors.cardBorder} px-6 py-4`}>
          <div className="flex items-start gap-3">
            {/* 头像图标 */}
            <div className={`
              w-12 h-12 rounded-lg flex items-center justify-center flex-shrink-0 shadow-md
              ${account.provider === 'Google' 
                ? 'bg-gradient-to-br from-red-500 to-orange-500' 
                : isGitHubProvider(account.provider) 
                  ? 'bg-gradient-to-br from-gray-700 to-gray-900' 
                  : 'bg-gradient-to-br from-blue-500 to-indigo-600'
              }`}
            >
              <User size={22} className="text-white" strokeWidth={2} />
            </div>
            
            {/* 账号信息 */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <h2 className={`text-base font-semibold ${colors.text} truncate`}>
                  {account.email ? account.email : getAccountDisplayName(account)}
                </h2>
                <span className={`px-2 py-0.5 rounded-md text-xs font-medium whitespace-nowrap shadow-sm ${
                  (account.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('ENTERPRISE'))
                    ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-white shadow-amber-500/30'
                    : (account.usageData?.subscriptionInfo?.type?.includes('PRO+') || account.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO+'))
                      ? 'bg-gradient-to-r from-purple-500 to-pink-500 text-white shadow-purple-500/30'
                      : (account.usageData?.subscriptionInfo?.type?.includes('PRO') || account.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO'))
                        ? 'bg-gradient-to-r from-blue-500 to-indigo-500 text-white shadow-blue-500/30'
                        : (account.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('KIRO'))
                          ? 'bg-gradient-to-r from-teal-500 to-cyan-500 text-white shadow-teal-500/30'
                          : `${colors.cardSecondary} ${colors.textMuted}`
                }`}>
                  {account.usageData?.subscriptionInfo?.subscriptionTitle || 'Free'}
                </span>
              </div>
              
              <div className={`flex items-center gap-2 text-xs ${colors.textMuted} mb-2`}>
                <span className={`flex items-center gap-1 font-medium ${
                  account.provider === 'Google' ? 'text-red-500'
                    : isGitHubProvider(account.provider) ? colors.text
                    : account.provider === 'BuilderId' ? 'text-orange-500'
                    : colors.textMuted
                }`}>
                  <div className="w-1 h-1 rounded-full bg-current"></div>
                  {getProviderDisplayName(account.provider) || t('common.unknown')}
                </span>
                <span>·</span>
                <span>{t('detail.addedAt')} {account.addedAt?.split(' ')[0]}</span>
              </div>
              
              {/* 机器码 */}
              {account.machineId && (
                <div className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md ${colors.cardSecondary}`}>
                  <span className={`text-[10px] font-medium ${colors.textMuted}`}>Machine ID:</span>
                  <code className="text-[10px] font-mono text-red-400">
                    {account.machineId}
                  </code>
                  <button 
                    type="button" 
                    onClick={() => handleCopy(account.machineId, 'machineId')} 
                    className={`p-0.5 rounded ${colors.cardHover} cursor-pointer transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500/30`}
                  >
                    {copied === 'machineId' ? <Check size={10} className="text-green-500" /> : <Copy size={10} className={colors.textMuted} />}
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
        
        {/* Body - 使用 DialogBody 的 noPadding，自己控制每个区域的 padding */}
        <DialogBody noPadding>
          {/* 配额总览 */}
          <div className={`border-b ${colors.cardBorder} px-6 py-4`}>
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <div className={`p-1.5 rounded-lg ${colors.cardSecondary}`}>
                  <CreditCard size={18} className={colors.textMuted} />
                </div>
                <span className={`text-sm font-semibold ${colors.text}`}>{t('detail.quotaOverview')}</span>
              </div>
              <button 
                type="button" 
                onClick={handleRefresh} 
                disabled={refreshing} 
                className={`
                  p-2 rounded-lg transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30
                  ${refreshing ? 'bg-blue-500/20' : 'bg-blue-500/20 hover:bg-blue-500/30'}
                  disabled:opacity-50 disabled:cursor-not-allowed
                `} 
                title={t('detail.syncQuota')}
              >
                <RefreshCw size={15} className={`text-blue-500 ${refreshing ? 'animate-spin' : ''}`} />
              </button>
            </div>
              
            <div className="mb-5">
              <div className="flex items-baseline justify-between mb-3">
                <div>
                  <span className={`text-4xl font-semibold ${colors.text}`}>{formatUsage(totalUsed)}</span>
                  <span className={`text-lg ${colors.textMuted} ml-2`}>/ {formatUsage(totalQuota)}</span>
                </div>
                <span className={`text-base font-medium px-3 py-1 rounded-lg ${
                  totalPercent > 80 ? 'bg-red-500/20 text-red-500' 
                  : totalPercent > 50 ? 'bg-yellow-500/20 text-yellow-600' 
                  : 'bg-green-500/20 text-green-600'
                }`}>
                  {totalPercent.toFixed(0)}% {t('detail.used')}
                </span>
              </div>
              <div className={`h-4 ${colors.cardSecondary} rounded-full overflow-hidden shadow-inner`}>
                <div 
                  className={`h-full rounded-full transition-all duration-500 shadow-lg ${
                    totalPercent > 80 ? 'bg-gradient-to-r from-red-400 to-red-500' 
                    : totalPercent > 50 ? 'bg-gradient-to-r from-yellow-400 to-orange-500' 
                    : 'bg-gradient-to-r from-green-400 to-emerald-500'
                  }`} 
                  style={{ width: `${totalPercent}%` }} 
                />
              </div>
            </div>
            
            <div className="grid grid-cols-3 gap-3">
              {/* 主配额卡片 */}
              <QuotaCard
                title={t('detail.mainQuota')}
                used={form.used}
                quota={form.quota}
                icon="🔄"
                expiry={account.usageData?.nextDateReset ? `${new Date(account.usageData.nextDateReset * 1000).toLocaleDateString()} ${t('detail.reset')}` : null}
                colors={colors}
                t={t}
              />
              
              {/* 试用配额卡片 */}
              <QuotaCard
                title={t('detail.freeTrial')}
                used={freeTrialUsed}
                quota={freeTrialQuota}
                status={freeTrialInfo?.freeTrialStatus}
                icon="⏰"
                expiry={freeTrialInfo?.freeTrialExpiry ? `${new Date(freeTrialInfo.freeTrialExpiry * 1000).toLocaleDateString()} ${t('detail.expires')}` : null}
                colors={colors}
                t={t}
              />
              
              {/* 奖励配额卡片 */}
              <QuotaCard
                title={t('detail.bonusTotal')}
                used={bonusUsed}
                quota={bonusQuota}
                icon="🎁"
                expiry={bonuses.length > 0 ? `${bonuses.length} ${t('detail.bonusCount')}` : null}
                colors={colors}
                t={t}
              />
            </div>
            
            {/* Bonuses 列表 */}
            {bonuses.length > 0 && (
              <div className="mt-6 pt-5 border-t" style={{ borderColor: colors.cardBorder }}>
                <div className="flex items-center gap-2 mb-4">
                  <span className="text-lg">🎁</span>
                  <span className={`text-sm font-medium ${colors.text}`}>{t('detail.bonusDetails')}</span>
                  <span className={`text-xs px-2 py-0.5 rounded-full ${colors.badgeInfo} font-medium`}>{bonuses.length}</span>
                </div>
                <div className="space-y-3">
                  {bonuses.map((bonus, idx) => (
                    <div key={idx} className={`flex items-center justify-between p-4 rounded-xl border transition-colors duration-200 hover:shadow-md ${
                      bonus.status === 'ACTIVE' 
                        ? 'bg-purple-500/10 border-purple-500/30' 
                        : bonus.status === 'EXHAUSTED' 
                          ? `${colors.cardSecondary} ${colors.cardBorder}` 
                          : `${colors.cardSecondary} ${colors.cardBorder}`
                    }`}>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <span className={`text-sm font-medium ${colors.text}`}>{bonus.displayName || bonus.bonusCode}</span>
                          <span className={`text-xs px-2 py-0.5 rounded-md font-medium ${
                            bonus.status === 'ACTIVE' 
                              ? 'bg-green-500/20 text-green-500' 
                              : bonus.status === 'EXHAUSTED' 
                                ? `${colors.cardSecondary} ${colors.textMuted}` 
                                : 'bg-yellow-500/20 text-yellow-600'
                          }`}>
                            {bonus.status}
                          </span>
                        </div>
                        <div className={`text-xs ${colors.textMuted} leading-relaxed`}>
                          {bonus.description && <span>{bonus.description} · </span>}
                          {bonus.redeemedAt && <span>{t('detail.redeemed')}: {new Date(bonus.redeemedAt * 1000).toLocaleDateString()} · </span>}
                          {bonus.expiresAt && <span>{t('detail.expires')}: {new Date(bonus.expiresAt * 1000).toLocaleDateString()}</span>}
                        </div>
                      </div>
                      <div className="text-right ml-4 flex-shrink-0">
                        <div className={`text-base font-semibold ${colors.text}`}>{formatUsage(bonus.currentUsage || 0)} <span className={`text-sm ${colors.textMuted} font-normal`}>/ {formatUsage(bonus.usageLimit || 0)}</span></div>
                        <div className={`text-xs ${colors.textMuted} font-mono mt-0.5`}>{bonus.bonusCode}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
            
            {/* 订阅信息 */}
            <div className="mt-6 pt-5 border-t" style={{ borderColor: colors.cardBorder }}>
              <div className="flex items-center gap-2 mb-4">
                <span className="text-lg">📋</span>
                <span className={`text-sm font-medium ${colors.text}`}>订阅信息</span>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div className={`p-3 rounded-lg ${colors.cardSecondary}`}>
                  <div className={`text-xs ${colors.textMuted} mb-1`}>{t('detail.userId')}</div>
                  <div className={`${colors.text} font-mono text-xs truncate`} title={account.usageData?.userInfo?.userId}>
                    {account.usageData?.userInfo?.userId?.slice(-12) || '-'}
                  </div>
                </div>
                <div className={`p-3 rounded-lg ${colors.cardSecondary}`}>
                  <div className={`text-xs ${colors.textMuted} mb-1`}>{t('detail.email')}</div>
                  <div className={`${colors.text} text-xs truncate`}>
                    {account.usageData?.userInfo?.email || account.email || getAccountDisplayName(account)}
                  </div>
                </div>
                <div className={`p-3 rounded-lg ${colors.cardSecondary}`}>
                  <div className={`text-xs ${colors.textMuted} mb-1`}>{t('detail.subscriptionType')}</div>
                  <div className={`${colors.text} font-mono text-xs truncate`} title={account.usageData?.subscriptionInfo?.type}>
                    {account.usageData?.subscriptionInfo?.type || '-'}
                  </div>
                </div>
                <div className={`p-3 rounded-lg ${colors.cardSecondary}`}>
                  <div className={`text-xs ${colors.textMuted} mb-1`}>{t('detail.upgradeable')}</div>
                  <div className={colors.text}>
                    {account.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? (
                      <span className="text-green-500 font-medium">✓ {t('common.yes')}</span>
                    ) : (
                      <span className={colors.textMuted}>✗ {t('common.no')}</span>
                    )}
                  </div>
                </div>
                {breakdown?.overageRate != null && (
                  <>
                    <div className={`p-3 rounded-lg ${colors.cardSecondary}`}>
                      <div className={`text-xs ${colors.textMuted} mb-1`}>{t('detail.overageRate')}</div>
                      <div className={`${colors.text} font-medium`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageRate}/{t('detail.perCredit')}
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg ${colors.cardSecondary}`}>
                      <div className={`text-xs ${colors.textMuted} mb-1`}>{t('detail.overageCap')}</div>
                      <div className={`${colors.text} font-medium`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCap}
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>

          {/* 基本信息 */}
          <div className={`border-b ${colors.cardBorder} px-6 py-4`}>
            <div className="flex items-center gap-2 mb-4">
              <div className={`p-1.5 rounded-lg ${colors.cardSecondary}`}>
                <User size={18} className={colors.textMuted} />
              </div>
              <span className={`text-sm font-semibold ${colors.text}`}>{t('detail.basicInfo')}</span>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <TextInput
                  label={t('detail.emailAddress')}
                  type="email"
                  value={form.email}
                  onChange={(e) => setForm({ ...form, email: e.target.value })}
                  required
                  classNames={{
                    label: `text-sm font-medium ${colors.textMuted} mb-2`,
                    input: `${colors.text} ${colors.input} ${colors.inputFocus}`
                  }}
                  styles={{
                    input: {
                      fontSize: '0.875rem',
                      padding: '0.75rem 1rem',
                      borderRadius: '0.75rem',
                    }
                  }}
                />
              </div>
              <div>
                <TextInput
                  label={t('detail.remarkLabel')}
                  value={form.label}
                  readOnly
                  placeholder={t('common.none')}
                  classNames={{
                    label: `text-sm font-medium ${colors.textMuted} mb-2`,
                    input: `${colors.text} ${colors.input} opacity-60`
                  }}
                  styles={{
                    input: {
                      fontSize: '0.875rem',
                      padding: '0.75rem 1rem',
                      borderRadius: '0.75rem',
                    }
                  }}
                />
              </div>
            </div>
          </div>

          {/* Token 凭证 JSON 视图 */}
          <TokenJsonView account={account} />
        </DialogBody>

        {/* Footer */}
        <DialogFooter>
          <div className={`text-sm ${colors.textMuted} flex items-center gap-2`}>
            {statusMeta.tone === 'success'
              ? <><Shield size={15} className="text-green-500" /><span className="text-green-500 font-medium">{statusMeta.label}</span></>
              : statusMeta.tone === 'danger'
                ? <><Shield size={15} className="text-red-500" /><span className="text-red-500 font-medium">{statusMeta.label}</span></>
                : <><Shield size={15} className="text-orange-500" /><span className="text-orange-500 font-medium">{statusMeta.label}</span></>}
          </div>
          <Button onClick={onClose}>
            {t('common.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </DialogRoot>
  )
}

export default AccountDetailModal
