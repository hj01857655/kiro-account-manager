# extension 到 runtime 交叉映射

> 分析目标：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 整理日期：2026-03-18
> 目标：把宿主扩展层 `src/extension/**` 与内核运行时 `packages/kiro-agent/dist/**` 的主链路串起来

## 结论

- `src/extension/**` 是宿主层，负责 VS Code 激活、命令注册、webview、状态栏、配置、工作区桥接。
- `packages/kiro-agent/dist/**` 是运行时内核，负责 agent 执行、上下文窗口、工具过滤、MCP 配置、Powers、spec agent、sub-agent、execution log。
- 两者不是上下级替代关系，而是“宿主装配层 + 可复用执行内核”的关系。

## 总体分层

```text
VS Code / Kiro IDE 宿主
  └─ src/extension/**
       ├─ activation / commands / views / status bar
       ├─ capabilities / editor / mcp / hooks / spec-editor / steering
       ├─ telemetry / storage / onboarding / polling / context-usage
       └─ 把 IDE 资源装配给 runtime
            ↓
packages/kiro-agent/dist/**
  ├─ AgentController / AgentExecution / workspace-connection
  ├─ model provider / token monitor / pruning / prompt processor
  ├─ MCP config manager / remote tools discovery / powers manager
  ├─ spec-agent / hooks / disclose-context / command-approval
  └─ execution-log controller / pending changes / session persistence
            ↓
Q Developer / MCP Servers / Local Files / Terminal / Workspace State
```

## 主链路 1：激活与装配

### extension 侧

- `activation.md`
- `capabilities.md`
- `commands.md`
- `config.md`
- `platform.md`

宿主扩展启动后主要做三件事：

1. 激活 VS Code 扩展对象与 UI 注册。
2. 创建工作区连接和工具能力装配。
3. 把这些装配后的对象交给 runtime 内核。

### runtime 侧

- `docs/kiro-agent/runtime/packages-kiro-agent-dist.md`
- 重点模块：
  - `index.js`
  - `workspace-connection-impl-Dee9nf40.js`
  - `model-provider-DwyIQZZf.js`
  - `telemetry-Bf0GI6nJ.js`

`index.js` 是 runtime 总出口，暴露 `KiroAgent`、`AgentController`、`AgentExecution`、`RemoteToolsDiscovery`、`PowersManager`、`MCPConfigManager`、`createACPWorkspaceConnection()` 等装配点。

### 对应关系

- extension `activate2 / registerExtension`
  -> runtime `index.js` 导出的装配 API
- extension `capabilities/create-workspace-connection.ts`
  -> runtime `workspace-connection-impl-Dee9nf40.js`
- extension telemetry/config/provider 注入
  -> runtime `telemetry-Bf0GI6nJ.js`、`model-provider-DwyIQZZf.js`

## 主链路 2：命令到 agent 执行

### extension 侧

- `commands.md`
- `agent-chat.md`
- `capabilities.md`
- `utils.md`

extension 侧负责：

- 接收 `kiroAgent.*` 命令
- 收集用户输入、上下文文件、当前工作区状态
- 通过 `trigger-agent`、controller accessor、tool registry 把请求送入执行内核

### runtime 侧

- `chat-agent-IUIL54gd.js`
- `execution-log-controller-BGxzu20b.js`
- `prompt-processor-Dw_f2vjD.js`
- `token-monitor-QTYxqQ8j.js`
- `pruning-service-DJ-mli7u.js`
- `command-approval-MQd-5ajF.js`
- `command-approval-_g4NkBJ8.js`

runtime 侧负责：

- 生成执行对象
- 维护上下文窗口与 token 预算
- 根据 autonomy/supervised 模式处理审批
- 推动模型流式输出与 action 执行

### 对应关系

- extension `commands/agent/*`
  -> runtime `chat-agent-IUIL54gd.js`
