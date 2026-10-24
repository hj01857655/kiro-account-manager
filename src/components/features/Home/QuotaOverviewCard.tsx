import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
import { Badge } from '@/components/ui/badge'
import { TrendingUp } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../AgentConfig/themeAccent'
import { useMemo } from 'react'


// 配额总览卡片
function QuotaOverviewCard({ stats, colors, t }) {
  const { theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  
  const getBadgeVariant = (percent) => {
    if (percent > 80) return 'destructive'
    if (percent > 50) return 'secondary'
    return 'default'
  }

  return (
    <Card className="card-glow animate-scale-in delay-400">
      <CardHeader>
        <div className="flex items-center gap-2">
          <div className={`w-10 h-10 rounded-xl flex items-center justify-center bg-muted/30`}>
            <TrendingUp size={20} className={accent.text} />
          </div>
          <span className={`font-semibold text-foreground`}>{t('home.quotaOverview')}</span>
        </div>
      </CardHeader>

      <CardContent className="space-y-3">
        <div className="flex items-center gap-3">
          <Progress
            value={stats.usagePercent}
            className="flex-1"
          />
          <Badge variant={getBadgeVariant(stats.usagePercent)}>
            {stats.usagePercent}%
          </Badge>
        </div>

        <div className="flex justify-between items-center">
          <span className={`text-sm text-muted-foreground`}>{t('home.usedTotal')}</span>
          <span className={`text-sm font-medium text-foreground`}>
            {stats.totalUsedStr} / {stats.totalQuotaStr}
          </span>
        </div>
      </CardContent>
    </Card>
  )
}

export default QuotaOverviewCard
