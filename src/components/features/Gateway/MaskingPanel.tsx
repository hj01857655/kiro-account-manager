import { useState, useEffect, useCallback, useMemo } from 'react'
import {
  BarChart3,
  ShieldCheck,
  RefreshCw,
  Eye,
  EyeOff,
  Play,
  Trash2
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
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
import { toast } from 'sonner'
import { useApp } from '../../../hooks/useApp'
import { getMaskAuditEntries, previewMasking, type MaskAuditEntry } from '../../../api/gatewayApi'
import { shortClientLabel } from './gatewayPageUtils'

const KIND_COLORS: Record<string, string> = {
  api_key: '#3b82f6',
  aws_secret: '#3b82f6',
  private_key: '#8b5cf6',
  token: '#10b981',
  email: '#f59e0b',
  credit_card: '#ef4444',
  id_card: '#ec4899',
  custom: '#64748b',
}

/** 汇总卡 Tooltip：小卡片样式，与 Usage 页一致 */
function MaskTooltip({ active, payload, label }: any) {
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
          </strong>
        </div>
      ))}
    </div>
  )
}

/** 动作徽章：Mask 遮蔽 / Warn 警告 / Block 拒绝 */
function ActionBadge({ action }: { action: string }) {
  const cls =
    action === 'mask'
      ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
      : action === 'block'
        ? 'bg-red-500/10 text-red-600 dark:text-red-400'
        : 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
  return (
    <span className={`rounded px-1.5 py-0.5 text-[10px] font-medium ${cls}`}>
      {action}
    </span>
  )
}

/**
 * 遮蔽审计页：隐私遮蔽的账本。
 *
 * 三块内容：
 * 1. 汇总卡——拦截了多少值、按类型分布、按动作分布
 * 2. 记录列表——每条的命中条目（类型/动作/占位符/长度/首字符）+ 并排对照
 *    （原文 ↔ vendor 实际收到的占位符版）
 * 3. Try-it——试一段文本看会变成什么（不落存储）
 *
 * 记录与真值分离：后端只存元数据，原文从本地请求日志取（条目带 requestId）。
 */
