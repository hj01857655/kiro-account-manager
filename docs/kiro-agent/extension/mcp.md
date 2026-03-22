# mcp 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/mcp/**` 注释边界去重

## 结论

- `mcp/**` 当前共识别 `7` 个唯一模块块。
- 这一层负责 extension 侧的 MCP 视图、命令和 registry 模式控制，不等同于 runtime 中的 MCP 连接实现。

## 模块清单

```text
mcp/mcp-availability.ts
mcp/index.ts
mcp/mcp-tree-data-provider.ts
mcp/registry-mode-utils.ts
mcp/mcp-utility-commands.ts
mcp/registry-picker-command.ts
mcp/registry-sync-controller.ts
```

## 分组

### 1. 入口与可用性

- `mcp/mcp-availability.ts`
- `mcp/index.ts`

这组负责 MCP 功能是否可用、以及 extension 侧 MCP 功能的统一注册入口。

### 2. 视图与命令

- `mcp/mcp-tree-data-provider.ts`
- `mcp/mcp-utility-commands.ts`
- `mcp/registry-picker-command.ts`

这组负责 MCP tree view、MCP 辅助命令和 registry QuickPick 安装/卸载流。

### 3. Registry 模式

- `mcp/registry-mode-utils.ts`
- `mcp/registry-sync-controller.ts`

这组处理 enterprise registry mode：

- `mcp/registry-mode-utils.ts` 判断是否启用 registry mode，并同步 `kiroAgent.mcp.registryMode` context。
- `mcp/registry-sync-controller.ts` 负责 registry 同步控制。

## 判断

- `mcp/**` 是 extension UI/命令层。
- 真正的 MCP 连接、tools/resources/prompts 同步和 OAuth 细节更多落在 runtime/manager 与 capability wrapper 侧。
