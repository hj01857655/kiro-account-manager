import { Card, Group, Stack, Text, Badge, Progress, ActionIcon, Tooltip } from '@mantine/core'
import { RefreshCw } from 'lucide-react'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'

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
  const accent = getThemeAccent(theme)
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
    <Card
      className="card-glow animate-scale-in delay-500"
      shadow="sm"
      padding={0}
      radius="xl"
      withBorder
    >
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
      
      <Stack p="lg" gap="md">
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
        <Group grow align="flex-start">
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
        </Group>

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
      </Stack>
    </Card>
  )
}

// 账号头部
function AccountHeader({ currentAccount, userInfo, subInfo, daysUntilReset, refreshingAccount, handleRefreshCurrentAccount, maskEmail, colors, t, accent }) {
  return (
    <Group 
      justify="space-between" 
      p="md" 
      className={`border-b ${colors.cardBorder}`}
      wrap="nowrap"
    >
      <Group gap="md" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
        <div className={`w-12 h-12 rounded-xl flex items-center justify-center text-white font-bold text-lg shadow-md flex-shrink-0 ${
          currentAccount.provider === 'Google' ? 'bg-gradient-to-br from-red-500 to-orange-500' :
          isGitHubProvider(currentAccount.provider) ? 'bg-gradient-to-br from-gray-700 to-gray-900' :
          `bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo}`
        }`}>
          {currentAccount.provider?.[0] || 'K'}
        </div>
        <Stack gap={4} style={{ flex: 1, minWidth: 0 }}>
          <Group gap="xs" wrap="nowrap">
            <Text fw={600} className={colors.text} truncate>
              {userInfo?.email ? maskEmail(userInfo.email) : (currentAccount.email ? maskEmail(currentAccount.email) : getAccountDisplayName(currentAccount))}
            </Text>
            {subInfo?.type && (
              <Badge
                size="xs"
                variant="filled"
                style={{
                  background: subInfo.type.includes('PRO+') ? 'linear-gradient(to right, rgb(168, 85, 247), rgb(236, 72, 153))' :
                             subInfo.type.includes('PRO') ? 'rgb(59, 130, 246)' :
                             undefined
                }}
                className={subInfo.type.includes('PRO') ? 'shrink-0' : `shrink-0 ${colors.badgeDisabled}`}
              >
                {subInfo.subscriptionTitle || 'Free'}
              </Badge>
            )}
          </Group>
          <Text size="xs" className={colors.textMuted}>
            {getProviderDisplayName(currentAccount.provider)}
            {daysUntilReset != null && ` · ${daysUntilReset === 0 ? t('home.resetToday') : `${daysUntilReset} ${t('home.daysUntilReset')}`}`}
          </Text>
        </Stack>
      </Group>
      <Tooltip label={t('home.refreshAccount')}>
        <ActionIcon
          onClick={handleRefreshCurrentAccount}
          disabled={refreshingAccount}
          variant="subtle"
          radius="xl"
          loading={refreshingAccount}
          className={refreshingAccount ? 'spinning' : ''}
        >
          <RefreshCw size={16} className={colors.textMuted} />
        </ActionIcon>
      </Tooltip>
    </Group>
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
      padding="md"
      radius="lg"
      className={colors.cardSecondary}
    >
      <Group justify="space-between" mb="xs">
        <Text size="sm" fw={500} className={colors.text}>
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
          <Text size="xs" className={colors.textMuted}>
            {currentUsed} / {currentQuota}
          </Text>
        </Group>
      </Group>
      <Progress
        value={currentPercent}
        color={getProgressColor()}
        size="md"
        radius="xl"
        animated
      />
    </Card>
  )
}

