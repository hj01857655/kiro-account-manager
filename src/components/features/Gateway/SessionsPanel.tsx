import { useState, useEffect, useCallback, useMemo } from 'react'
import { MessagesSquare, RefreshCw, Coins, Zap, History, Trash2, ChevronDown, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import {
  getGatewayRequestLogHistory,
  getGatewayConversationSessions,
  getGatewayConversationTurns,
  clearGatewayConversations,
  type ConvSessionSummary,
  type ConvTurn,
} from '../../../api/gatewayApi'

interface SessionLogRow {
  occurredAt: string
  sessionId?: string | null
  model?: string | null
  statusCode: number
  inputTokens?: number | null
  outputTokens?: number | null
  cacheReadInputTokens?: number | null
  cacheCreationInputTokens?: number | null
  credits?: number | null
}

interface SessionAgg {
  name: string
  requests: number
  errors: number
  input: number
  output: number
  cacheRead: number
  cacheWrite: number
  credits: number
  models: Set<string>
  firstSeen: string
  lastSeen: string
}

function fmtInt(n: number): string {
  return n.toLocaleString()
}

/** 会话页：按 sessionId 聚合请求日志——每个 agent 会话一行，token / credits / 请求数一目了然。 */
export function SessionsPanel() {
  const { t } = useApp()
  const [rows, setRows] = useState<SessionLogRow[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const history = await getGatewayRequestLogHistory<SessionLogRow[]>(20000)
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

  const sessions = useMemo<SessionAgg[]>(() => {
    const map = new Map<string, SessionAgg>()
    for (const row of rows) {
      // sessionId 由后端写入：x-session-id 头优先，回退「IP|UA」派生键；
      // 旧历史条目（字段上线前）没有该字段，跳过不归属
      const key = row.sessionId
      if (!key) continue
      let agg = map.get(key)
      if (!agg) {
        agg = {
          name: key,
          requests: 0,
          errors: 0,
          input: 0,
          output: 0,
          cacheRead: 0,
          cacheWrite: 0,
          credits: 0,
          models: new Set<string>(),
          firstSeen: row.occurredAt,
          lastSeen: row.occurredAt,
        }
        map.set(key, agg)
      }
      agg.requests += 1
      if (row.statusCode >= 400) agg.errors += 1
      agg.input += row.inputTokens ?? 0
      agg.output += row.outputTokens ?? 0
      agg.cacheRead += row.cacheReadInputTokens ?? 0
      agg.cacheWrite += row.cacheCreationInputTokens ?? 0
      agg.credits += row.credits ?? 0
      if (row.model) agg.models.add(row.model)
      if (row.occurredAt < agg.firstSeen) agg.firstSeen = row.occurredAt
      if (row.occurredAt > agg.lastSeen) agg.lastSeen = row.occurredAt
    }
    return Array.from(map.values()).sort((a, b) => b.credits - a.credits || b.requests - a.requests)
  }, [rows])

  // ===== 对话回放（捕获开关开启后才有数据）=====
  const [captures, setCaptures] = useState<ConvSessionSummary[]>([])
  const [expandedDir, setExpandedDir] = useState<string | null>(null)
  const [turns, setTurns] = useState<ConvTurn[]>([])
  const [loadingTurns, setLoadingTurns] = useState(false)

  const loadCaptures = useCallback(async () => {
    try {
      setCaptures(await getGatewayConversationSessions<ConvSessionSummary[]>())
    } catch {
      // 捕获未开启/无数据时静默为空
      setCaptures([])
    }
  }, [])

  useEffect(() => {
    loadCaptures()
  }, [loadCaptures])

  const toggleTurns = useCallback(async (dir: string) => {
    if (expandedDir === dir) {
      setExpandedDir(null)
      return
    }
    setExpandedDir(dir)
    setLoadingTurns(true)
    try {
      setTurns(await getGatewayConversationTurns<ConvTurn[]>(dir))
    } catch {
      setTurns([])
    } finally {
      setLoadingTurns(false)
    }
  }, [expandedDir])

  const handleClear = useCallback(async () => {
    await clearGatewayConversations()
    setCaptures([])
    setExpandedDir(null)
    setTurns([])
  }, [])

  const partStyle = (role: string, kind: string): string => {
    if (role === 'system') return 'border-l-2 border-amber-500/60 bg-amber-500/5'
    if (kind === 'tool_use') return 'border-l-2 border-blue-500/60 bg-blue-500/5'
    if (kind === 'tool_result') return 'border-l-2 border-purple-500/60 bg-purple-500/5'
    if (role === 'assistant') return 'border-l-2 border-green-500/60 bg-green-500/5'
    return 'border-l-2 border-border bg-muted/30'
  }

  return (
    <div className="flex flex-col gap-4 p-4 h-full overflow-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <MessagesSquare className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">{t('gatewaySessions.title')}</h3>
        </div>
        <Button variant="outline" size="sm" onClick={load} disabled={loading}>
          <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${loading ? 'animate-spin' : ''}`} />
          {t('common.refresh')}
        </Button>
      </div>

      {error && (
        <div className="text-sm text-red-500">{t('gatewaySessions.loadFailed', { error })}</div>
      )}

      {sessions.length === 0 && !loading && !error && (
        <div className="text-sm text-muted-foreground py-8 text-center">
          {t('gatewaySessions.empty')}
        </div>
      )}

      {sessions.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-muted/50">
              <tr>
                <th className="text-left px-3 py-2 font-medium">{t('gatewaySessions.session')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewaySessions.requests')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.tokens')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.cacheRead')}</th>
                <th className="text-right px-3 py-2 font-medium">
                  <Coins className="h-3.5 w-3.5 inline mr-1" />
                  {t('gatewayUsage.credits')}
                </th>
                <th className="text-left px-3 py-2 font-medium">{t('gatewaySessions.models')}</th>
                <th className="text-left px-3 py-2 font-medium">{t('gatewaySessions.lastActive')}</th>
              </tr>
            </thead>
            <tbody>
              {sessions.map((s) => (
                <tr key={s.name} className="border-t">
                  <td className="px-3 py-2 max-w-[220px] truncate font-mono text-xs" title={s.name}>
                    {s.name}
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums">
                    {fmtInt(s.requests)}
                    {s.errors > 0 && (
                      <span className="text-red-500 ml-1">({fmtInt(s.errors)})</span>
                    )}
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums">
                    <Zap className="h-3 w-3 inline mr-1 text-muted-foreground" />
                    {fmtInt(s.input + s.output)}
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
                    {fmtInt(s.cacheRead)}
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums">{s.credits.toFixed(2)}</td>
                  <td className="px-3 py-2 max-w-[180px] truncate text-xs text-muted-foreground">
                    {Array.from(s.models).join(', ') || '-'}
                  </td>
                  <td className="px-3 py-2 text-xs text-muted-foreground">{s.lastSeen}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {captures.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <div className="flex items-center justify-between px-3 py-2 bg-muted/50">
            <div className="flex items-center gap-2 text-sm font-medium">
              <History className="h-4 w-4 text-muted-foreground" />
              {t('gatewayReplay.title')}
            </div>
            <Button variant="ghost" size="sm" onClick={handleClear} className="h-7 text-xs text-muted-foreground">
              <Trash2 className="h-3.5 w-3.5 mr-1" />
              {t('gatewayReplay.clear')}
            </Button>
          </div>
          <div className="divide-y">
            {captures.map((c) => (
              <div key={c.dir}>
                <button
                  className="w-full flex items-center justify-between px-3 py-2 text-sm hover:bg-muted/40 text-left"
                  onClick={() => toggleTurns(c.dir)}
                >
                  <span className="flex items-center gap-2 min-w-0">
                    {expandedDir === c.dir ? (
                      <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    )}
                    <span className="font-mono text-xs truncate">{c.dir}</span>
                  </span>
                  <span className="text-xs text-muted-foreground shrink-0 ml-3 tabular-nums">
                    {c.turns} {t('gatewayReplay.turns')} · {c.last}
                  </span>
                </button>
                {expandedDir === c.dir && (
                  <div className="px-3 pb-3 flex flex-col gap-3 max-h-[420px] overflow-auto">
                    {loadingTurns && (
                      <div className="text-xs text-muted-foreground py-2">{t('gatewayReplay.loading')}</div>
                    )}
                    {!loadingTurns && turns.length === 0 && (
                      <div className="text-xs text-muted-foreground py-2">{t('gatewayReplay.empty')}</div>
                    )}
                    {turns.map((turn, ti) => (
                      <div key={turn.requestId} className="border rounded-md p-2.5">
                        <div className="flex items-center gap-2 text-xs text-muted-foreground mb-2">
                          <span className="font-medium text-foreground">#{ti + 1}</span>
                          <span>{turn.occurredAt}</span>
                          {turn.model && <span>{turn.model}</span>}
                          <span className={turn.statusCode >= 400 ? 'text-red-500' : ''}>{turn.statusCode}</span>
                          {turn.cut && <span className="text-amber-500">{t('gatewayReplay.truncated')}</span>}
                        </div>
                        <div className="flex flex-col gap-1.5">
                          {[...turn.input, ...turn.output].map((p, pi) => (
                            <div key={pi} className={`rounded px-2 py-1.5 text-xs whitespace-pre-wrap break-all ${partStyle(p.role, p.kind)}`}>
                              <span className="font-mono text-[10px] text-muted-foreground mr-1.5">
                                {p.role}/{p.kind}
                                {p.name ? `:${p.name}` : ''}
                              </span>
                              {p.text}
                              {p.cut > 0 && (
                                <span className="text-muted-foreground ml-1">…(+{p.cut})</span>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      <p className="text-xs text-muted-foreground">{t('gatewaySessions.hint')}</p>
    </div>
  )
}
