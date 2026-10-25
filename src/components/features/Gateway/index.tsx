import { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Network } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { getGatewayRequestLogs, getGatewayRequestStats } from '../../../api/gatewayApi'
import { useApp } from '../../../hooks/useApp'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody
} from '@/components/shared/dialog'
import GatewayConfigComponent from './GatewayConfig'
import { GatewayStatusBar } from './GatewayStatusBar'
import { GatewayOverview } from './GatewayOverview'
import { MaskingPanel } from './MaskingPanel'
import { RequestLogsDialog } from './RequestLogsDialog'
import { ProtocolTestDialog } from './ProtocolTestDialog'
import { GatewayConfig, GatewayStatus, type GatewayOverviewStats } from './gatewayPageState'
import { ErrorHistoryEntry } from './gatewayPageUtils'

interface Account {
  id: string
  email?: string
  userId?: string
  /** 后端 Account.enabled：禁用与封禁都会写回 false，默认视为启用 */
  enabled?: boolean
}

interface Group {
  id: string
  name: string
}

import {
  applyGatewayLocalOnlyChange,
  buildClientSamples,
  buildGatewayActionSummary,
  buildGatewayBaseUrl,
  buildGatewayClientRecipes,
  buildGatewayRoutingSummary,
  buildGatewaySecuritySummary,
  CLIENT_API_KEY,
  createGatewayFieldErrors,
  formatGatewayAccountOptionLabel,
  formatGatewayTimestamp,
  mergeErrorHistory,
} from './gatewayPageUtils'
import {
  buildGatewayConfigSnapshot,
  buildGatewayRuntimeSnapshot,
  buildGatewayStatusState,
  DEFAULT_GATEWAY_CONFIG,
  DEFAULT_GATEWAY_STATUS,
  loadGatewayPageData,
  saveGatewayConfig,
  startGateway,
  stopGateway,
  hydrateGatewayConfig
} from './gatewayPageState'
import { useGatewayPolling } from './useGatewayPolling'

/**
 * 网关页面骨架：紧凑页头 + 常驻状态条 + 「概览 / 请求日志 / 配置」三个 tab。
 *
 * 三个 tab 按使用频率切分：看它（概览：状态、统计、把客户端接上）、查它
 * （请求日志：每次出问题必看）、调它（配置：装好一次就不再碰）。原来的单纵向
 * 滚动把 650 行配置压在状态与接入之上，这个分界就是那处割裂的解法。
 *
 * tab 叫「配置」而不是「设置」：侧边菜单已有「设置」页，同名会让人误以为是
 * 同一个东西。
 *
 * 状态条不进 tab：无论在看哪一页，「它现在活着吗」都是同一个问题。
 */
