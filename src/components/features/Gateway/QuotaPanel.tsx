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

/** 按真实 getUsageLimits 结构解析额度：usageBreakdownList[0] 的用量/上限，
 *  subscriptionInfo.overageCapability 标注超额能力。与 core/usage.rs 同源。
 *  解析失败回退 JSON 截断，保证总能看到点什么。 */
function summarizeUsage(usage: unknown, t: (k: string) => string): string {
  if (!usage || typeof usage !== 'object') return '-'
  const u = usage as Record<string, unknown>
  const readAmount = (item: Record<string, unknown>, base: string): number | undefined => {
    const withPrecision = item[`${base}WithPrecision`]
    if (typeof withPrecision === 'number') return withPrecision
    const plain = item[base]
    if (typeof plain === 'number') return plain
    return undefined
  }
  const parts: string[] = []
  const breakdown = (u.usageBreakdownList as Array<Record<string, unknown>> | undefined)?.[0]
  if (breakdown) {
    const current = readAmount(breakdown, 'currentUsage')
    const limit = readAmount(breakdown, 'usageLimit')
    if (typeof current === 'number' && typeof limit === 'number' && limit > 0) {
      parts.push(
        `${current.toLocaleString(undefined, { maximumFractionDigits: 2 })} / ${limit.toLocaleString(undefined, { maximumFractionDigits: 2 })}`,
        `${Math.round((current / limit) * 100)}%`,
      )
    } else if (typeof current === 'number') {
      parts.push(current.toLocaleString(undefined, { maximumFractionDigits: 2 }))
    }
    const overageCap = readAmount(breakdown, 'overageCap')
    if (typeof overageCap === 'number' && overageCap > 0) {
      parts.push(
        `${t('gatewayQuota.overage')} ${overageCap.toLocaleString(undefined, { maximumFractionDigits: 2 })}`,
      )
    }
  }
  const overage = (u.subscriptionInfo as Record<string, unknown> | undefined)?.overageCapability
  if (overage === 'OVERAGE_CAPABLE') parts.push(t('gatewayQuota.overageCapable'))
  if (overage === 'OVERAGE_INCAPABLE') parts.push(t('gatewayQuota.overageIncapable'))
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
                  <td className="px-3 py-2 max-w-[260px] truncate text-xs" title={summarizeUsage(e.usage, t)}>
                    {e.error ? '-' : summarizeUsage(e.usage, t)}
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
