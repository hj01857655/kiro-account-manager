// KiroConfig（agents / hooks / powers / skills / steering）API 调用
import { invoke } from '@tauri-apps/api/core'

// ============================================================
// 自定义 Agents
// ============================================================

export function getCustomAgents<T = any[]>(projectDir: string | null = null) {
  return invoke<T>('get_custom_agents', { projectDir })
}

export function saveCustomAgent(fileName: string, content: string, scope: string, projectDir: string | null = null) {
  return invoke('save_custom_agent', { fileName, content, scope, projectDir })
}

export function deleteCustomAgent(fileName: string, scope: string, projectDir: string | null = null) {
  return invoke('delete_custom_agent', { fileName, scope, projectDir })
}

export function createCustomAgent<T = any>(fileName: string, content: string, scope: string, projectDir: string | null = null) {
  return invoke<T>('create_custom_agent', { fileName, content, scope, projectDir })
}

// ============================================================
// Hooks
// ============================================================

export function getHooks<T = any[]>(projectDir: string | null = null) {
  return invoke<T>('get_hooks', { projectDir })
}

// scope: "user" = 用户级（~/.kiro/hooks，Kiro IDE 1.0.182+）；"project" = 项目级
export function saveHook(fileName: string, content: string, scope: string, projectDir: string | null = null) {
  return invoke('save_hook', { fileName, content, scope, projectDir })
}

export function deleteHook(fileName: string, scope: string, projectDir: string | null = null) {
  return invoke('delete_hook', { fileName, scope, projectDir })
}

export function createHook<T = any>(fileName: string, content: string, scope: string, projectDir: string | null = null) {
  return invoke<T>('create_hook', { fileName, content, scope, projectDir })
}

// ============================================================
// Powers
// ============================================================

export function getPowers<T = any[]>() {
  return invoke<T>('get_powers')
}

export function getPowerRegistries<T = any[]>() {
  return invoke<T>('get_power_registries')
}

export function getRecommendedPowers<T = any[]>() {
  return invoke<T>('get_recommended_powers')
}

export function installPower(name: string, cloneUrl: string, pathInRepo: string, branch: string) {
  return invoke('install_power', { name, cloneUrl, pathInRepo, branch })
}

export function uninstallPower(name: string) {
  return invoke('uninstall_power', { name })
}

/** Power 来源（对应 Kiro user-added 注册表的 source 字段） */
export type PowerSource =
  | { type: 'local'; path: string }
  | { type: 'repo'; repositoryCloneUrl: string; pathInRepo: string; repositoryBranch: string }

export interface UserAddedPowerEntry {
  name: string
  description: string
  repositoryUrl?: string
  source: PowerSource
}

/**
 * 从本地文件夹安装 Power（对应 Kiro「Import power from a folder」）。
 * 返回实际使用的 Power 名称（由目录名 sanitize 得出）。
 * 要求所选文件夹含 plugin.json 或 POWER.md。
 */
export function installPowerFromLocal(sourceDir: string) {
  return invoke<string>('install_power_from_local', { sourceDir })
}

/**
 * 从公开 GitHub URL 导入 Power（对应 Kiro「Import power from GitHub」）。
 * 支持 `https://github.com/owner/repo` 与 `/tree/<branch>/<subdir>` 形式。
 */
export function installPowerFromUrl(url: string) {
  return invoke<string>('install_power_from_url', { url })
}

/** 读取自定义 Power 来源注册表（本地文件夹 / GitHub 导入） */
export function getUserAddedPowers() {
  return invoke<UserAddedPowerEntry[]>('get_user_added_powers')
}

// ============================================================
// Skills / Steering
// ============================================================

export function getSkills<T = any[]>(projectDir: string | null = null) {
  return invoke<T>('get_skills', { projectDir })
}

export function saveSkill(name: string, content: string, scope: string, projectDir: string | null = null): Promise<string[]> {
  // 返回内容级合规诊断（空数组 = 通过）；保存本身不因诊断失败
  return invoke('save_skill', { name, content, scope, projectDir })
}

export function deleteSkill(name: string, scope: string, projectDir: string | null = null) {
  return invoke('delete_skill', { name, scope, projectDir })
}

export function createSkill<T = any>(name: string, content: string, scope: string, projectDir: string | null = null) {
  return invoke<T>('create_skill', { name, content, scope, projectDir })
}

export function getSteeringFiles<T = any[]>(projectDir: string | null = null) {
  return invoke<T>('get_steering_files', { projectDir })
}

export function saveSteeringFile(fileName: string, content: string, scope: string, projectDir: string | null = null): Promise<string[]> {
  // 返回内容级诊断列表（空数组 = 通过）；保存本身不因诊断失败
  return invoke('save_steering_file', { fileName, content, scope, projectDir })
}

export function deleteSteeringFile(fileName: string, scope: string, projectDir: string | null = null) {
  return invoke('delete_steering_file', { fileName, scope, projectDir })
}

export function createSteeringFile<T = any>(fileName: string, content: string, scope: string, projectDir: string | null = null) {
  return invoke<T>('create_steering_file', { fileName, content, scope, projectDir })
}

export function createDefaultSteeringFile<T = any>(scope: string, projectDir: string | null = null) {
  return invoke<T>('create_default_steering_file', { scope, projectDir })
}

