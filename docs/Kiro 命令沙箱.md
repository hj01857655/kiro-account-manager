# Kiro 命令沙箱：实现事实

适用版本：IDE 1.3+ / CLI 2.31+（local V3），默认 **Off**。

## 存储与路径

独立 JSON 文件，**不是** settings.json 的键。文件不存在 = 沙箱关闭（默认态）。

| 作用域 | 路径 |
| --- | --- |
| 全局 | `~/.kiro/settings/sandbox.json` |
| 项目 | `<workspace-root>/.kiro/settings/sandbox.json`（多根工作区只取第一个文件夹） |

## 键

| 键 | 必填 | 取值 | 默认 |
| --- | --- | --- | --- |
| `enabled` | 是 | `true` / `false` | — |
| `networkAccess` | 否 | `DEFAULT_BLOCKED` / `COMMON_DEPENDENCIES` / `DEFAULT_ALLOWED`（大小写敏感，未知键忽略） | `DEFAULT_BLOCKED` |
| `mcpSandboxing` | 否 | `true` / `false` | `false` |

## 出网三档

- `DEFAULT_BLOCKED`：全拦，仅 `sandbox_network` 权限规则放行的主机可达
- `COMMON_DEPENDENCIES`：常用包/源/构建/容器服务可达，其余仍拦
- `DEFAULT_ALLOWED`：全开；此模式下不要依赖 deny 规则

## 组合规则

- 全局激活时，项目文件**只能收紧**：不能设 `enabled:false`、不能更开放、不能关全局的 MCP wrapping
- 全局关闭或缺失时，项目 `enabled:true` 对该项目生效
- 克隆来的仓库不能借项目文件打开网络或关掉全局策略

## 平台要求

| 平台 | 要求 |
| --- | --- |
| macOS | 无（内置 Seatbelt，经 `sandbox-exec`） |
| Linux | PATH 上有 `bwrap`；`COMMON_DEPENDENCIES`/`DEFAULT_ALLOWED` 另需 `socat` |
| Windows | `COMMON_DEPENDENCIES`/`DEFAULT_ALLOWED` 需一次性管理员设置（IDE 弹提示；CLI `/sandbox setup`）；`DEFAULT_BLOCKED` 无需 |

后端缺失时状态为 `Unavailable`：命令照跑、无 OS 隔离，除非当前模式要求保护。

## 覆盖范围

- 沙箱内：agent 的 shell 命令及其全部子进程；`mcpSandboxing:true` 时的本地 stdio MCP 进程（切换会重启本地 MCP，约 1 分钟）
- 沙箱外：agent harness 本体、内置文件工具（仍受权限规则约束）、远程 HTTP/SSE MCP

## 与权限系统的联动

- `sandbox_network` 是独立 capability，**不在 `all` 元能力里**（`capability:all, effect:allow` 不会放开沙箱网络）
- 主机规则按**顺序首个命中即决定**（与其他 capability 的 deny 优先不同）；精确主机要写在通配之前
- `fs_read` / `fs_write` 规则同样约束沙箱内命令的可见/可写路径

## 状态机

Off / Active / Applying / Error / Unavailable。保存合法变更即热生效，无需重启会话。

## 本机状态（2026-10-24）

`~/.kiro/settings/sandbox.json` 不存在 → Off。本机 IDE 1.4.5 / CLI 2.33+，均满足版本要求。
