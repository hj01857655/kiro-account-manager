import { useState, useRef, useEffect, memo, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { listAvailableModels, syncAccount } from '../../../api/accountApi'
import { Copy, Check, RefreshCw, User, CreditCard, Shield, Cpu, Loader2, FileText, Image as ImageIcon, Zap, Hash, ChevronDown, X } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { formatUsage, getAccountDisplayName, calcTotalUsageWithExtras } from '../../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus } from '../../../utils/accountStatus'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import {
  DialogRoot,
  DialogContent,
  DialogBody} from '../../shared/dialog'
import { Switch } from '../../ui/switch'
import { Account, AvailableModel, ListAvailableModelsResponse } from '../../../types/account'
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

const formatTokenLimit = (value?: number | null) => {
  if (!value) return '-'
  if (value >= 1000000) return `${(value / 1000000).toFixed(value % 1000000 === 0 ? 0 : 1)}M`
  if (value >= 1000) return `${(value / 1000).toFixed(value % 1000 === 0 ? 0 : 1)}K`
  return String(value)
}

const formatModelList = (values?: string[] | null) => values?.filter(Boolean).join(', ') || '-'

const formatEffortLabel = (model: AvailableModel) => {
  const levels = formatModelList(model.effortLevels)
  return model.effortSchemaPath ? `${model.effortSchemaPath}: ${levels}` : levels
}

const getPrimaryUsage = (account: Account) => {
  const breakdown = account.usageData?.usageBreakdownList?.[0]
  return {
    quota: breakdown?.usageLimit ?? 0,
    used: breakdown?.currentUsage ?? 0,
  }
}

