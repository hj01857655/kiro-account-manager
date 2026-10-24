// 应用设置与 Kiro IDE 配置 API 调用
import { invoke } from '@tauri-apps/api/core'

// ============================================================
// 应用设置
// ============================================================

// 读取应用设置
export function getAppSettings<T = any>() {
  return invoke<T>('get_app_settings')
}

// 保存应用设置（settings 为待更新字段）
export function saveAppSettings(settings: Record<string, any>) {
  return invoke('save_app_settings', { settings })
}

// 应用数据目录
export function getAppDataDir() {
  return invoke<string>('get_app_data_dir')
}

export function openAppDataDir() {
  return invoke('open_app_data_dir')
}

// ============================================================
// Kiro IDE 配置
// ============================================================

// 读取 Kiro IDE 设置
export function getKiroSettings<T = any>() {
  return invoke<T>('get_kiro_settings')
}

// 纯读取云配置开关（kiroAgent.experiments.cloudConfig），不写回 settings.json。
// 用于 AgentConfig 进入时判断是否需要挂只读横幅并禁用写操作。
export function readCloudConfigEnabled() {
  return invoke<boolean>('read_cloud_config_enabled')
}

// 读取会覆盖 kiroAgent.remoteSessions.endpoint 的环境变量
// （KIRO_REMOTE_SESSIONS_ENDPOINT）。该变量优先级高于 settings.json 里的配置项，
// 返回非 null 表示配置项当前不生效。
export function readRemoteSessionsEnvOverride() {
  return invoke<string | null>('read_remote_sessions_env_override')
}

// 用系统默认程序打开 Kiro IDE 的 settings.json（文件不存在时会先创建）
export function openKiroSettingsFile() {
  return invoke('open_kiro_settings_file')
}

// 设置 Kiro IDE 代理
export function setKiroProxy(proxy: string) {
  return invoke('set_kiro_proxy', { proxy })
}

// 设置 Kiro IDE 模型
export function setKiroModel(model: string) {
  return invoke('set_kiro_model', { model })
}

// 权限作用域：global = ~/.kiro/settings/permissions.yaml
//             project = ~/.kiro/workspace-roots/<workspace-id>/permissions.yaml
export type PermissionScope = 'global' | 'project'

// IDE 已建过目录的 workspace-root 条目
export interface WorkspaceRootInfo {
  id: string // 16 位 workspace-id（目录名）
  projectPath: string | null // 来自 .trust-migration.json 的 root；无记录时为 null
  hasPermissions: boolean
}

export interface PermissionScopeArg {
  scope?: PermissionScope
  projectPath?: string
}

// 读取权限策略（可指定作用域）
export function getPermissions<T = any>(arg: PermissionScopeArg = {}) {
  return invoke<T>('get_permissions', {
    scope: arg.scope ?? 'global',
    projectPath: arg.projectPath ?? null,
  })
}

// 覆盖写入权限策略（可指定作用域）
export function savePermissions(
  policy: { rules: any[]; policies?: string[] | null },
  arg: PermissionScopeArg = {},
) {
  return invoke('save_permissions', {
    policy,
    scope: arg.scope ?? 'global',
    projectPath: arg.projectPath ?? null,
  })
}

// 读取 IDE 1.0 已知的能力(capability)名列表，供前端下拉候选
export function getPermissionCapabilities<T = any>() {
  return invoke<T>('get_permission_capabilities')
}

// 列出项目级权限的候选 workspace-root
export function listPermissionWorkspaceRoots<T = WorkspaceRootInfo[]>() {
  return invoke<T>('list_permission_workspace_roots')
}

// 设置 Kiro IDE 通知开关
export function setKiroNotification(key: string, enabled: boolean) {
  return invoke('set_kiro_notification', { key, enabled })
}

// 设置 Kiro IDE 遥测开关
export function setKiroTelemetry(key: string, enabled: boolean) {
  return invoke('set_kiro_telemetry', { key, enabled })
}

// 设置 Kiro IDE 设置项（key 由后端白名单校验；value 为 null 时删除该键）。
// 后端会写入 settings.json 并镜像回 app-settings.json（双向同步）。
export function setKiroAgentSetting(key: string, value: unknown) {
  return invoke('set_kiro_agent_setting', { key, value })
}

// ============================================================
// 自定义 Kiro 安装路径
// ============================================================

export function getCustomKiroPath() {
  return invoke<string | null>('get_custom_kiro_path')
}

export function setCustomKiroPath(path: string) {
  return invoke('set_custom_kiro_path', { path })
}

export function clearCustomKiroPath() {
  return invoke('clear_custom_kiro_path')
}

// ============================================================
// 环境检测
// ============================================================

// 检测 Kiro IDE 安装状态
export function checkIdeInstallation<T = any>() {
  return invoke<T>('check_ide_installation')
}

// 检测已安装的浏览器
export function detectInstalledBrowsers<T = any>() {
  return invoke<T[]>('detect_installed_browsers')
}

// 检测系统代理
export function detectSystemProxy<T = any>() {
  return invoke<T>('detect_system_proxy')
}

// Command Sandbox 配置（~/.kiro/settings/sandbox.json，独立文件而非 settings.json 键）
export interface SandboxConfig {
  enabled: boolean
  networkAccess: string
  mcpSandboxing: boolean
}

export function getSandboxConfig() {
  return invoke<SandboxConfig>('get_sandbox_config')
}

export function saveSandboxConfig(config: SandboxConfig) {
  return invoke<void>('save_sandbox_config', { config })
}

/** 全局 + 项目两级生效配置（项目只能收紧） */
export interface EffectiveSandbox {
  global: SandboxConfig
  project?: SandboxConfig
  effective: SandboxConfig
  /** 项目配置被收紧/忽略的说明，空数组 = 无冲突 */
  notes: string[]
}

export function getEffectiveSandbox(workspaceRoot?: string) {
  return invoke<EffectiveSandbox>('get_effective_sandbox', { workspaceRoot })
}

export function saveProjectSandboxConfig(workspaceRoot: string, config: SandboxConfig) {
  return invoke<void>('save_project_sandbox_config', { workspaceRoot, config })
}
