import { useState, useEffect, useCallback, useMemo } from 'react'
import { ScrollText, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import { getGatewayRequestLogHistory } from '../../../api/gatewayApi'

interface LedgerRow {
  occurredAt: string
  endpoint: string
  model?: string | null
  accountId?: string | null
  sessionId?: string | null
  statusCode: number
  stream?: boolean
  durationMs: number
  inputTokens?: number | null
  outputTokens?: number | null
  cacheReadInputTokens?: number | null
  credits?: number | null
}

const PAGE_SIZE = 100

/** 请求台账：逐行请求记录（倒序分页），对应 magpie 用量页的「请求」标签。 */
export function RequestsPanel() {
  const { t } = useApp()
  const [rows, setRows] = useState<LedgerRow[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [page, setPage] = useState(0)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const history = await getGatewayRequestLogHistory<LedgerRow[]>(20000)
      // occurredAt 是 "YYYY-MM-DD HH:MM:SS" 字符串，字典序即时间序
      setRows([...history].reverse())
      setPage(0)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const pageCount = Math.max(1, Math.ceil(rows.length / PAGE_SIZE))
  const visible = useMemo(
    () => rows.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE),
    [rows, page],
  )

  return (
    <div className="flex flex-col gap-3 h-full overflow-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ScrollText className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">{t('gatewayUsage.subRequests')}</h3>
          <span className="text-xs text-muted-foreground">
            {rows.length.toLocaleString()} {t('gatewayUsage.requests')}
          </span>
        </div>
        <Button variant="outline" size="sm" onClick={load} disabled={loading}>
          <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${loading ? 'animate-spin' : ''}`} />
          {t('common.refresh')}
        </Button>
      </div>

      {error && <div className="text-sm text-red-500">{t('gatewaySessions.loadFailed', { error })}</div>}

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="text-left px-3 py-2 font-medium">{t('gatewayUsage.time')}</th>
              <th className="text-left px-3 py-2 font-medium">{t('gatewayUsage.model')}</th>
              <th className="text-left px-3 py-2 font-medium">{t('gatewayUsage.endpoint')}</th>
              <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.statusCode')}</th>
              <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.duration')}</th>
              <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.tokens')}</th>
              <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.credits')}</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((r, i) => (
              <tr key={`${r.occurredAt}-${i}`} className="border-t">
                <td className="px-3 py-1.5 text-xs whitespace-nowrap text-muted-foreground">{r.occurredAt}</td>
                <td className="px-3 py-1.5 max-w-[180px] truncate" title={r.model ?? ''}>
                  {r.model ?? '-'}
                </td>
                <td className="px-3 py-1.5 text-xs text-muted-foreground">{r.endpoint}</td>
                <td className="px-3 py-1.5 text-right tabular-nums">
                  <span className={r.statusCode >= 400 ? 'text-red-500' : ''}>{r.statusCode}</span>
                  {r.stream && <span className="ml-1 text-[10px] text-muted-foreground">SSE</span>}
                </td>
                <td className="px-3 py-1.5 text-right tabular-nums text-xs">{r.durationMs.toLocaleString()}ms</td>
                <td className="px-3 py-1.5 text-right tabular-nums">
                  {((r.inputTokens ?? 0) + (r.outputTokens ?? 0)).toLocaleString()}
                </td>
                <td className="px-3 py-1.5 text-right tabular-nums">
                  {r.credits != null ? r.credits.toFixed(2) : '-'}
                </td>
              </tr>
            ))}
            {visible.length === 0 && (
              <tr>
                <td colSpan={7} className="px-3 py-8 text-center text-sm text-muted-foreground">
                  {t('gatewayUsage.noData')}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {pageCount > 1 && (
        <div className="flex items-center justify-center gap-2 text-sm">
          <Button variant="outline" size="sm" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            ←
          </Button>
          <span className="tabular-nums text-muted-foreground">
            {page + 1} / {pageCount}
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= pageCount - 1}
            onClick={() => setPage((p) => p + 1)}
          >
            →
          </Button>
        </div>
      )}
    </div>
  )
}