function GatewayPage() {
  const { t } = useApp()

  const [config, setConfig] = useState<GatewayConfig>(DEFAULT_GATEWAY_CONFIG)
  const [status, setStatus] = useState<GatewayStatus>(DEFAULT_GATEWAY_STATUS)
  const [errorHistory, setErrorHistory] = useState<ErrorHistoryEntry[]>([])
  const [accounts, setAccounts] = useState<Account[]>([])
  const [groups, setGroups] = useState<Group[]>([])
  // 状态条与日志弹窗共用同一份 LogStore 聚合，避免同一数字两套口径
  const [stats, setStats] = useState<GatewayOverviewStats | null>(null)
  // 概览「最近请求」迷你表的数据源，与统计同源同频
  const [recentLogs, setRecentLogs] = useState<any[]>([])
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [restarting, setRestarting] = useState(false)
  const [activeTab, setActiveTab] = useState('overview')
  const [showProtocolTest, setShowProtocolTest] = useState(false)
  const [savedConfigSnapshot, setSavedConfigSnapshot] = useState(() => buildGatewayConfigSnapshot(DEFAULT_GATEWAY_CONFIG))
  const [appliedRuntimeSnapshot, setAppliedRuntimeSnapshot] = useState<any>(null)
  const [toast, setToast] = useState('')

  const accountOptions = useMemo(
    () => accounts.map(account => ({
      value: account.id,
      label: formatGatewayAccountOptionLabel(account, t),
      account,
    })),
    [accounts]
  )

  const groupOptions = useMemo(
    () => groups.map(group => ({ value: group.id, label: group.name })),
    [groups]
  )

  const fieldErrors = useMemo(() => createGatewayFieldErrors(config, t), [config, t])
  const hasFieldErrors = Object.keys(fieldErrors).length > 0
  const configSnapshot = useMemo(() => buildGatewayConfigSnapshot(config), [config])
  const runtimeSnapshot = useMemo(() => buildGatewayRuntimeSnapshot(config), [config])
  const hasUnsavedChanges = configSnapshot !== savedConfigSnapshot
  const hasRuntimeChanges = !!status.running && !!appliedRuntimeSnapshot && runtimeSnapshot !== appliedRuntimeSnapshot

  const pushError = useCallback((msg: any) => {
    const normalized = String(msg?.message || msg || '').trim()
    if (!normalized) return
    setErrorHistory(prev => mergeErrorHistory(prev, normalized, formatGatewayTimestamp(), 8))
  }, [])

  const showToast = useCallback((message: string) => {
    setToast(message)
    setTimeout(() => setToast(''), 1800)
  }, [])

  const loadAll = useCallback(async () => {
    setLoading(true)
    try {
      const { gatewayConfig, gatewayStatus, accounts: accountList, groups: groupList } = await loadGatewayPageData()

      const nextConfig = hydrateGatewayConfig(gatewayConfig)
      const nextStatus = buildGatewayStatusState(gatewayStatus, gatewayConfig, nextConfig)
      const runtimeConfig = gatewayStatus?.running && nextStatus.runtimeConfig ? nextStatus.runtimeConfig : null
      setConfig(nextConfig)
      setSavedConfigSnapshot(buildGatewayConfigSnapshot(nextConfig))
      setAppliedRuntimeSnapshot(runtimeConfig ? buildGatewayRuntimeSnapshot(runtimeConfig) : null)
      setStatus(nextStatus)
      setAccounts(accountList)
      setGroups(groupList)

      if (gatewayStatus?.lastError) {
        pushError(gatewayStatus.lastError)
      }
    } catch (e) {
      pushError(e)
    } finally {
      setLoading(false)
    }
  }, [pushError])

  const refreshStats = useCallback(async () => {
    try {
      const next = await getGatewayRequestStats<GatewayOverviewStats>()
      setStats(next || null)
    } catch {
      // 统计读不到不掉页面——概览会显示占位符，日志入口仍然可点
      setStats(null)
    }
  }, [])

  // 概览的「最近请求」迷你表：只取 8 条，让错误作为表格里的一行状态出现，
  // 不再单设「最近错误」卡片（同粒度不两处呈现）。
  const refreshRecentLogs = useCallback(async () => {
    try {
      setRecentLogs(await getGatewayRequestLogs<any[]>(5, true))
    } catch {
      // 网关未运行时后端改读落盘日志；仍失败就留空，由表格显示占位
      setRecentLogs([])
    }
  }, [])

  useEffect(() => {
    startTransition(() => {
      loadAll()
    })
  }, [loadAll])

  // 概览统计跟随状态轮询刷新，与状态条同源同频
  useEffect(() => {
    refreshStats()
    const timer = setInterval(() => {
      if (!document.hidden) refreshStats()
    }, 10000)
    return () => clearInterval(timer)
  }, [refreshStats])

  // 最近请求与统计同频，避免表格与指标数字对不上
  useEffect(() => {
    refreshRecentLogs()
    const timer = setInterval(() => {
      if (!document.hidden) refreshRecentLogs()
    }, 10000)
    return () => clearInterval(timer)
  }, [refreshRecentLogs])

  const handleStatusPoll = useCallback(({ status: nextStatus, fallbackConfig }: any) => {
    const nextState = buildGatewayStatusState(nextStatus, nextStatus, fallbackConfig)
    setStatus(nextState)
    setAppliedRuntimeSnapshot(nextState.running && nextState.runtimeConfig
      ? buildGatewayRuntimeSnapshot(nextState.runtimeConfig)
      : null)
    if (nextStatus?.lastError) {
      pushError(nextStatus.lastError)
    }
  }, [pushError])

  useGatewayPolling({
    fallbackConfig: useMemo(() => ({ host: config.host, port: config.port }), [config.host, config.port]),
    onStatus: handleStatusPoll
  })

  const setField = (key: string, value: any) => setConfig(prev => ({ ...prev, [key]: value }))

  const applyStatusFromResponse = (st: any, fallback: GatewayConfig) => {
    const nextStatus = buildGatewayStatusState(st, st, fallback)
    setStatus(nextStatus)
    setAppliedRuntimeSnapshot(nextStatus.runtimeConfig
      ? buildGatewayRuntimeSnapshot(nextStatus.runtimeConfig)
      : buildGatewayRuntimeSnapshot(fallback))
  }

  // 自动保存 + 自动重启（防抖 1.5 秒）。重启期间状态条会显示「重启中…」，
  // 不再像以前那样静默 stop/start 而界面一言不发。
  const autoSaveTimer = useRef<NodeJS.Timeout | null>(null)
  const isInitialLoad = useRef(true)
  useEffect(() => {
    if (isInitialLoad.current) {
      isInitialLoad.current = false
      return
    }
    if (!hasUnsavedChanges) return
    if (autoSaveTimer.current) clearTimeout(autoSaveTimer.current)
    autoSaveTimer.current = setTimeout(async () => {
      try {
        await saveGatewayConfig(config)
        setSavedConfigSnapshot(buildGatewayConfigSnapshot(config))
        if (status.running) {
          setRestarting(true)
          await stopGateway()
          const st = await startGateway(config)
          applyStatusFromResponse(st, config)
        }
      } catch (e) {
        pushError(e)
      } finally {
        setRestarting(false)
      }
    }, 1500)
    return () => { if (autoSaveTimer.current) clearTimeout(autoSaveTimer.current) }
  }, [configSnapshot])

  const effectiveConfig = useMemo(
    () => (status.running && status.runtimeConfig ? status.runtimeConfig : config),
    [status.running, status.runtimeConfig, config]
  )
  const effectiveBaseUrl = useMemo(
    () => buildGatewayBaseUrl(effectiveConfig.host, effectiveConfig.port, effectiveConfig.localOnly),
    [effectiveConfig.host, effectiveConfig.port, effectiveConfig.localOnly]
  )
  const effectiveSecuritySummary = useMemo(
    () => buildGatewaySecuritySummary({ config: effectiveConfig, t }),
    [effectiveConfig, t]
  )
  const effectiveRoutingSummary = useMemo(() => buildGatewayRoutingSummary({
    config: effectiveConfig,
    counts: { accounts: accounts.length, groups: groups.length },
    selectedLabels: {
      single: accountOptions.find(item => item.value === effectiveConfig.accountId)?.label,
      group: groupOptions.find(item => item.value === effectiveConfig.groupId)?.label
    },
    t
  }), [effectiveConfig, accounts.length, groups.length, accountOptions, groupOptions, t])

  const clientRecipes = useMemo(
    () => buildGatewayClientRecipes({
      baseUrl: effectiveBaseUrl,
      samples: buildClientSamples(effectiveBaseUrl),
      t,
    }),
    [effectiveBaseUrl, t]
  )

  const actionSummary = useMemo(
    () => buildGatewayActionSummary({ running: status.running, isDirty: hasUnsavedChanges, hasUnsavedChanges, hasRuntimeChanges, hasFieldErrors, t }),
    [status.running, hasUnsavedChanges, hasRuntimeChanges, hasFieldErrors]
  )

  const latestErrorEntry = errorHistory[0] || null

  // 账号池能不能取到号：LoadBalancer 按 enabled 过滤（封禁也写回 enabled=false），
  // 用全库数量判断会让「全部禁用」这个最需要告警的状态反而不报警。
  const hasUsableAccounts = accounts.some(account => account.enabled !== false)

  // 状态条提示：只在「配置有错」或「改了但未生效」时出现，正常态不占位
  const statusHint = hasFieldErrors
    ? actionSummary.title
    : (status.running && hasRuntimeChanges ? actionSummary.title : null)

  const handleStart = async () => {
    if (hasFieldErrors) {
      pushError(t('gateway.fixFormErrors'))
      return
    }
    setSaving(true)
    try {
      const st = await startGateway(config)
      applyStatusFromResponse(st, config)
      await refreshStats()
    } catch (e) {
      pushError(e)
    } finally {
      setSaving(false)
    }
  }

  const handleStop = async () => {
    setSaving(true)
    try {
      await stopGateway()
      setStatus(prev => ({ ...prev, running: false }))
      setAppliedRuntimeSnapshot(null)
    } catch (e) {
      pushError(e)
    } finally {
      setSaving(false)
    }
  }

  const handleRestart = async () => {
    if (hasFieldErrors) {
      pushError(t('gateway.fixFormErrors'))
      return
    }
    setSaving(true)
    setRestarting(true)
    try {
      if (status.running) {
        await stopGateway()
      }
      const st = await startGateway(config)
      applyStatusFromResponse(st, config)
      await refreshStats()
    } catch (e) {
      pushError(e)
    } finally {
      setSaving(false)
      setRestarting(false)
    }
  }

  const handleAutoStartToggle = async (checked: boolean) => {
    const nextConfig = { ...config, enabled: checked }
    setConfig(nextConfig)
    try {
      await saveGatewayConfig(nextConfig)
      setSavedConfigSnapshot(buildGatewayConfigSnapshot(nextConfig))
      if (checked && !hasFieldErrors) {
        const st = await startGateway(nextConfig)
        applyStatusFromResponse(st, nextConfig)
      } else if (!checked && status.running) {
        await stopGateway()
        setStatus(prev => ({ ...prev, running: false }))
        setAppliedRuntimeSnapshot(null)
      }
    } catch (e) {
      pushError(e)
    }
  }

  // 静默保存（弹窗关闭时用，不校验、不重启）
  const handleSilentSave = async () => {
    try {
      await saveGatewayConfig(config)
      setSavedConfigSnapshot(buildGatewayConfigSnapshot(config))
    } catch (e) {
      pushError(e)
    }
  }

  const copyText = async (text: string, message: string) => {
    try {
      await navigator.clipboard.writeText(text)
      showToast(message)
    } catch (e) {
      pushError(e)
    }
  }

  return (
    <div className="flex h-full flex-col max-w-full overflow-hidden surface-main">
      {/* 页头：只承担「这是什么」，动作全部下沉到状态条 */}
      <div className="flex items-center gap-3 px-5 py-3">
        <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary/80 to-primary flex items-center justify-center shadow-md ring-1 ring-primary/20 flex-shrink-0">
          <Network size={20} className="text-primary-foreground" />
        </div>
        <div className="flex flex-col min-w-0">
          <h1 className="text-lg font-semibold text-foreground leading-tight">{t('nav.gateway')}</h1>
          <p className="text-sm text-muted-foreground leading-tight truncate">{t('gateway.gatewayDescription')}</p>
        </div>
      </div>

      <GatewayStatusBar
        running={status.running}
        baseUrl={effectiveBaseUrl}
        clientKey={CLIENT_API_KEY}
        stats={stats}
        latestError={latestErrorEntry}
        hint={statusHint}
        busy={saving || loading}
        restarting={restarting}
        onStart={handleStart}
        onStop={handleStop}
        onRestart={handleRestart}
        onOpenLogs={() => setActiveTab('logs')}
        onCopy={copyText}
      />

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <div className="px-5 pt-3 pb-1">
          <TabsList className="surface-card flex h-10 w-full justify-start rounded-lg border-none p-0.5 lg:w-fit">
            <TabsTrigger value="overview" className="gap-1.5 px-4 h-9 text-sm font-medium data-[active]:shadow-sm">
              {t('gateway.tabOverview')}
            </TabsTrigger>
            <TabsTrigger value="logs" className="gap-1.5 px-4 h-9 text-sm font-medium data-[active]:shadow-sm">
              {t('gateway.tabRequestLogs')}
            </TabsTrigger>
            <TabsTrigger value="masking" className="gap-1.5 px-4 h-9 text-sm font-medium data-[active]:shadow-sm">
              {t('gatewayMasking.tabMasking')}
            </TabsTrigger>
            <TabsTrigger value="settings" className="gap-1.5 px-4 h-9 text-sm font-medium data-[active]:shadow-sm">
              {t('gateway.tabSettings')}
              {hasFieldErrors && (
                <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-red-500/15 text-red-500">
                  {Object.keys(fieldErrors).length}
                </span>
              )}
            </TabsTrigger>
          </TabsList>
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto px-5 pb-5 pt-2">
          <TabsContent value="overview" className="m-0">
            <GatewayOverview
              recipes={clientRecipes}
              onCopy={copyText}
              recentLogs={recentLogs}
              hasUsableAccountPool={hasUsableAccounts}
              hasNoAccountsAtAll={accounts.length === 0}
              onOpenLogs={() => setActiveTab('logs')}
              onOpenProtocolTest={() => setShowProtocolTest(true)}
            />
          </TabsContent>

          <TabsContent value="logs" className="m-0 h-full">
            <RequestLogsDialog
              embedded
              open
              onOpenChange={() => {}}
              logLevel={config.logLevel}
              onLogLevelChange={(v) => setField('logLevel', v)}
              logRequests={config.logRequests}
              onLogRequestsChange={(enabled) => setField('logRequests', enabled)}
            />
          </TabsContent>

          <TabsContent value="masking" className="m-0">
            <MaskingPanel />
          </TabsContent>


          <TabsContent value="settings" className="m-0">
            <GatewayConfigComponent
              config={config}
              fieldErrors={fieldErrors}
              setField={setField}
              accountOptions={accountOptions}
              groupOptions={groupOptions}
              setConfig={setConfig}
              applyGatewayLocalOnlyChange={applyGatewayLocalOnlyChange}
              handleSaveConfig={handleSilentSave}
              handleAutoStartToggle={handleAutoStartToggle}
            />
          </TabsContent>
        </div>
      </Tabs>

      <ProtocolTestDialog
        open={showProtocolTest}
        onOpenChange={setShowProtocolTest}
        accounts={accounts}
        gatewayBaseUrl={effectiveBaseUrl}
        gatewayApiKey={CLIENT_API_KEY}
        routing={{
          accountMode: effectiveConfig.accountMode,
          accountId: effectiveConfig.accountId,
          groupId: effectiveConfig.groupId,
          poolAccountIds: effectiveConfig.poolAccountIds,
        }}
      />

      {/* 复制成功提示：原先 copySuccess 被 set 却无人渲染 */}
      {toast && (
        <div className="pointer-events-none fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded-lg border border-border bg-popover px-3 py-1.5 text-xs text-foreground shadow-lg">
          {toast}
        </div>
      )}
    </div>
  )
}

export default GatewayPage
