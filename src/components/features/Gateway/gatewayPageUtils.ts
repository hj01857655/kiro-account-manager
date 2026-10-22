// 与后端 src-tauri/src/clients/http_client.rs::SUPPORTED_KIRO_REGIONS 对齐。
// 改这里时同步改后端那份；GatewayConfig.tsx 的 <SelectItem> 列表也要保持一致。
export const ALLOWED_REGIONS = [
  'us-east-1',
  'us-east-2',
  'us-west-1',
  'us-west-2',
  'eu-west-1',
  'eu-west-2',
  'eu-west-3',
  'eu-central-1',
  'eu-central-2',
  'eu-north-1',
  'eu-south-1',
  'eu-south-2',
  'ap-northeast-1',
  'ap-northeast-2',
  'ap-northeast-3',
  'ap-southeast-1',
  'ap-southeast-2',
  'ap-southeast-3',
  'ap-southeast-4',
  'ap-southeast-5',
  'ap-southeast-7',
  'ap-south-1',
  'ap-south-2',
  'ap-east-1',
  'ca-central-1',
  'ca-west-1',
  'sa-east-1',
  'me-south-1',
  'me-central-1',
  'il-central-1',
  'mx-central-1',
  'af-south-1',
  'us-gov-west-1',
  'us-gov-east-1',
  'cn-north-1',
  'cn-northwest-1',
] as const

export const parseAllowedIps = (value: string | string[]): string[] => {
  if (Array.isArray(value)) return value
  return String(value || '')
    .split(/[\n,]+/)
    .map(item => item.trim())
    .filter(Boolean)
}

/** 客户端鉴权密钥，与后端 `gateway::CLIENT_API_KEY` 一致。
 * 多 Key 管理已移除，前后端固定用这个值。 */
export const CLIENT_API_KEY = 'kiro'

const isValidIpv4Address = (value: string): boolean => {
  if (!/^\d{1,3}(\.\d{1,3}){3}$/.test(value)) {
    return false
  }

  return value
    .split('.')
    .every(part => {
      const number = Number(part)
      return Number.isInteger(number) && number >= 0 && number <= 255
    })
}

const isValidIpv6Address = (value: string): boolean => {
  try {
    const parsed = new URL(`http://[${value}]/`)
    return parsed.hostname === value
  } catch {
    return false
  }
}

const isValidGatewayHost = (value: string): boolean => {
  const host = String(value || '').trim()
  if (!host) {
    return false
  }

  if (host === 'localhost' || host === '0.0.0.0' || host === '::' || host === '::1') {
    return true
  }

  return isValidIpv4Address(host) || isValidIpv6Address(host)
}

const isValidAllowlistEntry = (value: string): boolean => {
  const entry = String(value || '').trim()
  if (!entry) {
    return false
  }

  if (isValidIpv4Address(entry) || isValidIpv6Address(entry)) {
    return true
  }

  const [host, prefix, ...rest] = entry.split('/')
  if (!host || !prefix || rest.length) {
    return false
  }

  if (!/^\d+$/.test(prefix)) {
    return false
  }

  const prefixNumber = Number(prefix)
  if (isValidIpv4Address(host)) {
    return prefixNumber >= 0 && prefixNumber <= 32
  }
  if (isValidIpv6Address(host)) {
    return prefixNumber >= 0 && prefixNumber <= 128
  }

  return false
}

export const buildGatewayConnectHost = (host: string, localOnly: boolean): string => {
  const value = String(host || '').trim()
  if (!value) {
    return '127.0.0.1'
  }
  if (value === '0.0.0.0' || value === '::') {
    return localOnly ? '127.0.0.1' : 'localhost'
  }

  return value
}

export const buildGatewayBaseUrl = (host: string, port: number, localOnly: boolean): string => {
  const connectHost = buildGatewayConnectHost(host, localOnly)
  const needsBrackets = connectHost.includes(':') && !connectHost.startsWith('[')
  const normalizedHost = needsBrackets ? `[${connectHost}]` : connectHost
  return `http://${normalizedHost}:${Number(port) || 8765}`
}

