const ALLOWED_REGIONS = [
  'us-east-1',
  'eu-central-1',
  'us-west-2',
  'ap-northeast-1',
  'ap-southeast-1',
  'us-gov-west-1',
]

export const parseAllowedIps = (value) => String(value || '')
  .split(/[\n,]+/)
  .map(item => item.trim())
  .filter(Boolean)

const isValidIpv4Address = (value) => {
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

const isValidIpv6Address = (value) => {
  try {
    const parsed = new URL(`http://[${value}]/`)
    return parsed.hostname === `[${value}]`
  } catch {
    return false
  }
}

const isValidAllowlistEntry = (value) => {
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

export const applyGatewayLocalOnlyChange = (config, nextLocalOnly, createApiKey) => {
  const nextConfig = {
    ...config,
    localOnly: !!nextLocalOnly,
  }

  if (!nextLocalOnly && !String(config?.apiKey || '').trim()) {
    return {
      ...nextConfig,
      apiKey: createApiKey(),
    }
  }

  return nextConfig
}

export const formatGatewayTimestamp = (date = new Date()) => {
  const pad = (value) => String(value).padStart(2, '0')

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

export const createGatewayFieldErrors = (config) => {
  const errors = {}
  const host = String(config?.host || '').trim()
  const port = Number(config?.port)
  const region = String(config?.region || '').trim()
  const accountMode = String(config?.accountMode || 'single').trim()
  const allowedIps = parseAllowedIps(config?.allowedIpsText)

  if (!host) {
    errors.host = '监听地址不能为空'
  }

  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    errors.port = '端口必须在 1-65535 之间'
  }

  if (!region) {
    errors.region = 'region 不能为空'
  } else if (!ALLOWED_REGIONS.includes(region)) {
    errors.region = `region 不受支持: ${region}`
  }

  if (!['single', 'group'].includes(accountMode)) {
    errors.accountMode = 'accountMode 必须是 single/group'
  } else if (accountMode === 'single' && !String(config?.accountId || '').trim()) {
    errors.accountId = 'single 模式必须选择账号'
  } else if (accountMode === 'group' && !String(config?.groupId || '').trim()) {
    errors.groupId = 'group 模式必须选择分组'
  }

  if (!String(config?.apiKey || '').trim()) {
    errors.apiKey = '必须填写客户端 API Key'
  }

  const invalidAllowlistEntry = allowedIps.find(entry => !isValidAllowlistEntry(entry))
  if (invalidAllowlistEntry) {
    errors.allowedIpsText = `白名单条目无效: ${invalidAllowlistEntry}`
  }

  return errors
}

export const mergeErrorHistory = (history, message, seenAt, limit = 8) => {
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
      lastSeenAt: seenAt,
    }
    return next
  }

  return [
    {
      message: normalizedMessage,
      firstSeenAt: seenAt,
      lastSeenAt: seenAt,
      count: 1,
    },
    ...history,
  ].slice(0, limit)
}

export const buildClientSamples = (baseUrl, apiKey) => {
  const safeKey = apiKey || 'sk-your-gateway-api-key'
  const anthropicEnv = `ANTHROPIC_BASE_URL=${baseUrl}\nANTHROPIC_API_KEY=${safeKey}`
  const openaiEnv = `OPENAI_BASE_URL=${baseUrl}\nOPENAI_API_KEY=${safeKey}`
  const openaiCurl = [
    `curl ${baseUrl}/v1/responses \\`,
    '  -H "Content-Type: application/json" \\',
    `  -H "Authorization: Bearer ${safeKey}" \\`,
    '  -d "{\\"model\\":\\"gpt-4.1\\",\\"input\\":[{\\"role\\":\\"user\\",\\"content\\":[{\\"type\\":\\"input_text\\",\\"text\\":\\"hello\\"}]}]}"',
  ].join('\n')

  return {
    anthropic: {
      env: anthropicEnv,
    },
    openai: {
      env: openaiEnv,
      curl: openaiCurl,
    },
  }
}

export const formatGatewayAccountOptionLabel = (account) => {
  const label = String(account?.label || '').trim()
  const email = String(account?.email || '').trim()
  const userId = String(account?.userId || account?.user_id || '').trim()
  const status = String(account?.status || 'unknown').trim()
  const id = String(account?.id || '').trim()
  const primary = label || email || userId || id || '未知账号'
  const secondary = [email, userId]
    .filter(Boolean)
    .filter(value => value !== primary)
    .join(' / ')
  const shortId = id ? id.slice(0, 8) : ''
  const parts = [primary]

  if (secondary) {
    parts.push(secondary)
  }
  if (shortId) {
    parts.push(`ID:${shortId}`)
  }
  parts.push(status)

  return parts.join(' | ')
}

