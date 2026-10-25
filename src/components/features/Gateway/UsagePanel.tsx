import { useState, useEffect, useCallback, useMemo } from 'react'
import { BarChart3, RefreshCw, Coins, Zap, Database, Cpu, Users, Globe } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { getGatewayRequestLogs } from '../../../api/gatewayApi'
import { shortClientLabel } from './gatewayPageUtils'

interface UsageLogRow {
  occurredAt: string
  endpoint: string
  clientUserAgent?: string | null
  model?: string | null
  statusCode: number
  stream?: boolean
  inputTokens?: number | null
  outputTokens?: number | null
  cacheReadInputTokens?: number | null
  cacheCreationInputTokens?: number | null
  credits?: number | null
  contextUsagePercentage?: number | null
}

interface Agg {
  name: string
  count: number
  errors: number
  input: number
  output: number
  cacheRead: number
  cacheWrite: number
  credits: number
}

function emptyAgg(name: string): Agg {
  return { name, count: 0, errors: 0, input: 0, output: 0, cacheRead: 0, cacheWrite: 0, credits: 0 }
}

/** 汇总一维（模型/客户端/端点共用）。keyOf 返回 null 的行跳过。 */
function aggregate(rows: UsageLogRow[], keyOf: (r: UsageLogRow) => string | null): Agg[] {
  const map = new Map<string, Agg>()
  for (const row of rows) {
    const key = keyOf(row)
    if (key == null) continue
    const agg = map.get(key) ?? emptyAgg(key)
    agg.count += 1
    if (row.statusCode >= 400) agg.errors += 1
    agg.input += row.inputTokens ?? 0
    agg.output += row.outputTokens ?? 0
    agg.cacheRead += row.cacheReadInputTokens ?? 0
    agg.cacheWrite += row.cacheCreationInputTokens ?? 0
    agg.credits += row.credits ?? 0
    map.set(key, agg)
  }
  return Array.from(map.values()).sort((a, b) => b.count - a.count)
}

/** 时间桶：occurredAt 形如 `2026-10-25 14:03:22`，取整到小时做趋势横轴。 */
function hourBucket(occurredAt: string): string {
  // 取「MM-DD HH」段，秒丢弃；非法输入原样返回（不会抛）
  const m = occurredAt.match(/(\d{2}-\d{2})\s+(\d{2}):/)
  return m ? `${m[1]} ${m[2]}:00` : occurredAt.slice(0, 10)
}

/** 图表统一色板（跟随 Tailwind 调色板，深浅背景都可读） */
const CHART_COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ef4444', '#06b6d4', '#ec4899', '#84cc16']

/** 自定义 Tooltip：与深/浅背景都协调的小卡片 */
function ChartTooltip({ active, payload, label, unit }: any) {
  if (!active || !payload?.length) return null
  return (
    <div className="rounded-lg border border-border bg-popover px-2.5 py-1.5 text-[11px] shadow-md">
      {label != null && <div className="mb-0.5 font-medium text-foreground">{label}</div>}
      {payload.map((p: any, i: number) => (
        <div key={i} className="flex items-center gap-1.5 text-muted-foreground">
          <span className="inline-block h-2 w-2 rounded-full" style={{ background: p.color || p.fill }} />
          <span>{p.name}</span>
          <strong className="font-mono text-foreground">
            {typeof p.value === 'number' ? p.value.toLocaleString() : p.value}
            {unit ?? ''}
          </strong>
        </div>
      ))}
    </div>
  )
}

/**
 * 用量页（Usage）：本次网关运行的流量账本。
 *
 * 与请求日志 tab 的分工：日志 tab 是逐条明细，这里是聚合仪表盘——
 * 趋势面积图（按小时的请求/token/计费）+ 缓存命中环形图 + credits 柱状 +
 * 三个维度条形图（模型/客户端/端点）。数据全部从请求日志的当前列表
 * 客户端聚合，不另开统计接口。
 *
 * 计费说明：credits 是上游 meteringEvent 的**真实**计费消耗，
 * 不是 tokens × 单价估算；非流式请求无此值。
 */
