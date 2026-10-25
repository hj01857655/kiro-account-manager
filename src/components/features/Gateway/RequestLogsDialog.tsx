import React, { useState, useEffect, useCallback, useRef } from 'react'
import {
  clearAllCache,
  clearGatewayRequestLogs,
  clearRateLimitAccount,
  cleanupExpiredCache,
  cleanupStaleHealth,
  getAllAccountHealth,
  getCacheStats,
  getGatewayRequestLogs,
  getGatewayRequestStats,
  getRateLimitedAccounts,
  openGatewayLogDir,
  readGatewaySseLog,
  resetAccountHealth
} from '../../../api/gatewayApi'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { Input } from '@/components/ui/input'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody
} from '@/components/shared/dialog'
import {
  Activity,
  RefreshCw,
  XCircle,
  Trash2,
  Database,
  FolderOpen,
  Download,
  Search,
  Check,
  Copy,
  ChevronDown,
  ChevronRight
} from 'lucide-react'
import { toast } from 'sonner'
import { cn } from '@/lib/utils'
import { useApp } from '@/hooks/useApp'
import { shortClientLabel } from './gatewayPageUtils'

interface ProcessedRequestLog {
  id: string
  /** 后端 request_id（UUID）：唯一键，展开详情与定位都靠它 */
  requestId?: string
  timestamp: string
  path: string
  status: number
  duration: number
  model?: string
  error?: string
  inputTokens?: number
  outputTokens?: number
  cacheReadTokens?: number
  cacheCreationTokens?: number
  /** 上游 meteringEvent 的真实计费消耗；非流式请求为 undefined */
  credits?: number
  upstream?: string
  errorType?: string
  outcome?: string
  stream?: boolean
  /** 客户端发来的 User-Agent（区分 Claude Code / Codex / 自建客户端） */
  clientUserAgent?: string
  requestBody?: string
  responseBody?: string
}

interface GatewayRequestStats {
  total: number
  success: number
  error: number
  streaming: number
  totalInputTokens: number
  totalOutputTokens: number
  totalCacheReadTokens: number
  totalCacheCreationTokens: number
  requestsWithCache: number
  maxDurationMs: number
  avgDurationMs: number
}

interface CacheStats {
  delta_cache_size: number
  lru_cache_size: number
  persistent_cache_enabled: boolean
}

// 网关运行时账号健康度（键为账号 ID）
interface AccountHealth {
  accountId: string
  activeConnections: number
  recentFailures: number
  recentSuccesses: number
  isHealthy: boolean
  avgResponseTimeMs: number
  healthScore: number
}

export interface RequestLogsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  logLevel: string
  onLogLevelChange: (level: string) => void
  logRequests: boolean
  onLogRequestsChange: (enabled: boolean) => void
  onSave?: () => void
  /** 内嵌模式：作为网关页的「请求日志」tab 内容渲染，不套 Dialog 外壳。
   *  数据、轮询、交互与弹窗完全共用，只是少了 open/onOpenChange 这一层。 */
  embedded?: boolean
}

/**
 * 把「多个 JSON 对象首尾相连」的文本切成逐个美化后的块。
 *
 * 后端把上游 EventStream 的每个事件解码成 JSON 后直接 join('')，所以响应体
 * 形如 `{"a":1}{"b":2}` —— 整体不是合法 JSON，`JSON.parse` 必失败。
 * 这里用括号配对扫描（忽略字符串内的括号与转义）找出每个顶层对象的边界，
 * 逐个 parse + 缩进；扫到不配对的残片就原样保留，不丢数据。
 */
function splitConcatenatedJson(text: string): string[] {
  const out: string[] = []
  let depth = 0
  let start = -1
  let inString = false
  let escaped = false

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i]

    if (inString) {
      if (escaped) {
        escaped = false
      } else if (ch === '\\') {
        escaped = true
      } else if (ch === '"') {
        inString = false
      }
      continue
    }

    if (ch === '"') {
      inString = true
      if (start < 0) start = i
      continue
    }

    if (ch === '{' || ch === '[') {
      if (depth === 0) start = i
      depth += 1
      continue
    }

    if (ch === '}' || ch === ']') {
      depth -= 1
      if (depth === 0 && start >= 0) {
        const chunk = text.slice(start, i + 1)
        try {
          out.push(JSON.stringify(JSON.parse(chunk), null, 2))
        } catch {
          out.push(chunk)
        }
        start = -1
      } else if (depth < 0) {
        // 不配对的收尾括号：重置，避免后续全部错位
        depth = 0
        start = -1
      }
    }
  }

  return out
}

function formatRequestLogAccount(upstream?: string) {
  if (!upstream) return '-'

  if (upstream.startsWith('single:') || upstream.startsWith('pool:')) {
    return upstream.split(':').slice(1).join(':') || upstream
  }

  if (upstream.startsWith('group:')) {
    return upstream.split(':').slice(2).join(':') || upstream
  }

  return upstream
}

