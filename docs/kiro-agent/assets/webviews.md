# Webviews 与静态资源（kiro.kiro-agent）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent`
> 分析日期：2026-03-17

本文件聚焦 webview 相关的静态资源与入口文件。

---

## bundled-webviews

- `bundled-webviews/requirements-webview.css`
- `bundled-webviews/requirements-webview.js`

用途：Spec 工作流的 requirements 文档编辑器 UI（1.1MB React 应用）。

核心组件：
- `RequirementsView`：读取 `specDocument.extractedRequirements` 与 `analysisItems`，展示需求列表，支持分析状态（isAnalyzing + progressMessage）。
- `RequirementItem`：单条需求展示，`REQ-\d+` 格式 ID 规范化匹配。
- `QuestionsModal` / `OpenQuestionsSection`：展示待确认问题的弹窗/区块。
- `WithEditorContext`：通过 `callApi(key, ...params)` 与主扩展通信，基于 UUID + Promise-deferred 的请求/响应模式，`onMessage` 处理 `response`/`error`/`event` 三种消息类型。

---

## hook-editor

- `packages/hook-editor/dist/index.html`
- `packages/hook-editor/dist/index.js`（664KB）
- `packages/hook-editor/dist/index.css`

用途：Hook 编辑器 UI。

Hook 触发类型枚举（比 `hook.json` schema 更完整）：
- `fileEdited` / `fileCreated` / `fileDeleted` / `userTriggered`
- `promptSubmit`（用户发送 prompt）
- `agentStop`
- `preToolUse` / `postToolUse`
- `preTaskExecution` / `postTaskExecution`

Hook 执行动作：`askAgent`、`runCommand`。

Hook 配置字段（`Ti` 枚举）：`enableDevMode`、`trustedCommands`、`commandDenylist`、`autoApproveAgentCommands`、`configureMCP`、`executeBashTimeoutMs`、`enableRepositoryMapIndex`、`trustedTools`。

---

## kiro-ui-agent-chat

- `packages/kiro-ui-agent-chat/dist/session-view/main.js`（1.1MB）
- `packages/kiro-ui-agent-chat/dist/styles.css`

用途：Chat/Session 视图。

布局结构：`session-view-root` → `session-view-container` → `session-view-content`（滚动区）+ `session-view-input`（输入行）。

包含完整的 `AutonomyMode`（Autopilot/Supervised）和 `ActionState`（not_started/queued/in_progress/completed）枚举。

telemetry 命名空间枚举：`kiro.application`、`kiro.feature`、`kiro.continue`、`kiro.agent`、`kiro.tool`、`kiro.parser`、`kiro.onboarding`、`kiro.webview`、`kiro.auth`。

Spec 工作流模式枚举：`RequirementsFirst` / `DesignFirst`，类型：`Feature` / `Bugfix`。

与主扩展通信同样走 `callApi(key, ...params)` + `postMessage` 模式（`EditorContext`）。

---

## kiro-ui-powers

- `packages/kiro-ui-powers/dist/installed-powers-list/main.js`（1.4KB，入口）
- `packages/kiro-ui-powers/dist/recommended-powers-list/main.js`（1KB，入口）
- `packages/kiro-ui-powers/dist/power-details/main.js`（10KB，主逻辑）
- `packages/kiro-ui-powers/dist/assets/ErrorMessage-CPVBN9Ao.js`（255KB，共享组件库）

用途：Powers 列表与详情。

requirements webview 的 bundle 细节见 `bundled-webviews.md`。

PowerDetail 组件通过 `callApi` 调用的接口：
- `getPowerDetails({name})` — 获取 power 详情
- `installPowerFromRepository({powerName})` — 安装
- `uninstallPower({name})` — 卸载
- `checkPowerForUpdates({name})` — 检查更新
- `updatePower({name})` — 更新
- `getMcpJsonContent({name})` — 读取 mcp.json 内容
- `openExternal({url})` — 打开外部链接
- `openPowersConfigFile()` — 打开 powers 配置文件

Powers 错误类型枚举：`InvalidPowerError`、`McpJsonNotFoundError`、`McpJsonFetchError`、`PowerAlreadyInstalledError`、`PowerNotFoundError`、`PowerNotInstalledError`、`PowerRegistryLoadError`、`PowerRegistrySaveError`、`PowerValidationError`、`PowerRegistryDataError`、`RepositoryCloneError`、`InstallFileSystemError`、`UnknownError`。

Power 状态枚举（Session/Task）：`Queued`/`InProgress`/`Paused`/`NeedAction`/`Success`/`Failed`/`Canceled`/`Yielded`。

---

## Webview 通信架构图

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                       extension.js (Extension Host)                                     │
│                                                                                        │
│  createWebviewPanel()                                                                  │
│  registerWebviewApi(callApi)                                                           │
│  postMessage({ type: 'request', id, key, params })                                      │
│             │                                                                         │
│             ├───────────────────────────────────────────────────────────────────────► │
│             │                                                                          │
│             │                                       ┌────────────────────────────────┐ │
│             │                                       │ WebviewPanel                   │ │
│             │                                       │  onMessage()                   │ │
│             │                                       │   request → callApi()          │ │
│             │                                       │   response → resolve()         │ │
│             │                                       │   error → reject()             │ │
│             │                                       │   event → listeners            │ │
│             │                                       └────────────────────────────────┘ │
│             │                                                                          │
│             │  ◄──────────────────────────────────────────────────────────────────────┤
│             │   postMessage({ type: 'response' | 'error' | 'event' })                  │
│             ▼                                                                         │
│  各 Webview 分工                                                                        │
│  ┌──────────────────────────────────────────────────────────────────────────────┐     │
│  │ requirements-webview  Spec 需求列表编辑器                                     │     │
│  │  RequirementsView → RequirementItem → QuestionsModal                         │     │
│  │  callApi: analyzeRequirements / updateSpec / saveSpec / readSpec              │     │
│  ├──────────────────────────────────────────────────────────────────────────────┤     │
│  │ hook-editor  Hook 规则编辑器                                                  │     │
│  │  触发: fileEdited/Created/Deleted/userTriggered                               │     │
│  │       promptSubmit/agentStop/preToolUse/postToolUse                           │     │
│  │  动作: askAgent / runCommand / openFile                                       │     │
│  ├──────────────────────────────────────────────────────────────────────────────┤     │
│  │ kiro-ui-agent-chat  Chat/Session 视图                                         │     │
│  │  session-view-root → content(滚动) + input(输入行)                             │     │
│  │  AutonomyMode: Autopilot / Supervised                                         │     │
│  │  ActionState: not_started/queued/in_progress/completed                        │     │
│  ├──────────────────────────────────────────────────────────────────────────────┤     │
│  │ kiro-ui-powers  Powers 列表与详情                                             │     │
│  │  installed-list / recommended-list / power-details                            │     │
│  │  callApi: getPowerDetails / installPower / uninstallPower / updatePower       │     │
│  └──────────────────────────────────────────────────────────────────────────────┘     │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## continuedev GUI

- `packages/continuedev/gui/dist/index.html`
- `packages/continuedev/gui/dist/assets/index.js`
- `packages/continuedev/gui/dist/assets/index.css`

用途：Continue GUI webview。
