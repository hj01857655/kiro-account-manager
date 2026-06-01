import { RefreshCw } from 'lucide-react'
import { getAccountDisplayName } from '../../../utils/accountStats'

import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import { Card, CardHeader, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Progress } from '@/components/ui/progress'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { Stack, Group, Text } from '@/components/shared/layout'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { useMemo } from 'react'

// 当前账号配额详情
function AccountQuotaDetail({
  currentAccount,
  currentQuotaInfo,
  refreshingAccount,
  handleRefreshCurrentAccount,
  maskEmail,
  theme,
  colors,
  t
}) {
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  
  const usageData = currentAccount.usageData
  const breakdown = usageData?.usageBreakdownList?.[0] || usageData?.usageBreakdown
  const subInfo = usageData?.subscriptionInfo
  const userInfo = usageData?.userInfo
  const overageConfig = usageData?.overageConfiguration
  const freeTrial = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []
  const mainUsed = breakdown?.currentUsage ?? 0
  const mainLimit = breakdown?.usageLimit ?? 0
  const mainPercent = mainLimit > 0 ? Math.round((mainUsed / mainLimit) * 100) : 0
  const nextDateReset = usageData?.nextDateReset
  const isTrial = subInfo?.subscriptionTitle?.toLowerCase()?.includes('trial') || 
                  subInfo?.subscriptionTitle?.toLowerCase()?.includes('free')
  
  // 计算剩余天数
  let daysUntilReset = null
  let resetTimestamp = null
  
  if (isTrial && freeTrial?.freeTrialExpiry) {
    resetTimestamp = freeTrial.freeTrialExpiry
  } else if (nextDateReset) {
    resetTimestamp = nextDateReset
  }
  
  if (resetTimestamp) {
    const resetDate = new Date(resetTimestamp * 1000)
    const now = new Date()
    const diffTime = resetDate.getTime() - now.getTime()
    daysUntilReset = Math.max(0, Math.ceil(diffTime / (1000 * 60 * 60 * 24)))
  }

  const { quota: currentQuota, used: currentUsed, percent: currentPercent } = currentQuotaInfo

  return (
    <Card className="card-glow animate-scale-in delay-500">
      {/* 头部 */}
        <AccountHeader
          currentAccount={currentAccount}
          userInfo={userInfo}
          subInfo={subInfo}
          daysUntilReset={daysUntilReset}
          refreshingAccount={refreshingAccount}
          handleRefreshCurrentAccount={handleRefreshCurrentAccount}
          maskEmail={maskEmail}
          accent={accent}
          colors={colors}
          t={t}
        />

      <CardContent className="p-6 flex flex-col gap-6">
        {/* 本月用量进度 */}
        <MonthlyUsageProgress
          currentPercent={currentPercent}
          currentUsed={currentUsed}
          currentQuota={currentQuota}
          accent={accent}
          colors={colors}
          t={t}
        />

        {/* 两列布局 */}
        <div className="grid grid-cols-2 gap-6">
          {subInfo && (
            <SubscriptionDetails
              subInfo={subInfo}
              overageConfig={overageConfig}
              colors={colors}
              t={t}
            />
          )}
        <AccountInfo
          currentAccount={currentAccount}
          userInfo={userInfo}
          breakdown={breakdown}
          nextDateReset={nextDateReset}
          accent={accent}
          colors={colors}
          t={t}
        />
        </div>

        {/* 额度明细 */}
        <QuotaBreakdown
          mainUsed={mainUsed}
          mainLimit={mainLimit}
          mainPercent={mainPercent}
          freeTrial={freeTrial}
          bonuses={bonuses}
          accent={accent}
          colors={colors}
          t={t}
        />
      </CardContent>
    </Card>
  )
}

