import { useState, useRef, useEffect, memo, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { invoke } from '@tauri-apps/api/core'
import { Copy, Check, RefreshCw, User, CreditCard, Shield, Cpu, Loader2, FileText, Image as ImageIcon, Zap, Hash, ChevronDown, X } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { formatUsage, getAccountDisplayName } from '../../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus } from '../../../utils/accountStatus'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import {
  DialogRoot,
  DialogContent,
  DialogBody} from '../../shared/dialog'
import { Switch } from '../../ui/switch'
import { Account } from '../../../types/account'
import React from 'react'

interface QuotaCardProps {
  title: string;
  used: number;
  quota: number;
  icon: string | React.ReactNode;
  status?: string;
  expiry?: string | null;
  colors: any;
  t: any;
}

// 配额卡片组件（优化性能）
const QuotaCard = memo(({ title, used, quota, icon, status, expiry, colors, t }: QuotaCardProps) => {
  const isActive = status === 'ACTIVE'
  const hasQuota = quota > 0
  
  return (
    <div className={`rounded-lg p-3 border transition-colors duration-200 hover:shadow-md ${
      hasQuota && isActive
        ? 'border-blue-500/30 bg-blue-500/5 shadow-blue-500/10'
        : `border-border bg-muted/30`
    }`}>
      <div className="flex items-center gap-2 mb-3">
        <div className={`w-2.5 h-2.5 rounded-full ${
          hasQuota && isActive
            ? icon === '⏰'
              ? 'bg-cyan-500 shadow-lg shadow-cyan-500/50'
              : icon === '🎁'
                ? 'bg-purple-500 shadow-lg shadow-purple-500/50'
                : 'bg-blue-500 shadow-lg shadow-blue-500/50'
            : 'bg-gray-400'
        }`}></div>
        <span className={`text-xs font-medium uppercase tracking-wide ${
          hasQuota && isActive
            ? icon === '⏰'
              ? 'text-cyan-500'
              : icon === '🎁'
                ? 'text-purple-500'
                : "text-muted-foreground"
            : "text-muted-foreground"
        }`}>{title}</span>
        {status && status !== 'ACTIVE' && (
          <span className={`text-xs px-2 py-0.5 rounded-md font-medium bg-muted/30 text-muted-foreground`}>
            {status}
          </span>
        )}
      </div>
      <div className={`text-2xl font-semibold text-foreground mb-1`}>
        {hasQuota ? (
          <>{formatUsage(used)} <span className={`text-base text-muted-foreground font-normal`}>/ {formatUsage(quota)}</span></>
        ) : (
          <span className={"text-muted-foreground"}>-</span>
        )}
      </div>
      {expiry && (
        <div className={`text-xs text-muted-foreground mt-2 flex items-center gap-1`}>
          <span>{icon}</span>
          <span>{expiry}</span>
        </div>
      )}
    </div>
  )
})

QuotaCard.displayName = 'QuotaCard'

interface AccountDetailModalProps {
  account: Account;
  onClose: () => void;
  onRefresh?: () => void;
}