export const applyGatewayLocalOnlyChange = (
  config: any,
  nextLocalOnly: boolean
): any => {
  // host 完全由 localOnly 推导：只监听 127.0.0.1，或监听全部网卡。
  // 监听地址曾经是一个可编辑字段，与这个开关重叠且能互相矛盾，已从设置页移除；
  // 这里同步写 host，保证旧配置里遗留的 0.0.0.0 / 127.0.0.1 不跟开关唱反调。
  return {
    ...config,
    localOnly: !!nextLocalOnly,
    host: nextLocalOnly ? '127.0.0.1' : '0.0.0.0'
  }
}

export const formatGatewayTimestamp = (date = new Date()): string => {
  const pad = (value: number) => String(value).padStart(2, '0')

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

/**
 * 构建表单校验错误。
 *
 * `t` 由调用方传入（见 `index.tsx` 的 `useApp().t`）——
 * 文案不放字面量，否则英文/俄文界面会显示中文。
 */
export const createGatewayFieldErrors = (config: any, t: (key: string, vars?: Record<string, any>) => string) => {
  const errors: any = {}
  const host = String(config?.host || '').trim()
  const port = Number(config?.port)
  const region = String(config?.region || '').trim()
  const accountMode = String(config?.accountMode || 'single').trim()
  const localOnly = config?.localOnly ?? true
  const allowedIps = parseAllowedIps(config?.allowedIpsText)

  if (!host) {
    errors.host = t('gatewaySummary.field.hostEmpty')
  } else if (!isValidGatewayHost(host)) {
    errors.host = t('gatewaySummary.field.hostInvalid')
  }

  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    errors.port = t('gatewaySummary.field.portRange')
  }

  if (!region) {
    errors.region = t('gatewaySummary.field.regionEmpty')
  } else if (!(ALLOWED_REGIONS as readonly string[]).includes(region)) {
    errors.region = t('gatewaySummary.field.regionUnsupported', { region })
  }

  if (!['single', 'group', 'pool'].includes(accountMode)) {
    errors.accountMode = t('gatewaySummary.field.accountMode')
  } else if (accountMode === 'single' && !String(config?.accountId || '').trim()) {
    errors.accountId = t('gatewaySummary.field.accountRequired')
  } else if (accountMode === 'group' && !String(config?.groupId || '').trim()) {
    errors.groupId = t('gatewaySummary.field.groupRequired')
  }

  if (!localOnly && !allowedIps.length) {
    errors.allowedIpsText = t('gatewaySummary.field.allowlistRequired')
  }

  const invalidAllowlistEntry = allowedIps.find(entry => !isValidAllowlistEntry(entry))
  if (invalidAllowlistEntry) {
    errors.allowedIpsText = t('gatewaySummary.field.allowlistInvalid', { entry: invalidAllowlistEntry })
  }

  return errors
}

export interface ErrorHistoryEntry {
  message: string
  firstSeenAt: string
  lastSeenAt: string
  count: number
}

export const mergeErrorHistory = (
  history: ErrorHistoryEntry[],
  message: string,
  seenAt: string,
  limit = 8
): ErrorHistoryEntry[] => {
  const normalizedMessage = String(message || '').trim()
  if (!normalizedMessage) {
    return history
  }

  const existingIndex = history.findIndex(item => item.message === normalizedMessage)
  if (existingIndex >= 0) {
    const next = [...history]
    const existing = next[existingIndex]
    next[existingIndex] = {
      ...existing,
      count: existing.count + 1,
      lastSeenAt: seenAt
    }
    return next
  }

  return [
    {
      message: normalizedMessage,
      firstSeenAt: seenAt,
      lastSeenAt: seenAt,
      count: 1
    },
    ...history,
  ].slice(0, limit)
}

