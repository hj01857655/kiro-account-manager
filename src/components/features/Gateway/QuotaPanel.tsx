import { useState, useEffect, useCallback } from 'react'
import { Gauge, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '../../../hooks/useApp'
import { getGatewayQuota } from '../../../api/gatewayApi'

interface QuotaEntry {
  accountId: string
  email?: string | null
  provider?: string | null
  region?: string | null
  usage: unknown
  creditsConsumed: number
  totalRequests: number
  totalTokens: number
  error?: string | null
}

function fmtInt(n: number): string {
  return n.toLocaleString()
}

/** 从 usage JSON 里尽力提取一行人类可读的额度摘要（各渠道结构不同，宽松处理） */
function summarizeUsage(usage: unknown): string {
  if (!usage || typeof usage !== 'object') return '-'
  const u = usage as Record<string, unknown>
  const parts: string[] = []
  // Kiro usageLimits: subscription / usageLimit / overagePageSize 等结构随渠道变化
  const sub = u.subscription as Record<string, unknown> | undefined
  if (sub) {
    const util = (sub.usagePercentage ?? sub.usageUtilizationPercentage) as number | undefined
    if (typeof util === 'number') parts.push(`${Math.round(util)}%`)
    const renewal = (sub.renewalDate ?? sub.nextDate) as string | undefined
    if (renewal) parts.push(String(renewal))
  }
  if (parts.length === 0) {
    const s = JSON.stringify(usage)
    return s.length > 120 ? s.slice(0, 120) + '…' : s
  }
  return parts.join(' · ')
}

/** 额度页：网关作用域内每个账号的上游配额 + 本地 credits 消耗聚合。 */
export function QuotaPanel() {
  const { t } = useApp()
  const [entries, setEntries] = useState<QuotaEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await getGatewayQuota<QuotaEntry[]>()
      setEntries(data)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  return (
    <div className="flex flex-col gap-4 p-4 h-full overflow-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Gauge className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">{t('gatewayQuota.title')}</h3>
        </div>
        <Button variant="outline" size="sm" onClick={load} disabled={loading}>
          <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${loading ? 'animate-spin' : ''}`} />
          {t('common.refresh')}
        </Button>
      </div>

      {error && (
        <div className="text-sm text-red-500">{t('gatewayQuota.loadFailed', { error })}</div>
      )}

      {entries.length === 0 && !loading && !error && (
        <div className="text-sm text-muted-foreground py-8 text-center">
          {t('gatewayQuota.empty')}
        </div>
      )}

      {entries.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-muted/50">
              <tr>
                <th className="text-left px-3 py-2 font-medium">{t('gatewayQuota.account')}</th>
                <th className="text-left px-3 py-2 font-medium">{t('gatewayQuota.upstreamQuota')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayQuota.creditsUsed')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayBills.requests')}</th>
                <th className="text-right px-3 py-2 font-medium">{t('gatewayUsage.tokens')}</th>
                <th className="text-left px-3 py-2 font-medium">{t('gatewayQuota.status')}</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => (
                <tr key={e.accountId} className="border-t">
                  <td className="px-3 py-2 max-w-[200px] truncate" title={e.email ?? e.accountId}>
                    <div className="truncate">{e.email || e.accountId}</div>
                    <div className="text-xs text-muted-foreground">
                      {e.provider ?? '-'} · {e.region ?? '-'}
                    </div>
                  </td>
                  <td className="px-3 py-2 max-w-[260px] truncate text-xs" title={summarizeUsage(e.usage)}>
                    {e.error ? '-' : summarizeUsage(e.usage)}
                  </td>
                  <td className="px-3 py-2 text-right tabular-nums">{e.creditsConsumed.toFixed(2)}</td>
                  <td className="px-3 py-2 text-right tabular-nums">{fmtInt(e.totalRequests)}</td>
                  <td className="px-3 py-2 text-right tabular-nums">{fmtInt(e.totalTokens)}</td>
                  <td className="px-3 py-2 text-xs">
                    {e.error ? (
                      <span className="text-red-500" title={e.error}>
                        {t('gatewayQuota.queryFailed')}
                      </span>
                    ) : (
                      <span className="text-green-600 dark:text-green-400">
                        {t('gatewayQuota.ok')}
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <p className="text-xs text-muted-foreground">{t('gatewayQuota.hint')}</p>
    </div>
  )
}
