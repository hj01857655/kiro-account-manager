import { PieChart, BarChart2 } from 'lucide-react'
import { useMemo } from 'react'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { formatUsage, getAccountDisplayName, getQuota, getUsed } from '../../../utils/accountStats'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../AgentConfig/themeAccent'

import { Card, CardHeader, CardContent } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
// 使用率分布统计
function UsageDistribution({ tokens, colors, t }) {
  const { maskEmail } = usePrivacy()
  const { theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  
  
  // 计算使用率（使用统一的 getQuota 和 getUsed 函数）
  const getUsagePercent = (account) => {
    const quota = getQuota(account)
    const used = getUsed(account)
    return quota > 0 ? (used / quota) * 100 : 0
  }
  
  const usageGroups = {
    low: tokens.filter(a => getUsagePercent(a) < 30).length,
    medium: tokens.filter(a => { const p = getUsagePercent(a); return p >= 30 && p < 70 }).length,
    high: tokens.filter(a => getUsagePercent(a) >= 70).length
  }
  
  // 账号配额排行（前5，使用统一的 getQuota 和 getUsed 函数）
  const topAccounts = [...tokens]
    .map(a => {
      const used = getUsed(a)
      const limit = getQuota(a)
      return { 
        email: a.email, 
        used, 
        limit, 
        percent: limit > 0 ? Math.round((used / limit) * 100) : 0,
        usedStr: formatUsage(used),
        limitStr: formatUsage(limit)
      }
    })
    .sort((a, b) => b.limit - a.limit)
    .slice(0, 5)

  return (
    <div className="grid grid-cols-2 gap-6 mt-6">
      {/* 使用率分布 */}
      <Card className="card-glow animate-scale-in">
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <PieChart size={18} className={accent.text} />
            <h3 className={`text-sm font-semibold text-foreground`}>{t('stats.usageDistribution')}</h3>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-4">
            {[
              { label: t('stats.lowUsage'), value: usageGroups.low, color: 'bg-green-500', desc: '< 30%' },
              { label: t('stats.mediumUsage'), value: usageGroups.medium, color: 'bg-yellow-500', desc: '30-70%' },
              { label: t('stats.highUsage'), value: usageGroups.high, color: 'bg-red-500', desc: '> 70%' }
            ].map((item, i) => {
              const percent = tokens.length > 0 ? (item.value / tokens.length * 100) : 0
              return (
                <div key={i}>
                  <div className="flex items-center justify-between mb-1">
                    <span className={`text-sm text-foreground`}>
                      {item.label} <span className={"text-muted-foreground"}>({item.desc})</span>
                    </span>
                    <span className={`text-sm font-medium text-foreground`}>
                      {item.value} {t('stats.accounts')}
                    </span>
                  </div>
                  <Progress value={percent} className="h-2" />
                </div>
              )
            })}
          </div>
        </CardContent>
      </Card>

      {/* 账号配额排行 */}
      <Card className="card-glow animate-scale-in">
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <BarChart2 size={18} className={accent.text} />
            <h3 className={`text-sm font-semibold text-foreground`}>{t('stats.accountUsage')}</h3>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-3">
            {topAccounts.map((account, i) => (
              <div key={i}>
                <div className="flex items-center justify-between mb-1">
                  <span className={`text-xs text-foreground truncate`} style={{ maxWidth: 140 }}>
                    {account.email ? maskEmail(account.email).split('@')[0] : getAccountDisplayName(account)}
                  </span>
                  <span className={`text-xs text-muted-foreground`}>
                    {account.usedStr}/{account.limitStr} ({account.percent}%)
                  </span>
                </div>
                <Progress value={account.percent} className="h-1" />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

export default UsageDistribution