interface ClientSamples {
  /**
   * 展示用（界面渲染）：**掩码**形式，Key 不可直接使用。
   * 拿去剪贴板请用 `copy` 变体。
   */
  anthropic: { env: string }
  openai: { env: string }
  openaiChat: { env: string }
  /**
   * 复制到剪贴板用的连接配置：**含真实 Key**。
   *
   * 认证是逐字比较，掩码形式（`sk-xxx…abcd`）贴进客户端必然 401，
   * 所以「复制配置」必须给真身；界面上给人看的才是掩码版。
   */
  copy: { anthropic: string; openai: string; openaiChat: string }
}

/** 接入引导卡片所需的一切信息。 */
export interface ClientRecipe {
  /** 稳定标识 */
  id: string
  /** 展示名 */
  label: string
  /** 协议标签 */
  protocol: string
  /** 客户端应当填的 Base URL（注意：Codex 类需要带 /v1） */
  baseUrl: string
  /** 该客户端请求的服务端端点，仅作说明 */
  endpoint: string
  /** 鉴权头写法。不同客户端不同，填错必401，故必须显式告知 */
  authHeader: string
  /** 掩码后的 Key（展示用） */
  keyMasked: string
  /** 真实 Key（复制用） */
  key: string
  /** 复制动作给出的连接配置：Base URL + Key 的环境变量写法，用户贴进自己客户端的配置里。 */
  copySample: string
}

/**
 * 生成各客户端的接入配方。
 *
 * 这里是「接一个客户端该填什么」的**唯一真相源**。历史上这些信息散落在
 * index.tsx 的 consoleHighlights、写入预览等六处，各写各的，导致 API key
 * 过滤语义漂移（见 commit 55d6be1）。
 *
 * 关键差异（均有代码依据，不可"拉齐"）：
 * - Base URL：Anthropic 系给 origin，OpenAI 系还要带 `/v1`
 *   （对照 gateway_cmd.rs 的 `format!("{}/v1", proxy_origin)`）
 * - 鉴权头：Anthropic 系用 `x-api-key`，OpenAI 系用 `Authorization: Bearer`
 *   （对照 gateway/proxy/auth.rs 的 verify_client_auth，两者都认）
 */
export const buildGatewayClientRecipes = ({
  baseUrl,
  samples,
  t,
}: {
  baseUrl: string
  samples: ClientSamples
  t: (key: string) => string
}): ClientRecipe[] => {
  const key = CLIENT_API_KEY
  const keyMasked = key
  const origin = String(baseUrl || '').replace(/\/+$/, '')

  return [
    {
      id: 'anthropic',
      label: 'Anthropic Messages',
      protocol: 'Anthropic Messages',
      baseUrl: origin,
      endpoint: '/v1/messages',
      authHeader: 'x-api-key',
      keyMasked,
      key,
      copySample: samples.copy.anthropic,
    },
    {
      id: 'openaiResponses',
      label: 'OpenAI Responses',
      protocol: 'OpenAI Responses',
      baseUrl: `${origin}/v1`,
      endpoint: '/v1/responses',
      authHeader: 'Authorization: Bearer',
      keyMasked,
      key,
      copySample: samples.copy.openai,
    },
    {
      id: 'openaiChat',
      label: 'OpenAI Chat',
      protocol: 'OpenAI Chat',
      baseUrl: `${origin}/v1`,
      endpoint: '/v1/chat/completions',
      authHeader: 'Authorization: Bearer',
      keyMasked,
      key,
      copySample: samples.copy.openai,
    },
  ]
}

