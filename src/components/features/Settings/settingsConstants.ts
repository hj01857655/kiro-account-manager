import { THEME_REGISTRY } from '@/lib/themeRegistry'

export const AI_MODELS = [
  { value: 'claude-opus-5.5', label: 'Claude Opus 5.5 (1M) - 2.0x', recommended: false },
  { value: 'claude-opus-5', label: 'Claude Opus 5 (1M) - 2.2x', recommended: false },
  { value: 'claude-opus-4.8', label: 'Claude Opus 4.8 (1M) - 2.2x', recommended: false },
  { value: 'claude-opus-4.7', label: 'Claude Opus 4.7 (1M) - 2.2x', recommended: false },
  { value: 'claude-opus-4.6', label: 'Claude Opus 4.6 (1M) - 2.2x', recommended: false },
  { value: 'claude-opus-4.5', label: 'Claude Opus 4.5 (200K) - 2.2x', recommended: false },
  { value: 'claude-sonnet-5.5', label: 'Claude Sonnet 5.5 (1M) - 1.3x', recommended: false },
  { value: 'claude-sonnet-5', label: 'Claude Sonnet 5 (1M) - 1.3x', recommended: true },
  { value: 'claude-sonnet-4.6', label: 'Claude Sonnet 4.6 (1M) - 1.3x', recommended: false },
  { value: 'claude-sonnet-4.5', label: 'Claude Sonnet 4.5 (200K) - 1.3x', recommended: false },
  // claude-sonnet-4（Sonnet 4.0）已于 2026-10-14 EOL 并从 Kiro ListAvailableModels 摘除，不再提供
  // Claude Fable 5.1 为 Enterprise 预览（仅 US East），普通套餐不可见，按并集列出
  { value: 'claude-fable-5', label: 'Claude Fable 5.1 (1M) - 6x', recommended: false },
  { value: 'auto', label: 'Auto (智能选择) - 1.0x', recommended: false },
  // GPT-5.6 已升级至 1M 上下文（原 272K），改为两档计费：≤272K 按下列短上下文费率，>272K 翻倍
  { value: 'gpt-5.6-sol', label: 'GPT-5.6 Sol (1M) - 4.4x/8.8x', recommended: false },
  { value: 'gpt-5.6-terra', label: 'GPT-5.6 Terra (1M) - 2.2x/4.4x', recommended: false },
  { value: 'gpt-5.6-luna', label: 'GPT-5.6 Luna (1M) - 1.1x/2.2x', recommended: false },
  { value: 'glm-5', label: 'GLM-5 (200K) - 0.5x', recommended: false },
  { value: 'claude-haiku-4.5', label: 'Claude Haiku 4.5 (200K) - 0.4x', recommended: false },
  { value: 'deepseek-3.2', label: 'DeepSeek 3.2 (128K) - 0.25x', recommended: false },
  { value: 'minimax-m2.5', label: 'MiniMax M2.5 (200K) - 0.25x', recommended: false },
  { value: 'minimax-m2.1', label: 'MiniMax M2.1 (200K) - 0.15x', recommended: false },
  { value: 'kimi-k3', label: 'Kimi K3 (1M) - 2.4x', recommended: false },
  { value: 'qwen3-coder-next', label: 'Qwen3 Coder Next (256K) - 0.05x', recommended: false },
]

// Kiro IDE settings.json 通知键 -> app-settings.json 字段名
export const NOTIFICATION_SETTINGS_FIELD_MAP = {
  'kiroAgent.notifications.agent.actionRequired': 'notifyActionRequired',
  'kiroAgent.notifications.agent.failure': 'notifyFailure',
  'kiroAgent.notifications.agent.success': 'notifySuccess',
  'kiroAgent.notifications.billing': 'notifyBilling'
}

// ---- 通知 / 遥测开关（同属 Kiro IDE settings.json，已合并进 Kiro 设置页）----

export interface NotificationState {
  notifyActionRequired: boolean
  notifyFailure: boolean
  notifySuccess: boolean
  notifyBilling: boolean
}

export interface TelemetryState {
  telemetryContentCollection: boolean
  telemetryUsageAnalytics: boolean
  telemetryEditStats: boolean
  telemetryFeedback: boolean
  telemetryPromptLogging: boolean
  telemetryEditStatsDetails: boolean
  telemetryEditStatsDecorations: boolean
  telemetryEditStatsStatusBar: boolean
}

// 默认值对齐 Kiro 1.0：notify.failure / notify.success 默认 false
export const DEFAULT_NOTIFICATIONS: NotificationState = {
  notifyActionRequired: true,
  notifyFailure: false,
  notifySuccess: false,
  notifyBilling: true,
}

export const DEFAULT_TELEMETRY: TelemetryState = {
  telemetryContentCollection: false,
  telemetryUsageAnalytics: false,
  telemetryEditStats: false,
  telemetryFeedback: false,
  telemetryPromptLogging: false,
  telemetryEditStatsDetails: false,
  telemetryEditStatsDecorations: false,
  telemetryEditStatsStatusBar: false,
}

// key = Kiro IDE settings.json 中的通知键；field = 本地 state 字段
export const NOTIFICATION_ROWS: { key: string; label: string; field: keyof NotificationState }[] = [
  { key: 'kiroAgent.notifications.agent.actionRequired', label: 'settings.notifyActionRequired', field: 'notifyActionRequired' },
  { key: 'kiroAgent.notifications.agent.failure', label: 'settings.notifyFailure', field: 'notifyFailure' },
  { key: 'kiroAgent.notifications.agent.success', label: 'settings.notifySuccess', field: 'notifySuccess' },
  { key: 'kiroAgent.notifications.billing', label: 'settings.notifyBilling', field: 'notifyBilling' },
]

// ideKey = Kiro IDE settings.json 中的遥测键；field = 本地 state 字段（与 app-settings 同名，可直接映射）
export const TELEMETRY_ROWS: { ideKey: string; label: string; field: keyof TelemetryState }[] = [
  { ideKey: 'telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement', label: 'settings.telemetryContentCollection', field: 'telemetryContentCollection' },
  { ideKey: 'telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics', label: 'settings.telemetryUsageAnalytics', field: 'telemetryUsageAnalytics' },
  { ideKey: 'telemetry.editStats.enabled', label: 'settings.telemetryEditStats', field: 'telemetryEditStats' },
  { ideKey: 'telemetry.feedback.enabled', label: 'settings.telemetryFeedback', field: 'telemetryFeedback' },
  { ideKey: 'telemetry.dataSharingAndPromptLogging.promptLogging', label: 'settings.telemetryPromptLogging', field: 'telemetryPromptLogging' },
  { ideKey: 'telemetry.editStats.details.enabled', label: 'settings.telemetryEditStatsDetails', field: 'telemetryEditStatsDetails' },
  { ideKey: 'telemetry.editStats.showDecorations', label: 'settings.telemetryEditStatsDecorations', field: 'telemetryEditStatsDecorations' },
  { ideKey: 'telemetry.editStats.showStatusBar', label: 'settings.telemetryEditStatsStatusBar', field: 'telemetryEditStatsStatusBar' },
]

// 主题清单统一来自 @/lib/themeRegistry，保证与 index.css 的 [data-theme=...] 一一对应
export const buildThemeOptions = (t: (key: string) => string) =>
  THEME_REGISTRY.map((opt) => ({
    key: opt.key,
    name: t(opt.nameKey) || opt.fallbackName,
    iconName: opt.iconName,
    color: opt.color,
  }))
