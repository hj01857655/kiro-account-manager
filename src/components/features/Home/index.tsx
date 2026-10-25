import { useState, useEffect, useMemo, useCallback } from 'react'
import { Users, Zap, Shield, TrendingUp, Sparkles, Server, RefreshCw, ArrowRightLeft, Terminal } from 'lucide-react'
import { checkCliInstallation, getKiroCliDefaultPath, readCliDbSnapshot } from '../../../api/systemApi'
import { quickSwitchNext } from '../../../api/accountApi'
import { checkIdeInstallation as checkIdeInstallationApi } from '../../../api/settingsApi'
import { getMcpToolStats } from '../../../api/mcpApi'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { showSuccess } from '../../../utils/toast'
import { useAccount } from '../../../contexts/AccountContext'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { getThemeAccent } from '../AgentConfig/themeAccent'
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
  const [switching, setSwitching] = useState(false)
  const [mcpToolCount, setMcpToolCount] = useState(0)
  const [ideInstallInfo, setIdeInstallInfo] = useState<any>(null)

  const handleRefresh = useCallback(() => refresh(), [refresh])

  // 检测 IDE 安装状态
  useEffect(() => {
    const checkIdeInstallation = async () => {
      try {
        const info = await checkIdeInstallationApi<any>()
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
        const statsResult = await getMcpToolStats(null)
        setMcpToolCount(statsResult.estimatedTools)
      } catch (e) {
        // 静默处理
      }
    }
    loadMcpToolCount()
  }, [])

  // 监听自动切号事件，弹气泡提醒
  useEffect(() => {
    let unlisten: (() => void) | null = null
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{ email: string }>('account-switched', (event) => {
        showSuccess(`自动切号：已切换到 ${event.payload?.email || '新账号'}`)
        refresh()
      }).then(fn => { unlisten = fn })
    })
    return () => { unlisten?.() }
  }, [refresh])

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

  // 一键换号
  const handleQuickSwitch = useCallback(async () => {
    if (switching) return
    setSwitching(true)
    try {
      const email = await quickSwitchNext()
      showSuccess(`已切换到 ${email}`)
      refresh()
    } catch (e: any) {
      showError('切换失败', String(e))
    } finally {
      setSwitching(false)
    }
  }, [switching, refresh, showError])

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
        const info = await checkCliInstallation()
        // 只根据可执行文件是否存在判断 CLI 是否安装
        setCliInstalled(info?.cli_installed || false)

        const path = await getKiroCliDefaultPath()
        if (path) {
          setCliPath(path)
          try {
            const snapshot = await readCliDbSnapshot(path)
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
      label: 'MCP 工具', 
      delay: 'delay-500',
      onClick: () => onNavigate?.('agentConfig'),
      warning: mcpToolCount > 50
    },
  ], [accent, stats, mcpToolCount, t, onNavigate])

  if (loading) {
    return <LoadingSkeleton />
  }

  return (
    <div className="h-full overflow-auto surface-main p-6">
      <div className="w-full">
        {/* Header（紧凑）*/}
        <div className="mb-4 flex items-center gap-2.5 animate-slide-in-left">
          <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ring-1 ring-primary/20`}>
            <Sparkles size={20} className="text-white" />
          </div>
          <div className="flex flex-col">
            <h1 className="text-lg font-semibold text-foreground leading-tight">{t('home.title')}</h1>
            <p className="text-sm text-muted-foreground leading-tight">{t('home.subtitle')}</p>
          </div>
        </div>

        {/* 统计卡片 */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 mb-3">
          {statCards.map((card, index) => (
            <StatCard key={index} {...card} />
          ))}
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2 mb-3 animate-scale-in delay-200">
          <Button
            onClick={handleQuickSwitch}
            disabled={switching || tokens.length < 2}
            className="flex-1 h-11 text-sm font-semibold gap-2"
            variant="default"
          >
            <Zap size={16} className={switching ? 'animate-spin' : ''} />
            {switching ? '切换中...' : '一键换号'}
          </Button>
          <Button
            onClick={() => onNavigate?.('accounts')}
            variant="outline"
            className="flex-1 h-11 text-sm font-medium gap-2"
          >
            <ArrowRightLeft size={14} />
            查看全部账号
          </Button>
        </div>

        {/* 主卡片：当前账号 | CLI 账号 */}
        <Card className="card-glow animate-scale-in delay-300">
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
            <div className="flex items-center gap-2">
              <Sparkles size={14} className={accent.text} />
              <span className="text-sm font-semibold text-foreground">Kiro 账号</span>
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
              {/* 左：当前 IDE 账号 */}
              <div className="p-4 flex flex-col gap-3">
                <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider">
                  当前 IDE 账号
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
                    {localToken ? '未匹配到账号' : (
                      ideInstallInfo?.ide_installed === false
                        ? (ideInstallInfo?.ide_executable_exists === false
                            ? 'Kiro IDE 未安装'
                            : 'Kiro IDE 已安装，未登录')
                        : t('home.notLoggedIn')
                    )}
                  </div>
                )}
              </div>

              {/* 右：CLI 账号 */}
              <div className="p-4 flex flex-col gap-3 bg-muted/20 border-t md:border-t-0 md:border-l border-border">
                <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider flex items-center gap-1.5">
                  <Terminal size={11} />
                  当前 CLI 账号
                </span>
                {cliLoading ? (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
                    加载中...
                  </div>
                ) : cliSnapshot ? (
                  <CliAccountDetail snapshot={cliSnapshot} cliPath={cliPath} />
                ) : cliInstalled ? (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-1.5 py-8">
                    <Terminal size={20} className="text-muted-foreground/50" />
                    <span>CLI 已安装，未登录</span>
                    <span className="text-[11px] text-muted-foreground/70">请运行 kiro-cli login 登录</span>
                  </div>
                ) : (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-1.5 py-8">
                    <Terminal size={20} className="text-muted-foreground/50" />
                    <span>CLI 未安装</span>
                    <span className="text-[11px] text-muted-foreground/70">请安装 Kiro CLI 后重启</span>
                  </div>
                )}
              </div>
            </div>

          </CardContent>
        </Card>
      </div>
    </div>
  )
}

// CLI 账号详情解析
function CliAccountDetail({ snapshot, cliPath }: { snapshot: any; cliPath: string }) {
  const entries = snapshot?.token_entries || []
  const deviceReg = snapshot?.device_registration

  // 找到主 token 条目
  const mainEntry = entries[0]
  const tokenData = mainEntry?.parsed_token

  if (!tokenData) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-2">
        <span>无有效 Token</span>
        <span className="text-[10px] font-mono truncate max-w-full">{cliPath}</span>
      </div>
    )
  }

  // 判断认证类型
  const isOidc = mainEntry.key?.includes('odic')
  const isSocial = mainEntry.key?.includes('social')
  const authMethod = isSocial ? 'Social' : isOidc ? 'IdC (BuilderId)' : 'Unknown'

  // Token 过期判断
  let expiresStr = '-'
  let isExpired = false
  if (tokenData.expires_at) {
    try {
      const expiresDate = new Date(tokenData.expires_at)
      if (!isNaN(expiresDate.getTime())) {
        expiresStr = expiresDate.toLocaleString('zh-CN', { 
          year: 'numeric',
          month: '2-digit', 
          day: '2-digit', 
          hour: '2-digit', 
          minute: '2-digit' 
        })
        isExpired = expiresDate.getTime() < Date.now()
      } else {
        expiresStr = 'Invalid Date'
      }
    } catch {
      expiresStr = 'Invalid Date'
    }
  }

  // 截断显示
  const truncate = (s: string, len = 16) => s ? (s.length > len ? s.substring(0, len) + '...' : s) : '-'

  return (
    <div className="flex-1 flex flex-col gap-3">
      {/* 状态 */}
      <div className="flex items-center gap-2 bg-muted/30 border border-border rounded-xl p-3">
        <div className="w-9 h-9 rounded-lg flex items-center justify-center bg-gradient-to-br from-emerald-500 to-teal-600 text-white font-bold text-sm shrink-0">
          C
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-semibold text-foreground">{authMethod}</span>
          <span className="text-[11px] text-muted-foreground font-mono truncate">{mainEntry.key}</span>
        </div>
        <Badge variant="default" className={`shrink-0 text-[10px] px-1.5 py-0 ${isExpired ? 'bg-red-500' : 'bg-green-500'}`}>
          {isExpired ? '已过期' : '有效'}
        </Badge>
      </div>

      {/* Token 信息 */}
      <div className="bg-muted/30 border border-border rounded-xl p-3">
        <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">Token</span>
        <div className="flex flex-col gap-1.5">
          <InfoRow label="Access Token" value={truncate(tokenData.access_token, 20)} mono />
          <InfoRow label="Refresh Token" value={truncate(tokenData.refresh_token, 20)} mono />
          <InfoRow label="过期时间" value={expiresStr} valueClass={isExpired ? 'text-red-500' : 'text-green-500'} />
          <InfoRow label="Region" value={tokenData.region || 'us-east-1'} mono />
          {tokenData.start_url && (
            <InfoRow label="Start URL" value={truncate(tokenData.start_url, 24)} mono />
          )}
          {tokenData.oauth_flow && (
            <InfoRow label="OAuth Flow" value={tokenData.oauth_flow} />
          )}
          {tokenData.scopes && tokenData.scopes.length > 0 && (
            <InfoRow label="Scopes" value={`${tokenData.scopes.length} 个`} />
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

      {/* 数据库路径 */}
      <div className="bg-muted/30 border border-border rounded-xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">DB 路径</span>
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

  // 超额相关字段
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

  // 重置时间
  let resetStr = ''
  let daysUntilReset: number | null = null
  let resetDateStr = ''
  if (nextReset) {
    const resetDate = new Date(typeof nextReset === 'string' ? nextReset : (nextReset < 1e12 ? nextReset * 1000 : nextReset))
    if (!isNaN(resetDate.getTime())) {
      resetDateStr = resetDate.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })
    } else {
      resetDateStr = '-'
    }
    const now = new Date()
    daysUntilReset = Math.max(0, Math.ceil((resetDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)))
    resetStr = daysUntilReset === 0 ? '今日重置' : `${daysUntilReset}天后重置`
  }

  // 超额使用百分比（相对于超额上限）
  const overagePercent = overageCap > 0 ? Math.round((currentOverages / overageCap) * 100) : 0

  return (
    <div className="flex-1 flex flex-col gap-3">
      {/* 头部：邮箱 + 计划 + Provider */}
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

      {/* 总配额进度 */}
      <div className="bg-muted/30 border border-border rounded-xl p-3">
        <div className="flex items-center justify-between mb-1.5">
          <span className="text-xs font-medium text-foreground">
            本月用量 ({displayNamePlural})
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
            <span className="text-[11px] font-semibold text-purple-500">超额 {currentOveragesPrecision}</span>
          ) : (
            <span className={`text-[11px] font-semibold ${getPercentClass(percent)}`}>剩余 {remaining}</span>
          )}
        </div>
      </div>

      {/* 超额详情（仅超额时显示） */}
      {currentOverages > 0 && (
        <div className="bg-purple-500/5 border border-purple-500/20 rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-purple-500 tracking-wider mb-2 block">超额详情</span>
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">超额用量</span>
              <span className="text-[11px] font-mono text-purple-500 font-semibold">{currentOveragesPrecision} / {overageCapPrecision}</span>
            </div>
            {/* 超额进度条 */}
            <div className="h-[3px] bg-purple-500/10 rounded-full overflow-hidden">
              <div className="h-full rounded-full bg-purple-500 transition-all" style={{ width: `${Math.min(overagePercent, 100)}%` }} />
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">超额费用</span>
              <span className="text-[11px] font-mono text-purple-500 font-semibold">${overageCharges.toFixed(2)} {currency}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">费率</span>
              <span className="text-[11px] font-mono text-muted-foreground">${overageRate}/{displayName}</span>
            </div>
          </div>
        </div>
      )}

      {/* 额度明细（基础 + 试用 + 奖励） */}
      {breakdown && (
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">额度明细</span>
          <div className="flex flex-col gap-2">
            {/* 基础配额 */}
            <QuotaRow label="基础" used={mainUsed} limit={mainLimit} percent={mainPercent} color="blue" accent={accent} />

            {/* 试用配额 */}
            {freeTrial && freeTrial.freeTrialStatus === 'ACTIVE' && freeTrial.usageLimit > 0 && (
              <QuotaRow
                label="试用"
                used={freeTrial.currentUsage ?? 0}
                limit={freeTrial.usageLimit}
                percent={freeTrial.usageLimit > 0 ? Math.round((freeTrial.currentUsage ?? 0) / freeTrial.usageLimit * 100) : 0}
                color="purple"
                accent={accent}
              />
            )}

            {/* 奖励配额 */}
            {bonuses.filter((b: any) => {
              const now = Date.now()
              const expiry = b.expiresAt ? b.expiresAt * 1000 : Infinity
              return expiry > now && b.status === 'ACTIVE'
            }).map((bonus: any, idx: number) => (
              <QuotaRow
                key={idx}
                label={bonus.displayName?.substring(0, 4) || `奖励${idx + 1}`}
                used={Math.round(bonus.currentUsage ?? 0)}
                limit={Math.round(bonus.usageLimit ?? 0)}
                percent={bonus.usageLimit > 0 ? Math.round((bonus.currentUsage ?? 0) / bonus.usageLimit * 100) : 0}
                color="amber"
                accent={accent}
                expiry={bonus.expiresAt}
              />
            ))}
          </div>
        </div>
      )}

      {/* 订阅 & 账号信息 两列 */}
      <div className="grid grid-cols-2 gap-2">
        {/* 订阅信息 */}
        {subInfo && (
          <div className="bg-muted/30 border border-border rounded-xl p-3">
            <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">订阅</span>
            <div className="flex flex-col gap-1.5">
              <InfoRow label="类型" value={subInfo.subscriptionTitle || 'Free'} />
              <InfoRow label="计划" value={subInfo.type?.replace('Q_DEVELOPER_STANDALONE_', '') || '-'} mono />
              <InfoRow label="超额能力" value={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? '✓ 支持' : '✗'} valueClass={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? 'text-green-500' : ''} />
              <InfoRow label="升级能力" value={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? '✓ 可升级' : '✗'} valueClass={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? 'text-green-500' : ''} />
              {overageConfig && (
                <InfoRow label="超额开关" value={overageConfig.overageStatus === 'ENABLED' ? '⚡ 已开启' : '已关闭'} valueClass={overageConfig.overageStatus === 'ENABLED' ? 'text-purple-500 font-semibold' : ''} />
              )}
              {subInfo.subscriptionManagementTarget && (
                <InfoRow label="管理" value={subInfo.subscriptionManagementTarget} mono />
              )}
            </div>
          </div>
        )}

        {/* 账号 & 资源信息 */}
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">账号 & 资源</span>
          <div className="flex flex-col gap-1.5">
            <InfoRow label="IDP" value={getProviderDisplayName(provider) || '-'} />
            <InfoRow label="重置日" value={resetDateStr || '-'} />
            {userInfo?.userId && (
              <InfoRow label="用户ID" value={userInfo.userId.split('.').pop()?.substring(0, 12) || '-'} mono />
            )}
            {resourceType && (
              <InfoRow label="资源类型" value={resourceType} mono />
            )}
            {currency && (
              <InfoRow label="货币" value={currency} />
            )}
            {unit && (
              <InfoRow label="计量单位" value={unit === 'INVOCATIONS' ? '调用次数' : unit} />
            )}
            {overageCap > 0 && (
              <InfoRow label="超额上限" value={`${overageCapPrecision}`} />
            )}
            {overageRate > 0 && (
              <InfoRow label="超额费率" value={`$${overageRate}/${displayName}`} mono />
            )}
          </div>
        </div>
      </div>

      {/* IDE Token 路径 */}
      <div className="bg-muted/30 border border-border rounded-xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Token 路径</span>
          <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[220px]" title="~/.aws/sso/cache/">
            .aws/sso/cache/
          </span>
        </div>
      </div>
    </div>
  )
}

// 配额行
function QuotaRow({ label, used, limit, percent, color, accent, expiry }: {
  label: string;
  used: number;
  limit: number;
  percent: number;
  color: 'blue' | 'purple' | 'amber';
  accent: any;
  expiry?: number;
}) {
  const colorMap = {
    blue: { dot: 'bg-blue-500', bar: 'bg-blue-500', text: 'text-blue-600' },
    purple: { dot: 'bg-purple-500', bar: 'bg-purple-500', text: 'text-purple-600' },
    amber: { dot: 'bg-amber-500', bar: 'bg-amber-500', text: 'text-amber-600' },
  }
  const c = colorMap[color]
  const expiryStr = expiry ? (() => {
    try {
      const date = new Date(expiry * 1000)
      return !isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' }) : null
    } catch {
      return null
    }
  })() : null

  return (
    <div className="flex items-center gap-2">
      <div className={`w-1.5 h-1.5 rounded-full ${c.dot} shrink-0`} />
      <span className="text-[11px] text-muted-foreground w-10 shrink-0" title={expiryStr ? `${expiryStr} 到期` : ''}>{label}</span>
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