export const buildGatewayStatusSummary = ({ config, status, errorHistory, lastStatusSyncAt }) => {
  const mode = config?.accountMode || 'single'
  const strategy = config?.strategy || 'round_robin'
  const listenHost = (status?.running ? status?.host : null) || config?.host || status?.host || '127.0.0.1'
  const listenPort = (status?.running ? status?.port : null) || config?.port || status?.port || 8765
  const errorCount = Array.isArray(errorHistory) ? errorHistory.length : 0
  const errorHits = Array.isArray(errorHistory) ? errorHistory.reduce((sum, item) => sum + Number(item.count || 0), 0) : 0

  return {
    listen: `${listenHost}:${listenPort}`,
    requests: String(status?.requestCount || 0),
    region: config?.region || 'us-east-1',
    logLevel: config?.logLevel || 'debug',
    sync: lastStatusSyncAt || '-',
    routing: `${mode} / ${strategy}`,
    exposure: config?.localOnly ? '仅本机' : '允许远程',
    errorCount: `${errorCount} 条 / ${errorHits} 次`,
  }
}

export const buildGatewayRoutingSummary = ({ config, counts, selectedLabels = {} }) => {
  const mode = config?.accountMode || 'single'
  const inventorySummary = `账号 ${counts?.accounts || 0} 个 / 分组 ${counts?.groups || 0} 个`

  if (mode === 'single') {
    return {
      modeLabel: '指定单账号',
      modeDescription: '网关会固定使用一个账号，适合调试或绑定到单一租户场景。',
      selectionLabel: '当前账号',
      selectionValue: selectedLabels.single || '未选择账号',
      inventorySummary,
      strategySummary: '固定账号，不参与轮换',
    }
  }

  if (mode === 'group') {
    return {
      modeLabel: '按分组账号池',
      modeDescription: '先锁定账号分组，再按策略和阈值从该分组中挑选可用账号。',
      selectionLabel: '当前分组',
      selectionValue: selectedLabels.group || '未选择分组',
      inventorySummary,
      strategySummary: `策略 ${config?.strategy || 'round_robin'} / 阈值 ${Number(config?.threshold) || 90}%`,
    }
  }

  return {
    modeLabel: '按分组账号池',
    modeDescription: '先锁定账号分组，再按策略和阈值从该分组中挑选可用账号。',
    selectionLabel: '当前分组',
    selectionValue: selectedLabels.group || '未选择分组',
    inventorySummary,
    strategySummary: `策略 ${config?.strategy || 'round_robin'} / 阈值 ${Number(config?.threshold) || 90}%`,
  }
}

export const buildGatewayActionSummary = ({
  running,
  isDirty,
  hasUnsavedChanges,
  hasRuntimeChanges,
  hasFieldErrors,
}) => {
  const unsavedChanges = hasUnsavedChanges ?? isDirty ?? false
  const runtimeChanges = hasRuntimeChanges ?? (running && unsavedChanges)

  if (hasFieldErrors) {
    return {
      tone: 'red',
      title: '先修正配置错误',
      description: '当前表单存在无效配置，保存、启动和重启都会被拦截，先修正标红字段。',
    }
  }

  if (running && unsavedChanges && runtimeChanges) {
    return {
      tone: 'yellow',
      title: '配置已变更，重启后生效',
      description: '网关仍按已启动时的配置运行。先保存，再执行重启网关，才能让新配置生效。',
    }
  }

  if (running && unsavedChanges) {
    return {
      tone: 'blue',
      title: '当前运行配置尚未保存',
      description: '当前页面配置已经用于运行网关，但还没有写回配置文件；如需保留下次启动沿用，请保存配置。',
    }
  }

  if (running) {
    return {
      tone: 'teal',
      title: '网关运行中',
      description: '当前配置与已保存状态一致；如需中断流量可直接停止网关。',
    }
  }

  if (unsavedChanges) {
    return {
      tone: 'blue',
      title: '可按当前配置直接启动',
      description: '启动网关会使用当前表单里的配置；如果希望下次应用启动也沿用这些设置，先点保存配置。',
    }
  }

  return {
    tone: 'blue',
    title: '网关当前未启动',
    description: '可以直接启动现有配置，或先调整表单后再启动。',
  }
}

export const buildGatewaySecuritySummary = ({ config }) => {
  const allowedIpsCount = parseAllowedIps(config?.allowedIpsText).length

  return {
    exposureLabel: config?.localOnly ? '仅本机访问' : '允许远程访问',
    allowedIpsCount,
    apiKeyState: String(config?.apiKey || '').trim() ? '已配置客户端 Key' : '未配置客户端 Key',
    logLevel: config?.logLevel || 'debug',
  }
}