export function RequestLogsDialog({
  open,
  onOpenChange,
  logLevel,
  onLogLevelChange,
  logRequests,
  onLogRequestsChange,
  onSave,
  embedded = false
}: RequestLogsDialogProps) {
  const { t } = useApp()
  const [requestLogs, setRequestLogs] = useState<ProcessedRequestLog[]>([])
  /** 流式请求的 SSE 明细：按 log.id 缓存，展开时按需拉取 */
  const [sseEventsByLogId, setSseEventsByLogId] = useState<Record<string, string[]>>({})
  const [sseLoadingId, setSseLoadingId] = useState<string | null>(null)
  const [requestStats, setRequestStats] = useState<GatewayRequestStats | null>(null)
  const [cacheStats, setCacheStats] = useState<CacheStats | null>(null)
  // 账号健康度：折叠区，默认收起（不抢日志表的视觉）。
  // 维度统计（按模型/端点/客户端计数）已删：信息全部来自日志表本身，
  // 且每轮轮询要多打 3 个统计接口，白付成本。
  const [health, setHealth] = useState<Record<string, AccountHealth>>({})
  const [healthUnavailable, setHealthUnavailable] = useState(false)
  // 被限流的账号 ID 集合：健康度表里标红并给「解除」入口
  const [rateLimited, setRateLimited] = useState<Set<string>>(new Set())
  const [showHealth, setShowHealth] = useState(false)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [activeFilter, setActiveFilter] = useState<'all' | 'success' | 'error'>('all')
  const [expandedLogId, setExpandedLogId] = useState<string | null>(null)
  const [displayLimit, setDisplayLimit] = useState(50)
  const [searchText, setSearchText] = useState('')
  const [copiedPayloadId, setCopiedPayloadId] = useState<string | null>(null)
  const [clearConfirm, setClearConfirm] = useState(false)

  const filteredLogs = requestLogs.filter(log => {
    if (activeFilter === 'success') return log.status < 400
    if (activeFilter === 'error') return log.status >= 400
    return true
  }).filter(log => {
    if (!searchText) return true
    const lower = searchText.toLowerCase()
    return (log.model?.toLowerCase().includes(lower))
      || log.path.toLowerCase().includes(lower)
      || log.upstream?.toLowerCase().includes(lower)
      || formatRequestLogAccount(log.upstream).toLowerCase().includes(lower)
      || log.error?.toLowerCase().includes(lower)
      || String(log.status).includes(lower)
  })

  // 拉取代次：清空时把在途请求全部作废。否则清空前发出的拉取会带着旧数据
  // 晚于清空落地，把刚置空的列表重新填上——表现为「清空后日志又回来了」。
  const fetchGenerationRef = useRef(0)
  const clearedGenerationRef = useRef(0)

  const fetchRequestLogs = useCallback(async (limit?: number) => {
    const generation = ++fetchGenerationRef.current
    setIsRefreshing(true)
    try {
      const [logs, stats, cache] = await Promise.all([
        getGatewayRequestLogs<any[]>(limit || displayLimit),
        getGatewayRequestStats<GatewayRequestStats>(),
        getCacheStats<CacheStats>().catch(() => null)
      ])

      // 清空后落在路上的旧响应整批丢弃：数据是清空前的快照，
      // 统计/维度/健康度同理，不能用旧值覆盖已清空的状态
      if (generation <= clearedGenerationRef.current) return

      // 网关未启动时后端返回 Err，这是正常状态（"Gateway not initialized"），
      // 不是故障，因此用健康度是否可用来驱动提示而不是弹错误
      getAllAccountHealth<Record<string, AccountHealth>>()
        .then(v => { setHealth(v || {}); setHealthUnavailable(false) })
        .catch(() => { setHealth({}); setHealthUnavailable(true) })
      // 限流列表与健康度同源，一并拉取；失败同样视为网关未运行
      getRateLimitedAccounts()
        .then(ids => setRateLimited(new Set(ids || [])))
        .catch(() => setRateLimited(new Set()))

      setRequestLogs(logs.map(log => ({
        // 用后端的 request_id（UUID）作唯一键：requestIndex 重启归零、
        // occurredAt 只到秒，两者拼接在同一秒的落盘历史里会撞 key。
        id: log.requestId || `${log.requestIndex}-${log.occurredAt}`,
        timestamp: log.occurredAt,
        path: log.endpoint,
        status: log.statusCode,
        duration: log.durationMs,
        model: log.model,
        error: log.error?.length > 500 ? log.error.substring(0, 500) + '...' : log.error,
        inputTokens: log.inputTokens,
        outputTokens: log.outputTokens,
        cacheReadTokens: log.cacheReadInputTokens,
        cacheCreationTokens: log.cacheCreationInputTokens,
        credits: log.credits,
        upstream: log.upstreamSource,
        errorType: log.errorType,
        outcome: log.outcome,
        stream: log.stream,
        clientUserAgent: log.clientUserAgent,
        requestBody: log.requestBody,
        responseBody: log.responseBody
      })))
      setRequestStats(stats)
      if (cache) setCacheStats(cache)
    } catch (error) {
      console.error('Failed to fetch request logs:', error)
    } finally {
      setIsRefreshing(false)
    }
  }, [displayLimit])

  const handleClearCache = async () => {
    try {
      await clearAllCache()
      setCacheStats(prev => prev ? { ...prev, delta_cache_size: 0, lru_cache_size: 0 } : null)
      toast.success(t('gatewayLogs.toastCacheCleared'))
    } catch (err) {
      toast.error(t('gatewayLogs.toastClearCacheFailed', { error: String(err) }))
    }
  }

  // 清理过期缓存：后端返回清理条数，直接回显使用户知道到底动了什么
  const handleCleanupExpired = async () => {
    try {
      const removed = await cleanupExpiredCache()
      toast.success(t('gatewayLogs.toastExpiredCleaned', { count: removed }))
      await fetchRequestLogs()
    } catch (err) {
      toast.error(t('gatewayLogs.toastClearCacheFailed', { error: String(err) }))
    }
  }

  // 重置单个账号健康度：让刚恢复的账号立刻重新参与调度，无需等滑动窗口自然衰减
  const handleResetHealth = async (accountId: string) => {
    try {
      await resetAccountHealth(accountId)
      const next = await getAllAccountHealth<Record<string, AccountHealth>>().catch(() => null)
      if (next) setHealth(next)
      toast.success(t('gatewayLogs.toastHealthReset'))
    } catch (err) {
      toast.error(t('gatewayLogs.toastHealthResetFailed', { error: String(err) }))
    }
  }

  // 解除限流：让被限流的账号立即重新参与调度（无需等滑动窗口自然过期）
  const handleClearRateLimit = async (accountId: string) => {
    try {
      await clearRateLimitAccount(accountId)
      setRateLimited(prev => {
        const next = new Set(prev)
        next.delete(accountId)
        return next
      })
      const next = await getRateLimitedAccounts().catch(() => null)
      if (next) setRateLimited(new Set(next))
      toast.success(t('gatewayLogs.toastRateLimitCleared'))
    } catch (err) {
      toast.error(t('gatewayLogs.toastHealthResetFailed', { error: String(err) }))
    }
  }

  // 清理超过 1 小时未检查的健康条目（后端 stale_duration 固定 3600s）
  const handleCleanupStale = async () => {
    try {
      await cleanupStaleHealth()
      const next = await getAllAccountHealth<Record<string, AccountHealth>>().catch(() => null)
      if (next) setHealth(next)
      toast.success(t('gatewayLogs.toastStaleCleaned'))
    } catch (err) {
      toast.error(t('gatewayLogs.toastHealthResetFailed', { error: String(err) }))
    }
  }

  const handleClearLogs = async () => {
    if (!clearConfirm) {
      setClearConfirm(true)
      setTimeout(() => setClearConfirm(false), 3000)
      return
    }
    try {
      await clearGatewayRequestLogs()
      // 作废所有在途拉取：它们的响应晚于清空落地就会把旧日志带回来
      clearedGenerationRef.current = fetchGenerationRef.current
      setRequestLogs([])
      setRequestStats(null)
      setClearConfirm(false)
      toast.success(t('gatewayLogs.toastLogsCleared'))
    } catch (err) {
      toast.error(t('gatewayLogs.toastClearFailed', { error: String(err) }))
    }
  }

  const handleExport = () => {
    const content = filteredLogs.map(log =>
      `[${log.timestamp}] ${log.status} ${log.path} ${log.model || '-'} account:${formatRequestLogAccount(log.upstream)} ${log.duration}ms in:${log.inputTokens || 0} out:${log.outputTokens || 0} cR:${log.cacheReadTokens || 0} cW:${log.cacheCreationTokens || 0}${log.error ? ' ERR:' + log.error : ''}`
    ).join('\n')
    const blob = new Blob([content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `gateway-logs-${new Date().toISOString().replace(/[:.]/g, '-')}.log`
    a.click()
    URL.revokeObjectURL(url)
    toast.success(t('gatewayLogs.toastExported'))
  }

  const handleCopyPayload = async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopiedPayloadId(id)
      toast.success(t('gatewayLogs.toastPayloadCopied'))
      setTimeout(() => setCopiedPayloadId(null), 1500)
    } catch {
      toast.error(t('gatewayLogs.toastCopyFailed'))
    }
  }

  /** 按 request_id 拉取该流式请求的 SSE 逐事件明细（结果按 log.id 缓存）。 */
  const handleLoadSse = async (log: ProcessedRequestLog) => {
    if (!log.requestId) return
    if (sseEventsByLogId[log.id]) {
      // 已拉过：再点一次收起
      setSseEventsByLogId((prev) => {
        const next = { ...prev }
        delete next[log.id]
        return next
      })
      return
    }
    setSseLoadingId(log.id)
    try {
      const events = await readGatewaySseLog(log.requestId, log.path)
      setSseEventsByLogId((prev) => ({ ...prev, [log.id]: events ?? [] }))
    } catch (err) {
      toast.error(t('gatewayLogs.toastSseLoadFailed', { error: String(err) }))
    } finally {
      setSseLoadingId(null)
    }
  }

  const formatPayload = (bodyStr?: string) => {
    if (!bodyStr) return '-'
    try {
      const parsed = JSON.parse(bodyStr)
      return JSON.stringify(parsed, null, 2)
    } catch {
      // 不是单个 JSON。EventStream 解码后的响应体是**多个 JSON 对象首尾相连**
      // （每个上游事件一个），整体永远不是合法 JSON。逐对象切开逐个美化，
      // 否则整段会挤成一行、完全不可读。
      const split = splitConcatenatedJson(bodyStr)
      if (split.length > 1) {
        return split.join('\n')
      }
      return bodyStr
    }
  }

  // 依赖 displayLimit，修复 5 秒轮询重置显示条数 Bug
  // 内嵌模式下始终拉取（isActive 恒真）；弹窗模式仍由 open 驱动。
  const isActive = embedded || open

  useEffect(() => {
    if (!isActive) return
    fetchRequestLogs()
    const interval = setInterval(() => {
      fetchRequestLogs()
    }, 5000)
    return () => clearInterval(interval)
  }, [isActive, fetchRequestLogs])

  // 请求日志面板内容：统计/缓存 + 维度折叠区 + 日志表。
  // 弹窗与内嵌 tab 共用同一段 JSX，避免两处各写一份导致口径漂移。
  const renderLogsPanel = () => (
    <div className="space-y-3 flex flex-col min-h-0">

      {/* 顶部：统计与缓存 */}
      <div className="flex items-center justify-between gap-2 bg-muted/20 p-2.5 rounded-lg flex-wrap">
        <div className="flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-1.5 text-xs">
            <span className="text-muted-foreground">{t('gatewayLogs.total')}</span>
            <span className="font-semibold">{requestStats?.total || 0}</span>
            <span className="text-green-600 border-l pl-2 ml-1">{t('gatewayLogs.success')}</span>
            <span className="font-semibold text-green-600">{requestStats?.success || 0}</span>
            <span className="text-red-500 border-l pl-2 ml-1">{t('gatewayLogs.error')}</span>
            <span className="font-semibold text-red-500">{requestStats?.error || 0}</span>
          </div>

          {requestStats && requestStats.totalInputTokens > 0 && (
            <div className="text-xs text-muted-foreground border-l pl-3 ml-1">
              <span>Tokens: </span>
              <strong>{(requestStats.totalInputTokens / 1000).toFixed(1)}K</strong> {t('gatewayLogs.tokensInput')} /
              <strong> {(requestStats.totalOutputTokens / 1000).toFixed(1)}K</strong> {t('gatewayLogs.tokensOutput')}
              {requestStats.totalCacheReadTokens > 0 && (
                <span className="text-blue-500 font-medium">
                  {t('gatewayLogs.cacheSaved', { pct: ((requestStats.totalCacheReadTokens / (requestStats.totalInputTokens + requestStats.totalCacheReadTokens)) * 100).toFixed(0) })}
                </span>
              )}
            </div>
          )}

          {cacheStats && (
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground border-l pl-3 ml-1">
              <Database size={11} className="text-blue-500" />
              <span>{t('gatewayLogs.cacheLabel')} <strong>{cacheStats.lru_cache_size + cacheStats.delta_cache_size}</strong> {t('gatewayLogs.cacheItems', { delta: cacheStats.delta_cache_size, lru: cacheStats.lru_cache_size })}</span>
            </div>
          )}
        </div>

        {/* 开关与缓存清理 */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <Switch size="sm" checked={logRequests} onCheckedChange={onLogRequestsChange} />
            <span className="text-xs text-muted-foreground">{t('gatewayLogs.logRequests')}</span>
          </div>

          {cacheStats && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClearCache}
              className="h-6 text-[10px] text-blue-600 hover:text-blue-700 hover:bg-blue-50 dark:hover:bg-blue-950/20 px-1.5 gap-1 font-normal"
            >
              <Database size={10} />
              {t('gatewayLogs.clearCache')}
            </Button>
          )}

          {cacheStats && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCleanupExpired}
              className="h-6 text-[10px] text-muted-foreground hover:text-foreground px-1.5 gap-1 font-normal"
              title={t('gatewayLogs.cleanupExpiredTitle')}
            >
              <Trash2 size={10} />
              {t('gatewayLogs.cleanupExpired')}
            </Button>
          )}
        </div>
      </div>

      {/* 工具栏 */}
      <div className="flex items-center justify-between gap-2 flex-wrap">
        <div className="flex items-center gap-2 flex-1 min-w-[200px]">
          <div className="relative flex-1 max-w-xs">
            <Input
              placeholder={t('gatewayLogs.searchPlaceholder')}
              className="h-8 text-xs pl-7"
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
            />
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
          </div>

          <select
            className="text-xs border rounded bg-background px-2 h-8 text-muted-foreground outline-none hover:border-primary/50 cursor-pointer transition-colors"
            value={activeFilter}
            onChange={(e) => setActiveFilter(e.target.value as any)}
          >
            <option value="all">{t('gatewayLogs.filterAll')}</option>
            <option value="success">{t('gatewayLogs.filterSuccess')}</option>
            <option value="error">{t('gatewayLogs.filterError')}</option>
          </select>

          <select
            className="text-xs border rounded bg-background px-2 h-8 text-muted-foreground outline-none hover:border-primary/50 cursor-pointer transition-colors"
            value={displayLimit}
            onChange={(e) => { setDisplayLimit(Number(e.target.value)) }}
          >
            <option value={50}>{t('gatewayLogs.keepLatest50')}</option>
            <option value={100}>{t('gatewayLogs.keepLatest100')}</option>
            <option value={200}>{t('gatewayLogs.keepLatest200')}</option>
          </select>
        </div>

        {/* 操作控制 */}
        <div className="flex gap-1.5 items-center">
          <select
            className="text-xs border rounded bg-background px-2 h-8 text-muted-foreground outline-none hover:border-primary/50 cursor-pointer transition-colors mr-1"
            value={logLevel}
            onChange={(e) => onLogLevelChange(e.target.value)}
            title={t('gatewayLogs.logLevelTitle')}
          >
            <option value="debug">{t('gatewayLogs.levelDebug')}</option>
            <option value="info">{t('gatewayLogs.levelInfo')}</option>
            <option value="warn">{t('gatewayLogs.levelWarn')}</option>
            <option value="error">{t('gatewayLogs.levelError')}</option>
          </select>

          <Button
            variant="outline"
            size="sm"
            className="h-8 px-2 gap-1 text-xs"
            onClick={() => openGatewayLogDir()}
            title={t('gatewayLogs.openLogDirTitle')}
          >
            <FolderOpen size={12} />
            {t('gatewayLogs.logDir')}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className="h-8 px-2 gap-1 text-xs"
            onClick={handleExport}
            disabled={filteredLogs.length === 0}
            title={t('gatewayLogs.exportTitle')}
          >
            <Download size={12} />
            {t('gatewayLogs.export')}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className={cn(
              "h-8 px-2 text-xs transition-all",
              clearConfirm && "bg-destructive text-destructive-foreground hover:bg-destructive/90 hover:text-destructive-foreground border-destructive"
            )}
            onClick={handleClearLogs}
            disabled={requestLogs.length === 0}
          >
            <Trash2 size={12} className="mr-1 inline-block" />
            {clearConfirm ? t('gatewayLogs.confirmClear') : t('gatewayLogs.clear')}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className="h-8 w-8 p-0"
            onClick={() => fetchRequestLogs()}
            disabled={isRefreshing}
          >
            <RefreshCw size={12} className={cn(isRefreshing && 'animate-spin')} />
          </Button>
        </div>
      </div>

      {/* 账号健康度：折叠区，默认收起以免抢日志表的视觉 */}
      <div className="flex items-center gap-2 flex-wrap">
        <button
          onClick={() => setShowHealth(v => !v)}
          className="flex items-center gap-1 rounded border px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:border-primary/50 hover:text-foreground"
        >
          {showHealth ? <ChevronDown size={11} /> : <ChevronRight size={11} />}
          {t('gatewayLogs.healthTitle')}
          {!healthUnavailable && <span className="opacity-60">({Object.keys(health).length})</span>}
        </button>
      </div>

      {showHealth && (
        <div className="rounded-lg border">
          <div className="flex items-center justify-between border-b bg-muted/20 px-2.5 py-1.5">
            <span className="text-[11px] font-medium">{t('gatewayLogs.healthTitle')}</span>
            {!healthUnavailable && Object.keys(health).length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCleanupStale}
                className="h-6 gap-1 px-1.5 text-[10px] font-normal text-muted-foreground"
              >
                <Trash2 size={10} />
                {t('gatewayLogs.cleanupStale')}
              </Button>
            )}
          </div>
          {healthUnavailable ? (
            <p className="px-2.5 py-3 text-[11px] text-muted-foreground">{t('gatewayLogs.healthUnavailable')}</p>
          ) : Object.keys(health).length === 0 ? (
            <p className="px-2.5 py-3 text-[11px] text-muted-foreground">{t('gatewayLogs.noData')}</p>
          ) : (
            <div className="max-h-52 overflow-auto">
              <table className="w-full text-[11px]">
                <thead className="sticky top-0 bg-muted">
                  <tr className="border-b">
                    <th className="px-2.5 py-1.5 text-left font-sans font-medium">{t('gatewayLogs.colAccount')}</th>
                    <th className="w-14 px-1 py-1.5 text-right font-sans font-medium">{t('gatewayLogs.colHealthScore')}</th>
                    <th className="w-12 px-1 py-1.5 text-right font-sans font-medium">{t('gatewayLogs.colSuccess')}</th>
                    <th className="w-12 px-1 py-1.5 text-right font-sans font-medium">{t('gatewayLogs.colFailure')}</th>
                    <th className="w-14 px-1 py-1.5 text-right font-sans font-medium">{t('gatewayLogs.colConns')}</th>
                    <th className="w-16 px-1 py-1.5 text-right font-sans font-medium">{t('gatewayLogs.colAvgMs')}</th>
                    <th className="w-16 px-1 py-1.5 text-center font-sans font-medium">{t('gatewayLogs.colRateLimit')}</th>
                    <th className="w-14 px-2 py-1.5" />
                  </tr>
                </thead>
                <tbody>
                  {Object.values(health).map(h => (
                    <tr key={h.accountId} className="border-b border-border/40 last:border-0">
                      <td className="truncate px-2.5 py-1 font-mono" title={h.accountId}>{h.accountId}</td>
                      <td className={cn('px-1 py-1 text-right font-semibold', h.healthScore >= 80 ? 'text-green-600' : h.healthScore >= 50 ? 'text-amber-500' : 'text-red-500')}>
                        {h.healthScore}
                      </td>
                      <td className="px-1 py-1 text-right text-green-600">{h.recentSuccesses}</td>
                      <td className="px-1 py-1 text-right text-red-500">{h.recentFailures}</td>
                      <td className="px-1 py-1 text-right">{h.activeConnections}</td>
                      <td className="px-1 py-1 text-right">{h.avgResponseTimeMs}</td>
                      <td className="px-1 py-1 text-center">
                        {rateLimited.has(h.accountId) && (
                          <button
                            onClick={() => handleClearRateLimit(h.accountId)}
                            title={t('gatewayLogs.rateLimitedHint')}
                            className="rounded px-1.5 py-0.5 text-[10px] font-normal text-amber-600 bg-amber-500/10 hover:bg-amber-500/20 dark:text-amber-400 cursor-pointer"
                          >
                            {t('gatewayLogs.rateLimited')}
                          </button>
                        )}
                      </td>
                      <td className="px-2 py-1 text-right">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleResetHealth(h.accountId)}
                          className="h-5 px-1.5 text-[10px] font-normal text-muted-foreground"
                        >
                          {t('gatewayLogs.resetHealth')}
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* 日志表格区域 */}
      <div className="border rounded-lg flex-1 min-h-0 flex flex-col">
        <div className="flex-1 overflow-auto">
          <table className="w-full text-xs font-mono border-collapse">
            <thead className="sticky top-0 bg-muted z-10">
              <tr className="border-b shadow-[0_1px_0_0_rgba(0,0,0,0.1)]">
                <th className="px-2 py-2 text-left w-[10px] font-sans"></th>
                <th className="px-2 py-2 text-left w-[100px] font-sans">{t('gatewayLogs.colTime')}</th>
                <th className="px-2 py-2 text-left w-[100px] font-sans">{t('gatewayLogs.colPath')}</th>
                <th className="px-2 py-2 text-left w-[110px] font-sans" title={t('gatewayLogs.colClientTitle')}>{t('gatewayLogs.colClient')}</th>
                <th className="px-2 py-2 text-left w-[140px] font-sans">{t('gatewayLogs.colAccount')}</th>
                <th className="px-2 py-2 text-left w-[150px] font-sans">{t('gatewayLogs.colModel')}</th>
                <th className="px-2 py-2 text-center w-[50px] font-sans" title={t('gatewayLogs.colStatusTitle')}>{t('gatewayLogs.colStatus')}</th>
                <th className="px-2 py-2 text-center w-[50px] font-sans">{t('gatewayLogs.colInput')}</th>
                <th className="px-2 py-2 text-center w-[50px] font-sans">{t('gatewayLogs.colOutput')}</th>
                <th className="px-2 py-2 text-center w-[50px] font-sans" title={t('gatewayLogs.colCacheReadTitle')}>{t('gatewayLogs.colCacheRead')}</th>
                <th className="px-2 py-2 text-center w-[50px] font-sans" title={t('gatewayLogs.colCacheWriteTitle')}>{t('gatewayLogs.colCacheWrite')}</th>
                <th className="px-2 py-2 text-center w-[70px] font-sans" title={t('gatewayLogs.colCreditsTitle')}>{t('gatewayLogs.colCredits')}</th>
                <th className="px-2 py-2 text-center w-[50px] font-sans">{t('gatewayLogs.colDuration')}</th>
              </tr>
            </thead>
            <tbody>
              {filteredLogs.length === 0 ? (
                <tr>
                  <td colSpan={12} className="text-center py-16 text-muted-foreground font-sans text-sm">
                    {requestLogs.length === 0 ? t('gatewayLogs.waitingTraffic') : t('gatewayLogs.noMatch')}
                  </td>
                </tr>
              ) : (
                filteredLogs.map((log) => {
                  const isExpanded = expandedLogId === log.id;
                  return (
                    <React.Fragment key={log.id}>
                      <tr
                        className={cn(
                          "border-b border-border/50 hover:bg-muted/40 cursor-pointer transition-colors",
                          isExpanded && "bg-muted/20"
                        )}
                        onClick={() => setExpandedLogId(isExpanded ? null : log.id)}
                      >
                        <td className="px-2 py-2 text-center text-muted-foreground">
                          {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                        </td>
                        <td className="px-2 py-2 text-muted-foreground whitespace-nowrap">{log.timestamp}</td>
                        <td className="px-2 py-2 text-muted-foreground font-semibold truncate max-w-[140px]" title={log.path}>
                          {log.outcome?.includes('cache') && <span className="text-blue-500 mr-1" title={t('gatewayLogs.cacheHitTitle')}>⚡</span>}
                          {log.path}
                        </td>
                        <td className="px-2 py-2 truncate max-w-[110px] text-muted-foreground" title={log.clientUserAgent || '-'}>
                          {shortClientLabel(log.clientUserAgent)}
                        </td>
                        <td className="px-2 py-2 truncate max-w-[140px] text-muted-foreground" title={log.upstream || '-'}>
                          {formatRequestLogAccount(log.upstream)}
                        </td>
                        <td className="px-2 py-2 truncate max-w-[120px]" title={log.model}>{log.model || '-'}</td>
                        <td className="px-2 py-2 text-center">
                          <span className={cn(
                            "px-1.5 py-0.5 rounded text-[10px] font-semibold",
                            log.status >= 400
                              ? 'bg-destructive/10 text-destructive'
                              : log.status >= 300
                                ? 'bg-yellow-500/10 text-yellow-600'
                                : 'bg-green-500/10 text-green-600'
                          )}>
                            {log.status}
                          </span>
                          {log.stream && <span className="ml-1 text-blue-400 font-semibold" title={t('gatewayLogs.streamingTitle')}>⇣</span>}
                        </td>
                        <td className="px-2 py-2 text-right text-muted-foreground">{log.inputTokens?.toLocaleString() || '-'}</td>
                        <td className="px-2 py-2 text-right text-muted-foreground">{log.outputTokens?.toLocaleString() || '-'}</td>
                        <td className="px-2 py-2 text-right text-blue-500 font-medium" title={t('gatewayLogs.colCacheReadTitle')}>{log.cacheReadTokens?.toLocaleString() || '-'}</td>
                        <td className="px-2 py-2 text-right text-purple-500 font-medium" title={t('gatewayLogs.colCacheWriteTitle')}>{log.cacheCreationTokens?.toLocaleString() || '-'}</td>
                        <td className="px-2 py-2 text-right font-mono text-emerald-600 dark:text-emerald-400" title={t('gatewayLogs.colCreditsTitle')}>{log.credits != null ? log.credits.toFixed(4) : '-'}</td>
                        <td className="px-2 py-2 text-right">
                          <span className={log.duration > 5000 ? 'text-orange-500 font-bold' : log.duration > 2000 ? 'text-yellow-600 font-medium' : 'text-muted-foreground'}>
                            {log.duration}ms
                          </span>
                        </td>
                      </tr>

                      {/* 展开详细信息（包括 Payload 格式化） */}
                      {isExpanded && (
                        <tr className="bg-muted/15 border-b">
                          <td colSpan={11} className="px-4 py-3">
                            <div className="space-y-3 font-sans">
                              {log.error && (
                                <div className="flex items-start gap-2 border border-destructive/20 bg-destructive/5 p-3 rounded-lg">
                                  <XCircle size={14} className="text-destructive mt-0.5 shrink-0" />
                                  <pre className="text-xs font-mono text-destructive dark:text-red-400 whitespace-pre-wrap break-all">{log.error}</pre>
                                </div>
                              )}

                              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                {/* 请求体：GET 类端点（health / models）没有请求体，
                                    后端记为长度 0。整块不渲染，否则会留一个空白框。 */}
                                {log.requestBody && (
                                <div className="space-y-1">
                                  <div className="flex items-center justify-between text-xs text-muted-foreground font-semibold">
                                    <span>{t('gatewayLogs.requestPayload')}</span>
                                    {log.requestBody && (
                                      <Button
                                        variant="ghost"
                                        size="sm"
                                        className="h-5 px-1.5 gap-1 text-[10px] hover:bg-muted"
                                        onClick={(e) => {
                                          e.stopPropagation()
                                          handleCopyPayload(log.requestBody!, 'req-' + log.id)
                                        }}
                                      >
                                        {copiedPayloadId === 'req-' + log.id ? <Check size={10} className="text-green-600" /> : <Copy size={10} />}
                                        {t('gatewayLogs.copy')}
                                      </Button>
                                    )}
                                  </div>
                                  <pre className="text-[10px] font-mono bg-background p-3 rounded-lg border max-h-48 overflow-y-auto whitespace-pre-wrap break-all text-muted-foreground">
                                    {formatPayload(log.requestBody)}
                                  </pre>
                                </div>
                                )}

                                {/* 响应体：流式请求后端只存占位符，空时不渲染块 */}
                                {log.responseBody && (
                                <div className="space-y-1">
                                  <div className="flex items-center justify-between text-xs text-muted-foreground font-semibold">
                                    <span>{t('gatewayLogs.responsePayload')}</span>
                                    {log.responseBody && (
                                      <Button
                                        variant="ghost"
                                        size="sm"
                                        className="h-5 px-1.5 gap-1 text-[10px] hover:bg-muted"
                                        onClick={(e) => {
                                          e.stopPropagation()
                                          handleCopyPayload(log.responseBody!, 'resp-' + log.id)
                                        }}
                                      >
                                        {copiedPayloadId === 'resp-' + log.id ? <Check size={10} className="text-green-600" /> : <Copy size={10} />}
                                        {t('gatewayLogs.copy')}
                                      </Button>
                                    )}
                                  </div>
                                  <pre className="text-[10px] font-mono bg-background p-3 rounded-lg border max-h-48 overflow-y-auto whitespace-pre-wrap break-all text-muted-foreground">
                                    {formatPayload(log.responseBody)}
                                  </pre>
                                </div>
                                )}
                              </div>

                              {/* 流式请求：SSE 逐事件明细只落文件、不进内存日志。
                                  提供「查看事件」就地展开，不用跳出去翻文件。 */}
                              {log.stream && (
                                <div className="mt-2 rounded-lg border border-amber-500/30 bg-amber-500/5 p-2.5 text-[11px]">
                                  <div className="flex items-center justify-between gap-2">
                                    <span className="text-muted-foreground">
                                      {t('gatewayLogs.sseDetailHint')}
                                      <code className="ml-1 font-mono text-foreground">{log.path.includes('/responses') ? 'openai-responses-response-sse.log' : log.path.includes('/chat/completions') ? 'openai-chat-response-sse.log' : 'anthropic-messages-response-sse.log'}</code>
                                    </span>
                                    <div className="flex shrink-0 gap-1">
                                      <Button
                                        variant="outline"
                                        size="sm"
                                        className="h-6 px-2 text-[10px]"
                                        disabled={!log.requestId}
                                        onClick={(e) => {
                                          e.stopPropagation()
                                          void handleLoadSse(log)
                                        }}
                                      >
                                        {sseLoadingId === log.id ? t('gatewayLogs.sseLoading') : t('gatewayLogs.sseShowEvents')}
                                      </Button>
                                      <Button
                                        variant="outline"
                                        size="sm"
                                        className="h-6 px-2 text-[10px]"
                                        onClick={(e) => {
                                          e.stopPropagation()
                                          void openGatewayLogDir()
                                        }}
                                      >
                                        {t('gatewayLogs.openLogDir')}
                                      </Button>
                                    </div>
                                  </div>
                                  {sseEventsByLogId[log.id] && (
                                    <pre className="mt-2 max-h-64 overflow-y-auto whitespace-pre-wrap break-all rounded border bg-background p-2 font-mono text-[10px] text-muted-foreground">
                                      {sseEventsByLogId[log.id].length === 0
                                        ? t('gatewayLogs.sseNoEvents')
                                        : sseEventsByLogId[log.id].join('\n')}
                                    </pre>
                                  )}
                                </div>
                              )}
                            </div>
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  )
                })
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )

  // 内嵌模式：直接返回面板内容，不套 Dialog 外壳（供网关页的「请求日志」tab 复用）
  if (embedded) {
    return (
      <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto p-5">
        {renderLogsPanel()}
      </div>
    )
  }

  return (
    <DialogRoot open={open} onOpenChange={(v) => { onOpenChange(v); if (!v && onSave) onSave() }}>
      <DialogContent maxWidth="1000px" className="max-h-[95vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Activity className="w-5 h-5 text-primary" />
            {t('gatewayLogs.title')}
          </DialogTitle>
          <DialogDescription>{t('gatewayLogs.description')}</DialogDescription>
        </DialogHeader>

        <DialogBody className="space-y-3 flex flex-col min-h-0">
          {renderLogsPanel()}
        </DialogBody>
      </DialogContent>
    </DialogRoot>
  )
}

export default RequestLogsDialog