export const buildClientSamples = (baseUrl: string): ClientSamples => {
  // 密钥固定为 kiro：不再是秘密，展示与复制同一个值
  const key = CLIENT_API_KEY
  const anthropicEnv = `ANTHROPIC_BASE_URL=${baseUrl}\nANTHROPIC_API_KEY=${key}`
  // OpenAI 系客户端的 Base URL 必须带 /v1（对照 buildGatewayClientRecipes 的 `${origin}/v1`）
  const openaiEnv = `OPENAI_BASE_URL=${baseUrl}/v1\nOPENAI_API_KEY=${key}`

  return {
    anthropic: {
      env: anthropicEnv
    },
    openai: {
      env: openaiEnv
    },
    openaiChat: {
      env: openaiEnv
    },
    copy: {
      anthropic: anthropicEnv,
      openai: openaiEnv,
      openaiChat: openaiEnv
    }
  }
}

import { getQuota, getUsed, formatUsage } from '../../../utils/accountStats'

export const formatGatewayAccountOptionLabel = (
  account: any,
  t: (key: string, vars?: Record<string, any>) => string
): string => {
  const email = String(account?.email || '').trim()
  const userId = String(account?.userId || '').trim()
  const baseLabel = email || userId || t('gatewaySummary.unknownAccount')

  const quota = getQuota(account)
  const used = getUsed(account)
  const remaining = quota - used
  const breakdown = account?.usageData?.usageBreakdownList?.[0]
  const currentOverages = breakdown?.currentOverages ?? 0
  const overageCap = breakdown?.overageCap ?? 0
  const quotaInfo = currentOverages > 0
    ? t('gatewaySummary.account.overage', {
        used: formatUsage(currentOverages),
        cap: overageCap > 0 ? '/' + formatUsage(overageCap) : ''
      })
    : t('gatewaySummary.account.remaining', {
        remaining: formatUsage(Math.max(0, remaining)),
        quota: formatUsage(quota)
      })

  const status = String(account?.status || '').trim()
  const isActive = status === 'active' || status === ''
  const statusInfo = !isActive ? ` [${status}]` : ''

  return `${baseLabel} ${quotaInfo}${statusInfo}`
}

export const buildGatewayStatusSummary = ({ config, status, errorHistory, lastStatusSyncAt, t }: any) => {
  const mode = config?.accountMode || 'single'
  const strategy = config?.strategy || 'round_robin'
  const listenHost = (status?.running ? status?.host : null) || config?.host || status?.host || '127.0.0.1'
  const listenPort = (status?.running ? status?.port : null) || config?.port || status?.port || 8765
  const errorCount = Array.isArray(errorHistory) ? errorHistory.length : 0
  const errorHits = Array.isArray(errorHistory) ? errorHistory.reduce((sum, item) => sum + Number(item.count || 0), 0) : 0
  return {
    listen: `http://${listenHost}:${listenPort}`,
    requests: String(status?.requestCount || 0),
    region: config?.region || 'us-east-1',
    logLevel: config?.logLevel || 'debug',
    sync: lastStatusSyncAt || '-',
    routing: `${mode} / ${strategy}`,
    exposure: config?.localOnly
      ? t('gatewaySummary.exposureLocal')
      : t('gatewaySummary.exposureRemote'),
    errorCount: errorCount
  }
}
export const buildGatewayRoutingSummary = ({ config, counts, selectedLabels = {}, t }: any) => {
  const mode = config?.accountMode || 'single'
  const inventorySummary = t('gatewaySummary.inventory', {
    accounts: counts?.accounts || 0,
    groups: counts?.groups || 0
  })

  if (mode === 'single') {
    return {
      modeLabel: t('gatewaySummary.routing.single'),
      modeDescription: t('gatewaySummary.routing.singleDesc'),
      selectionLabel: t('gatewaySummary.routing.currentAccount'),
      selectionValue: selectedLabels.single || t('gatewaySummary.routing.noAccount'),
      inventorySummary,
      strategySummary: t('gatewaySummary.routing.fixed')
    }
  }

  if (mode === 'group') {
    return {
      modeLabel: t('gatewaySummary.routing.group'),
      modeDescription: t('gatewaySummary.routing.groupDesc'),
      selectionLabel: t('gatewaySummary.routing.currentGroup'),
      selectionValue: selectedLabels.group || t('gatewaySummary.routing.noGroup'),
      inventorySummary,
      strategySummary: t('gatewaySummary.routing.strategyThreshold', {
        strategy: config?.strategy || 'round_robin',
        threshold: Number(config?.threshold) || 90
      })
    }
  }

  if (mode === 'pool') {
    return {
      modeLabel: t('gatewaySummary.routing.pool'),
      modeDescription: t('gatewaySummary.routing.poolDesc'),
      selectionLabel: t('gatewaySummary.routing.accountScope'),
      selectionValue: t('gatewaySummary.routing.allAccounts'),
      inventorySummary,
      strategySummary: t('gatewaySummary.routing.strategyThreshold', {
        strategy: config?.strategy || 'round_robin',
        threshold: Number(config?.threshold) || 90
      })
    }
  }

  return {
    modeLabel: t('gatewaySummary.routing.group'),
    modeDescription: t('gatewaySummary.routing.groupDesc'),
    selectionLabel: t('gatewaySummary.routing.currentGroup'),
    selectionValue: selectedLabels.group || t('gatewaySummary.routing.noGroup'),
    inventorySummary,
    strategySummary: t('gatewaySummary.routing.strategyThreshold', {
      strategy: config?.strategy || 'round_robin',
      threshold: Number(config?.threshold) || 90
    })
  }
}