export function UsagePanel() {
  const { t } = useApp()
  const [rows, setRows] = useState<UsageLogRow[]>([])
  const [loading, setLoading] = useState(false)

  const fetch = useCallback(async () => {
    setLoading(true)
    try {
      setRows(await getGatewayRequestLogs<UsageLogRow[]>(500))
    } catch {
      setRows([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetch()
    const timer = setInterval(fetch, 30000)
    return () => clearInterval(timer)
  }, [fetch])

  const summary = useMemo(() => {
    let success = 0, errors = 0, streaming = 0
    let input = 0, output = 0, cacheRead = 0, cacheWrite = 0, credits = 0
    let cacheHitRequests = 0
    const ctxValues: number[] = []
    for (const r of rows) {
      if (r.statusCode < 400) success++
      else errors++
      if (r.stream) streaming++
      input += r.inputTokens ?? 0
      output += r.outputTokens ?? 0
      cacheRead += r.cacheReadInputTokens ?? 0
      cacheWrite += r.cacheCreationInputTokens ?? 0
      credits += r.credits ?? 0
      if ((r.cacheReadInputTokens ?? 0) > 0 || (r.cacheCreationInputTokens ?? 0) > 0) cacheHitRequests++
      if (r.contextUsagePercentage != null) ctxValues.push(r.contextUsagePercentage)
    }
    return {
      total: rows.length, success, errors, streaming,
      input, output, cacheRead, cacheWrite, credits, cacheHitRequests,
      ctxAvg: ctxValues.length ? ctxValues.reduce((a, b) => a + b, 0) / ctxValues.length : null,
      ctxMax: ctxValues.length ? Math.max(...ctxValues) : null,
    }
  }, [rows])

  // 按小时的趋势序列：请求数 / token 进出 / 计费三条线一张图太挤，
  // 拆成「请求数 + token」双轴面积图与「credits」柱状图
  const trend = useMemo(() => {
    const buckets = new Map<string, { hour: string; requests: number; errors: number; input: number; output: number; credits: number }>()
    for (const r of rows) {
      const hour = hourBucket(r.occurredAt)
      const b = buckets.get(hour) ?? { hour, requests: 0, errors: 0, input: 0, output: 0, credits: 0 }
      b.requests += 1
      if (r.statusCode >= 400) b.errors += 1
      b.input += r.inputTokens ?? 0
      b.output += r.outputTokens ?? 0
      b.credits += r.credits ?? 0
      buckets.set(hour, b)
    }
    return Array.from(buckets.values()).sort((a, b) => a.hour.localeCompare(b.hour))
  }, [rows])

  // 缓存命中环：命中（读+写>0）vs 未命中
  const cachePie = useMemo(() => {
    let hit = 0, miss = 0
    for (const r of rows) {
      if ((r.cacheReadInputTokens ?? 0) > 0 || (r.cacheCreationInputTokens ?? 0) > 0) hit++
      else miss++
    }
    return [
      { name: t('gatewayUsage.cacheHit'), value: hit, color: '#3b82f6' },
      { name: t('gatewayUsage.cacheMiss'), value: miss, color: '#94a3b8' },
    ]
  }, [rows, t])

  const byModel = useMemo(() => aggregate(rows, r => r.model ?? null), [rows])
  const byClient = useMemo(
    () => aggregate(rows, r => shortClientLabel(r.clientUserAgent ?? undefined) || null),
    [rows]
  )
  const byEndpoint = useMemo(() => aggregate(rows, r => r.endpoint ?? null), [rows])

  const hasData = rows.length > 0

  return (
    <div className="space-y-3">
      {/* 汇总条 */}
      <div className="flex items-center gap-2 flex-wrap">
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
          <BarChart3 size={12} className="text-primary" />
          {t('gatewayUsage.requests', { count: summary.total })}
          <span className="text-green-600">{t('gatewayUsage.ok', { count: summary.success })}</span>
          {summary.errors > 0 && <span className="text-red-500">{t('gatewayUsage.failed', { count: summary.errors })}</span>}
          {summary.streaming > 0 && <span>{t('gatewayUsage.streamed', { count: summary.streaming })}</span>}
        </div>
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
          <Zap size={12} className="text-amber-500" />
          {t('gatewayUsage.tokens', {
            input: (summary.input / 1000).toFixed(1),
            output: (summary.output / 1000).toFixed(1),
          })}
        </div>
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
          <Database size={12} className="text-blue-500" />
          {t('gatewayUsage.cache', {
            read: (summary.cacheRead / 1000).toFixed(1),
            write: (summary.cacheWrite / 1000).toFixed(1),
            hit: summary.cacheHitRequests,
          })}
        </div>
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground" title={t('gatewayUsage.creditsTitle')}>
          <Coins size={12} className="text-emerald-500" />
          {t('gatewayUsage.credits')} <strong className="font-mono text-foreground">{summary.credits.toFixed(4)}</strong>
        </div>
        {summary.ctxAvg != null && (
          <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
            {t('gatewayUsage.contextAvg')} <strong className="text-foreground">{summary.ctxAvg.toFixed(1)}%</strong>
            {summary.ctxMax != null && (
              <span> / {t('gatewayUsage.contextMax')} <strong>{summary.ctxMax.toFixed(1)}%</strong></span>
            )}
          </div>
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={fetch}
          disabled={loading}
          className="ml-auto h-6 gap-1 px-1.5 text-[10px] font-normal text-muted-foreground"
        >
          <RefreshCw size={10} className={loading ? 'animate-spin' : ''} />
          {t('common.refresh')}
        </Button>
      </div>

      {!hasData ? (
        <p className="py-8 text-center text-[11px] text-muted-foreground">{t('gatewayUsage.noData')}</p>
      ) : (
        <>
          {/* 趋势：请求数 + token 双轴面积图 */}
          <div className="rounded-lg border p-2.5">
            <div className="mb-1.5 flex items-center gap-2">
              <BarChart3 size={12} className="text-primary" />
              <span className="text-[11px] font-medium">{t('gatewayUsage.trendTitle')}</span>
              <span className="text-[10px] text-muted-foreground">{t('gatewayUsage.trendHint')}</span>
            </div>
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={trend} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
                <defs>
                  <linearGradient id="usageRequests" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#3b82f6" stopOpacity={0.35} />
                    <stop offset="100%" stopColor="#3b82f6" stopOpacity={0.02} />
                  </linearGradient>
                  <linearGradient id="usageTokens" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#10b981" stopOpacity={0.3} />
                    <stop offset="100%" stopColor="#10b981" stopOpacity={0.02} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
                <XAxis
                  dataKey="hour"
                  tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                  tickLine={false}
                  axisLine={false}
                  minTickGap={24}
                />
                <YAxis
                  yAxisId="left"
                  tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                  tickLine={false}
                  axisLine={false}
                  width={36}
                />
                <YAxis
                  yAxisId="right"
                  orientation="right"
                  tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                  tickLine={false}
                  axisLine={false}
                  width={44}
                  tickFormatter={(v: number) => v >= 1000 ? `${(v / 1000).toFixed(0)}K` : String(v)}
                />
                <Tooltip content={<ChartTooltip />} />
                <Legend wrapperStyle={{ fontSize: 10 }} />
                <Area
                  yAxisId="left"
                  type="monotone"
                  dataKey="requests"
                  name={t('gatewayUsage.seriesRequests')}
                  stroke="#3b82f6"
                  fill="url(#usageRequests)"
                  strokeWidth={1.5}
                  dot={false}
                />
                <Area
                  yAxisId="left"
                  type="monotone"
                  dataKey="errors"
                  name={t('gatewayUsage.seriesErrors')}
                  stroke="#ef4444"
                  fill="transparent"
                  strokeWidth={1.5}
                  dot={false}
                />
                <Area
                  yAxisId="right"
                  type="monotone"
                  dataKey="input"
                  name={t('gatewayUsage.seriesInputTokens')}
                  stroke="#10b981"
                  fill="url(#usageTokens)"
                  strokeWidth={1.5}
                  dot={false}
                />
                <Area
                  yAxisId="right"
                  type="monotone"
                  dataKey="output"
                  name={t('gatewayUsage.seriesOutputTokens')}
                  stroke="#f59e0b"
                  fill="transparent"
                  strokeWidth={1.5}
                  dot={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* 缓存命中环 + credits 柱状 */}
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
            <div className="rounded-lg border p-2.5">
              <div className="mb-1.5 flex items-center gap-2">
                <Database size={12} className="text-blue-500" />
                <span className="text-[11px] font-medium">{t('gatewayUsage.cachePieTitle')}</span>
              </div>
              <ResponsiveContainer width="100%" height={160}>
                <PieChart>
                  <Pie
                    data={cachePie}
                    dataKey="value"
                    nameKey="name"
                    innerRadius={38}
                    outerRadius={58}
                    paddingAngle={2}
                    strokeWidth={0}
                  >
                    {cachePie.map((entry, i) => (
                      <Cell key={i} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip content={<ChartTooltip />} />
                  <Legend wrapperStyle={{ fontSize: 10 }} />
                </PieChart>
              </ResponsiveContainer>
            </div>

            <div className="rounded-lg border p-2.5">
              <div className="mb-1.5 flex items-center gap-2">
                <Coins size={12} className="text-emerald-500" />
                <span className="text-[11px] font-medium">{t('gatewayUsage.creditsTrendTitle')}</span>
              </div>
              <ResponsiveContainer width="100%" height={160}>
                <BarChart data={trend} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
                  <XAxis
                    dataKey="hour"
                    tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                    tickLine={false}
                    axisLine={false}
                    minTickGap={24}
                  />
                  <YAxis
                    tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                    tickLine={false}
                    axisLine={false}
                    width={44}
                    tickFormatter={(v: number) => v.toFixed(2)}
                  />
                  <Tooltip content={<ChartTooltip />} />
                  <Bar
                    dataKey="credits"
                    name={t('gatewayUsage.colCredits')}
                    fill="#10b981"
                    radius={[3, 3, 0, 0]}
                    maxBarSize={22}
                  />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* 三维条形图 */}
          <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
            {[
              { rows: byModel, label: t('gatewayUsage.byModel'), icon: <Cpu size={12} className="text-violet-500" /> },
              { rows: byClient, label: t('gatewayUsage.byClient'), icon: <Users size={12} className="text-blue-500" /> },
              { rows: byEndpoint, label: t('gatewayUsage.byEndpoint'), icon: <Globe size={12} className="text-amber-500" /> },
            ].map(({ rows: aggRows, label, icon }) => (
              <div key={label} className="rounded-lg border p-2.5">
                <div className="mb-1.5 flex items-center gap-2">
                  {icon}
                  <span className="text-[11px] font-medium">{label}</span>
                </div>
                <ResponsiveContainer width="100%" height={Math.max(120, aggRows.length * 26)}>
                  <BarChart
                    data={aggRows.slice(0, 8)}
                    layout="vertical"
                    margin={{ top: 0, right: 12, bottom: 0, left: 0 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" horizontal={false} />
                    <XAxis type="number" hide />
                    <YAxis
                      type="category"
                      dataKey="name"
                      tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                      tickLine={false}
                      axisLine={false}
                      width={110}
                      tickFormatter={(v: string) => v.length > 14 ? `${v.slice(0, 13)}…` : v}
                    />
                    <Tooltip content={<ChartTooltip />} />
                    <Bar dataKey="count" name={t('gatewayUsage.colCount')} radius={[0, 3, 3, 0]} maxBarSize={14}>
                      {aggRows.slice(0, 8).map((_, i) => (
                        <Cell key={i} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