export function createInitialProjectSteering<T = any>(projectDir: string) {
  return invoke<T>('create_initial_project_steering', { projectDir })
}

// ---------- 嵌套 AGENTS.md（Kiro 1.1.14）----------
// 与 .kiro/steering/*.md 同属 Steering 子系统，但按**目录层级**分布，
// 所以需要独立的递归扫描，不能复用普通 steering 的扁平列表接口。

export interface AgentsMdFile {
  relPath: string // 相对项目根，如 "AGENTS.md" / "src/api/AGENTS.md"
  dirRel: string // 所在目录，根目录为 ""
  depth: number // 0 = 项目根
  content: string
  size: number
  modifiedAt?: string | null
  scope: string
}

// 影响 AGENTS.md 扫描的忽略文件（.kiroignore / .gitignore）
export interface IgnoreFileInfo {
  relPath: string // 相对项目根，如 ".kiroignore" / "src/.gitignore"
  ruleCount: number // 有效规则条数（不含空行与注释）
}

export function scanAgentsMd<T = AgentsMdFile[]>(projectDir: string) {
  return invoke<T>('scan_agents_md', { projectDir })
}

// 列出影响 AGENTS.md 扫描的忽略文件（.kiroignore / .gitignore）
export function listAgentsMdIgnoreFiles(projectDir: string) {
  return invoke<IgnoreFileInfo[]>('list_agents_md_ignore_files', { projectDir })
}

export function getAgentsMd(projectDir: string, relPath: string) {
  return invoke<string>('get_agents_md', { projectDir, relPath })
}

export function saveAgentsMd(projectDir: string, relPath: string, content: string) {
  return invoke('save_agents_md', { projectDir, relPath, content })
}

export function refineSteeringFile<T = any>(fileName: string, scope: string, projectDir: string | null = null) {
  return invoke<T>('refine_steering_file', { fileName, scope, projectDir })
}

// ============================================================
// Specs（Kiro 1.1.14）
// ~/.kiro/specs/<name>/{requirements,design,tasks}.md
// ============================================================

export interface SpecFile {
  fileKind: string // "requirements" | "design" | "tasks"
  content: string
  size: number
  modifiedAt?: string | null
  exists: boolean
  scope: string
}

export interface SpecInfo {
  name: string
  scope: string
  requirements: SpecFile
  design: SpecFile
  tasks: SpecFile
}

export interface SpecSummary {
  name: string
  scope: string
}

export function listSpecs<T = SpecSummary[]>(scope: string, projectDir: string | null = null) {
  return invoke<T>('list_specs', { scope, projectDir })
}

export function readSpec<T = SpecInfo>(scope: string, projectDir: string | null, name: string) {
  return invoke<T>('read_spec', { scope, projectDir, name })
}

export function saveSpecFile(
  scope: string,
  projectDir: string | null,
  name: string,
  fileKind: string,
  content: string,
) {
  return invoke('save_spec_file', { scope, projectDir, name, fileKind, content })
}

export function createSpec(scope: string, projectDir: string | null, name: string) {
  return invoke('create_spec', { scope, projectDir, name })
}

export function deleteSpec(scope: string, projectDir: string | null, name: string) {
  return invoke('delete_spec', { scope, projectDir, name })
}

// ============================================================
// Workflows（Kiro 1.1.14）
// <project>/.kiro/workflows/*.workflow.{json,yaml,yml} 与 ~/.kiro/workflows/*
// （bundled:// 与 generated:// 属运行时/只读来源，本管理端不编辑）
// ============================================================

export interface WorkflowFile {
  fileName: string
  content: string
  size: number
  modifiedAt?: string | null
  scope: string
}

export function listWorkflows<T = WorkflowFile[]>(scope: string, projectDir: string | null = null) {
  return invoke<T>('list_workflows', { scope, projectDir })
}

export function readWorkflow<T = WorkflowFile>(scope: string, projectDir: string | null, fileName: string) {
  return invoke<T>('read_workflow', { scope, projectDir, fileName })
}

export function saveWorkflow(scope: string, projectDir: string | null, fileName: string, content: string): Promise<string[]> {
  // 返回 schema 诊断列表（空数组 = 通过）；保存本身不因诊断失败
  return invoke('save_workflow', { scope, projectDir, fileName, content })
}

export function createWorkflow(scope: string, projectDir: string | null, fileName: string) {
  return invoke('create_workflow', { scope, projectDir, fileName })
}

export function deleteWorkflow(scope: string, projectDir: string | null, fileName: string) {
  return invoke('delete_workflow', { scope, projectDir, fileName })
}

// ============================================================
// MCP
// ============================================================

export function saveMcpServer(name: string, config: any, projectDir: string | null = null) {
  return invoke('save_mcp_server', { name, config, projectDir })
}

// ============================================================
// Skill Import
// ============================================================

export function importSkillLocal<T = any>(sourcePath: string, scope: string, projectDir: string | null = null, overwrite = false) {
  return invoke<T>('import_skill_local', { sourcePath, scope, projectDir, overwrite })
}

export function importSkillFromGithub<T = any>(args: {
  repoUrl: string
  pathInRepo: string | null
  branch: string | null
  targetName: string | null
  scope: string
  projectDir: string | null
  overwrite?: boolean
}) {
  return invoke<T>('import_skill_from_github', args)
}