// 账号头部
function AccountHeader({ currentAccount, userInfo, subInfo, daysUntilReset, refreshingAccount, handleRefreshCurrentAccount, maskEmail, colors, t, accent }) {
  return (
    <div className={`flex items-center justify-between p-6 border-b border-border`}>
      <div className="flex items-center gap-3 flex-1 min-w-0">
        <div className={`w-12 h-12 rounded-xl flex items-center justify-center text-white font-bold text-lg shadow-md flex-shrink-0 ${
          currentAccount.provider === 'Google' ? 'bg-gradient-to-br from-red-500 to-orange-500' :
          isGitHubProvider(currentAccount.provider) ? 'bg-gradient-to-br from-gray-700 to-gray-900' :
          `bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo}`
        }`}>
          {currentAccount.provider?.[0] || 'K'}
        </div>
        <div className="flex flex-col gap-1 flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className={`font-semibold text-foreground truncate`}>
              {userInfo?.email ? maskEmail(userInfo.email) : (currentAccount.email ? maskEmail(currentAccount.email) : getAccountDisplayName(currentAccount))}
            </span>
            {subInfo?.type && (
              <Badge
                variant="default"
                className="shrink-0"
                style={{
                  background: subInfo.type.includes('PRO+') ? 'linear-gradient(to right, rgb(168, 85, 247), rgb(236, 72, 153))' :
                             subInfo.type.includes('PRO') ? 'rgb(59, 130, 246)' :
                             undefined
                }}
              >
                {subInfo.subscriptionTitle || 'Free'}
              </Badge>
            )}
          </div>
          <span className={`text-xs text-muted-foreground`}>
            {getProviderDisplayName(currentAccount.provider)}
            {daysUntilReset != null && ` · ${daysUntilReset === 0 ? t('home.resetToday') : `${daysUntilReset} ${t('home.daysUntilReset')}`}`}
          </span>
        </div>
      </div>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            onClick={handleRefreshCurrentAccount}
            disabled={refreshingAccount}
            variant="ghost"
            size="icon"
            className={refreshingAccount ? 'spinning' : ''}
          >
            <RefreshCw size={16} className={"text-muted-foreground"} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{t('home.refreshAccount')}</TooltipContent>
      </Tooltip>
    </div>
  )
}

// 本月用量进度
function MonthlyUsageProgress({ currentPercent, currentUsed, currentQuota, accent, colors, t }) {
  const getProgressColor = () => {
    if (currentPercent > 80) return 'red'
    if (currentPercent > 50) return 'yellow'
    return 'blue'
  }

  const getPercentColorClass = () => {
    if (currentPercent > 80) return colors.quotaHigh
    if (currentPercent > 50) return colors.quotaMedium
    return accent.text
  }

  return (
    <Card
      className={"bg-muted/30 p-6 rounded-xl"}
    >
      <Group justify="space-between" className="mb-xs">
        <Text size="sm" fw={500} className={"text-foreground"}>
          {t('home.monthlyUsage')}
        </Text>
        <Group gap="xs">
          <Text 
            size="lg" 
            fw={700}
            className={getPercentColorClass()}
          >
            {currentPercent}%
          </Text>
          <Text size="xs" className={"text-muted-foreground"}>
            {currentUsed} / {currentQuota}
          </Text>
        </Group>
      </Group>
      <Progress
        value={currentPercent}
        className="h-2 rounded-full"
      />
    </Card>
  )
}

// 订阅详情
function SubscriptionDetails({ subInfo, overageConfig, colors, t }) {
  return (
    <Card
      className={"bg-muted/30 p-6 rounded-xl"}
    >
      <Text
        size="10px"
        fw={500}
        tt="uppercase"
        className="mb-xs text-primary"
      >
        {t('home.subscriptionDetails')}
      </Text>
      <Stack gap={6}>
        <Group justify="space-between">
          <Text size="xs" className={"text-muted-foreground"}>{t('home.type')}</Text>
          <Text size="xs" className={"text-foreground"}>{subInfo.subscriptionTitle || '-'}</Text>
        </Group>
        <Group justify="space-between">
          <Text size="xs" className={"text-muted-foreground"}>{t('home.overage')}</Text>
          <Text 
            size="xs" 
            className={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? colors.iconSuccess : "text-muted-foreground"}
          >
            {subInfo.overageCapability === 'OVERAGE_CAPABLE' ? '✓' : '✗'}
          </Text>
        </Group>
        <Group justify="space-between">
          <Text size="xs" className={"text-muted-foreground"}>{t('home.upgrade')}</Text>
          <Text 
            size="xs"
            className={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? colors.iconSuccess : "text-muted-foreground"}
          >
            {subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? '✓' : '✗'}
          </Text>
        </Group>
        {overageConfig && (
          <Group justify="space-between">
            <Text size="xs" className={"text-muted-foreground"}>{t('home.status')}</Text>
            <Text 
              size="xs"
              className={overageConfig.overageStatus === 'ENABLED' ? colors.iconSuccess : "text-muted-foreground"}
            >
              {overageConfig.overageStatus === 'ENABLED' ? t('home.enabled') : t('home.disabled')}
            </Text>
          </Group>
        )}
      </Stack>
    </Card>
  )
}

