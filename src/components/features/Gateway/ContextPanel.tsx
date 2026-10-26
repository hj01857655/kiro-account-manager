import { useState, useEffect, useCallback, useMemo } from 'react'
import { Gauge, RefreshCw, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import {
  LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend,
} from 'recharts'
import { getGatewayRequestLogHistory } from '../../../api/gatewayApi'
import { ContextMap } from './ContextMap'

interface PromptPartRow { kind: string; tokens: number }
interface PromptRow { tokens: number; counted: boolean; parts?: PromptPartRow[] }
interface ContextLogRow {
  occurredAt: string
  sessionId?: string | null
  statusCode: number
  cacheReadInputTokens?: number | null
  contextUsagePercentage?: number | null
  prompt?: PromptRow | null
}

interface SessionStat {
  id: string
  requests: number
  errors: number
  medianTokens: number
  baseline: number
  growth: number
  compacts: number
  cacheRate: number | null
  peakFill: number | null
  score: number
  tags: string[]
  points: { idx: number; tokens: number; cache: number }[]
  /** 最新一次请求的 prompt 构成（地图用） */
  latestParts: PromptPartRow[]
  /** 最新一次请求的 prompt 总 tokens */
  latestTokens: number
  /** 窗口大小（tokens，中位数推断） */
  window: number
}

const partSum = (row: ContextLogRow, kinds: string[]): number =>
  row.prompt?.parts?.filter((p) => kinds.includes(p.kind)).reduce((s, p) => s + p.tokens, 0) ?? 0

function median(values: number[]): number {
  if (values.length === 0) return 0
  const s = [...values].sort((a, b) => a - b)
  const mid = s.length >> 1
  return s.length % 2 ? s[mid] : Math.round((s[mid - 1] + s[mid]) / 2)
}

/** 窗口大小推断：优先用上游 contextUsagePercentage 反推，缺省 200k。 */
function inferWindow(row: ContextLogRow, tokens: number): number {
  const pct = row.contextUsagePercentage
  if (pct && pct > 0 && tokens > 0) return Math.round((tokens / pct) * 100)
  return 200_000
}

/** 100 分制：缓存 35 / 起点 25 / 增速 20 / 可靠 20。 */
function computeScore(cacheRate: number | null, baseline: number, growth: number, errorRate: number, window: number): number {
  const cache = cacheRate == null ? 0 : Math.min(35, (cacheRate / 0.9) * 35)
  const baseRatio = baseline / window
  const lean = Math.max(0, 25 - (baseRatio - 0.05) * (25 / 0.25))
  const growthRatio = growth / window
  const pace = Math.max(0, 20 - (growthRatio - 0.01) * (20 / 0.05))
  const reliable = Math.max(0, 20 - errorRate * 100)
  return Math.round(cache + lean + pace + reliable)
}

function computeTags(s: {
  cacheRate: number | null; baseline: number; growth: number; window: number
  errorRate: number; compacts: number; peakFill: number | null
  share: Record<string, number>
}): string[] {
  const tags: string[] = []
  if (s.cacheRate != null && s.cacheRate >= 0.8) tags.push('cache-friendly')
  if (s.cacheRate != null && s.cacheRate < 0.5) tags.push('cache-misses')
  if (s.baseline / s.window <= 0.05) tags.push('lean')
  if (s.baseline / s.window > 0.3) tags.push('heavy-start')
  if (s.growth / s.window > 0.06) tags.push('fills-fast')
  if (s.growth / s.window <= 0.01) tags.push('steady')
  if (s.compacts > 0) tags.push('compacts')
  if (s.peakFill != null && s.peakFill >= 0.85) tags.push('near-full')
  if ((s.share.results ?? 0) >= 0.3) tags.push('tool-heavy')
  if ((s.share.files ?? 0) >= 0.25) tags.push('file-reader')
  if ((s.share.memory ?? 0) >= 0.1) tags.push('memory-heavy')
  if (s.errorRate > 0.2) tags.push('flaky')
  return tags
}

/** 上下文页：每请求 prompt 构成 → 会话级基线/增速/缓存/评分/标签 + 填充曲线。 */
export function ContextPanel() {
  const { t } = useApp()
  const [rows, setRows] = useState<ContextLogRow[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [selected, setSelected] = useState<string | null>(null)

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

  const sessions = useMemo<SessionStat[]>(() => {
    // 按会话聚合；只统计「带工具定义」的会话——标题生成、配额检查这类
    // 不带工具的杂音请求会伪装成上下文压缩，剔除后曲线才可信。
    const groups = new Map<string, ContextLogRow[]>()
    for (const row of rows) {
      if (!row.sessionId || !row.prompt) continue
      const arr = groups.get(row.sessionId) ?? []
      arr.push(row)
      groups.set(row.sessionId, arr)
    }
    const out: SessionStat[] = []
    for (const [id, rs] of groups) {
      const withPrompt = rs.filter((r) => r.prompt)
      if (withPrompt.length === 0) continue
      const isToolSession = withPrompt.some((r) => partSum(r, ['tools']) > 0)
      if (!isToolSession) continue
      // occurredAt 为 "YYYY-MM-DD HH:MM:SS"，字典序即时间序
      const ordered = [...withPrompt].sort((a, b) => (a.occurredAt < b.occurredAt ? -1 : 1))
      const tokens = ordered.map((r) => r.prompt!.tokens)
      const baselines = ordered.map((r) => partSum(r, ['system', 'tools', 'memory']))
      const growths: number[] = []
      for (let i = 1; i < tokens.length; i++) {
        const delta = tokens[i] - tokens[i - 1]
        if (delta > 0) growths.push(delta)
      }
      let compacts = 0
      for (let i = 1; i < tokens.length; i++) {
        if (tokens[i - 1] >= 20_000 && tokens[i] < tokens[i - 1] * 0.6) compacts += 1
      }
      // 缓存命中率：只统计上游报了 usage 的请求（counted）；
      // 无 usage 的请求整条剔除而不是当 miss
      let cached = 0
      let counted = 0
      for (const r of ordered) {
        if (!r.prompt?.counted) continue
        const cr = Math.min(r.cacheReadInputTokens ?? 0, r.prompt.tokens)
        cached += cr
        counted += r.prompt.tokens
      }
      const cacheRate = counted > 0 ? cached / counted : null
      // share：各类构成占总 tokens 的比例（全会话合计）
      const totals: Record<string, number> = {}
      let grand = 0
      for (const r of ordered) {
        for (const p of r.prompt?.parts ?? []) {
          totals[p.kind] = (totals[p.kind] ?? 0) + p.tokens
          grand += p.tokens
        }
      }
      const share: Record<string, number> = {}
      for (const [k, v] of Object.entries(totals)) share[k] = grand > 0 ? v / grand : 0
      const errors = rs.filter((r) => r.statusCode >= 400).length
      const errorRate = rs.length > 0 ? errors / rs.length : 0
      const baseline = median(baselines)
      const growth = median(growths)
      const fills = ordered
        .map((r) => (r.contextUsagePercentage ?? 0) / 100)
        .filter((v) => v > 0)
      const peakFill = fills.length > 0 ? Math.max(...fills) : null
      const window = median(ordered.map((r) => inferWindow(r, r.prompt!.tokens)))
      const score = computeScore(cacheRate, baseline, growth, errorRate, window)
      const tags = computeTags({ cacheRate, baseline, growth, window, errorRate, compacts, peakFill, share })
      out.push({
        id,
        requests: rs.length,
        errors,
        medianTokens: median(tokens),
        baseline,
        growth,
        compacts,
        cacheRate,
        peakFill,
        score,
        tags,
        points: tokens.map((tk, i) => ({ idx: i + 1, tokens: tk, cache: ordered[i].cacheReadInputTokens ?? 0 })),
        // 地图画「最新一次请求」的构成：窗口是此刻的状态，不是会话平均
        latestParts: ordered[ordered.length - 1].prompt!.parts ?? [],
        latestTokens: ordered[ordered.length - 1].prompt!.tokens,
        window,
      })
    }
    return out.sort((a, b) => b.requests - a.requests)
  }, [rows])

  const selectedSession = useMemo(
    () => sessions.find((s) => s.id === selected) ?? sessions[0],
    [sessions, selected],
  )

  const overall = useMemo(() => {
    if (sessions.length === 0) return null
    const scores = sessions.map((s) => s.score)
    const rates = sessions.map((s) => s.cacheRate).filter((r): r is number => r != null)
    return {
      avgScore: Math.round(scores.reduce((a, b) => a + b, 0) / scores.length),
      cacheRate: rates.length > 0 ? rates.reduce((a, b) => a + b, 0) / rates.length : null,
      baseline: median(sessions.map((s) => s.baseline)),
      growth: median(sessions.map((s) => s.growth)),
      compacts: sessions.reduce((a, s) => a + s.compacts, 0),
    }
  }, [sessions])

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

      {overall && (
        <>
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
            <Card label={t('gatewayContext.avgScore')} value={String(overall.avgScore)} suffix="/100" />
            <Card
              label={t('gatewayContext.cacheRate')}
              value={overall.cacheRate != null ? `${(overall.cacheRate * 100).toFixed(1)}%` : '—'}
            />
            <Card label={t('gatewayContext.baseline')} value={fmtK(overall.baseline)} />
            <Card label={t('gatewayContext.growth')} value={`+${fmtK(overall.growth)}`} />
          </div>
          <p className="text-xs text-muted-foreground">
            {t('gatewayContext.hintSessions', { count: sessions.length, compacts: overall.compacts })}
          </p>
        </>
      )}

      {!overall && !loading && !error && (
        <div className="text-sm text-muted-foreground py-8 text-center">
          {t('gatewayContext.empty')}
        </div>
      )}

      {selectedSession && (
        <div className="grid gap-3 lg:grid-cols-[minmax(0,440px)_minmax(0,1fr)]">
          {/* 构成地图：窗口被谁占满了 */}
          <div className="border rounded-lg p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium">{t('gatewayContext.mapTitle')}</span>
              <span className="font-mono text-xs text-muted-foreground max-w-[50%] truncate" title={selectedSession.id}>
                {fmtK(selectedSession.latestTokens)} / {fmtK(selectedSession.window)}
              </span>
            </div>
            <ContextMap
              parts={selectedSession.latestParts}
              tokens={selectedSession.latestTokens}
              window={selectedSession.window}
            />
          </div>

          <div className="border rounded-lg p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium">{t('gatewayContext.curve')}</span>
              <span className="font-mono text-xs text-muted-foreground max-w-[60%] truncate" title={selectedSession.id}>
                {selectedSession.id}
              </span>
            </div>
            <div className="h-48">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={selectedSession.points} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                  <XAxis dataKey="idx" tick={{ fontSize: 11 }} stroke="var(--muted-foreground)" />
                  <YAxis tick={{ fontSize: 11 }} stroke="var(--muted-foreground)" tickFormatter={fmtK} width={48} />
                  <Tooltip
                    formatter={(v: number, name: string) => [fmtK(v), name === 'tokens' ? t('gatewayContext.promptTokens') : t('gatewayUsage.cacheRead')]}
                    labelFormatter={(l) => `#${l}`}
                  />
                  <Legend
                    formatter={(v: string) => (v === 'tokens' ? t('gatewayContext.promptTokens') : t('gatewayUsage.cacheRead'))}
                  />
                  <Line type="monotone" dataKey="tokens" stroke="var(--primary, #3b82f6)" dot={false} strokeWidth={2} />
                  <Line type="monotone" dataKey="cache" stroke="#22c55e" dot={false} strokeWidth={1.5} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>
      )}

      {sessions.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-muted/50">
              <tr>
                <th className="text-left px-3 py-2 font-medium">{t('gatewaySessions.session')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayContext.medianTokens')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayContext.baseline')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayContext.growth')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayContext.cacheRate')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayContext.avgScore')}</th>
                <th className="text-left px-3 py-2 font-medium">{t('gatewayContext.tags')}</th>
              </tr>
            </thead>
            <tbody>
              {sessions.slice(0, 50).map((s) => (
                <tr
                  key={s.id}
                  className={`border-t cursor-pointer hover:bg-muted/40 ${selectedSession?.id === s.id ? 'bg-muted/40' : ''}`}
                  onClick={() => setSelected(s.id)}
                >
                  <td className="px-3 py-2 max-w-[180px] truncate font-mono text-xs" title={s.id}>{s.id}</td>
                  <td className="px-3 py-2 text-right tabular-nums">{fmtK(s.medianTokens)}</td>
                  <td className="px-3 py-2 text-right tabular-nums">{fmtK(s.baseline)}</td>
                  <td className="px-3 py-2 text-right tabular-nums">+{fmtK(s.growth)}</td>
                  <td className="px-3 py-2 text-right tabular-nums">
                    {s.cacheRate != null ? `${(s.cacheRate * 100).toFixed(0)}%` : '—'}
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums font-medium">{s.score}</td>
                  <td className="px-3 py-2">
                    <div className="flex flex-wrap gap-1">
                      {s.tags.map((tag) => (
                        <span key={tag} className="text-[10px] px-1.5 py-0.5 rounded-full bg-muted text-muted-foreground font-mono">
                          {tag}
                        </span>
                      ))}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <p className="text-xs text-muted-foreground flex items-start gap-1.5">
        <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
        {t('gatewayContext.hintFull')}
      </p>
    </div>
  )
}

function Card({ label, value, suffix }: { label: string; value: string; suffix?: string }) {
  return (
    <div className="surface-card border rounded-lg p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-2xl font-medium tabular-nums">
        {value}
        {suffix && <span className="text-sm text-muted-foreground font-normal ml-1">{suffix}</span>}
      </div>
    </div>
  )
}

function fmtK(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10_000 ? 0 : 1)}K`
  return String(Math.round(n))
}