export const buildGatewayIntegrationSummary = ({ baseUrl, apiKey, logDir, errorHistory }) => {
  const safeKey = apiKey || 'sk-your-gateway-api-key'
  const errorCount = Array.isArray(errorHistory) ? errorHistory.length : 0
  const errorHits = Array.isArray(errorHistory) ? errorHistory.reduce((sum, item) => sum + Number(item.count || 0), 0) : 0

  return {
    endpointLabel: baseUrl,
    authLabel: `Bearer ${safeKey}`,
    logDirState: String(logDir || '').trim() ? '日志目录已定位' : '日志目录未获取',
    errorDigest: `${errorCount} 条错误 / ${errorHits} 次命中`,
  }
}

export const formatGatewayRequestDuration = (durationMs) => {
  const duration = Number(durationMs) || 0
  if (duration < 1000) {
    return `${duration} ms`
  }
  return `${(duration / 1000).toFixed(duration >= 10_000 ? 0 : 2)} s`
}

export const getGatewayRequestOutcomeColor = (outcome) => {
  if (outcome === 'success') return 'teal'
  if (outcome === 'stream') return 'blue'
  if (outcome === 'error') return 'red'
  return 'gray'
}

export const buildGatewayRequestLogSummary = (entries) => {
  const logs = Array.isArray(entries) ? entries : []
  const errors = logs.filter(item => item?.outcome === 'error').length
  const streaming = logs.filter(item => item?.outcome === 'stream').length
  const success = logs.filter(item => item?.outcome === 'success').length
  const maxDuration = logs.reduce((max, item) => Math.max(max, Number(item?.durationMs || 0)), 0)
  const latestOccurredAt = logs[0]?.occurredAt || '-'

  return {
    total: logs.length,
    errors,
    streaming,
    success,
    maxDurationLabel: formatGatewayRequestDuration(maxDuration),
    latestOccurredAt,
  }
}

const buildTopEntries = (values, limit = 5) => Object.entries(values)
  .sort((left, right) => {
    if (right[1] !== left[1]) {
      return right[1] - left[1]
    }
    return left[0].localeCompare(right[0], 'zh-CN')
  })
  .slice(0, limit)
  .map(([label, count]) => ({ label, count }))

export const buildGatewayMetricsSummary = (entries) => {
  const logs = Array.isArray(entries) ? entries : []
  const total = logs.length
  const durations = logs.map(item => Number(item?.durationMs || 0))
  const totalDuration = durations.reduce((sum, value) => sum + value, 0)
  const avgDuration = total ? Math.round(totalDuration / total) : 0
  const outcomeCounts = { success: 0, stream: 0, error: 0, other: 0 }
  const modelCounts = {}
  const upstreamCounts = {}
  const statusCounts = {}
  const endpointCounts = {}
  const regionCounts = {}

  logs.forEach(item => {
    const outcome = String(item?.outcome || 'other')
    if (outcomeCounts[outcome] !== undefined) {
      outcomeCounts[outcome] += 1
    } else {
      outcomeCounts.other += 1
    }

    const model = String(item?.model || '未记录模型').trim() || '未记录模型'
    modelCounts[model] = (modelCounts[model] || 0) + 1

    const upstream = String(item?.upstreamSource || '未解析上游来源').trim() || '未解析上游来源'
    upstreamCounts[upstream] = (upstreamCounts[upstream] || 0) + 1

    const status = String(item?.statusCode || 0)
    statusCounts[status] = (statusCounts[status] || 0) + 1

    const endpoint = String(item?.endpoint || '-')
    endpointCounts[endpoint] = (endpointCounts[endpoint] || 0) + 1

    const region = String(item?.region || '-')
    regionCounts[region] = (regionCounts[region] || 0) + 1
  })

  return {
    total,
    avgDurationLabel: formatGatewayRequestDuration(avgDuration),
    successRateLabel: total ? `${Math.round((outcomeCounts.success / total) * 100)}%` : '0%',
    errorRateLabel: total ? `${Math.round((outcomeCounts.error / total) * 100)}%` : '0%',
    uniqueModels: Object.keys(modelCounts).length,
    uniqueUpstreams: Object.keys(upstreamCounts).length,
    topModels: buildTopEntries(modelCounts),
    topUpstreams: buildTopEntries(upstreamCounts),
    topStatuses: buildTopEntries(statusCounts),
    topEndpoints: buildTopEntries(endpointCounts),
    topRegions: buildTopEntries(regionCounts),
  }
}

const stringifyGatewayRequestLog = (entry) => {
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

export const filterGatewayRequestLogs = (entries, { outcome = 'all', query = '' } = {}) => {
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