// 账户信息
function AccountInfo({ currentAccount, userInfo, breakdown, nextDateReset, accent, colors, t }) {
  return (
    <Card
      className={"bg-muted/30 p-6 rounded-xl"}
    >
      <Text
        size="10px"
        fw={500}
        tt="uppercase"
        className={`mb-xs ${accent.text}`}
      >
        {t('home.accountInfo')}
      </Text>
      <Stack gap={6}>
        <Group justify="space-between">
          <Text size="xs" className={"text-muted-foreground"}>IDP</Text>
          <Text size="xs" className={"text-foreground"}>{getProviderDisplayName(currentAccount.provider) || '-'}</Text>
        </Group>
        <Group justify="space-between">
          <Text size="xs" className={"text-muted-foreground"}>{t('home.reset')}</Text>
          <Text size="xs" className={"text-foreground"}>
            {nextDateReset ? new Date(nextDateReset * 1000).toLocaleDateString() : '-'}
          </Text>
        </Group>
        {breakdown?.overageRate && (
          <Group justify="space-between">
            <Text size="xs" className={"text-muted-foreground"}>{t('home.rate')}</Text>
            <Text size="xs" className={"text-foreground"}>${breakdown.overageRate}/{t('home.invocations')}</Text>
          </Group>
        )}
        <Group justify="space-between">
          <Text size="xs" className={"text-muted-foreground"}>ID</Text>
          <Tooltip>
            <TooltipTrigger>
              <Text size="xs" className={`text-foreground font-mono`} truncate style={{ maxWidth: 80 }}>
                {userInfo?.userId?.split('.').pop()?.substring(0, 8) || '-'}
              </Text>
            </TooltipTrigger>
            <TooltipContent>{userInfo?.userId}</TooltipContent>
          </Tooltip>
        </Group>
      </Stack>
    </Card>
  )
}

// 额度明细
function QuotaBreakdown({ mainUsed, mainLimit, mainPercent, freeTrial, bonuses, accent, colors, t }) {
  return (
    <Card
      className={"bg-muted/30 p-6 rounded-xl"}
    >
      <Text size="10px" fw={500} tt="uppercase" className="mb-xs text-foreground">
        {t('home.quotaDetails')}
      </Text>
      <Stack gap="xs">
        {/* 基础额度 */}
        <QuotaRow label={t('home.base')} used={mainUsed} limit={mainLimit} percent={mainPercent} color="blue" expiry={null} accent={accent} colors={colors} t={t} />

        {/* 试用额度 */}
        {freeTrial && freeTrial.usageLimit > 0 && (
          <QuotaRow
            label={t('home.trial')}
            used={freeTrial.currentUsage ?? 0}
            limit={freeTrial.usageLimit}
            percent={freeTrial.usageLimit > 0 ? ((freeTrial.currentUsage ?? 0) / freeTrial.usageLimit * 100) : 0}
            color="purple"
            expiry={freeTrial.freeTrialExpiry}
            accent={accent}
            colors={colors}
            t={t}
          />
        )}

        {/* 奖励额度 */}
        {bonuses.map((bonus, idx) => (
          <QuotaRow
            key={idx}
            label={bonus.displayName?.substring(0, 4) || `${t('home.bonus')}${idx + 1}`}
            used={Math.round(bonus.currentUsage ?? 0)} 
            limit={Math.round(bonus.usageLimit ?? 0)} 
            percent={bonus.usageLimit > 0 ? ((bonus.currentUsage ?? 0) / bonus.usageLimit * 100) : 0}
            color="amber" 
            expiry={bonus.expiresAt}
            accent={accent}
            colors={colors}
            t={t}
          />
        ))}
      </Stack>
    </Card>
  )
}

// 额度行
function QuotaRow({ label, used, limit, percent, color, expiry, accent, colors, t }) {
  const colorMap = {
    blue: { 
      dot: accent.solidBg, 
      bar: accent.solidBg, 
      text: "text-muted-foreground", 
      barBg: "bg-muted/30"
    },
    purple: { 
      dot: accent.solidBg, 
      bar: accent.solidBg, 
      text: accent.text, 
      barBg: "bg-purple-500/10 text-purple-500"
    },
    amber: { 
      dot: 'bg-amber-500', 
      bar: 'bg-amber-500', 
      text: 'text-amber-600', 
      barBg: "warning-badge"
    }
  }
  const c = colorMap[color] || colorMap.blue
  const expiryStr = expiry ? new Date(expiry * 1000).toLocaleDateString() : null

  return (
    <div className="flex items-center gap-2">
      <div className={`w-2 h-2 rounded-full ${c.dot} shrink-0`} />
      <span className={`text-xs ${c.text} w-14 shrink-0`} title={expiryStr ? `${expiryStr} ${t('home.expires')}` : ''}>{label}</span>
      <div className={`flex-1 h-1.5 ${c.barBg} rounded-full overflow-hidden`}>
        <div className={`h-full rounded-full ${c.bar} transition-all`} style={{ width: `${percent}%` }} />
      </div>
      <span className={`text-[10px] ${c.text} w-24 text-right shrink-0`}>
        {used}/{limit}{expiryStr ? ` · ${expiryStr}` : ''}
      </span>
    </div>
  )
}

export default AccountQuotaDetail
