import { useState, useEffect, useCallback, useMemo } from 'react'
import { Gauge, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import { getGatewayRequestLogHistory } from '../../../api/gatewayApi'

interface ContextLogRow {
  contextUsagePercentage?: number | null
}

interface Bucket {
  label: string
  count: number
  /** 该桶内最大占比，用于排序与提示 */
  max: number
}

/** 上下文页：聚合 contextUsagePercentage 分布——窗口占用健康度一眼可见。 */
export function ContextPanel() {
  const { t } = useApp()
  const [rows, setRows] = useState<ContextLogRow[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const history = await getGatewayRequestLogHistory<ContextLogRow[]>(20000)
      setRows(history)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const stats = useMemo(() => {
    // 仅流式请求有上游 contextUsageEvent，非流式为 null，不计入分布
    const values = rows
      .map((r) => r.contextUsagePercentage)
      .filter((v): v is number => typeof v === 'number')
    const buckets: Bucket[] = [
      { label: t('gatewayContext.bucketLow'), count: 0, max: 0 },
      { label: t('gatewayContext.bucketMid'), count: 0, max: 0 },
      { label: t('gatewayContext.bucketHigh'), count: 0, max: 0 },
    ]
    let max = 0
    let sum = 0
    for (const v of values) {
      sum += v
      if (v > max) max = v
      const idx = v >= 80 ? 2 : v >= 60 ? 1 : 0
      buckets[idx].count += 1
      buckets[idx].max = Math.max(buckets[idx].max, v)
    }
    const total = values.length
    return {
      total,
      withoutData: rows.length - total,
      avg: total > 0 ? sum / total : 0,
      max,
      buckets,
      risky: buckets[2].count,
    }
  }, [rows, t])

  const barMax = Math.max(1, ...stats.buckets.map((b) => b.count))

  return (
    <div className="flex flex-col gap-4 p-4 h-full overflow-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Gauge className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">{t('gatewayContext.title')}</h3>
        </div>
        <Button variant="outline" size="sm" onClick={load} disabled={loading}>
          <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${loading ? 'animate-spin' : ''}`} />
          {t('common.refresh')}
        </Button>
      </div>

      {error && <div className="text-sm text-red-500">{t('gatewaySessions.loadFailed', { error })}</div>}

      <div className="grid grid-cols-3 gap-3">
        <div className="surface-card border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">{t('gatewayContext.avgUsage')}</div>
          <div className="text-2xl font-medium tabular-nums">{stats.avg.toFixed(1)}%</div>
        </div>
        <div className="surface-card border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">{t('gatewayContext.maxUsage')}</div>
          <div className="text-2xl font-medium tabular-nums">{stats.max.toFixed(1)}%</div>
        </div>
        <div className="surface-card border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">{t('gatewayContext.measured')}</div>
          <div className="text-2xl font-medium tabular-nums">
            {stats.total.toLocaleString()}
            {stats.withoutData > 0 && (
              <span className="text-xs text-muted-foreground font-normal ml-1">
                (+{stats.withoutData.toLocaleString()} {t('gatewayContext.unmeasured')})
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        {stats.buckets.map((b, i) => (
          <div key={b.label} className="flex items-center gap-3">
            <span className="w-28 text-xs text-muted-foreground shrink-0">{b.label}</span>
            <div className="flex-1 h-5 bg-muted/50 rounded overflow-hidden">
              <div
                className={`h-full ${i === 2 ? 'bg-red-500/70' : i === 1 ? 'bg-amber-500/70' : 'bg-green-500/60'}`}
                style={{ width: `${(b.count / barMax) * 100}%` }}
              />
            </div>
            <span className="w-16 text-right text-sm tabular-nums shrink-0">{b.count.toLocaleString()}</span>
          </div>
        ))}
      </div>

      <p className="text-xs text-muted-foreground">
        {t('gatewayContext.hint', { count: stats.risky })}
      </p>
    </div>
  )
}