function AccountDetailModal({ account, onClose, onRefresh }: AccountDetailModalProps) {
  const { t } = useApp()
  const { showError } = useDialog()
  const [currentAccount, setCurrentAccount] = useState<Account>(account)

  // 样式定义
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])

  const initQuota = currentAccount.usageData?.usageBreakdownList?.[0]?.usageLimit ?? currentAccount.quota ?? 0
  const initUsed = currentAccount.usageData?.usageBreakdownList?.[0]?.currentUsage ?? currentAccount.used ?? 0
  
  const [form, setForm] = useState({
    email: currentAccount.email || getAccountDisplayName(currentAccount),
    label: currentAccount.label || '',
    quota: initQuota,
    used: initUsed,
    status: currentAccount.status,
    accessToken: currentAccount.accessToken || '',
    refreshToken: currentAccount.refreshToken || ''})

  const [refreshing, setRefreshing] = useState(false)
  const [copied, setCopied] = useState<string | null>(null)
  const copiedTimerRef = useRef<NodeJS.Timeout | null>(null)

  // Models 相关 state
  const [models, setModels] = useState<any[]>([])
  const [modelsLoading, setModelsLoading] = useState(false)
  const [modelsError, setModelsError] = useState<string | null>(null)
  const [modelsExpanded, setModelsExpanded] = useState(false)

  // 获取可用模型
  const fetchModels = async (forceRefresh = false) => {
    setModelsLoading(true)
    setModelsError(null)
    try {
      console.log('[AccountDetailModal] Fetching models for account:', account.id, 'forceRefresh:', forceRefresh)
      const response = await invoke<any>('list_available_models', { 
        id: account.id, 
        forceRefresh 
      })
      console.log('[AccountDetailModal] Models response:', response)
      const modelsList = Array.isArray(response?.availableModels) ? response.availableModels : []
      console.log('[AccountDetailModal] Models list:', modelsList.length, 'models')
      setModels(modelsList)
    } catch (e) {
      console.error('[AccountDetailModal] Failed to fetch models:', e)
      setModelsError(String(e))
    } finally {
      setModelsLoading(false)
    }
  }

  // 清理timer
  useEffect(() => {
    return () => {
      if (copiedTimerRef.current) {
        clearTimeout(copiedTimerRef.current)
      }
    }
  }, [])

  // 初始化时获取模型
  useEffect(() => {
    fetchModels()
  }, [account.id])

  useEffect(() => {
    setCurrentAccount(account)
    setForm({
      email: account.email || getAccountDisplayName(account),
      label: account.label || '',
      quota: account.usageData?.usageBreakdownList?.[0]?.usageLimit ?? account.quota ?? 0,
      used: account.usageData?.usageBreakdownList?.[0]?.currentUsage ?? account.used ?? 0,
      status: account.status,
      accessToken: account.accessToken || '',
      refreshToken: account.refreshToken || ''})
  }, [account])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const result = await invoke<{ account: Account, warning?: string }>('sync_account', { id: account.id })
      const updated = result.account
      setCurrentAccount(updated)
      
      // 如果有警告，显示提示
      if (result.warning) {
        await showError(t('common.warning'), result.warning)
      }
      
      // 封禁账号额度为 0
      const isBanned = isBannedStatus(updated)
      const quota = isBanned ? 0 : (updated.usageData?.usageBreakdownList?.[0]?.usageLimit ?? 0)
      const used = updated.usageData?.usageBreakdownList?.[0]?.currentUsage ?? 0
      setForm(prev => ({ ...prev, quota, used, status: updated.status }))
    } catch (e) {
      const errorMsg = String(e)
      let status = t('detail.refreshFailed')
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

  const handleCopy = (text: string, field: string) => {
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopied(field)
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current)
    }
    copiedTimerRef.current = setTimeout(() => setCopied(null), 1500)
  }

  // 从 usageData 读取免费试用和奖励信息
  const breakdown = currentAccount.usageData?.usageBreakdownList?.[0]
  const freeTrialInfo = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []
  const now = Date.now()
  
  // 检查试用是否过期
  const trialExpiry = freeTrialInfo?.freeTrialExpiry ? freeTrialInfo.freeTrialExpiry * 1000 : 0
  const trialActive = freeTrialInfo?.freeTrialStatus === 'ACTIVE' || (trialExpiry > now)
  const freeTrialQuota = trialActive ? (freeTrialInfo?.usageLimit || 0) : 0
  const freeTrialUsed = trialActive ? (freeTrialInfo?.currentUsage || 0) : 0
  
  // Count only active, unexpired bonuses.
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
  const statusMeta = getAccountStatusMeta({ status: form.status, usageData: currentAccount.usageData }, t)

  return createPortal(
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="800px" showClose={false}>
        {/* Top gradient background */}
        <div className="absolute top-0 left-0 right-0 h-40 bg-gradient-to-br from-blue-500/5 via-purple-500/3 to-transparent pointer-events-none rounded-t-2xl" />
        
        <div className={`sticky top-0 z-20 bg-background/95 backdrop-blur border-b border-border px-6 py-4 rounded-t-2xl`}>
          <div className="text-xs font-medium text-muted-foreground mb-3 uppercase tracking-wider">{t('detail.accountDetails', { defaultValue: 'Account Details' })}</div>
          <div className="flex items-start gap-3">
            {/* Avatar icon */}
            <div className={`
              w-12 h-12 rounded-lg flex items-center justify-center flex-shrink-0 shadow-md
              ${currentAccount.provider === 'Google'
                ? 'bg-gradient-to-br from-red-500 to-orange-500' 
                : isGitHubProvider(currentAccount.provider)
                  ? 'bg-gradient-to-br from-gray-700 to-gray-900' 
                  : 'bg-gradient-to-br from-blue-500 to-indigo-600'
              }`}
            >
              <User size={22} className="text-white" strokeWidth={2} />
            </div>
            
            {/* Account information */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <h2 className={`text-base font-semibold text-foreground truncate`}>
                  {currentAccount.email ? currentAccount.email : getAccountDisplayName(currentAccount)}
                </h2>
                <span className={`px-2 py-0.5 rounded-md text-xs font-medium whitespace-nowrap shadow-sm ${
                  (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('ENTERPRISE'))
                    ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-white shadow-amber-500/30'
                    : (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO+'))
                      ? 'bg-gradient-to-r from-purple-500 to-pink-500 text-white shadow-purple-500/30'
                      : (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO'))
                        ? 'bg-gradient-to-r from-blue-500 to-indigo-500 text-white shadow-blue-500/30'
                        : (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('KIRO'))
                          ? 'bg-gradient-to-r from-teal-500 to-cyan-500 text-white shadow-teal-500/30'
                          : `bg-muted/30 text-muted-foreground`
                }`}>
                  {currentAccount.usageData?.subscriptionInfo?.subscriptionTitle || 'Free'}
                </span>
              </div>
              
              <div className={`flex items-center gap-2 text-xs text-muted-foreground mb-2`}>
                <span className={`flex items-center gap-1 font-medium ${
                  currentAccount.provider === 'Google' ? 'text-red-500'
                    : isGitHubProvider(currentAccount.provider) ? "text-foreground"
                    : currentAccount.provider === 'BuilderId' ? 'text-orange-500'
                    : "text-muted-foreground"
                }`}>
                  <div className="w-1 h-1 rounded-full bg-current"></div>
                  {getProviderDisplayName(currentAccount.provider) || t('common.unknown')}
                </span>
                <span>·</span>
                <span>{t('detail.addedAt')} {currentAccount.addedAt?.split(' ')[0]}</span>
              </div>
              
              {/* Machine ID */}
              {currentAccount.machineId && (
                <div className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-muted/30`}>
                  <span className={`text-[10px] font-medium text-muted-foreground`}>Machine ID:</span>
                  <code className="text-[10px] font-mono text-red-400">
                    {currentAccount.machineId}
                  </code>
                  <button 
                    type="button"
                    onClick={() => handleCopy(currentAccount.machineId || '', 'machineId')}
                    className={`p-0.5 rounded hover:bg-muted/50 cursor-pointer transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500/30`}
                  >
                    {copied === 'machineId' ? <Check size={10} className="text-green-500" /> : <Copy size={10} className={"text-muted-foreground"} />}
                  </button>
                </div>
              )}
            </div>
            {/* Close button */}
            <button
              onClick={onClose}
              className="p-2 rounded-full hover:bg-muted transition-colors flex-shrink-0"
            >
              <X size={18} className="text-muted-foreground" />
            </button>
          </div>
        </div>
        
        {/* Body */}
        <DialogBody noPadding>
          {/* Quota overview */}
          <div className={`border-b border-border px-6 py-4`}>
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <div className={`p-1.5 rounded-lg bg-muted/30`}>
                  <CreditCard size={18} className={"text-muted-foreground"} />
                </div>
                <span className={`text-sm font-semibold text-foreground`}>{t('detail.quotaOverview')}</span>
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
                  <span className={`text-4xl font-semibold text-foreground`}>{formatUsage(totalUsed)}</span>
                  <span className={`text-lg text-muted-foreground ml-2`}>/ {formatUsage(totalQuota)}</span>
                </div>
                <span className={`text-base font-medium px-3 py-1 rounded-lg ${
                  totalPercent > 80 ? 'bg-red-500/20 text-red-500' 
                  : totalPercent > 50 ? 'bg-yellow-500/20 text-yellow-600' 
                  : 'bg-green-500/20 text-green-600'
                }`}>
                  {totalPercent.toFixed(0)}% {t('detail.used')}
                </span>
              </div>
              <div className={`h-4 bg-muted/30 rounded-full overflow-hidden shadow-inner`}>
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
              {/* Main quota card */}
              <QuotaCard
                title={t('detail.mainQuota')}
                used={form.used}
                quota={form.quota}
                icon="🔄"
                expiry={currentAccount.usageData?.nextDateReset ? `${new Date(currentAccount.usageData.nextDateReset * 1000).toLocaleDateString()} ${t('detail.reset')}` : null}
                colors={colors}
                t={t}
              />
              
              {/* Trial quota card */}
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
              
              {/* Bonus quota card */}
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
              <div className="mt-6 pt-5 border-t border-border">
                <div className="flex items-center gap-2 mb-4">
                  <span className="text-lg">🎁</span>
                  <span className={`text-sm font-medium text-foreground`}>{t('detail.bonusDetails')}</span>
                  <span className={`text-xs px-2 py-0.5 rounded-full info-badge font-medium`}>{bonuses.length}</span>
                </div>
                <div className="space-y-3">
                  {bonuses.map((bonus, idx) => (
                    <div key={idx} className={`flex items-center justify-between p-4 rounded-xl border transition-colors duration-200 hover:shadow-md ${
                      bonus.status === 'ACTIVE' 
                        ? 'bg-purple-500/10 border-purple-500/30' 
                        : `bg-muted/30 border-border`
                    }`}>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <span className={`text-sm font-medium text-foreground`}>{bonus.displayName || bonus.bonusCode}</span>
                          <span className={`text-xs px-2 py-0.5 rounded-md font-medium ${
                            bonus.status === 'ACTIVE' 
                              ? 'bg-green-500/20 text-green-500' 
                              : bonus.status === 'EXHAUSTED' 
                                ? `bg-muted/30 text-muted-foreground` 
                                : 'bg-yellow-500/20 text-yellow-600'
                          }`}>
                            {bonus.status}
                          </span>
                        </div>
                        <div className={`text-xs text-muted-foreground leading-relaxed`}>
                          {bonus.description && <span>{bonus.description} · </span>}
                          {bonus.redeemedAt && <span>{t('detail.redeemed')}: {new Date(bonus.redeemedAt * 1000).toLocaleDateString()} · </span>}
                          {bonus.expiresAt && <span>{t('detail.expires')}: {new Date(bonus.expiresAt * 1000).toLocaleDateString()}</span>}
                        </div>
                      </div>
                      <div className="text-right ml-4 flex-shrink-0">
                        <div className={`text-base font-semibold text-foreground`}>{formatUsage(bonus.currentUsage || 0)} <span className={`text-sm text-muted-foreground font-normal`}>/ {formatUsage(bonus.usageLimit || 0)}</span></div>
                        <div className={`text-xs text-muted-foreground font-mono mt-0.5`}>{bonus.bonusCode}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
            
            {/* Subscription information */}
            <div className="mt-6 pt-5 border-t border-border">
              <div className="flex items-center gap-2 mb-4">
                <span className="text-lg">📋</span>
                <span className={`text-sm font-medium text-foreground`}>{t('detail.subscriptionInfo', { defaultValue: 'Subscription Information' })}</span>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div className={`p-3 rounded-lg bg-muted/30`}>
                  <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.userId')}</div>
                  <div className={`text-foreground font-mono text-xs truncate`} title={currentAccount.usageData?.userInfo?.userId}>
                    {currentAccount.usageData?.userInfo?.userId?.slice(-12) || '-'}
                  </div>
                </div>
                <div className={`p-3 rounded-lg bg-muted/30`}>
                  <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.email')}</div>
                  <div className={`text-foreground text-xs truncate`}>
                    {currentAccount.usageData?.userInfo?.email || currentAccount.email || getAccountDisplayName(currentAccount)}
                  </div>
                </div>
                <div className={`p-3 rounded-lg bg-muted/30`}>
                  <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.subscriptionType')}</div>
                  <div className={`text-foreground font-mono text-xs truncate`} title={currentAccount.usageData?.subscriptionInfo?.type}>
                    {currentAccount.usageData?.subscriptionInfo?.type || '-'}
                  </div>
                </div>
                <div className={`p-3 rounded-lg bg-muted/30`}>
                  <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.upgradeable')}</div>
                  <div className={"text-foreground"}>
                    {currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? (
                      <span className="text-green-500 font-medium">✓ {t('common.yes')}</span>
                    ) : (
                      <span className={"text-muted-foreground"}>✗ {t('common.no')}</span>
                    )}
                  </div>
                </div>
                <div className={`p-3 rounded-lg bg-muted/30`}>
                  <div className={`text-xs text-muted-foreground mb-1`}>{t('home.overageCapability', { defaultValue: 'Overage Capability' })}</div>
                  <div className={"text-foreground"}>
                    {currentAccount.usageData?.subscriptionInfo?.overageCapability === 'OVERAGE_CAPABLE' ? (
                      <span className="text-green-500 font-medium">✓ {t('home.supported', { defaultValue: 'Supported' })}</span>
                    ) : (
                      <span className={"text-muted-foreground"}>✗ {t('home.notSupported', { defaultValue: 'Not supported' })}</span>
                    )}
                  </div>
                </div>
                {currentAccount.usageData?.subscriptionInfo?.overageCapability === 'OVERAGE_CAPABLE' && (
                  <div className={`p-3 rounded-lg bg-muted/30`}>
                    <div className={`text-xs text-muted-foreground mb-1`}>{t('home.overageStatus', { defaultValue: 'Overage Status' })}</div>
                    <div className={"text-foreground"}>
                      {currentAccount.usageData?.overageConfiguration?.overageStatus === 'ENABLED' ? (
                        <span className="text-green-500 font-medium">✓ {t('home.enabled')}</span>
                      ) : (
                        <span className={"text-muted-foreground"}>✗ {t('home.disabled')}</span>
                      )}
                    </div>
                  </div>
                )}
                {breakdown?.overageRate != null && (
                  <>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>{t('home.overageRate', { defaultValue: 'Overage Rate' })}</div>
                      <div className={`text-foreground font-medium`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageRate}/Credit
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>{t('home.overageCap', { defaultValue: 'Overage Cap' })}</div>
                      <div className={`text-foreground font-medium`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCap}
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>{t('home.currentOverage', { defaultValue: 'Current Overage' })}</div>
                      <div className={`text-foreground font-bold ${breakdown.currentOverages > 0 ? 'text-orange-500' : ''}`}>
                        {formatUsage(breakdown.currentOverages || 0)}
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>{t('home.overageCharges', { defaultValue: 'Overage Charges' })}</div>
                      <div className={`text-foreground font-bold ${breakdown.overageCharges > 0 ? 'text-orange-500' : ''}`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCharges?.toFixed(2) || '0.00'}
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>

          {/* Basic information and subscription details */}
          <div className="px-6 py-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* Basic information */}
              <section className="space-y-3">
                <h3 className="flex items-center gap-2 font-bold text-sm text-foreground">
                  <User size={16} className="text-primary" />
                  {t('detail.basicInfo')}
                </h3>
                <div className="bg-muted/30 border rounded-xl p-4 space-y-4">
                  <div className="space-y-1">
                    <label className="text-xs font-medium text-muted-foreground">{t('detail.emailAddress')}</label>
                    <div className="text-sm font-mono break-all select-all">{currentAccount.email || getAccountDisplayName(currentAccount)}</div>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-1 min-w-0">
                      <label className="text-xs font-medium text-muted-foreground">{t('detail.remarkLabel')}</label>
                      <div className="text-sm font-medium truncate">{currentAccount.label || '-'}</div>
                    </div>
                    <div className="space-y-1">
                      <label className="text-xs font-medium text-muted-foreground">Provider</label>
                      <div className="text-sm font-medium">{getProviderDisplayName(currentAccount.provider) || '-'}</div>
                    </div>
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs font-medium text-muted-foreground">User ID</label>
                    <div className="text-xs font-mono break-all bg-background p-2 rounded border select-all">
                      {currentAccount.usageData?.userInfo?.userId || '-'}
                    </div>
                  </div>
                </div>
              </section>

              {/* Subscription details */}
              <section className="space-y-3">
                <h3 className="flex items-center gap-2 font-bold text-sm text-foreground">
                  <Shield size={16} className="text-primary" />
                  {t('detail.subscriptionDetails', { defaultValue: 'Subscription Details' })}
                </h3>
                <div className="bg-muted/30 border rounded-xl p-4 text-sm space-y-3">
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">Region</span>
                    <span className="font-mono text-xs px-1.5 py-0.5 bg-muted rounded-md">us-east-1</span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">{t('home.expiresAt')}</span>
                    <span className="font-medium text-xs">{currentAccount.expiresAt || '-'}</span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">{t('detail.subscriptionType')}</span>
                    <span className="font-mono text-xs">{currentAccount.usageData?.subscriptionInfo?.type || '-'}</span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">{t('home.overageRate', { defaultValue: 'Overage Rate' })}</span>
                    <span className="font-mono text-xs">
                      {breakdown?.overageRate ? `$${breakdown.overageRate}/${breakdown.unit === 'INVOCATIONS' ? 'Credit' : breakdown.unit}` : '-'}
                    </span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">{t('home.resourceType', { defaultValue: 'Resource Type' })}</span>
                    <span className="font-mono text-xs">{breakdown?.resourceType || '-'}</span>
                  </div>
                  <div className="flex justify-between items-center py-1">
                    <span className="text-muted-foreground text-xs">{t('home.upgradeable', { defaultValue: 'Upgradeable' })}</span>
                    <span className={`text-xs font-bold ${currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? 'text-green-600' : 'text-muted-foreground'}`}>
                      {currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? 'YES' : 'NO'}
                    </span>
                  </div>
                </div>
              </section>
            </div>
          </div>

          {/* Account available models */}
          <div className={`px-6 py-4`}>
            <div 
              className="flex items-center gap-2 cursor-pointer select-none"
              onClick={() => setModelsExpanded(!modelsExpanded)}
            >
              <div className={`p-1.5 rounded-lg bg-muted/30`}>
                <Cpu size={18} className={"text-muted-foreground"} />
              </div>
              <span className={`text-sm font-semibold text-foreground`}>{t('detail.availableModels')}</span>
              <span className={`ml-auto text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 font-medium`}>
                {models.length}
              </span>
              <button
                onClick={(e) => { e.stopPropagation(); fetchModels(true) }}
                disabled={modelsLoading}
                className="p-1.5 rounded-lg hover:bg-muted/50 transition-colors disabled:opacity-50"
                title={t('detail.forceRefreshModels', { defaultValue: 'Force refresh model list' })}
              >
                <RefreshCw size={14} className={modelsLoading ? "animate-spin text-muted-foreground" : "text-muted-foreground"} />
              </button>
              <ChevronDown size={16} className={`text-muted-foreground transition-transform duration-200 ${modelsExpanded ? '' : '-rotate-90'}`} />
            </div>
            {modelsExpanded && (
            <div className="bg-gradient-to-br from-muted/20 to-muted/40 border rounded-xl p-4 mt-4">
              {modelsLoading ? (
                <div className="flex items-center justify-center py-8 text-muted-foreground">
                  <Loader2 size={20} className="animate-spin mr-2" />
                  <span className="text-sm">{t('detail.loadingModels')}</span>
                </div>
              ) : modelsError ? (
                <div className="text-center py-8">
                  <p className="text-red-500 text-sm">{modelsError}</p>
                </div>
              ) : models.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground text-sm">
                  {t('detail.noModels')}
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-h-[320px] overflow-y-auto pr-1">
                  {models.map((model, index) => (
                    <div 
                      key={model.modelId} 
                      className={`group p-3 bg-background rounded-xl border shadow-sm hover:shadow-md hover:border-primary/30 transition-all duration-200 ${
                        index === 0 ? 'ring-1 ring-primary/20' : ''
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1.5">
                            <div className={`w-2 h-2 rounded-full shrink-0 ${
                              index === 0 ? 'bg-primary animate-pulse' : 'bg-muted-foreground/40'
                            }`} />
                            <code className="text-xs font-bold text-foreground truncate">
                              {model.modelId}
                            </code>
                          </div>
                          {model.modelName && model.modelName !== model.modelId && (
                            <p className="text-[11px] text-primary/80 font-medium mb-1 truncate">{model.modelName}</p>
                          )}
                          <p className="text-[11px] text-muted-foreground line-clamp-2 leading-relaxed">
                            {model.description || t('detail.noDescription')}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center justify-between mt-2 pt-2 border-t border-border/50">
                        <div className="flex items-center gap-1.5">
                          {model.supportedInputTypes?.includes('TEXT') && (
                            <span className="text-[10px] px-1.5 h-5 bg-blue-500/10 text-blue-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                              <FileText size={12} />Text
                            </span>
                          )}
                          {model.supportedInputTypes?.includes('IMAGE') && (
                            <span className="text-[10px] px-1.5 h-5 bg-purple-500/10 text-purple-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                              <ImageIcon size={12} />Image
                            </span>
                          )}
                          {model.rateMultiplier !== undefined && (
                            <span className="text-[10px] px-1.5 h-5 bg-amber-500/10 text-amber-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                              <Zap size={12} />{model.rateMultiplier}x
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-1 text-[10px] text-muted-foreground font-mono">
                          <Hash size={12} />
                          <span className="text-green-600">
                            {model.tokenLimits?.maxInputTokens ? (model.tokenLimits.maxInputTokens >= 1000000 ? `${(model.tokenLimits.maxInputTokens / 1000000).toFixed(0)}M` : `${(model.tokenLimits.maxInputTokens / 1000).toFixed(0)}K`) : '-'}
                          </span>
                          <span>/</span>
                          <span className="text-orange-600">
                            {model.tokenLimits?.maxOutputTokens ? (model.tokenLimits.maxOutputTokens >= 1000000 ? `${(model.tokenLimits.maxOutputTokens / 1000000).toFixed(0)}M` : `${(model.tokenLimits.maxOutputTokens / 1000).toFixed(0)}K`) : '-'}
                          </span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
            )}
          </div>
        </DialogBody>

        {/* 状态栏（底部简洁显示） */}
        <div className="px-6 py-3 border-t border-border flex items-center gap-2">
          {statusMeta.tone === 'success'
            ? <><Shield size={14} className="text-green-500" /><span className="text-xs text-green-500 font-medium">{statusMeta.label}</span></>
            : statusMeta.tone === 'danger'
              ? <><Shield size={14} className="text-red-500" /><span className="text-xs text-red-500 font-medium">{statusMeta.label}</span></>
              : <><Shield size={14} className="text-orange-500" /><span className="text-xs text-orange-500 font-medium">{statusMeta.label}</span></>}
        </div>
      </DialogContent>
    </DialogRoot>,
    document.body
  )
}

export default AccountDetailModal