export const buildGatewayActionSummary = ({
  running,
  isDirty,
  hasUnsavedChanges,
  hasRuntimeChanges,
  hasFieldErrors,
  t }: any) => {
  const unsavedChanges = hasUnsavedChanges ?? isDirty ?? false
  const runtimeChanges = hasRuntimeChanges ?? (running && unsavedChanges)

  if (hasFieldErrors) {
    return {
      tone: 'red',
      title: t('gatewaySummary.action.fixErrors'),
      description: t('gatewaySummary.action.fixErrorsDesc')
    }
  }

  if (running && unsavedChanges && runtimeChanges) {
    return {
      tone: 'yellow',
      title: t('gatewaySummary.action.restartNeeded'),
      description: t('gatewaySummary.action.restartNeededDesc')
    }
  }

  if (running && unsavedChanges) {
    return {
      tone: 'blue',
      title: t('gatewaySummary.action.unsavedRunning'),
      description: t('gatewaySummary.action.unsavedRunningDesc')
    }
  }

  if (running) {
    return {
      tone: 'teal',
      title: t('gatewaySummary.action.running'),
      description: t('gatewaySummary.action.runningDesc')
    }
  }

  if (unsavedChanges) {
    return {
      tone: 'blue',
      title: t('gatewaySummary.action.canStart'),
      description: t('gatewaySummary.action.canStartDesc')
    }
  }

  return {
    tone: 'blue',
    title: t('gatewaySummary.action.stopped'),
    description: t('gatewaySummary.action.stoppedDesc')
  }
}

export const buildGatewaySecuritySummary = ({ config, t }: any) => {
  const allowedIpsCount = parseAllowedIps(config?.allowedIpsText).length

  return {
    exposureLabel: config?.localOnly
      ? t('gatewaySummary.exposureLocalFull')
      : t('gatewaySummary.exposureRemoteFull'),
    allowedIpsCount,
    // 密钥固定为 kiro，不再有启用/禁用之分；文案只说明这一事实
    apiKeyState: t('gatewaySummary.security.keysFixed'),
    logLevel: config?.logLevel || 'debug'
  }
}

export const buildGatewayIntegrationSummary = ({
  baseUrl,
  logDir,
  errorHistory,
  t
}: any) => {
  const errorCount = Array.isArray(errorHistory) ? errorHistory.length : 0
  const errorHits = Array.isArray(errorHistory) ? errorHistory.reduce((sum, item) => sum + Number(item.count || 0), 0) : 0

  return {
    endpointLabel: baseUrl,
    authLabel: `Bearer ${CLIENT_API_KEY}`,
    logDirState: String(logDir || '').trim()
      ? t('gatewaySummary.integration.logDirReady')
      : t('gatewaySummary.integration.logDirMissing'),
    errorDigest: t('gatewaySummary.integration.errorDigest', {
      errors: errorCount,
      hits: errorHits
    })
  }
}