// 订阅详情
function SubscriptionDetails({ subInfo, overageConfig, colors, t }) {
  return (
    <Card
      padding="sm"
      radius="md"
      className={colors.cardSecondary}
    >
      <Text 
        size="10px" 
        fw={500} 
        tt="uppercase" 
        mb="xs"
        className={colors.primary}
      >
        {t('home.subscriptionDetails')}
      </Text>
      <Stack gap={6}>
        <Group justify="space-between">
          <Text size="xs" className={colors.textMuted}>{t('home.type')}</Text>
          <Text size="xs" className={colors.text}>{subInfo.subscriptionTitle || '-'}</Text>
        </Group>
        <Group justify="space-between">
          <Text size="xs" className={colors.textMuted}>{t('home.overage')}</Text>
          <Text 
            size="xs" 
            className={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? colors.iconSuccess : colors.textMuted}
          >
            {subInfo.overageCapability === 'OVERAGE_CAPABLE' ? '✓' : '✗'}
          </Text>
        </Group>
        <Group justify="space-between">
          <Text size="xs" className={colors.textMuted}>{t('home.upgrade')}</Text>
          <Text 
            size="xs"
            className={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? colors.iconSuccess : colors.textMuted}
          >
            {subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? '✓' : '✗'}
          </Text>
        </Group>
        {overageConfig && (
          <Group justify="space-between">
            <Text size="xs" className={colors.textMuted}>{t('home.status')}</Text>
            <Text 
              size="xs"
              className={overageConfig.overageStatus === 'ENABLED' ? colors.iconSuccess : colors.textMuted}
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
      padding="sm"
      radius="md"
      className={colors.cardSecondary}
    >
      <Text 
        size="10px" 
        fw={500} 
        tt="uppercase" 
        mb="xs"
        className={accent.text}
      >
        {t('home.accountInfo')}
      </Text>
      <Stack gap={6}>
        <Group justify="space-between">
          <Text size="xs" className={colors.textMuted}>IDP</Text>
          <Text size="xs" className={colors.text}>{getProviderDisplayName(currentAccount.provider) || '-'}</Text>
        </Group>
        <Group justify="space-between">
          <Text size="xs" className={colors.textMuted}>{t('home.reset')}</Text>
          <Text size="xs" className={colors.text}>
            {nextDateReset ? new Date(nextDateReset * 1000).toLocaleDateString() : '-'}
          </Text>
        </Group>
        {breakdown?.overageRate && (
          <Group justify="space-between">
            <Text size="xs" className={colors.textMuted}>{t('home.rate')}</Text>
            <Text size="xs" className={colors.text}>${breakdown.overageRate}/次</Text>
          </Group>
        )}
        <Group justify="space-between">
          <Text size="xs" className={colors.textMuted}>ID</Text>
          <Tooltip label={userInfo?.userId}>
            <Text size="xs" className={`${colors.text} font-mono`} truncate style={{ maxWidth: 80 }}>
              {userInfo?.userId?.split('.').pop()?.substring(0, 8) || '-'}
            </Text>
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
      padding="sm"
      radius="md"
      className={colors.cardSecondary}
    >
      <Text size="10px" fw={500} tt="uppercase" mb="xs" className={colors.text}>
        {t('home.quotaDetails')}
      </Text>
      <Stack gap="xs">
        {/* 基础额度 */}
        <QuotaRow label={t('home.base')} used={mainUsed} limit={mainLimit} percent={mainPercent} color="blue" accent={accent} colors={colors} />

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
            label={bonus.displayName?.substring(0, 4) || `奖励${idx+1}`} 
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
      text: colors.textMuted, 
      barBg: colors.cardSecondary
    },
    purple: { 
      dot: accent.solidBg, 
      bar: accent.solidBg, 
      text: accent.text, 
      barBg: colors.badgePurple
    },
    amber: { 
      dot: 'bg-amber-500', 
      bar: 'bg-amber-500', 
      text: 'text-amber-600', 
      barBg: colors.badgeWarning
    }
  }
  const c = colorMap[color] || colorMap.blue
  const expiryStr = expiry ? new Date(expiry * 1000).toLocaleDateString() : null

  return (
    <div className="flex items-center gap-2">
      <div className={`w-2 h-2 rounded-full ${c.dot} shrink-0`} />
      <span className={`text-xs ${c.text} w-14 shrink-0`} title={expiryStr ? `${expiryStr} ${t?.('home.expires') || '到期'}` : ''}>{label}</span>
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