- extension `rich-execution-log/**`
  -> runtime `execution-log-controller-BGxzu20b.js`
- extension `capabilities/tool-registry.ts`
  -> runtime tool filtering / command approval / action execution

## 主链路 3：Capabilities 到 Workspace Connection

### extension 侧

- `capabilities.md`

关键点：

- `capabilities/tool-registry.ts` 决定 chat/spec/sub-agent/system 的可见工具集合
- `capabilities/tool-factories/*` 批量生成 fs/search/shell/remote 工具
- `capabilities/tools/*` 实现 editCode、readCode、kiroPowers、mcp-wrapper 等高价值工具
- `capabilities/create-workspace-connection.ts` 把 VS Code 文件系统、终端、后台进程、workspace resolver 装成连接对象

### runtime 侧

- `workspace-connection-impl-Dee9nf40.js`
- `remote-tools-C9QNKWUJ.js`
- `remote-tools-discovery-DUpCRp4S.js`
- `tool-filter-CS5Fsu0N.js`
- `tool-tags-NGeoUHCI.js`
- `tool-usage-meter-BVm5olm7.js`

### 对应关系

- extension 提供“工具实现”和“IDE 资源”
  -> runtime 提供“工具编排、可见性、发现与统计”
- extension `capabilities/tools/mcp-wrapper.ts`
  -> runtime `remote-tools*` 与 MCP 发现链路
- extension `capabilities/tools/kiro-powers.ts`
  -> runtime `powers-manager-CbGmy5n_.js`

## 主链路 4：MCP

### extension 侧

- `mcp.md`
- `capabilities.md`
- `context-usage.md`

宿主层负责：

- MCP tree view 与 utility commands
- registry mode UI/context
- 把远程 MCP tools 包装成本地可调用工具
- 在上下文预算里估算 MCP tools token 占用

### runtime 侧

- `mcp-config-manager-BVKB8dJE.js`
- `remote-tools-discovery-DUpCRp4S.js`
- `remote-tools-C9QNKWUJ.js`
- `acp-remote-mcp-client-DTe6uFFL.js`
- `web-fetch-utils-C1Z4KMmp.js`

### 对应关系

- extension `mcp/index.ts`
  -> runtime `mcp-config-manager-BVKB8dJE.js`
- extension `capabilities/tool-factories/remote.ts`
  -> runtime `remote-tools-discovery-DUpCRp4S.js`
- extension `capabilities/tools/mcp-wrapper.ts`
  -> runtime MCP 工具调用与审批链

## 主链路 5：Powers

### extension 侧

- `powers.md`
- `capabilities.md`
- `steering.md`

宿主层负责：

- 展示 powers UI
- 允许 agent 通过 `kiroPowers` 工具发现、激活和调用 power 内 MCP server
- 读取 power steering 文档

### runtime 侧

- `powers-manager-CbGmy5n_.js`
- `steering-Dcn_tjkT.js`
- `mcp-config-manager-BVKB8dJE.js`

### 对应关系

- extension `capabilities/tools/kiro-powers.ts`
  -> runtime `powers-manager-CbGmy5n_.js`
- extension `steering/**`
  -> runtime `steering-Dcn_tjkT.js`
- power 中的 `mcp.json`
  -> runtime MCP config manager / remote tools

## 主链路 6：Spec

### extension 侧

- `contextual-spec.md`
- `spec-editor.md`
- `editor.md`

宿主层负责：

- 注册 spec 创建与问答命令
- 保存 spec editor 文档与 webview 状态
- 让 requirements/spec 页面调用 editor API

### runtime 侧

- `spec-agent-OdZ7esxm.js`
- `spec-platform-361SGdHa.js`
- `spec/tasks/index.js`
- `orchestrator-prompt-Vzhr2QeL.js`

### 对应关系

- extension `contextual-spec/invoke-spec-agent.ts`
  -> runtime `spec-agent-OdZ7esxm.js`