export const formatGatewayRequestDuration = (durationMs: number): string => {
  const duration = Number(durationMs) || 0
  if (duration < 1000) {
    return `${duration} ms`
  }
  return `${(duration / 1000).toFixed(duration >= 10_000 ? 0 : 2)} s`
}

export const getGatewayRequestOutcomeColor = (outcome: string): string => {
  if (outcome === 'success') return 'teal'
  if (outcome === 'stream') return 'blue'
  if (outcome === 'error') return 'red'
  return 'gray'
}

export const buildGatewayRequestLogSummary = (entries: any) => {
  const logs = Array.isArray(entries) ? entries : []
  const errors = logs.filter(item => item?.outcome === 'error').length
  const streaming = logs.filter(item => item?.outcome === 'stream').length
  const success = logs.filter(item => item?.outcome === 'success').length
  const maxDuration = logs.reduce((max, item) => Math.max(max, Number(item?.durationMs || 0)), 0)
  const latestOccurredAt = logs[0]?.occurredAt || '-'

  // Prompt Caching 统计
  let totalInputTokens = 0
  let totalOutputTokens = 0
  let totalCacheReadTokens = 0
  let totalCacheCreationTokens = 0
  let requestsWithCache = 0

  logs.forEach(item => {
    const inputTokens = Number(item?.inputTokens || 0)
    const outputTokens = Number(item?.outputTokens || 0)
    const cacheReadTokens = Number(item?.cacheReadInputTokens || 0)
    const cacheCreationTokens = Number(item?.cacheCreationInputTokens || 0)

    totalInputTokens += inputTokens
    totalOutputTokens += outputTokens
    totalCacheReadTokens += cacheReadTokens
    totalCacheCreationTokens += cacheCreationTokens

    if (cacheReadTokens > 0 || cacheCreationTokens > 0) {
      requestsWithCache++
    }
  })

  const total = logs.length
  const successRateLabel = total > 0 ? `${((success / total) * 100).toFixed(1)}%` : '0%'
  const errorRateLabel = total > 0 ? `${((errors / total) * 100).toFixed(1)}%` : '0%'

  // 计算缓存命中率
  const cacheHitRate = total > 0
    ? Math.round((requestsWithCache / total) * 100)
    : 0

  // 计算节省成本百分比（缓存读取成本是输入成本的 10%）
  const totalCacheableTokens = totalCacheReadTokens + totalCacheCreationTokens
  const costSavings = totalCacheableTokens > 0
    ? Math.round((totalCacheReadTokens / totalCacheableTokens) * 90)
    : 0

  return {
    total,
    errors,
    streaming,
    success,
    successRateLabel,
    errorRateLabel,
    maxDurationLabel: formatGatewayRequestDuration(maxDuration),
    latestOccurredAt,
    // Prompt Caching 统计
    totalInputTokens,
    totalOutputTokens,
    totalCacheReadTokens,
    totalCacheCreationTokens,
    requestsWithCache,
    cacheHitRate: `${cacheHitRate.toFixed(1)}%`,
    costSavings: `${costSavings.toFixed(2)}`
  }
}
interface MetricEntry {
  label: string
  count: number
  percent: string
}

const buildTopEntries = (values: Record<string, number>, total: number, limit = 5): MetricEntry[] =>
  Object.entries(values)
    .sort((left, right) => {
      if (right[1] !== left[1]) {
        return right[1] - left[1]
      }
      return left[0].localeCompare(right[0], 'zh-CN')
    })
    .slice(0, limit)
    .map(([label, count]) => ({
      label,
      count,
      percent: total > 0 ? `${Math.round((count / total) * 100)}%` : '0%'
    }))

