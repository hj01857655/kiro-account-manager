import { useState, useEffect, useCallback, useMemo } from 'react'
import { Receipt, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import { getGatewayRequestLogHistory } from '../../../api/gatewayApi'

interface BillLogRow {
  occurredAt: string
  accountId?: string | null
  model?: string | null
  statusCode: number
  inputTokens?: number | null
  outputTokens?: number | null
  cacheReadInputTokens?: number | null
  cacheCreationInputTokens?: number | null
  credits?: number | null
}

interface BillAgg {
  account: string
  model: string
  requests: number
  errors: number
  input: number
  output: number
  cacheRead: number
  cacheWrite: number
  credits: number
}

function emptyAgg(account: string, model: string): BillAgg {
  return { account, model, requests: 0, errors: 0, input: 0, output: 0, cacheRead: 0, cacheWrite: 0, credits: 0 }
}

function fmtInt(n: number): string {
  return n.toLocaleString()
}

/** 账单页：按「账号 × 模型」二维聚合请求日志。 */
export function BillsPanel() {
  const { t } = useApp()
  const [rows, setRows] = useState<BillLogRow[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const history = await getGatewayRequestLogHistory<BillLogRow[]>(20000)
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

  const bills = useMemo<BillAgg[]>(() => {
    const map = new Map<string, BillAgg>()
    for (const row of rows) {
      // accountId 字段上线前的历史条目无法归属账号，跳过
      const account = row.accountId
      if (!account) continue
      const model = row.model ?? 'unknown'
      const key = `${account}\u0000${model}`
      let agg = map.get(key)
      if (!agg) {
        agg = emptyAgg(account, model)
        map.set(key, agg)
      }
      agg.requests += 1
      if (row.statusCode >= 400) agg.errors += 1
      agg.input += row.inputTokens ?? 0
      agg.output += row.outputTokens ?? 0
      agg.cacheRead += row.cacheReadInputTokens ?? 0
      agg.cacheWrite += row.cacheCreationInputTokens ?? 0
      agg.credits += row.credits ?? 0
    }
    return Array.from(map.values()).sort((a, b) => b.credits - a.credits || b.requests - a.requests)
  }, [rows])

  const totals = useMemo(() => {
    return bills.reduce(
      (acc, b) => ({
        requests: acc.requests + b.requests,
        credits: acc.credits + b.credits,
      }),
      { requests: 0, credits: 0 },
    )
  }, [bills])

  return (
    <div className="flex flex-col gap-4 p-4 h-full overflow-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Receipt className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">{t('gatewayBills.title')}</h3>
        </div>
        <Button variant="outline" size="sm" onClick={load} disabled={loading}>
          <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${loading ? 'animate-spin' : ''}`} />
          {t('common.refresh')}
        </Button>
      </div>

      {error && (
        <div className="text-sm text-red-500">{t('gatewayBills.loadFailed', { error })}</div>
      )}

      {bills.length === 0 && !loading && !error && (
        <div className="text-sm text-muted-foreground py-8 text-center">
          {t('gatewayBills.empty')}
        </div>
      )}

      {bills.length > 0 && (
        <>
          <div className="flex gap-4 text-sm">
            <span>
              {t('gatewayBills.totalRequests')}: <span className="tabular-nums font-medium">{fmtInt(totals.requests)}</span>
            </span>
            <span>
              {t('gatewayUsage.credits')}: <span className="tabular-nums font-medium">{totals.credits.toFixed(2)}</span>
            </span>
          </div>
          <div className="border rounded-lg overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-muted/50">
                <tr>
                  <th className="text-left px-3 py-2 font-medium">{t('gatewayBills.account')}</th>
                  <th className="text-left px-3 py-2 font-medium">{t('gatewayBills.model')}</th>
                  <th className="text-right px-3 py-2 font-medium">{t('gatewayBills.requests')}</th>
                  <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.tokens')}</th>
                  <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.cacheRead')}</th>
                  <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.credits')}</th>
                </tr>
              </thead>
              <tbody>
                {bills.map((b) => (
                  <tr key={`${b.account}\u0000${b.model}`} className="border-t">
                    <td className="px-3 py-2 max-w-[180px] truncate font-mono text-xs" title={b.account}>
                      {b.account}
                    </td>
                    <td className="px-3 py-2 max-w-[200px] truncate" title={b.model}>
                      {b.model}
                    </td>
                    <td className="px-3 py-2 text-right tabular-nums">
                      {fmtInt(b.requests)}
                      {b.errors > 0 && <span className="text-red-500 ml-1">({fmtInt(b.errors)})</span>}
                    </td>
                    <td className="px-3 py-2 text-right tabular-nums">{fmtInt(b.input + b.output)}</td>
                    <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
                      {fmtInt(b.cacheRead)}
                    </td>
                    <td className="px-3 py-2 text-right tabular-nums">{b.credits.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="text-xs text-muted-foreground">{t('gatewayBills.hint')}</p>
        </>
      )}
    </div>
  )
}