export function MaskingPanel() {
  const { t } = useApp()
  const [entries, setEntries] = useState<MaskAuditEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [tryText, setTryText] = useState('')
  const [tryResult, setTryResult] = useState<{ text: string; items: unknown[]; blocked: boolean } | null>(null)
  const [trying, setTrying] = useState(false)

  const fetch = useCallback(async () => {
    setLoading(true)
    try {
      setEntries(await getMaskAuditEntries<MaskAuditEntry[]>(100))
    } catch {
      // 网关未运行时后端返回空数组，不弹错误
      setEntries([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetch()
    const timer = setInterval(fetch, 10000)
    return () => clearInterval(timer)
  }, [fetch])

  const handleTry = async () => {
    if (!tryText.trim()) return
    setTrying(true)
    try {
      setTryResult(await previewMasking(tryText))
    } catch (e) {
      toast.error(String(e))
    } finally {
      setTrying(false)
    }
  }

  // 汇总：总命中数、按类型、按动作
  const totalItems = entries.reduce((sum, e) => sum + e.items.length, 0)
  const maskedCount = entries.reduce(
    (sum, e) => sum + e.items.filter(i => i.action === 'mask').length, 0
  )
  const warnedCount = entries.reduce(
    (sum, e) => sum + e.items.filter(i => i.action === 'warn').length, 0
  )
  const kindCounts = new Map<string, number>()
  for (const e of entries) {
    for (const item of e.items) {
      kindCounts.set(item.kind, (kindCounts.get(item.kind) ?? 0) + 1)
    }
  }

  // 类型分布环图数据：按敏感值类别聚合命中数，颜色映射 KIND_COLORS
  const kindPie = useMemo(() => {
    const palette = ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#06b6d4', '#64748b']
    return Array.from(kindCounts.entries()).map(([kind, value], i) => ({
      name: kind,
      value,
      color: KIND_COLORS[kind] ?? palette[i % palette.length],
    }))
  }, [entries]) // eslint-disable-line react-hooks/exhaustive-deps

  // 按小时趋势：masked/warned 堆叠柱状（occurredAt 取整到小时）
  const maskTrend = useMemo(() => {
    const buckets = new Map<string, { hour: string; masked: number; warned: number }>()
    for (const e of entries) {
      const m = e.occurredAt.match(/(\d{2}-\d{2})\s+(\d{2}):/)
      const hour = m ? `${m[1]} ${m[2]}:00` : e.occurredAt.slice(0, 10)
      const b = buckets.get(hour) ?? { hour, masked: 0, warned: 0 }
      for (const item of e.items) {
        if (item.action === 'mask') b.masked++
        else if (item.action === 'warn') b.warned++
      }
      buckets.set(hour, b)
    }
    return Array.from(buckets.values()).sort((a, b) => a.hour.localeCompare(b.hour))
  }, [entries])

  return (
    <div className="space-y-3">
      {/* 汇总卡 */}
      <div className="flex items-center gap-2 flex-wrap">
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
          <ShieldCheck size={12} className="text-emerald-500" />
          {t('gatewayMasking.totalMasked', { count: maskedCount })}
        </div>
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
          {t('gatewayMasking.totalWarned', { count: warnedCount })}
        </div>
        <div className="flex items-center gap-1.5 rounded border px-2.5 py-1 text-[11px] text-muted-foreground">
          {t('gatewayMasking.requestsAffected', { count: entries.length })}
        </div>
        {Array.from(kindCounts.entries()).map(([kind, count]) => (
          <span key={kind} className="rounded border px-2 py-0.5 text-[10px] text-muted-foreground">
            {kind} × {count}
          </span>
        ))}
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

      {/* 类型分布环图 + 按小时趋势柱状图 */}
      {kindPie.length > 0 && (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
          <div className="rounded-lg border p-2.5">
            <div className="mb-1.5 flex items-center gap-2">
              <ShieldCheck size={12} className="text-emerald-500" />
              <span className="text-[11px] font-medium">{t('gatewayMasking.kindPieTitle')}</span>
            </div>
            <ResponsiveContainer width="100%" height={160}>
              <PieChart>
                <Pie
                  data={kindPie}
                  dataKey="value"
                  nameKey="name"
                  innerRadius={38}
                  outerRadius={58}
                  paddingAngle={2}
                  strokeWidth={0}
                >
                  {kindPie.map((entry, i) => (
                    <Cell key={i} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip content={<MaskTooltip />} />
                <Legend wrapperStyle={{ fontSize: 10 }} />
              </PieChart>
            </ResponsiveContainer>
          </div>

          <div className="rounded-lg border p-2.5">
            <div className="mb-1.5 flex items-center gap-2">
              <BarChart3 size={12} className="text-blue-500" />
              <span className="text-[11px] font-medium">{t('gatewayMasking.trendTitle')}</span>
              <span className="text-[10px] text-muted-foreground">{t('gatewayMasking.trendSessionHint')}</span>
            </div>
            <ResponsiveContainer width="100%" height={160}>
              <BarChart data={maskTrend} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
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
                  width={32}
                  allowDecimals={false}
                />
                <Tooltip content={<MaskTooltip />} />
                <Legend wrapperStyle={{ fontSize: 10 }} />
                <Bar
                  dataKey="masked"
                  name={t('gatewayMasking.seriesMasked')}
                  stackId="a"
                  fill="#10b981"
                  maxBarSize={22}
                />
                <Bar
                  dataKey="warned"
                  name={t('gatewayMasking.seriesWarned')}
                  stackId="a"
                  fill="#f59e0b"
                  radius={[3, 3, 0, 0]}
                  maxBarSize={22}
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}

      {/* Try-it：试一段文本 */}
      <div className="rounded-lg border p-2.5">
        <div className="flex items-center gap-2 mb-1.5">
          <Eye size={12} className="text-blue-500" />
          <span className="text-[11px] font-medium">{t('gatewayMasking.tryIt')}</span>
        </div>
        <div className="flex gap-2">
          <textarea
            value={tryText}
            onChange={e => setTryText(e.target.value)}
            placeholder={t('gatewayMasking.tryPlaceholder')}
            className="h-16 flex-1 rounded-md border border-input bg-background px-2 py-1.5 text-xs"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={handleTry}
            disabled={trying || !tryText.trim()}
            className="h-7 gap-1 text-[10px]"
          >
            <Play size={10} />
            {trying ? t('common.loading') : t('gatewayMasking.run')}
          </Button>
        </div>
        {tryResult && (
          <div className="mt-2 space-y-1.5">
            <div className="rounded bg-muted/30 p-2 font-mono text-[11px] break-all">
              {tryResult.text}
            </div>
            {tryResult.items.length > 0 ? (
              <div className="space-y-0.5">
                {(tryResult.items as Array<{ kind: string; action: string; valueHead: string; valueLength: number; rule: string }>).map((item, idx) => (
                  <div key={idx} className="flex items-center gap-2 text-[10px] text-muted-foreground">
                    <ActionBadge action={item.action} />
                    <span className="font-mono">{item.kind}</span>
                    <span>「{item.valueHead}…」</span>
                    <span>{item.valueLength} 字符</span>
                    <span className="opacity-60">{item.rule}</span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-[10px] text-muted-foreground">{t('gatewayMasking.noHits')}</p>
            )}
          </div>
        )}
      </div>

      {/* 记录列表 */}
      <div className="rounded-lg border">
        <div className="border-b bg-muted/20 px-2.5 py-1.5 text-[11px] font-medium">
          {t('gatewayMasking.records')}
        </div>
        {entries.length === 0 ? (
          <p className="px-2.5 py-3 text-[11px] text-muted-foreground">
            {t('gatewayMasking.noRecords')}
          </p>
        ) : (
          <div className="max-h-96 overflow-auto">
            {entries.map(entry => {
              const rowKey = `${entry.requestId}-${entry.occurredAt}`
              const expanded = expandedId === rowKey
              return (
                <div key={rowKey} className="border-b border-border/40 last:border-0">
                  <button
                    onClick={() => setExpandedId(expanded ? null : rowKey)}
                    className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-[11px] transition-colors hover:bg-primary/5"
                  >
                    <span className="font-mono text-muted-foreground">{entry.occurredAt}</span>
                    <span className="text-foreground">{entry.endpoint}</span>
                    <span className="max-w-[120px] truncate text-muted-foreground" title={entry.clientUserAgent ?? ''}>
                      {shortClientLabel(entry.clientUserAgent ?? undefined) || '-'}
                    </span>
                    <span className="ml-auto flex items-center gap-1">
                      {entry.items.slice(0, 3).map((item, idx) => (
                        <ActionBadge key={idx} action={item.action} />
                      ))}
                      {entry.items.length > 3 && (
                        <span className="text-[10px] text-muted-foreground">+{entry.items.length - 3}</span>
                      )}
                    </span>
                  </button>
                  {expanded && (
                    <div className="space-y-2 bg-muted/10 px-2.5 py-2">
                      {/* 并排对照：vendor 实际收到的（占位符版）↔ 命中条目 */}
                      <div>
                        <p className="mb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                          {t('gatewayMasking.vendorSaw')}
                        </p>
                        <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all rounded-lg border border-border/50 bg-background/60 p-2 font-mono text-[10px]">
                          {entry.maskedPreview}
                        </pre>
                      </div>
                      <div>
                        <p className="mb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                          {t('gatewayMasking.hits')}
                        </p>
                        <div className="space-y-0.5">
                          {entry.items.map((item, idx) => (
                            <div key={idx} className="flex items-center gap-2 text-[10px]">
                              <ActionBadge action={item.action} />
                              <span className="font-mono">{item.kind}</span>
                              {item.placeholder && (
                                <code className="rounded bg-muted px-1 text-[10px]">{item.placeholder}</code>
                              )}
                              <span className="text-muted-foreground">
                                {item.valueHead}… ({item.valueLength})
                              </span>
                              <span className="ml-auto opacity-60">{item.rule}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                      <p className="text-[10px] text-muted-foreground">
                        {t('gatewayMasking.requestId')}: <code className="font-mono">{entry.requestId}</code>
                      </p>
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        )}
      </div>

      {/* 说明 */}
      <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-2.5">
        <EyeOff size={12} className="mt-0.5 shrink-0 text-amber-500" />
        <p className="text-[10px] leading-relaxed text-muted-foreground">
          {t('gatewayMasking.notice')}
        </p>
      </div>
      <div className="flex items-start gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
        <BarChart3 size={12} className="mt-0.5 shrink-0 text-muted-foreground" />
        <p className="text-[10px] leading-relaxed text-muted-foreground">
          {t('gatewayMasking.sessionScopeNotice')}
        </p>
      </div>
    </div>
  )
}