export const buildGatewayMetricsSummary = (entries: any) => {
  const logs = Array.isArray(entries) ? entries : []
  const total = logs.length

  if (total === 0) {
    return {
      total: 0,
      avgDurationLabel: '0 ms',
      successRateLabel: '0%',
      errorRateLabel: '0%',
      uniqueModels: 0,
      uniqueUpstreams: 0,
      topModels: [],
      topUpstreams: [],
      topStatuses: [],
      topEndpoints: [],
      topRegions: []
    }
  }

  const outcomeCounts = { success: 0, stream: 0, error: 0, other: 0 }
  const modelCounts: Record<string, number> = {}
  const upstreamCounts: Record<string, number> = {}
  const statusCounts: Record<string, number> = {}
  const endpointCounts: Record<string, number> = {}
  const regionCounts: Record<string, number> = {}
  let totalDuration = 0

  logs.forEach(item => {
    totalDuration += Number(item?.durationMs || 0)

    const outcome = String(item?.outcome || 'other')
    if (outcomeCounts[outcome] !== undefined) {
      outcomeCounts[outcome] += 1
    } else {
      outcomeCounts.other += 1
    }

    const model = String(item?.model || '').trim()
    modelCounts[model] = (modelCounts[model] || 0) + 1

    const upstream = String(item?.upstreamSource || '').trim()
    upstreamCounts[upstream] = (upstreamCounts[upstream] || 0) + 1

    const status = String(item?.statusCode || 0)
    statusCounts[status] = (statusCounts[status] || 0) + 1

    const endpoint = String(item?.endpoint || '-')
    endpointCounts[endpoint] = (endpointCounts[endpoint] || 0) + 1

    const region = String(item?.region || '-')
    regionCounts[region] = (regionCounts[region] || 0) + 1
  })

  const avgDuration = Math.round(totalDuration / total)

  return {
    total,
    avgDurationLabel: formatGatewayRequestDuration(avgDuration),
    successRateLabel: `${Math.round((outcomeCounts.success / total) * 100)}%`,
    errorRateLabel: `${Math.round((outcomeCounts.error / total) * 100)}%`,
    uniqueModels: Object.keys(modelCounts).length,
    uniqueUpstreams: Object.keys(upstreamCounts).length,
    topModels: buildTopEntries(modelCounts, total),
    topUpstreams: buildTopEntries(upstreamCounts, total),
    topStatuses: buildTopEntries(statusCounts, total),
    topEndpoints: buildTopEntries(endpointCounts, total),
    topRegions: buildTopEntries(regionCounts, total)
  }
}

const stringifyGatewayRequestLog = (entry: any): string => {
  if (!entry || typeof entry !== 'object') {
    return ''
  }

  return [
    entry.endpoint,
    entry.outcome,
    entry.model,
    entry.region,
    entry.clientIp,
    entry.upstreamSource,
    entry.error,
    entry.requestBody,
    entry.responseBody,
    entry.statusCode,
    entry.requestIndex,
    entry.occurredAt,
  ]
    .filter(value => value !== undefined && value !== null)
    .join(' ')
    .toLowerCase()
}

interface FilterOptions {
  outcome?: string
  query?: string
}

export const filterGatewayRequestLogs = (entries: any[], options: FilterOptions = {}): any[] => {
  const { outcome = 'all', query = '' } = options
  const logs = Array.isArray(entries) ? entries : []
  const normalizedOutcome = String(outcome || 'all').trim()
  const normalizedQuery = String(query || '').trim().toLowerCase()

  return logs.filter(entry => {
    if (normalizedOutcome !== 'all' && entry?.outcome !== normalizedOutcome) {
      return false
    }

    if (!normalizedQuery) {
      return true
    }

    return stringifyGatewayRequestLog(entry).includes(normalizedQuery)
  })
}

