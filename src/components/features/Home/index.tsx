import { useState, useEffect, useMemo, useCallback } from 'react'
import { Users, Zap, Shield, TrendingUp, Sparkles, Server, RefreshCw, ArrowRightLeft, Terminal } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { useAccount } from '../../../contexts/AccountContext'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { getQuota, getUsed, getSubPlan } from '../../../utils/accountStats'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

// 子组件
import LoadingSkeleton from './LoadingSkeleton'
import StatCard from './StatCard'

interface HomeProps {
  onNavigate: (path: string) => void;
}

function Home({ onNavigate }: HomeProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])

  const { showError } = useDialog()
  const { maskEmail } = usePrivacy()
  const {
    accounts: tokens,
    localToken,
    loading,
    refreshing,
    stats,
    currentAccount,
    currentQuotaInfo,
    refresh,
    refreshAccount,
  } = useAccount()
  
  const [refreshingAccount, setRefreshingAccount] = useState(false)
  const [mcpToolCount, setMcpToolCount] = useState(0)
  const [ideInstallInfo, setIdeInstallInfo] = useState<any>(null)

  const handleRefresh = useCallback(() => refresh(), [refresh])

  // 检测 IDE 安装状态
  useEffect(() => {
    const checkIdeInstallation = async () => {
      try {
        const info = await invoke<any>('check_ide_installation')
        setIdeInstallInfo(info)
      } catch (e) {
        console.error('检测 IDE 安装状态失败:', e)
      }
    }
    checkIdeInstallation()
  }, [])

  // 加载 MCP 工具数量
  useEffect(() => {
    const loadMcpToolCount = async () => {
      try {
        const statsResult = await invoke<any>('get_mcp_tool_stats', { projectDir: null })
        setMcpToolCount(statsResult.estimatedTools)
      } catch (e) {
        // 静默处理
      }
    }
    loadMcpToolCount()
  }, [])

  // 刷新当前账号
  const handleRefreshCurrentAccount = useCallback(async () => {
    if (!currentAccount || refreshingAccount) return
    setRefreshingAccount(true)
    try {
      await refreshAccount(currentAccount.id)
    } catch (e) {
      showError(t('common.refreshFailed'), String(e))
    } finally {
      setRefreshingAccount(false)
    }
  }, [currentAccount, refreshingAccount, refreshAccount, showError, t])

  // CLI 账号数据
  const [cliSnapshot, setCliSnapshot] = useState<any>(null)
  const [cliLoading, setCliLoading] = useState(false)
  const [cliPath, setCliPath] = useState('')
  const [cliInstalled, setCliInstalled] = useState(false)

  // 加载 CLI 账号
  useEffect(() => {
    const loadCliData = async () => {
      setCliLoading(true)
      try {
        const info = await invoke<any>('check_cli_installation')
        // 只根据可执行文件是否存在判断 CLI 是否安装
        setCliInstalled(info?.cli_installed || false)

        const path = await invoke<string>('get_kiro_cli_default_path')
        if (path) {
          setCliPath(path)
          try {
            const snapshot = await invoke<any>('read_cli_db_snapshot', { dbPath: path })
            setCliSnapshot(snapshot)
          } catch {
            // 数据库存在但读取失败，或未登录
          }
        }
      } catch (e) {
        // CLI 未安装
      } finally {
        setCliLoading(false)
      }
    }
    loadCliData()
  }, [])

  // 统计卡片
  const statCards = useMemo(() => [
    { icon: Users, iconBg: "info-badge", iconColor: accent.text, value: stats.total, label: t('home.totalAccounts'), delay: 'delay-100' },
    { icon: Shield, iconBg: "success-badge", iconColor: accent.text, value: `${stats.active}/${stats.unavailable}`, label: t('home.activeVsUnavailable'), delay: 'delay-200' },
    { icon: Zap, iconBg: "bg-purple-500/10 text-purple-500", iconColor: accent.text, value: stats.proPlus + stats.pro, label: t('home.proAccounts'), delay: 'delay-300' },
    { icon: TrendingUp, iconBg: "warning-badge", iconColor: 'text-orange-500', value: `${stats.usagePercent}%`, label: t('home.usagePercent'), delay: 'delay-400' },
    { 
      icon: Server, 
      iconBg: "bg-cyan-500/10 text-cyan-500", 
      iconColor: accent.text,
      value: mcpToolCount, 
      label: t('home.mcpTools'),
      delay: 'delay-500',
      onClick: () => onNavigate?.('kiroConfig'),
      warning: mcpToolCount > 50
    },
  ], [accent, stats, mcpToolCount, t, onNavigate])

  if (loading) {
    return <LoadingSkeleton />
  }

  return (
    <div className="h-full overflow-auto glass-main p-6">
      <div className="w-full">
        {/* Header */}
        <div className="mb-4 flex items-center gap-2.5 animate-slide-in-left">
          <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ring-1 ring-primary/20`}>
            <Sparkles size={20} className="text-white" />
          </div>
          <div className="flex flex-col">
            <h1 className="text-lg font-semibold text-foreground leading-tight">{t('home.title')}</h1>
            <p className="text-sm text-muted-foreground leading-tight">{t('home.subtitle')}</p>
          </div>
        </div>

        {/* Stat cards */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 mb-3">
          {statCards.map((card, index) => (
            <StatCard key={index} {...card} />
          ))}
        </div>

        {/* Current IDE account and CLI account */}
        <Card className="card-glow animate-scale-in delay-300">
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
            <div className="flex items-center gap-2">
              <Sparkles size={14} className={accent.text} />
              <span className="text-sm font-semibold text-foreground">{t('home.kiroAccount', { defaultValue: 'Kiro Account' })}</span>
            </div>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={handleRefreshCurrentAccount}
                    disabled={refreshingAccount || refreshing}
                    className={`h-7 w-7 ${refreshingAccount ? 'spinning' : ''}`}
                  >
                    <RefreshCw size={13} className="text-muted-foreground" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('common.refresh')}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>

          <CardContent className="p-0">
            <div className="grid grid-cols-1 md:grid-cols-[3fr_2fr]">
              {/* Current IDE account */}
              <div className="p-4 flex flex-col gap-3">
                <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider">
                  {t('home.currentIdeAccount', { defaultValue: 'Current IDE Account' })}
                </span>
                {currentAccount ? (
                  <CurrentAccountDetail
                    account={currentAccount}
                    accent={accent}
                    maskEmail={maskEmail}
                    t={t}
                  />
                ) : (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm py-10">
                    {localToken ? t('home.noMatchingAccount', { defaultValue: 'No matching account' }) : (
                      ideInstallInfo?.ide_installed === false
                        ? (ideInstallInfo?.ide_executable_exists === false
                            ? t('home.kiroIdeNotInstalled', { defaultValue: 'Kiro IDE is not installed' })
                            : t('home.kiroIdeInstalledNotLoggedIn', { defaultValue: 'Kiro IDE is installed but not logged in' }))
                        : t('home.notLoggedIn')
                    )}
                  </div>
                )}
              </div>

              {/* Current CLI account */}
              <div className="p-4 flex flex-col gap-3 bg-muted/20 border-t md:border-t-0 md:border-l border-border">
                <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider flex items-center gap-1.5">
                  <Terminal size={11} />
                  {t('home.currentCliAccount', { defaultValue: 'Current CLI Account' })}
                </span>
                {cliLoading ? (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
                    {t('common.loading')}
                  </div>
                ) : cliSnapshot ? (
                  <CliAccountDetail snapshot={cliSnapshot} cliPath={cliPath} t={t} />
                ) : cliInstalled ? (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-1.5 py-8">
                    <Terminal size={20} className="text-muted-foreground/50" />
                    <span>{t('home.cliInstalledNotLoggedIn', { defaultValue: 'CLI is installed but not logged in' })}</span>
                    <span className="text-[11px] text-muted-foreground/70">{t('home.cliLoginHint', { defaultValue: 'Run kiro-cli login to sign in' })}</span>
                  </div>
                ) : (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-1.5 py-8">
                    <Terminal size={20} className="text-muted-foreground/50" />
                    <span>{t('home.cliNotInstalled', { defaultValue: 'CLI is not installed' })}</span>
                    <span className="text-[11px] text-muted-foreground/70">{t('home.cliInstallHint', { defaultValue: 'Install Kiro CLI and restart' })}</span>
                  </div>
                )}
              </div>
            </div>

            {/* Account navigation */}
            <button
              onClick={() => onNavigate?.('accounts')}
              className="w-full py-2.5 flex items-center justify-center gap-2 border-t border-border bg-primary/5 hover:bg-primary/10 text-primary text-sm font-medium transition-colors"
            >
              <ArrowRightLeft size={13} />
              {t('home.viewAllAccounts', { defaultValue: 'View all accounts' })}
            </button>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function CliAccountDetail({ snapshot, cliPath, t }: { snapshot: any; cliPath: string; t: any }) {
  const entries = snapshot?.token_entries || []
  const deviceReg = snapshot?.device_registration

  // Primary token entry.
  const mainEntry = entries[0]
  const tokenData = mainEntry?.parsed_token

  if (!tokenData) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-2">
        <span>{t('home.noValidToken', { defaultValue: 'No valid token' })}</span>
        <span className="text-[10px] font-mono truncate max-w-full">{cliPath}</span>
      </div>
    )
  }

  // Auth type.
  const isOidc = mainEntry.key?.includes('odic')
  const isSocial = mainEntry.key?.includes('social')
  const authMethod = isSocial ? 'Social' : isOidc ? 'IdC (BuilderId)' : 'Unknown'

  // Token expiration.
  let expiresStr = '-'
  let isExpired = false
  if (tokenData.expires_at) {
    const expiresDate = new Date(tokenData.expires_at)
    expiresStr = expiresDate.toLocaleString()
    isExpired = expiresDate.getTime() < Date.now()
  }

  // Truncate long values.
  const truncate = (s: string, len = 16) => s ? (s.length > len ? s.substring(0, len) + '...' : s) : '-'

  return (
    <div className="flex-1 flex flex-col gap-3">
      {/* Status */}
      <div className="flex items-center gap-2 bg-muted/30 border border-border rounded-xl p-3">
        <div className="w-9 h-9 rounded-lg flex items-center justify-center bg-gradient-to-br from-emerald-500 to-teal-600 text-white font-bold text-sm shrink-0">
          C
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-semibold text-foreground">{authMethod}</span>
          <span className="text-[11px] text-muted-foreground font-mono truncate">{mainEntry.key}</span>
        </div>
        <Badge variant="default" className={`shrink-0 text-[10px] px-1.5 py-0 ${isExpired ? 'bg-red-500' : 'bg-green-500'}`}>
          {isExpired ? t('filter.statusExpired') : t('home.valid', { defaultValue: 'Valid' })}
        </Badge>
      </div>

      {/* Token information */}
      <div className="bg-muted/30 border border-border rounded-xl p-3">
        <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">Token</span>
        <div className="flex flex-col gap-1.5">
          <InfoRow label="Access Token" value={truncate(tokenData.access_token, 20)} mono />
          <InfoRow label="Refresh Token" value={truncate(tokenData.refresh_token, 20)} mono />
          <InfoRow label={t('home.expiresAt')} value={expiresStr} valueClass={isExpired ? 'text-red-500' : 'text-green-500'} />
          <InfoRow label="Region" value={tokenData.region || 'us-east-1'} mono />
          {tokenData.start_url && (
            <InfoRow label="Start URL" value={truncate(tokenData.start_url, 24)} mono />
          )}
          {tokenData.oauth_flow && (
            <InfoRow label="OAuth Flow" value={tokenData.oauth_flow} />
          )}
          {tokenData.scopes && tokenData.scopes.length > 0 && (
            <InfoRow label="Scopes" value={`${tokenData.scopes.length}`} />
          )}
        </div>
      </div>

      {/* Device Registration */}
      {deviceReg && (
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">Device Registration</span>
          <div className="flex flex-col gap-1.5">
            <InfoRow label="Client ID" value={truncate(deviceReg.client_id, 20)} mono />
            <InfoRow label="Client Secret" value={truncate(deviceReg.client_secret, 20)} mono />
            <InfoRow label="Region" value={deviceReg.region || 'us-east-1'} mono />
          </div>
        </div>
      )}

      {/* Database path */}
      <div className="bg-muted/30 border border-border rounded-xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">{t('home.dbPath', { defaultValue: 'DB Path' })}</span>
          <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[180px]" title={cliPath}>
            {cliPath.split(/[/\\]/).slice(-2).join('/')}
          </span>
        </div>
      </div>
    </div>
  )
}

// 当前账号完整解析卡片
function CurrentAccountDetail({ account, accent, maskEmail, t }: {
  account: any;
  accent: any;
  maskEmail: (s: string) => string;
  t: any;
}) {
  const quota = getQuota(account)
  const used = getUsed(account)
  const remaining = Math.max(0, quota - used)
  const percent = quota > 0 ? Math.round((used / quota) * 100) : 0
  const plan = getSubPlan(account)
  const email = account.usageData?.userInfo?.email || account.email || ''
  const provider = account.provider || ''
  const usageData = account.usageData
  const breakdown = usageData?.usageBreakdownList?.[0]
  const overageConfig = usageData?.overageConfiguration
  const subInfo = usageData?.subscriptionInfo
  const userInfo = usageData?.userInfo
  const nextReset = usageData?.nextDateReset
  const freeTrial = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []
  const mainUsed = breakdown?.currentUsage ?? 0
  const mainUsedPrecision = breakdown?.currentUsageWithPrecision ?? mainUsed
  const mainLimit = breakdown?.usageLimit ?? 0
  const mainLimitPrecision = breakdown?.usageLimitWithPrecision ?? mainLimit
  const mainPercent = mainLimit > 0 ? Math.round((mainUsed / mainLimit) * 100) : 0

  // Overage-related fields.
  const currentOverages = breakdown?.currentOverages ?? 0
  const currentOveragesPrecision = breakdown?.currentOveragesWithPrecision ?? currentOverages
  const overageCap = breakdown?.overageCap ?? 0
  const overageCapPrecision = breakdown?.overageCapWithPrecision ?? overageCap
  const overageCharges = breakdown?.overageCharges ?? 0
  const overageRate = breakdown?.overageRate ?? 0

  const isOverage = currentOverages > 0
  const overageAmount = used > quota ? used - quota : 0
  const displayName = breakdown?.displayName || 'Credit'
  const displayNamePlural = breakdown?.displayNamePlural || 'Credits'
  const resourceType = breakdown?.resourceType || ''
  const currency = breakdown?.currency || ''
  const unit = breakdown?.unit || ''

  const getBarClass = (pct: number) => {
    if (pct > 100) return 'bg-purple-500'
    if (pct > 80) return 'bg-red-500'
    if (pct > 50) return 'bg-amber-500'
    return 'bg-green-500'
  }

  const getPercentClass = (pct: number) => {
    if (pct > 100) return 'text-purple-500'
    if (pct > 80) return 'text-red-500'
    if (pct > 50) return 'text-amber-500'
    return 'text-green-500'
  }

  // Reset time.
  let resetStr = ''
  let daysUntilReset: number | null = null
  let resetDateStr = ''
  if (nextReset) {
    const resetDate = new Date(typeof nextReset === 'string' ? nextReset : (nextReset < 1e12 ? nextReset * 1000 : nextReset))
    resetDateStr = resetDate.toLocaleDateString()
    const now = new Date()
    daysUntilReset = Math.max(0, Math.ceil((resetDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)))
    resetStr = daysUntilReset === 0 ? t('home.resetToday') : `${daysUntilReset} ${t('home.daysUntilReset')}`
  }

  // Overage usage percentage relative to the overage cap.
  const overagePercent = overageCap > 0 ? Math.round((currentOverages / overageCap) * 100) : 0

  return (
    <div className="flex-1 flex flex-col gap-3">
      {/* Header: email, plan, and provider */}
      <div className="flex items-center gap-2 bg-muted/30 border border-border rounded-xl p-3">
        <div className={`w-10 h-10 rounded-lg flex items-center justify-center text-white font-bold text-sm shrink-0 ${
          provider === 'Google' ? 'bg-gradient-to-br from-red-500 to-orange-500' :
          isGitHubProvider(provider) ? 'bg-gradient-to-br from-gray-700 to-gray-900' :
          `bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo}`
        }`}>
          {provider?.[0] || 'K'}
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-semibold text-foreground truncate">
            {email ? maskEmail(email) : getProviderDisplayName(provider)}
          </span>
          <span className="text-[11px] text-muted-foreground">
            {getProviderDisplayName(provider)}
            {daysUntilReset != null && ` · ${resetStr}`}
          </span>
        </div>
        {plan && (
          <Badge variant="default" className="shrink-0 text-[10px] px-1.5 py-0"
            style={{ background: plan.includes('PRO+') ? 'linear-gradient(to right, rgb(168, 85, 247), rgb(236, 72, 153))' : plan.includes('PRO') ? 'rgb(59, 130, 246)' : undefined }}>
            {plan}
          </Badge>
        )}
      </div>

      {/* Total quota progress */}
      <div className="bg-muted/30 border border-border rounded-xl p-3">
        <div className="flex items-center justify-between mb-1.5">
          <span className="text-xs font-medium text-foreground">
            {t('home.monthlyUsage')} ({displayNamePlural})
          </span>
          <div className="flex items-center gap-2">
            <span className={`text-sm font-bold font-mono ${getPercentClass(percent)}`}>{percent}%</span>
          </div>
        </div>
        <div className="h-[5px] bg-muted rounded-full overflow-hidden mb-1.5">
          <div className={`h-full rounded-full transition-all duration-500 ${getBarClass(percent)}`} style={{ width: `${Math.min(percent, 100)}%` }} />
        </div>
        <div className="flex items-center justify-between">
          <span className="text-[11px] text-muted-foreground font-mono">
            {isOverage ? mainLimitPrecision : mainUsedPrecision} / {mainLimitPrecision} {displayName}
          </span>
          {isOverage ? (
            <span className="text-[11px] font-semibold text-purple-500">{t('home.overage')} {currentOveragesPrecision}</span>
          ) : (
            <span className={`text-[11px] font-semibold ${getPercentClass(percent)}`}>{t('home.remaining', { defaultValue: 'Remaining' })} {remaining}</span>
          )}
        </div>
      </div>

      {/* Overage details */}
      {currentOverages > 0 && (
        <div className="bg-purple-500/5 border border-purple-500/20 rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-purple-500 tracking-wider mb-2 block">{t('home.overageDetails', { defaultValue: 'Overage Details' })}</span>
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">{t('home.overageUsage', { defaultValue: 'Overage Usage' })}</span>
              <span className="text-[11px] font-mono text-purple-500 font-semibold">{currentOveragesPrecision} / {overageCapPrecision}</span>
            </div>
            {/* Overage progress bar */}
            <div className="h-[3px] bg-purple-500/10 rounded-full overflow-hidden">
              <div className="h-full rounded-full bg-purple-500 transition-all" style={{ width: `${Math.min(overagePercent, 100)}%` }} />
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">{t('home.overageCharges', { defaultValue: 'Overage Charges' })}</span>
              <span className="text-[11px] font-mono text-purple-500 font-semibold">${overageCharges.toFixed(2)} {currency}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">{t('home.rate')}</span>
              <span className="text-[11px] font-mono text-muted-foreground">${overageRate}/{displayName}</span>
            </div>
          </div>
        </div>
      )}

      {/* Quota breakdown */}
      {breakdown && (
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">{t('home.quotaDetails')}</span>
          <div className="flex flex-col gap-2">
            {/* Base quota */}
            <QuotaRow label={t('home.base')} used={mainUsed} limit={mainLimit} percent={mainPercent} color="blue" accent={accent} t={t} />

            {/* Trial quota */}
            {freeTrial && freeTrial.freeTrialStatus === 'ACTIVE' && freeTrial.usageLimit > 0 && (
              <QuotaRow
                label={t('home.trial')}
                used={freeTrial.currentUsage ?? 0}
                limit={freeTrial.usageLimit}
                percent={freeTrial.usageLimit > 0 ? Math.round((freeTrial.currentUsage ?? 0) / freeTrial.usageLimit * 100) : 0}
                color="purple"
                accent={accent}
                t={t}
              />
            )}

            {/* Bonus quota */}
            {bonuses.filter((b: any) => {
              const now = Date.now()
              const expiry = b.expiresAt ? b.expiresAt * 1000 : Infinity
              return expiry > now && b.status === 'ACTIVE'
            }).map((bonus: any, idx: number) => (
              <QuotaRow
                key={idx}
                label={bonus.displayName?.substring(0, 4) || `${t('home.bonus', { defaultValue: 'Bonus' })}${idx + 1}`}
                used={Math.round(bonus.currentUsage ?? 0)}
                limit={Math.round(bonus.usageLimit ?? 0)}
                percent={bonus.usageLimit > 0 ? Math.round((bonus.currentUsage ?? 0) / bonus.usageLimit * 100) : 0}
                color="amber"
                accent={accent}
                expiry={bonus.expiresAt}
                t={t}
              />
            ))}
          </div>
        </div>
      )}

      {/* Subscription and account info */}
      <div className="grid grid-cols-2 gap-2">
        {/* Subscription info */}
        {subInfo && (
          <div className="bg-muted/30 border border-border rounded-xl p-3">
            <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">{t('accounts.subscription', { defaultValue: 'Subscription' })}</span>
            <div className="flex flex-col gap-1.5">
              <InfoRow label={t('home.type')} value={subInfo.subscriptionTitle || 'Free'} />
              <InfoRow label={t('home.plan', { defaultValue: 'Plan' })} value={subInfo.type?.replace('Q_DEVELOPER_STANDALONE_', '') || '-'} mono />
              <InfoRow label={t('home.overageCapability', { defaultValue: 'Overage Capability' })} value={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? `✓ ${t('home.supported', { defaultValue: 'Supported' })}` : '✗'} valueClass={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? 'text-green-500' : ''} />
              <InfoRow label={t('home.upgradeCapability', { defaultValue: 'Upgrade Capability' })} value={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? `✓ ${t('home.upgradeable', { defaultValue: 'Upgradeable' })}` : '✗'} valueClass={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? 'text-green-500' : ''} />
              {overageConfig && (
                <InfoRow label={t('home.overageToggle', { defaultValue: 'Overage Toggle' })} value={overageConfig.overageStatus === 'ENABLED' ? `⚡ ${t('home.enabled')}` : t('home.disabled')} valueClass={overageConfig.overageStatus === 'ENABLED' ? 'text-purple-500 font-semibold' : ''} />
              )}
              {subInfo.subscriptionManagementTarget && (
                <InfoRow label={t('home.management', { defaultValue: 'Management' })} value={subInfo.subscriptionManagementTarget} mono />
              )}
            </div>
          </div>
        )}

        {/* Account and resource info */}
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">{t('home.accountAndResource', { defaultValue: 'Account & Resource' })}</span>
          <div className="flex flex-col gap-1.5">
            <InfoRow label="IDP" value={getProviderDisplayName(provider) || '-'} />
            <InfoRow label={t('home.resetDay', { defaultValue: 'Reset Day' })} value={resetDateStr || '-'} />
            {userInfo?.userId && (
              <InfoRow label={t('home.userId', { defaultValue: 'User ID' })} value={userInfo.userId.split('.').pop()?.substring(0, 12) || '-'} mono />
            )}
            {resourceType && (
              <InfoRow label={t('home.resourceType', { defaultValue: 'Resource Type' })} value={resourceType} mono />
            )}
            {currency && (
              <InfoRow label={t('home.currency', { defaultValue: 'Currency' })} value={currency} />
            )}
            {unit && (
              <InfoRow label={t('home.unit', { defaultValue: 'Unit' })} value={unit === 'INVOCATIONS' ? t('home.invocations', { defaultValue: 'Invocations' }) : unit} />
            )}
            {overageCap > 0 && (
              <InfoRow label={t('home.overageCap', { defaultValue: 'Overage Cap' })} value={`${overageCapPrecision}`} />
            )}
            {overageRate > 0 && (
              <InfoRow label={t('home.overageRate', { defaultValue: 'Overage Rate' })} value={`$${overageRate}/${displayName}`} mono />
            )}
          </div>
        </div>
      </div>

      {/* IDE token path */}
      <div className="bg-muted/30 border border-border rounded-xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">{t('home.tokenPath', { defaultValue: 'Token Path' })}</span>
          <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[220px]" title="~/.aws/sso/cache/">
            .aws/sso/cache/
          </span>
        </div>
      </div>
    </div>
  )
}

// Quota row.
function QuotaRow({ label, used, limit, percent, color, accent, expiry, t }: {
  label: string;
  used: number;
  limit: number;
  percent: number;
  color: 'blue' | 'purple' | 'amber';
  accent: any;
  expiry?: number;
  t: any;
}) {
  const colorMap = {
    blue: { dot: 'bg-blue-500', bar: 'bg-blue-500', text: 'text-blue-600' },
    purple: { dot: 'bg-purple-500', bar: 'bg-purple-500', text: 'text-purple-600' },
    amber: { dot: 'bg-amber-500', bar: 'bg-amber-500', text: 'text-amber-600' },
  }
  const c = colorMap[color]
  const expiryStr = expiry ? new Date(expiry * 1000).toLocaleDateString() : null

  return (
    <div className="flex items-center gap-2">
      <div className={`w-1.5 h-1.5 rounded-full ${c.dot} shrink-0`} />
      <span className="text-[11px] text-muted-foreground w-10 shrink-0" title={expiryStr ? `${expiryStr} ${t('home.expires')}` : ''}>{label}</span>
      <div className="flex-1 h-[3px] bg-muted rounded-full overflow-hidden">
        <div className={`h-full rounded-full ${c.bar} transition-all`} style={{ width: `${Math.min(percent, 100)}%` }} />
      </div>
      <span className={`text-[10px] font-mono ${c.text} w-20 text-right shrink-0`}>
        {used}/{limit}{expiryStr ? ` · ${expiryStr}` : ''}
      </span>
    </div>
  )
}

// 信息行
function InfoRow({ label, value, valueClass, mono }: {
  label: string;
  value: string;
  valueClass?: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[11px] text-muted-foreground">{label}</span>
      <span className={`text-[11px] ${valueClass || 'text-foreground'} ${mono ? 'font-mono' : ''} truncate max-w-[100px]`}>{value}</span>
    </div>
  )
}

export default Home
