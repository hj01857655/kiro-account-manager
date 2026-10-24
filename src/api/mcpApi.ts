// MCP 服务配置 API 调用
import { invoke } from '@tauri-apps/api/core'

// 读取 MCP 配置（projectDir 为 null 时读全局配置）
export function getMcpConfig<T = any>(projectDir: string | null = null) {
  return invoke<T>('get_mcp_config', { projectDir })
}

// 每个 server 的配置来源：user / project / redundant / overridden / power
export function getMcpSources(projectDir: string | null = null) {
  return invoke<Record<string, string>>('get_mcp_sources', { projectDir })
}

// 获取 MCP 工具统计
export function getMcpToolStats<T = any>(projectDir: string | null = null) {
  return invoke<T>('get_mcp_tool_stats', { projectDir })
}

// 启用/禁用某个 MCP 服务
export function toggleMcpServer(name: string, disabled: boolean, projectDir: string | null = null) {
  return invoke('toggle_mcp_server', { name, disabled, projectDir })
}

// 删除某个 MCP 服务
export function deleteMcpServer(name: string, projectDir: string | null = null) {
  return invoke('delete_mcp_server', { name, projectDir })
}

// 保存/新增某个 MCP 服务
export function saveMcpServer(name: string, config: any, projectDir: string | null = null) {
  return invoke('save_mcp_server', { name, config, projectDir })
}
