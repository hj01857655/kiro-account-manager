// 网关（反代）相关 API 调用
import { invoke } from '@tauri-apps/api/core'

/**
 * 与后端 `gateway::GatewayConfig`（serde camelCase）对应的载荷形状。
 * 类型真相在 `gatewayPageState.ts` 的页面状态层，这里只收口写路径，
 * 读路径（getGatewayConfig/getGatewayStatus）仍返回后端原始结构，由 hydrate 处理。
 */
export interface GatewayConfigPayload {
  enabled: boolean
  host: string
  port: number
  region: string
  accountMode: string
  accountId: string | null
  groupId: string | null
  poolAccountIds: string[]
  strategy: string
  threshold: number
  localOnly: boolean
  allowedIps: string[]
  logLevel: string
  filterClaudeCode: boolean
  filterStripBoundaries: boolean
  filterEnvNoise: boolean
  promptFilterRules: Array<{
    id: string
    name: string
    enabled: boolean
    ruleType: string
    matchPattern: string
    replace: string
  }>
  logRequests: boolean
  responseCacheEnabled: boolean
  responseCacheTtl: number
  webSearchEnabled: boolean
  /** Tool Search（Anthropic defer_loading 工具发现规范）；关闭后 defer_loading 被忽略，全量下发 */
  toolSearchEnabled: boolean
  /** 网关上游（Kiro API）显式代理；空 = 走账号/应用代理链 */
  upstreamProxy: string | null
  /** 工具描述最大长度；0 = 不截断 */
  maxToolDescriptionLength: number
}

export function getGatewayConfig<T = any>() {
  return invoke<T>('get_gateway_config')
}

export function getGatewayStatus<T = any>() {
  return invoke<T>('get_gateway_status')
}

export function saveGatewayConfig(config: GatewayConfigPayload) {
  return invoke('save_gateway_config', { config })
}

export function startGateway<T = any>(config: GatewayConfigPayload) {
  return invoke<T>('start_gateway', { config })
}

export function stopGateway() {
  return invoke('stop_gateway')
}

export function getGatewayLogDir() {
  return invoke<string>('get_gateway_log_dir')
}

export function openGatewayLogDir() {
  return invoke<string>('open_gateway_log_dir')
}

export function getGatewayRequestLogs<T = any[]>(limit = 120) {
  return invoke<T>('get_gateway_request_logs', { limit })
}

export function clearGatewayRequestLogs() {
  return invoke('clear_gateway_request_logs')
}

export function getGatewayRequestStats<T = any>() {
  return invoke<T>('get_gateway_request_stats')
}

// 按模型 / 端点维度聚合的统计（与 request_stats 同源，均读 LogStore）
export function getGatewayModelStats<T = any[]>() {
  return invoke<T>('get_gateway_model_stats')
}

export function getGatewayEndpointStats<T = any[]>() {
  return invoke<T>('get_gateway_endpoint_stats')
}

/** 按客户端 UA 产品名分组的用量（Key 固定为 kiro 后不再有 Key 维度），回答「谁在用这个网关」 */
export function getGatewayClientStats<T = any[]>() {
  return invoke<T>('get_gateway_client_stats')
}

export function getCacheStats<T = any>() {
  return invoke<T>('get_cache_stats')
}

export function clearAllCache() {
  return invoke('clear_all_cache')
}

/** 清理过期缓存，返回清理条数 */
export function cleanupExpiredCache() {
  return invoke<number>('cleanup_expired_cache')
}

/** 清除指定会话的缓存 */
export function clearSessionCache(sessionId: string) {
  return invoke('clear_session_cache', { sessionId })
}

/**
 * 网关运行时账号健康度。
 * 键为账号 ID；网关未启动时后端返回错误，调用方应视为「未运行」而非真错误。
 */
export function getAllAccountHealth<T = Record<string, any>>() {
  return invoke<T>('get_all_account_health')
}

/** 重置单个账号的健康状态 */
export function resetAccountHealth(accountId: string) {
  return invoke('reset_account_health', { accountId })
}

/** 清理超过 1 小时未检查的健康状态条目 */
export function cleanupStaleHealth() {
  return invoke('cleanup_stale_health')
}

/**
 * 当前被限流的账号 ID 列表（负载均衡器滑动窗口内的限流标记）。
 * 网关未启动时后端返回错误，应与 getAllAccountHealth 一样视为「未运行」。
 */
export function getRateLimitedAccounts() {
  return invoke<string[]>('get_rate_limited_accounts')
}

/** 清除指定账号的限流标记，使其立即重新参与调度 */
export function clearRateLimitAccount(accountId: string) {
  return invoke('clear_rate_limit_account', { accountId })
}

// 获取可用模型列表
export function getAvailableModels() {
  return invoke<string[]>('get_available_models')
}

/** 按当前 kiro2api 配置模拟负载均衡选号（不发起真实上游请求） */
export function testRouteConfig(config: any) {
  return invoke<{
    matched_accounts: string[]
    selected_account: string | null
    error: string | null
  }>('test_route_config', { config })
}
