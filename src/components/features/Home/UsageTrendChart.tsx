// 使用量趋势图组件
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { useState, useEffect } from 'react'
import { getUsageHistory, saveUsageHistoryEntry } from '../../../api/usageApi'
import { Calendar } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../AgentConfig/themeAccent'


export default function UsageTrendChart({ accounts, stats }) {
  const { t, theme} = useApp()
  const accent = getThemeAccent(theme)

  const [usageHistory, setUsageHistory] = useState([])

  // 加载并保存历史记录
  useEffect(() => {
    const loadAndSaveHistory = async () => {
      try {
        // 加载历史记录
        const history = await getUsageHistory().catch(() => ({ entries: [] }))

        // 记录当天的使用量
        if (accounts.length > 0) {
          const now = new Date()
          const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
          await saveUsageHistoryEntry({
              date: today,
              totalQuota: Math.round(stats.totalQuota),
              totalUsed: Math.round(stats.totalUsed),
              accountCount: accounts.length
            }).catch(console.error)

          // 重新加载历史
          const updatedHistory = await getUsageHistory().catch(() => ({ entries: [] })) as { entries: any[] }
          setUsageHistory(updatedHistory.entries || [])
        } else {
          const history = await getUsageHistory().catch(() => ({ entries: [] })) as { entries: any[] }
          setUsageHistory(history.entries || [])
        }
      } catch (e) {
        console.error('Failed to load usage history:', e)
      }
    }

    loadAndSaveHistory()
  }, [accounts, stats.totalQuota, stats.totalUsed])

  if (usageHistory.length < 1) return null

  const maxUsed = Math.max(...usageHistory.map(h => h.totalUsed), 1)

  return (
    <Card className="card-glow animate-scale-in">
      <CardHeader>
        <div className="flex items-center gap-2">
          <Calendar size={18} className={accent.text} />
          <CardTitle className={"text-foreground"}>{t('stats.usageTrend')}</CardTitle>
        </div>
      </CardHeader>
      <CardContent>
        {/* SVG 折线图 */}
        <div className="relative h-40">
          <svg viewBox="0 0 400 140" className="w-full h-full" preserveAspectRatio="xMidYMid meet">
            {/* 网格线 */}
            {[0, 35, 70, 105, 140].map((y, i) => (
              <line
                key={i}
                x1="40" y1={y} x2="390" y2={y}
                stroke="currentColor"
                className={"text-muted-foreground"}
                strokeDasharray="2,2"
                opacity="0.2"
              />
            ))}

            {/* Y 轴标签 */}
            {[0, 0.25, 0.5, 0.75, 1].map((ratio, i) => (
              <text
                key={i}
                x="35" y={140 - ratio * 140 + 4}
                textAnchor="end"
                className={`text-[10px] fill-current text-muted-foreground`}
              >
                {Math.round(maxUsed * ratio)}
              </text>
            ))}

            {/* 使用量折线 */}
            {(() => {
              const points = usageHistory.map((h, i) => {
                const x = 50 + (340 / Math.max(usageHistory.length - 1, 1)) * i
                const y = 130 - (h.totalUsed / maxUsed) * 120
                return `${x},${y}`
              }).join(' ')

              // 填充区域
              const fillPoints = usageHistory.map((h, i) => {
                const x = 50 + (340 / Math.max(usageHistory.length - 1, 1)) * i
                const y = 130 - (h.totalUsed / maxUsed) * 120
                return `${x},${y}`
              })
              const lastX = 50 + (340 / Math.max(usageHistory.length - 1, 1)) * (usageHistory.length - 1)
              const fillPath = `M50,130 L${fillPoints.join(' L')} L${lastX},130 Z`

              return (
                <>
                  <defs>
                    <linearGradient id="usageGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="var(--app-primary-solid)" stopOpacity="0.3" />
                      <stop offset="100%" stopColor="var(--app-primary-solid)" stopOpacity="0" />
                    </linearGradient>
                  </defs>
                  <path d={fillPath} fill="url(#usageGradient)" />
                  <polyline
                    points={points}
                    fill="none"
                    stroke="var(--app-primary-solid)"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                  {/* 数据点 */}
                  {usageHistory.map((h, i) => {
                    const x = 50 + (340 / Math.max(usageHistory.length - 1, 1)) * i
                    const y = 130 - (h.totalUsed / maxUsed) * 120
                    return (
                      <circle
                        key={i}
                        cx={x} cy={y} r="3"
                        fill="var(--app-primary-solid)"
                        className="hover:r-5 transition-all cursor-pointer"
                      >
                        <title>{h.date}: {t('stats.totalUsed')} {h.totalUsed}</title>
                      </circle>
                    )
                  })}
                </>
              )
            })()}

            {/* X 轴日期标签 */}
            {usageHistory
              .filter((_, i) => i === 0 || i === usageHistory.length - 1 || i % Math.ceil(usageHistory.length / 5) === 0)
              .map((h) => {
                const originalIndex = usageHistory.indexOf(h)
                const x = 50 + (340 / Math.max(usageHistory.length - 1, 1)) * originalIndex
                return (
                  <text
                    key={h.date}
                    x={x} y="138"
                    textAnchor="middle"
                    className={`text-[9px] fill-current text-muted-foreground`}
                  >
                    {h.date.slice(5)}
                  </text>
                )
              })}
          </svg>
        </div>

        {/* 图例 */}
        <div className="flex justify-center gap-3 mt-2">
          <div className="flex items-center gap-1.5">
            <div className="w-3 h-3 rounded-full" style={{ backgroundColor: 'var(--app-primary-solid)' }} />
            <span className={`text-xs text-muted-foreground`}>{t('stats.totalUsed')}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