function AccountDetailModal({ account, onClose, onRefresh }: AccountDetailModalProps) {
  const { t } = useApp()
  const { showError } = useDialog()
  const [currentAccount, setCurrentAccount] = useState<Account>(account)

  // 样式定义
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])

  const initialUsage = getPrimaryUsage(currentAccount)

  const [form, setForm] = useState({
    email: currentAccount.email || getAccountDisplayName(currentAccount),
    label: currentAccount.label || '',
    quota: initialUsage.quota,
    used: initialUsage.used,
    status: currentAccount.status,
    accessToken: currentAccount.accessToken || '',
    refreshToken: currentAccount.refreshToken || ''})

  const [refreshing, setRefreshing] = useState(false)
  const [copied, setCopied] = useState<string | null>(null)
  const copiedTimerRef = useRef<NodeJS.Timeout | null>(null)

  // Models 相关 state
  const [models, setModels] = useState<AvailableModel[]>([])
  const [modelsLoading, setModelsLoading] = useState(false)
  const [modelsError, setModelsError] = useState<string | null>(null)
  const [modelsExpanded, setModelsExpanded] = useState(false)

  // 获取可用模型
  const fetchModels = async (forceRefresh = false) => {
    setModelsLoading(true)
    setModelsError(null)
    try {
      console.log('[AccountDetailModal] Fetching models for account:', account.id, 'forceRefresh:', forceRefresh)
      const response = await listAvailableModels(account.id, forceRefresh)
      console.log('[AccountDetailModal] Models response:', response)
      const modelsList = response.availableModels
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

  const handleToggleModelsExpanded = () => {
    const nextExpanded = !modelsExpanded
    setModelsExpanded(nextExpanded)
    if (nextExpanded && models.length === 0 && !modelsLoading) {
      fetchModels()
    }
  }

  useEffect(() => {
    setCurrentAccount(account)
    const usage = getPrimaryUsage(account)
    setForm({
      email: account.email || getAccountDisplayName(account),
      label: account.label || '',
      quota: usage.quota,
      used: usage.used,
      status: account.status,
      accessToken: account.accessToken || '',
      refreshToken: account.refreshToken || ''})
  }, [account])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const result = await syncAccount(account.id)
      const updated = result.account
      setCurrentAccount(updated)

      // 如果有警告，显示提示
      if (result.warning) {
        await showError('同步警告', result.warning)
      }

      // 封禁账号额度为 0
      const isBanned = isBannedStatus(updated)
      const quota = isBanned ? 0 : (updated.usageData?.usageBreakdownList?.[0]?.usageLimit ?? 0)
      const used = updated.usageData?.usageBreakdownList?.[0]?.currentUsage ?? 0
      setForm(prev => ({ ...prev, quota, used, status: updated.status }))
      void onRefresh?.()
    } catch (e) {
      const errorMsg = String(e)
      // sync_account 后端已处理状态更新，前端同步状态用于表单显示
      let status = account.status
      if (errorMsg.includes('BANNED')) {
        status = 'banned'
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid') || errorMsg.includes('失效')) {
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

  // 计算总配额和使用量（基于表单值 + usageData 中的额外配额）
  const {
    totalQuota,
    totalUsed,
    totalPercent,
    freeTrialQuota,
    freeTrialUsed,
    bonusQuota,
    bonusUsed
  } = calcTotalUsageWithExtras(
    form.quota,
    form.used,
    currentAccount.usageData
  )

  // 从 usageData 读取额外信息（用于显示详情）
  const breakdown = currentAccount.usageData?.usageBreakdownList?.[0]
  const freeTrialInfo = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []

  const statusMeta = getAccountStatusMeta({ status: form.status, usageData: currentAccount.usageData }, t)

  return createPortal(
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="800px" showClose={false}>
        {/* 顶部渐变背景 */}
        <div className="absolute top-0 left-0 right-0 h-40 bg-gradient-to-br from-blue-500/5 via-purple-500/3 to-transparent pointer-events-none rounded-t-2xl" />

        <div className={`sticky top-0 z-20 bg-background border-b border-border px-6 py-4 rounded-t-2xl`}>
          <div className="text-xs font-medium text-muted-foreground mb-3 uppercase tracking-wider">账号详情</div>
          <div className="flex items-start gap-3">
            {/* 头像图标 */}
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

            {/* 账号信息 */}
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

              {/* 机器码 */}
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
            {/* 关闭按钮 */}
            <button
              onClick={onClose}
              className="p-2 rounded-full hover:bg-muted transition-colors flex-shrink-0"
            >
              <X size={18} className="text-muted-foreground" />
            </button>
          </div>
        </div>

        {/* Body - 使用 DialogBody 的 noPadding，自己控制每个区域的 padding */}
        <DialogBody noPadding>
          {/* 配额总览 */}
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
              {/* 主配额卡片 */}
              <QuotaCard
                title={t('detail.mainQuota')}
                used={form.used}
                quota={form.quota}
                icon="🔄"
                expiry={currentAccount.usageData?.nextDateReset ? (() => {
                  try {
                    const date = new Date(currentAccount.usageData.nextDateReset * 1000)
                    return !isNaN(date.getTime()) ? `${date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })} ${t('detail.reset')}` : null
                  } catch {
                    return null
                  }
                })() : null}
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
                expiry={freeTrialInfo?.freeTrialExpiry ? (() => {
                  try {
                    const date = new Date(freeTrialInfo.freeTrialExpiry * 1000)
                    return !isNaN(date.getTime()) ? `${date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })} ${t('detail.expires')}` : null
                  } catch {
                    return null
                  }
                })() : null}
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
                          {bonus.redeemedAt && <span>{t('detail.redeemed')}: {(() => {
                            try {
                              const date = new Date(bonus.redeemedAt * 1000)
                              return !isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' }) : '-'
                            } catch {
                              return '-'
                            }
                          })()} · </span>}
                          {bonus.expiresAt && <span>{t('detail.expires')}: {(() => {
                            try {
                              const date = new Date(bonus.expiresAt * 1000)
                              return !isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' }) : '-'
                            } catch {
                              return '-'
                            }
                          })()}</span>}
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

            {/* 订阅信息 */}
            <div className="mt-6 pt-5 border-t border-border">
              <div className="flex items-center gap-2 mb-4">
                <span className="text-lg">📋</span>
                <span className={`text-sm font-medium text-foreground`}>订阅信息</span>
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
                  <div className={`text-xs text-muted-foreground mb-1`}>超额能力</div>
                  <div className={"text-foreground"}>
                    {currentAccount.usageData?.subscriptionInfo?.overageCapability === 'OVERAGE_CAPABLE' ? (
                      <span className="text-green-500 font-medium">✓ 支持</span>
                    ) : (
                      <span className={"text-muted-foreground"}>✗ 不支持</span>
                    )}
                  </div>
                </div>
                {currentAccount.usageData?.subscriptionInfo?.overageCapability === 'OVERAGE_CAPABLE' && (
                  <div className={`p-3 rounded-lg bg-muted/30`}>
                    <div className={`text-xs text-muted-foreground mb-1`}>超额状态</div>
                    <div className={"text-foreground"}>
                      {currentAccount.usageData?.overageConfiguration?.overageStatus === 'ENABLED' ? (
                        <span className="text-green-500 font-medium">✓ 已开启</span>
                      ) : (
                        <span className={"text-muted-foreground"}>✗ 已关闭</span>
                      )}
                    </div>
                  </div>
                )}
                {breakdown?.overageRate != null && (
                  <>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>超额费率</div>
                      <div className={`text-foreground font-medium`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageRate}/Credit
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>超额上限</div>
                      <div className={`text-foreground font-medium`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCap}
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>当前超额</div>
                      <div className={`text-foreground font-bold ${breakdown.currentOverages > 0 ? 'text-orange-500' : ''}`}>
                        {formatUsage(breakdown.currentOverages || 0)}
                      </div>
                    </div>
                    <div className={`p-3 rounded-lg bg-muted/30`}>
                      <div className={`text-xs text-muted-foreground mb-1`}>超额费用</div>
                      <div className={`text-foreground font-bold ${breakdown.overageCharges > 0 ? 'text-orange-500' : ''}`}>
                        {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCharges?.toFixed(2) || '0.00'}
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>

          {/* 基本信息 & 订阅详情 - 并排布局 */}
          <div className="px-6 py-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* 基本信息 */}
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

              {/* 订阅详情 */}
              <section className="space-y-3">
                <h3 className="flex items-center gap-2 font-bold text-sm text-foreground">
                  <Shield size={16} className="text-primary" />
                  订阅详情
                </h3>
                <div className="bg-muted/30 border rounded-xl p-4 text-sm space-y-3">
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">Region</span>
                    <span className="font-mono text-xs px-1.5 py-0.5 bg-muted rounded-md">us-east-1</span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">Token 到期</span>
                    <span className="font-medium text-xs">{currentAccount.expiresAt || '-'}</span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">订阅类型</span>
                    <span className="font-mono text-xs">{currentAccount.usageData?.subscriptionInfo?.type || '-'}</span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">超额费率</span>
                    <span className="font-mono text-xs">
                      {breakdown?.overageRate ? `$${breakdown.overageRate}/${breakdown.unit === 'INVOCATIONS' ? 'Credit' : breakdown.unit}` : '-'}
                    </span>
                  </div>
                  <div className="flex justify-between items-center py-1 border-b border-border/50">
                    <span className="text-muted-foreground text-xs">资源类型</span>
                    <span className="font-mono text-xs">{breakdown?.resourceType || '-'}</span>
                  </div>
                  <div className="flex justify-between items-center py-1">
                    <span className="text-muted-foreground text-xs">可升级</span>
                    <span className={`text-xs font-bold ${currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? 'text-green-600' : 'text-muted-foreground'}`}>
                      {currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? 'YES' : 'NO'}
                    </span>
                  </div>
                </div>
              </section>
            </div>
          </div>

          {/* 账户可用模型 */}
          <div className={`px-6 py-4`}>
            <div
              className="flex items-center gap-2 cursor-pointer select-none"
              onClick={handleToggleModelsExpanded}
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
                title="强制刷新模型列表"
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
                  {models.map((model) => (
                    <div
                      key={model.modelId}
                      className={`group p-3 bg-background rounded-xl border shadow-sm hover:shadow-md hover:border-primary/30 transition-all duration-200 ${
                        model.isDefault ? 'ring-1 ring-primary/20' : ''
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1.5">
                            <div className={`w-2 h-2 rounded-full shrink-0 ${
                              model.isDefault ? 'bg-primary animate-pulse' : 'bg-muted-foreground/40'
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
                      <div className="flex flex-wrap items-center gap-1.5 mt-2 pt-2 border-t border-border/50">
                        {model.provider && (
                          <span className="text-[10px] px-1.5 h-5 bg-slate-500/10 text-slate-600 dark:text-slate-300 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                            {model.provider}
                          </span>
                        )}
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
                        {model.rateMultiplier !== undefined && model.rateMultiplier !== null && (
                          <span className="text-[10px] px-1.5 h-5 bg-amber-500/10 text-amber-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                            <Zap size={12} />{model.rateMultiplier}x{model.rateUnit ? ` ${model.rateUnit}` : ''}
                          </span>
                        )}
                        <span className="text-[10px] px-1.5 h-5 bg-emerald-500/10 text-emerald-600 border-0 rounded inline-flex items-center gap-0.5 font-medium font-mono">
                          <Hash size={12} />{formatTokenLimit(model.tokenLimits?.maxInputTokens)} / {formatTokenLimit(model.tokenLimits?.maxOutputTokens)}
                        </span>
                        {model.promptCaching?.supportsPromptCaching !== undefined && model.promptCaching?.supportsPromptCaching !== null && (
                          <span className={`text-[10px] px-1.5 h-5 border-0 rounded inline-flex items-center gap-0.5 font-medium ${
                            model.promptCaching.supportsPromptCaching
                              ? 'bg-green-500/10 text-green-600'
                              : 'bg-muted text-muted-foreground'
                          }`}>
                            Cache {model.promptCaching.supportsPromptCaching ? 'On' : 'Off'}
                          </span>
                        )}
                        {model.contextWindow && (
                          <span className="text-[10px] px-1.5 h-5 bg-cyan-500/10 text-cyan-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                            Context {formatTokenLimit(model.contextWindow)}
                          </span>
                        )}
                        {model.effortLevels?.length ? (
                          <span className="text-[10px] px-1.5 h-5 bg-indigo-500/10 text-indigo-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                            Effort {formatEffortLabel(model)}
                          </span>
                        ) : null}
                        {model.capabilities?.length ? (
                          <span className="text-[10px] px-1.5 h-5 bg-muted text-muted-foreground border-0 rounded inline-flex items-center gap-0.5 font-medium max-w-full truncate">
                            Cap {formatModelList(model.capabilities)}
                          </span>
                        ) : null}
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