- extension `spec-editor/**`
  -> runtime `spec-platform-361SGdHa.js` 与 `spec/tasks/index.js`
- extension `editor/editor-api/spec-actions.ts`
  -> runtime spec orchestration

## 主链路 7：Hooks

### extension 侧

- `hooks.md`
- `editor.md`
- `rich-execution-log.md`

宿主层负责：

- hook 存储、校验、tree view、命令
- hook editor webview
- hook 运行状态和 execution log UI 联动

### runtime 侧

- `hooks-Dmyvf9cL.js`
- `execution-log-controller-BGxzu20b.js`
- `pending-changes-BxcOZUqX.js`

### 对应关系

- extension `hooks/**`
  -> runtime `hooks-Dmyvf9cL.js`
- extension `editor/hooks/*`
  -> runtime execution / hook state 联动
- extension `rich-execution-log/**`
  -> runtime execution log + pending changes

## 主链路 8：Execution Log / Diff / Session

### extension 侧

- `rich-execution-log.md`
- `session-resume.md`
- `storage.md`

宿主层负责：

- execution log webview / commands / diff 接受
- session snapshot 文件访问
- UI 查询 active execution、history、queued executions

### runtime 侧

- `execution-log-controller-BGxzu20b.js`
- `pending-changes-BxcOZUqX.js`
- `message-replay-Cjyo5CAH.js`
- `session-update-utils-S4kmZ2as.js`
- `session/schemas/index.js`

### 对应关系

- extension `rich-execution-log/controller/execution-log-controller.ts`
  -> runtime `execution-log-controller-BGxzu20b.js`
- extension `pending-changes-adapter.ts`
  -> runtime `pending-changes-BxcOZUqX.js`
- extension session persistence / replay UI
  -> runtime `message-replay-Cjyo5CAH.js`、`session-update-utils-S4kmZ2as.js`

## 主链路 9：上下文、Token 与压缩

### extension 侧

- `context-resolvers.md`
- `context-lsp.md`
- `context-usage.md`
- `capabilities/tools/read-code.ts`
- `utils/shared-parser.ts`

### runtime 侧

- `context-chat-message-DzjJbTBD.js`
- `token-monitor-QTYxqQ8j.js`
- `token-estimator-B45EKs9J.js`
- `pruning-service-DJ-mli7u.js`
- `node-progressive-context-source-C3Yk_3xF.js`
- `file-context-ClFefQMc.js`

### 对应关系

- extension 负责上下文采集、文件解析、LSP/steering/MCP 估算
  -> runtime 负责 token 记账、上下文窗口和裁剪策略

## 一张总图

```text
用户操作 / VS Code 事件
  ↓
src/extension/activation + commands + views
  ↓
src/extension/capabilities + editor + mcp + hooks + spec-editor
  ↓
workspace/file/terminal/search/mcp/powers 被装配为工具与连接
  ↓
packages/kiro-agent/dist/index.js
  ↓
AgentController / AgentExecution / workspace-connection / model-provider
  ↓
prompt-processor / token-monitor / pruning-service / command-approval
  ↓
spec-agent / hooks / powers-manager / mcp-config-manager / remote-tools
  ↓
execution-log-controller / pending-changes / session persistence
  ↓
Q Developer / MCP Servers / Local Workspace / Terminal / Webview UI
```

## 最终判断

- `src/extension/**` 解决的是“如何接进 IDE、如何让用户操作、如何把 IDE 能力交出去”。
- `packages/kiro-agent/dist/**` 解决的是“拿到这些能力后，agent 如何执行、审批、压缩上下文、记录日志、驱动 spec/hooks/powers/MCP”。
- 因此 Kiro 的 bundle 实际结构是：
  - 宿主扩展层负责装配与交互
  - runtime 层负责执行与编排
  - 两者通过 workspace connection、tool registry、controller、execution log 等桥接点紧密耦合
