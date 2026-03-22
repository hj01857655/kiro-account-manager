# Extension 入口（dist/extension.js）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 分析日期：2026-03-17
> 文件大小：约 49MB（高度压缩的 bundle）

本文件聚焦扩展入口 `dist/extension.js` 的激活流程与关键注册点。

---

## 模块分类索引

**启动与初始化**：激活流程 · activate2 初始化序列 · 注册命令完整列表 · 配置项监听

**账号与鉴权**：VSCode Authentication Provider · Profile 体系 · SessionResume · Enterprise

**UI 与交互**：WebView Provider · AgentChat · StatusBar · Notifications · QuickEdit

**能力层（Capabilities）**：Capabilities · EditorAPI · Powers API · Powers 控制器 · MCP 控制器 · Skill 控制器

**智能体执行**：Agent 执行引擎 · AgentEventPolling · ExecutionLog · Rich Execution Log · RemoteToolsDiscovery

**代码理解**：Autocomplete · Indexing/Embeddings · Context Provider · ContextLSP · ContextResolvers · ProgressiveContextManager

**Spec 与 Hook**：Spec 控制器 · Hook 控制器 · spec-editor · contextual-spec · HookController

**配置与设置**：Config · Steering · ImportSteering · Experiments · ModelSelection · Q Developer 自定义模型

**命令层**：Commands · generateCommitMessage · Terminal · SubscriptionPlans · UsageSummary · Custom Agent Registry · UriHandler

**存储与持久化**：Storage · Checkpoints · Onboarding · FirstTimeProject · BackgroundProcesses · ContextUsage · Usage Limits

**工具与辅助**：utils · platform · repos · custom-agent-loader · ACP Dev Inspector · Diff 体系

---

## 激活流程

扩展通过标准 VSCode `activate(context)` 入口启动，主要完成以下注册：

- 注册全部 `kiroAgent.*` 命令（见下方完整列表）
- 监听 `onDidChangeConfiguration`，响应 `kiroAgent.enableDebugLogs`、`kiroAgent.agentAutonomy`、`kiroAgent.usageSummary`、`kiroAgent.experiments`、`kiroAgent.modelSelection` 等配置变更
- 初始化 ACP Chat WebviewPanel（`openAgentChatPanel`）
- 创建右下角状态栏项（`StatusBar`，对齐 Right，priority -999），显示 experiments 入口
- 注册 Context Providers（通过 `registerProvider` / `unregisterProvider`）

---

## 注册命令完整列表

共 80 个 `kiroAgent.*` 命令，按功能分组如下：

**会话与聊天**
`startNewChatSession` `loadChatSession` `loadSessionWithPrompt` `newSession` `newSessionWithAssistantMessage` `focusChatInput` `focusContinueInput` `focusContinueInputWithoutClear` `focusContinueInputWithoutNewSession` `sendMainUserInput` `userInputFocusNoSubmit` `viewHistoryChats` `chat` `chatFocus`

**代码编辑与 Diff**
`acceptDiff` `rejectDiff` `acceptSuggestion` `rejectSuggestion` `acceptVerticalDiffBlock` `rejectVerticalDiffBlock` `streamingDiff` `diffVisible` `showDiffInfoMessage` `quickEdit` `quickFix` `fixCode` `fixGrammar` `optimizeCode` `writeCommentsForCode` `writeDocstringForCode`

**上下文与文件**
`addCodeBlockToChat` `selectFilesAsContext` `selectRange` `provideContext` `loadContextProviderItems` `refreshContextProviders` `recordReferences` `codeReferences` `notifyFilesCreated` `notifyFilesDeleted`

**Spec / 需求**
`initiateSpecCreation` `submitAnalysyisAnswers` `viewLetsBuild` `viewHome`

**MCP**
`configureMCP` `openActiveMcpConfig` `openUserMcpConfig` `openWorkspaceMcpConfig` `openMcpConfigForServerName`

**自主模式（Autonomy）**
`agentAutonomy` `getAutonomyMode` `setAutonomyMode` `toggleAutonomyMode` `approvedHookCommands`

**终端与调试**
`sendToTerminal` `debugTerminal` `enableShellIntegration` `enableDebugLogs` `createDebugLogZip` `viewLogs` `openExecutionLogView` `showExecutionInChatTab`

**模型与配额**
`modelSelection` `getTokenUsage` `usageSummary` `openTabAutocompleteConfigMenu` `toggleTabAutocompleteEnabled` `logAutocompleteOutcome`

**账号与设置**
`deleteAccount` `openSettingsUI` `openConfigJson` `openExperiments` `showExperiments` `experiments` `updateCoachingStatus` `fileFeedback`

**其他**
`acpChatView` `continueGUIView` `kiroAgent` `customQuickActionSendToChat` `customQuickActionStreamInlineEdit` `defaultQuickAction` `generateCommitMessage` `foldAndUnfold` `hideInlineTip` `log` `test`

---

## 配置项监听

| 配置键 | 触发行为 |
|---|---|
| `kiroAgent.enableDebugLogs` | 更新调试日志开关 |
| `kiroAgent.agentAutonomy` | 推送 Kiro 设置更新到 WebView |
| `kiroAgent.usageSummary` | 同上 |
| `kiroAgent.experiments` | 同上 |
| `kiroAgent.modelSelection` | 触发 WebviewProvider 重新渲染 |

## 激活流程（高层）

### 启动前置
- `setupCa()`：根据平台注入系统根证书（macOS 专用模块、Windows `win-ca`、其他平台读取系统证书）。
- `CONTINUE_GLOBAL_DIR` 设为 `context.globalStorageUri.fsPath`。

### dynamicImportAndActivate
- 延迟加载 Continue 扩展逻辑，调用 `activateExtension(context)`。
- 异常时弹出提示：`View Logs`（执行 `kiroAgent.viewLogs`）或 `Retry`（重载窗口）。

### activate2（Kiro 主入口）
- 启动时清理 `LANGSMITH_*` / `LANGCHAIN_*` 环境变量。
- 初始化扩展注册与遥测：`initializeAgentTelemetry()` + `MetricReporter.startPeriodicReporterLoop()`。
- 主要流程：`registerExtension` trace 内依次执行注册、Powers 初始化、账号/配置选择校验、激活 Continue 扩展、初始化可用模型缓存、异步发现远端工具。
- 完成后输出日志：`Congratulations, "Kiro" is now active!`

---

## Spec（contextual-spec）链路

源文件对应 `src/extension/contextual-spec/`，在 `activate2` 阶段通过 `register5({ context, agents })` 注册。

注册的核心命令：
- `kiroAgent.initiateSpecCreation`
- `kiroAgent.submitAnalysisAnswers`

`kiroAgent.initiateSpecCreation` 接收 `newSpecProps`，调用 `generateInitialSpec(newSpecProps, agents)`。

`generateInitialSpec` → `invokeSpecAgent`：
- 构造 Spec agent definition（包含 `userPrompt`、`executionId`、`chatSessionId`）
- 从 context items 里只保留 `providerTitle === "file"` 的条目
- 将文件映射为 `{ filePath, fileContent }` 作为 `context` 输入
- 最终调用 `agents.trigger(definition)` 启动 Spec 生成

`kiroAgent.submitAnalysisAnswers(chatSessionId, items, answers)`：
- 将回答写回 `specEditorStorage.saveSpec`
- 通过 `getOrCreateSpecSession(featureName)` 关联 chat session
- 再次 dispatch 到 agent 继续执行

`analyzeRequirements(documentUri)` 解析 Spec Markdown，提取 `analysisItems` 供 WebView 渲染问答表单。

`specEditorStorage` 以 `documentUri` 为 key 持久化 Spec 对象（含 `analysisItems`、已回答集合等），通过 `findSpecForDocumentUri` 查询，`saveSpec` 写入。

`extractSpecFeatureNameFromPath(documentUri)` 从文件路径提取 feature 名称，用于关联 chat session。

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Spec（contextual-spec）                                                     │
│                                                                              │
│  kiroAgent.initiateSpecCreation                                             │
│    newSpecProps                                                             │
│        │                                                                     │
│        ▼                                                                     │
│  generateInitialSpec → invokeSpecAgent → agents.trigger                      │
│        │                                                                     │
│        ▼                                                                     │
│  .kiro/specs/<feature>.md  (analysisItems / answers)                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

**扩展无 `deactivate` 导出**：Kiro extension 未实现卸载清理逻辑，所有资源通过 `context.subscriptions` 自动释放。

---

## Q Developer 自定义模型（q-custom-model）

模块位置：`src/extension/q-custom-model/`。

核心是 `createQDeveloperConverse(modelType, agentMode, fields)`，用于封装 Q Developer Streaming API 调用，注入认证、隐私/agent-mode headers，并挂载 `qChatLogger`（`OutputChannel` 标题为 `Q Chat API`）与 metrics。

详细链路见 `q-custom-model.md`。

---

## Rich Execution Log（rich-execution-log）

模块位置：`src/extension/rich-execution-log/`，提供执行日志视图、diff 适配、会话快照与回放，服务于「执行记录 → UI 展示」。

详细链路见 `rich-execution-log.md`。

---

## 待定位条目

以下条目已定位到 bundle 内部模块（非 Kiro 业务逻辑）：  

### platform:1683

对应 `node_modules/node-forge/lib/cipherModes.js` 的 `ECB` 模式实现：

- 初始化块大小 `_ints = blockSize / 4`
- 分配 `_inBlock` / `_outBlock` 数组

该段属于加密依赖（node-forge），与 Kiro 扩展业务无直接关系。

### utils:20434-20451

对应 `node_modules/undici/lib/llhttp/utils.js`：

- `enumToMap(obj)`：遍历枚举对象，将数字值映射成 `{ key: value }`  
- 为 `llhttp/constants.js` 提供枚举映射支持

该段属于 undici HTTP 依赖的 llhttp 工具函数，非业务逻辑。

---

## Terminal 集成体系

**debugTerminal**：命令执行后设置 500ms 防抖定时器（`debugTerminalTimeout`），到期后读取 `ide.getTerminalContents()`，自动拼成「I got the following error, can you please help explain how to fix it?」prompt，通过 `sidebar.webviewProtocol.request("userInput", ...)` 注入 Chat 面板。

**sendToTerminal**：直接调 `ide.runCommand(text)` 在活跃终端执行。

**Shell Integration**：`setupKiroShellIntegration()` 遍历 `shellConfigs`（bash / zsh / fish / PowerShell），按 shell 类型写入对应配置文件（`~/.bashrc`、`~/.zshrc` 等），注入 Kiro CLI source 行。`isKiroShellIntegrationComplete()` 检查每个 shell 配置文件是否已含该行。`kiroAgent.enableShellIntegration` 命令调用 `setupKiroShellIntegration()` 后弹出「Reload Window」提示；`EnableShellIntegrationTool.doExecute` 在 `hasKiroCommand()` 为 true 且 `input.install` 时同样调该函数。

---

## generateCommitMessage 链路

源文件 `src/extension/commands/source-control/`，通过 `registerInstrumentedCommand("kiroAgent.generateCommitMessage", ...)` 注册。

核心函数 `generateCommitMessageWithFastModel(prompt)` 使用 `u11(o4, l4, "vibe")` 取 fast model 实例（vibe tier），调 `fastModel.invoke([new HumanMessage(prompt)])` 获取响应。失败时抛 `CommitMessageGenerationError extends KiroError`，携带 reason 字符串。成功后将生成的 commit message 写入 SCM input box（`vscode.scm.inputBox.value`）。

---

## ExecutionLog 控制器

`ExecutionLogController`（单例，通过 `getExecutionLogController()` 获取，未初始化时抛错）在 activate 阶段由 `setExecutionLogController(n13)` 注入。

`executionLog.getExecution(executionId)` 返回当前执行的完整 action 列表。actions 按 `actionState` 区分：`PendingAction`（待处理）、`Rejected`（已拒绝）。

`F11` 类（CheckpointRestorer）持有 `checkpoint`、`fs`、`executionLog` 三个依赖：
- `applyPendingChanges`：遍历 actions，对 `PendingAction` 状态的 move/write action 执行文件系统操作
- `restorePendingChanges`：倒序遍历，回滚 PendingAction
- `restoreAllChanges`：倒序遍历，回滚除 Rejected 之外的所有 action

WebView 消息 `kiro/executionLog/operationUpdate` 在 Supervised 模式下触发，payload 含 `executionId` 和 `operation`（type: `"DeletePrompt"` 等），用于通知前端更新操作状态。

上下文临界值判断：`_t6(i)` 函数，`i >= 95` → `"critical"`，`>= 80` → `"warning"`，其余 → `"normal"`，用于 token 使用量 StatusBar 颜色判断。

---

## RemoteToolsDiscovery

`RemoteToolsDiscovery`（类 `c15`）在 `KiroAgent.init()` 时通过 `gs3(mcpClientOptions)` 构造，立即异步调 `discoverTools()`，失败只 warn 不中断启动。

`discoverTools(force = false)`：若 `!force && cachedTools` 则直接返回缓存；否则调 `mcpClient.listTools()`，结果经 `ListToolsResultSchema.safeParse` 校验后存入 `this.cachedTools`，失败抛错。`wrapTools(tools)` 将原始 tool 列表包装为 `RemoteWrapper` 实例。

`KiroAgent` 将 `remoteToolsDiscovery` 传入每次 `createAgentExecution()` 的 options，agent 执行时按需 lazy discovery（`discoveryInProgress` flag 防并发）。`clearModelCache()` 导出为 `p5`，供 model 配置变更后手动清除缓存。

---

## UsageSummary 与 Token 上报

每轮 agent 流式响应中，chunk 的 `additional_kwargs.usageSummaryEntry` 字段携带当轮 token 汇总，赋值给 `w23`；`additional_kwargs.contextUsagePercentage` 赋值给 `y23`（0~100 浮点数）。

流结束后 emit 两个事件：
- `AgentExecutionSummarizeUsage`：payload `{ executionId, usageSummaryEntry: w23 }`，供前端渲染 token 消耗面板
- `AgentExecutionContextUsageUpdate`：payload `{ executionId, contextUsagePercentage: y23 }`，驱动 StatusBar 颜色（`_t6` 函数映射 ≥95→critical / ≥80→warning / else→normal）

OTel 链路：`getUnifiedRunTokens(outputs)` 从 LangChain run outputs 中多路提取 `usage_metadata`（直接字段 → `kwargs` → `generations[].message.kwargs`），返回 `[inputTokens, outputTokens]`，写入 span 属性 `gen_ai.usage.input_tokens` / `gen_ai.usage.output_tokens` / `gen_ai.usage.total_tokens`。

BedrockAPI 层：`de_TokenUsage` 解码响应体，字段包含 `cacheReadInputTokens`、`cacheWriteInputTokens`、`normalizedTokenUsage`、`contextUsagePercentage`、`outputTokens`、`totalTokens`、`uncachedInputTokens`。

---

## Experiments 体系

`register6(context)` 在 activate 阶段初始化实验系统：

`ExperimentsConfigProvider(EXPERIMENT_DEFINITIONS)` 读取 `kiroAgent.experiments` VSCode 配置，`experimentsService.isEnabled(id)` 判断各实验开关。初始化后遍历 `EXPERIMENT_DEFINITIONS` 的所有 id，调 `setContext("kiroAgent.experiments.${id}", enabled)` 写入 VSCode context，供 `when` 子句控制命令/菜单可见性。

若 `experimentsService.hasVisibleExperiments` 为 true，则额外 `setContext("kiroAgent.showExperiments", true)` 并挂载 `ExperimentsTelemetry`。

命令 `kiroAgent.experiments.getExperiments` 通过 `registerInstrumentedCommand` 注册，WebviewProtocol 侧的 `getExperiments()` 调 `executeCommand("kiroAgent.experiments.getExperiments")` 获取当前实验列表。

配置变更监听：`event.affectsConfiguration("kiroAgent.experiments")` 触发 `sendKiroSettingsUpdate()`，推送最新实验状态至 WebView。

---

## ProgressiveContextManager

`ProgressiveContextManager`（变量名 `m24`）在 `createAgentExecution()` 的 options 中传入，若存在则从工具列表中过滤掉 `si2.toolId`（compaction 工具），防止 agent 主动触发上下文压缩。

构造时接收 `source`、`emitExtNotification`、`homeDir` 三个依赖。生命周期方法：
- `onSessionCreated(sessionId, workspacePaths)`：注册 session，首次调用时启动 `p14` FileWatcher，监听 `["skills", "steering"]` 子路径（同时监听 home 目录和 workspace）
- `onSessionDestroyed(sessionId)`：移除 session
- `dispose()`：清除 debounce timer、停止 fileWatcher、清空所有 session 和缓存

文件变更后经 debounce 触发 `scanAllSessions()`：遍历所有 session 的 workspacePaths，调 source 扫描，结果以 `displayName` 去重后存入 `cachedItems`（Map），然后对每个 session emit `itemsChanged` ACP 通知。扫描失败则 emit `itemsFailed`。

`getCachedItems()` 直接返回缓存，供 agent 每轮请求时注入 progressive context。

`NodeProgressiveContextSource` 读取具体文件内容，失败只 warn 不抛错（`[NodeProgressiveContextSource] Failed to read content`）。

Context Window 保护：Steering 刷新前调 `se11(e19, n27)` 检查是否超窗口，超出则 skip 并上报 `SteeringSkippedContextWindow` 指标。`ContextWindowExceededError` 在 agent FSM 中直接 `emitError`，不重试（与 `InsufficientModelCapacityError`/`HighLoadError` 的 retry 行为不同）。

---

## Profile 体系（ProfileLifecycleManager）

`ConfigHandler` 在构造时初始化一个 `LocalProfileLoader`（`profileId = "local"`，`profileTitle = "Local Config"`），包装为 `ProfileLifecycleManager` 放入 `this.profiles` 数组，`selectedProfileId` 默认为 `"local"`。

属性：
- `currentProfile`：`profiles.find(p => p.profileId === selectedProfileId) ?? fallbackProfile`
- `fallbackProfile`：`profiles[0]`（始终是 local）
- `inactiveProfiles`：未选中的 profiles

`setSelectedProfile(profileId)` 切换 profile：更新 `selectedProfileId`，重新 `loadConfig()`，通知所有 config 监听者，并将选择持久化到 `globalContext`（key `lastSelectedProfileForWorkspace`，以 workspace folder 名拼接的 id 做索引）。

WebView 协议 `getSerializedConfig` 响应体含 `{ config, profileId: currentProfile.profileId }`。

Auth 后 profile 选择流程：`handleProfiles(token)` 检查 `supportsProfiles(token)`，若只有一个 profile 则直接 `executeCommand("kiro.profiles.selectProfile", profiles[0])`，多个则 `executeCommand("kiro.profiles.showProfileSelector", { isDismissible: true })` 弹出选择器。

Bedrock API 响应中 `profileId` 是顶层字段（与 `userId`、`subscriptionTier`、`overageEnabled`、`overageCreditsUsed` 并列），用于计费和配额归因。

---

## Custom Agent Registry

`CustomAgentRegistry`（类 `i20`）持有 `definitions` Map，通过 `generateCustomAgentId()` 生成唯一 id。来源文件分两类：
- Markdown 格式：`parseCustomAgentFile()` 调 `parseFrontMatter()` 读取 YAML front matter，schema 为 `CustomAgentFileFrontMatterSchema`
- JSON 格式：`parseJsonAgentFile()` 直接反序列化

`customAgentRegistry`（变量名 `o25`）作为 option 传入 `createAgentExecution()`，agent 运行时通过 registry 查找 subagent 定义。

内置两个特殊 agent：
- `CUSTOM_AGENT_CREATOR_DEFINITION`（`E19`）：创建新 custom agent 的 meta-agent
- `CUSTOM_AGENT_CREATOR_PROMPT`（`m18`）：对应系统提示词

`SubAgentDefinition`（`ye4`）继承自 `te7`，构造时 `type = d2.SubAgent`，供 agent FSM 识别并以子 agent 模式执行。

`CustomAgentContext` 在系统提示词模板中插值 `{{providerId}}` 占位符；找不到 provider 则 warn `[CustomAgentContext] Provider '${id}' not found`，异常时 error `[CustomAgentContext] Error resolving provider '${id}'`。

---

## UriHandler 与 DeepLink

`uriEventHandler` 在 `initializeStep("UriHandler")` 阶段调 `vscode.window.registerUriHandler(this)` 注册，随后 `authCallbackHandler.initialize(uriEventHandler.onUri)` 挂载 OAuth 回调监听。

注册的 URI authority 路由：

`kiro://kiro.oauth/callback`（`AUTH_CALLBACK_REDIRECT_URI`）：OAuth 授权回调，超时 `AUTH_FLOW_TIMEOUT_MS = 600000ms`（10分钟），警告 `AUTH_WARNING_TIMEOUT_MS = 60000ms`（1分钟）。`AuthCallbackHandler.pendingCallbacks` 以 state 参数为 key 存储等待中的 Promise resolve。

`kiro://kiro.repo/clone`（`register50`）：`registerRepoUriHandler()` 处理，解析 query 中的 repo URL 触发 clone 流程。

`kiro://kiro.resume-session`（`register51`）：`registerResumeSessionUriHandler()` 处理，`LOG_PREFIX = "[SessionResume]"`，解析 `uri.path` 中的 sessionId，恢复历史 agent session。未匹配 authority 直接 return，不做任何处理。

`IntentType` 枚举（AWS Glue Sensei 集成）：`ARTIFACT | DEEPLINKS | GLUE_SENSEI | RESOURCE_DATA | SUPPORT`，用于跨服务意图分发，与 Kiro URI handler 共享同一 `uriEventHandler` 事件总线。

---

## activate2 初始化顺序（完整 initializeStep 序列）

`activate2` 的核心逻辑是顺序调用 `initializeStep(name, callback)`，每步用 `withSpan(TelemetryNamespace.Application, "init.${name}")` 包裹，异常统一抛 `TrustedError`。完整初始化顺序如下：

NotificationService → SpecTelemetryService → UsageEventPolling（轮询 `kiro.usageLimits.getUsageLimits`）→ Storage（register3）→ Experiments（register6）→ Hooks（register36）→ Checkpoints（register4）→ Onboarding（register8）→ Logger（register2）→ Commands（register34，注入 webviewDependencies + agents）→ ExecutionLog（register46）→ RequirementsViewer（实验门控 `requirementAnalyzer`，register40）→ Spec（register47，注入 storage/specStorage/taskService/agentController/documentManagers/executionLogController）→ BackgroundProcesses（register43）→ Terminal（register37）→ Mcp（register48）→ Powers（register49，async void）→ AgentChat（register38）→ AcpDevInspector（实验门控 `acpChat`，async 动态 import `acp_dev_inspector_exports`）→ HooksEditor（register39）→ Configs（register35）→ ProfileStorage（registerProfileStorage）→ Auth（registerAuthProviderExtension，async void）→ EnterpriseSettingsManager（register45）→ ProfileFileWatcher（register58）→ Steering（register54）→ ContextLsp（register44）→ FirstTimeProject（register41）→ ImportSteering（register42）→ UsageLimitsEventEmitter（register21）→ StatusBarFeedbackItem（register52）→ UsageMonitoring（register55，内含嵌套步骤 StatusBarUsageMeterItem/register53 和 ResourceNotificationMonitor/register56）→ Model Selection（register57）→ UriHandler（uriEventHandler.register）→ AuthCallbackHandler（authCallbackHandler.initialize(uriEventHandler.onUri)）→ Repos（register50）→ SessionResume（register51）→ CustomAgentFileLoader（registerListCustomAgentsCommand + new CustomAgentFileLoader(customAgentRegistry).initialize()）→ ContextUsage（register59）→ Completions（registerAutocomplete）

`initializeStep` 之后还调用 `initializeAvailableModelsCache()` 初始化模型缓存。

`agentController.cleanup()` 通过 `new vscode.Disposable(...)` 注册到 `context.subscriptions`，扩展停用时自动执行，这是唯一的清理逻辑（没有独立的 `deactivate` 导出）。

`webviewDependencies` 对象包含：`context`、`editorApiFactory`（接收 document 返回 `createEditorApi`）、`executionLogController`。

---

## Storage（src/extension/storage）

`StorageManager` 类（`Metrics6 = new MetricReporter(TelemetryNamespace.Application, "storage")`）是全局单例 `Storage`，持久化路径由 VSCode `context.globalStorageUri` 提供，`ensureInitialized()` 保证首次访问前完成初始化。

`register3(context)` 读取 `vscode.workspace.workspaceFolders[0].uri` 作为 workspace 存储根目录，并注册两个 debug 命令：
- `kiroAgent.debug.openMetadata`（`registerOpenMetadataCommand`）
- `kiroAgent.debug.purgeMetadata`（`registerPurgeMetadataCommand`）

`IDEStorageError` 继承自 `KiroError`，定义存储操作的错误类型。

---

## Checkpoints（src/extension/checkpoints）

`CheckpointController` 接收 `storage` 和 `workTree` 两个依赖，管理 agent 执行过程中的文件系统快照。

注册了两个自定义 VSCode FileSystem Provider：
- `KIRO_DIFF_SCHEME = "kiro-diff"`：`CheckpointFileSystemProvider`，用于 Diff 视图展示检查点与当前文件的差异
- `MetaFileSystemProvider`：挂载元数据文件系统，供 agent 执行日志与状态查询

`register4(context)` 取 `vscode.workspace.workspaceFolders[0]` 作为工作区根，实例化 `Checkpointer`（全局单例），注册两个 provider 并 push 到 `context.subscriptions`。

`NotInitializedError` 继承自 `KiroError`，在 `Checkpointer` 未初始化时抛出。

---

## Onboarding（src/extension/onboarding）

`OnboardingService`（`vscode67`）管理首次使用引导流程。核心步骤通过 `steps/` 子目录实现：
- `alias-code.ts`：向 shell 配置（`~/.bashrc`/`~/.zshrc`）写入 `kiro` CLI alias，使用 `fs/promises` + `os` 模块定位 home 目录
- `cli-command.ts`：注册 `kiro` CLI 命令到 PATH，依赖 `os` 模块检测平台

`register8(context)` 注册 onboarding 命令，`OnboardingService` 通过 `import_shared_types17.TelemetryNamespace.Onboarding`（值 `"kiro.onboarding"`）上报引导事件。

`TelemetryNamespace` 枚举完整定义：`Application="kiro.application"` / `Feature="kiro.feature"` / `Continue="kiro.continue"` / `Agent="kiro.agent"` / `Tool="kiro.tool"` / `Parser="kiro.parser"` / `Onboarding="kiro.onboarding"` / `Webview="kiro.webview"` / `Auth="kiro.auth"` / `Billing="kiro.billing"` / `Profiles="kiro.profiles"` / `RemoteTools="kiro.remote-tools"` / `Spec="kiro.spec"`

---

## FirstTimeProject（src/extension/first-time-project）

`FirstTimeProjectDetectionService` 用 `workspaceState` 持久化 `kiro.firstTimeProject` 标记（`static FIRST_TIME_KEY`），通过 observer 模式通知其他模块。`register41(context)` 在 activate 阶段初始化该服务，检测当前工作区是否首次打开，首次时触发 Onboarding 相关 UI 提示。

---

## ImportSteering（src/extension/import-steering）

支持从主流 AI 助手的规则文件自动迁移到 Kiro Steering 格式。`AIAssistantType` 枚举定义支持的来源：`cursor`、`windsurf`、`amazon-q`、`cline`。

`AI_ASSISTANT_CONFIGS` 映射各工具的规则目录路径，Cursor 对应 `.cursor/rules/`，其余类似。

核心类链路：
- `GenericSteeringScanner`：扫描目标工具的规则目录（`workspaceUri/.cursor/rules/` 等），收集待迁移文件
- `CursorSteeringParser` / `GenericSteeringParser`：解析源文件 front matter，`CursorFrontMatterSchema`（Zod）校验 `description`、`globs`、`alwaysApply` 三个字段，`cleanContent()` 剔除 front matter 后提取正文
- `GenericSteeringFileGenerator`：将 `ConvertedSteeringFile[]` 写入 `.kiro/steering/` 目录
- `AIAssistantImportService`：整合上述三步，`register42(context)` 注册，`showImportResult(result)` 弹出导入结果通知（imported 数量 / failed 列表）

`GenericSteeringScanner` 构造时拼接 `workspaceUri + PRODUCT_CONFIG_DIRECTORY + STEERING_DIRECTORY` 作为目标写入目录。

---

## BackgroundProcesses（src/extension/background-processes）

`BackgroundProcessTerminalManager`：维护 `terminals[]` 列表，管理 agent 启动的后台终端进程的生命周期（创建、列举、销毁）。

`BackgroundProcessManager`（`DEFAULT_OUTPUT_LENGTH = 100`）：依赖注入 `BackgroundProcessTerminalManager`，负责进程的启动、停止、输出截取（默认最多 100 行）。

`BackgroundProcessManagerAdapter`：适配层，将 `BackgroundProcessTerminalManager` 包装成 ACP tool 可调用的接口。

`register43(context)` 实例化三个单例并注册到 `context.subscriptions`。

---

## Repos UriHandler（src/extension/repos）

`registerRepoUriHandler()` 监听 `uriEventHandler.onUri`，识别 `authority=kiro.repo` + `path=/clone` 的 URI，触发仓库克隆流程。`register50(context)` 仅调用该函数，无其他注册逻辑。

---

## ContextUsage（src/extension/context-usage）

`InitialContextEstimator`（`WARNING_THRESHOLD_PERCENTAGE = 30`，`HIGH_MCP_THRESHOLD_PERCENTAGE = 20`）：在 agent 启动前估算初始 context 占用，若剩余 context 低于 30% 触发警告，MCP tools 占用超过 20% 时单独提示。

`register59(context)` 注册 `kiro.contextUsage.getEstimate` 命令，执行 `estimateInitialContextUsage()` 返回估算结果供 WebView 展示。

---

## Enterprise（src/extension/enterprise）

`EnterpriseSettingsManager`（`cachedPolicies = {}`）通过 `codewhisperer-runtime` 客户端拉取企业策略，本地缓存后供 MCP 和 Auth 模块查询。

`McpRegistryLoader` 加载企业级 MCP 注册表，`VscodeWarningDisplay` 在加载失败时调用 `vscode.window.showWarningMessage`。`RegistryHttpFetcher`（`DEFAULT_TIMEOUT_MS = 30000`，`DEFAULT_MAX_PAYLOAD_BYTES = 10MB`）负责远端 registry JSON 的 HTTP 拉取，含超时和 payload 大小限制。

`register45(context)` 实例化全局单例 `EnterpriseSettings = new EnterpriseSettingsManager()`，并调用 `setEnterpriseSettingsManager()`、`setEnterpriseSettingsManager2()` 两个 setter 将实例注入到 shared-types 和 kiro-client 的全局上下文。

---

## SessionResume（src/extension/session-resume）

支持通过外部 URI 恢复已有 agent 会话，典型场景是从 Web 端或邮件链接直接跳入 IDE 继续之前的对话。

`SessionZipExtractor`：从 S3 预签名 URL 下载 session 压缩包（`ALLOWED_S3_DOMAINS = ["s3.amazonaws.com", "s3-accelerate.amazonaws.com"]`），对 URL 做安全过滤（`DANGEROUS_PATTERNS = ["$(", "\`", ";", "|", "&&", "\n", "\r", "\0"]`），解压到系统 temp 目录后恢复 workspace 状态。`decodePresignedUrl(base64Url)` 对 base64 编码的 URL 解码。

`SessionResumeError` 包含 `code`、`message`、`cause` 三个字段。

`registerResumeSessionUriHandler()` 监听 `uriEventHandler.onUri`，识别 `authority=kiro.session-resume` 的 URI，触发下载 → 解压 → 恢复流程。`register51(context)` 仅调用该函数。

---

## Usage Limits（src/extension/usage）

`UsageLimitsAvailabilityCache`：依赖注入 `authProvider`，`initializeEventListeners()` 监听登录状态变化，登录后自动拉取最新用量数据。`UsageLimitsEventEmitter` 包装 `vscode.EventEmitter`，触发 `_onDidUpdateUsageLimits` 事件通知 WebView 刷新。

`initializeLoginEventListener(context)` 监听 `authProvider.onDidChangeLoginStatus`，登录成功时执行 `kiro.usageLimits.getUsageLimits` 命令拉取用量。`UsageMonitoring` 结合 `ProfileApprovalService` 做用量上报。

---

## StatusBar 扩展项（src/extension/status-bar）

除主 StatusBar（setupStatusBar）外，还有两个附加项：

`StatusBarFeedbackItem`（`FEEDBACK_ITEM_ID = "kiro.status.feedback"`）：右下角显示 `$(bug) Report issue`，点击触发 `kiroAgent.fileFeedback` 命令。

`StatusBarUsageMeterItem`：`getDecoratedUsageMeterText(text)` 拼接 `$(graph) {text}` 作为显示文本，实时展示 token 用量百分比。

---

## ModelSelection（src/extension/model-selection）

`availableModels = { models: [], defaultModel: { id: "", name: "" } }`：模块级全局缓存，`setAvailableModels(models)` 更新后自动触发 `kiro.updateModelsList` 命令通知 WebView 刷新下拉列表。

`getModelsList()` 返回 `getAvailableModels().models`，`InvalidModelIdentifierError` 继承自 `KiroError`，在配置了非法模型 ID 时抛出。

`register57(context)` 调用 `initializeModelConfigProvider()`，注册 `kiro.agentModels.getModelsList` 命令。

---

## Notifications（src/extension/notifications）

`NotificationSettingsManager`（`NOTIFICATION_CONFIG = \"notifications\"`，`CONFIG_NAMESPACE = ConfigNamespace.Extension`）：读写用户通知偏好设置（静音、类型过滤等）。

`NotificationService`（`Metrics30 = MetricReporter(TelemetryNamespace.Application, \"notifications\")`）：持有 `agentController` 引用，在 agent 完成任务、出错等关键节点触发系统级通知，上报 Telemetry。

依赖 `storageProvider`（`globalState.get/update`）持久化已推送过的通知，避免重复弹出。

`register56(context, initialEvent)` 注入 `storageProvider`，实例化并启动 `NotificationService` 和 `ResourceNotificationService`。

---

## Capabilities（src/extension/capabilities）

`client-tools.ts` 定义所有可供 Agent 调用的工具，注册到 `ToolRegistry`。工具分两类：

本地工具（通过 `EditorApi` 直接操作 VSCode）：`readFile` `writeFile` `searchFiles` `listDirectory` `runTerminal` `createFile` `deleteFile` `renameFile` `moveFile` `openFile` `diffFile` `insertContent` `replaceContent`。

远端工具（通过 ACP 协议转发到 Kiro 后端）：通过 `discoverRemoteTools()` 在 activate 后异步拉取，写入 `remoteToolsCache`，后续 Agent 请求时按名称匹配。

`capabilities/create-workspace-command.ts` 导出 `kiroAgent.createWorkspace` 命令，调用 `createWorkspace(uri)` 在指定路径初始化新工作区。

---

## AgentChat（src/extension/agent-chat）

`agent-chat-view-provider.ts`：WebviewViewProvider，挂载在 `kiroAgent.acpChatView`，实现会话队列与历史管理。核心字段：`chatSessionQueue`（待处理会话列表）、`activeChatSession`（当前会话）、`sessionHistory`（已完成会话）。

`session-manager.ts`：`getOrCreateChatSession(sessionId?)` 根据 id 查找或新建 `ChatSession`；`loadChatSession(sessionId)` 从持久化存储恢复历史会话并推送到 WebView；`kiroAgent.viewHistoryChats` 命令触发历史列表渲染。

`config-sync.ts`：监听 `kiroAgent.agentAutonomy`、`kiroAgent.modelSelection`、`kiroAgent.usageSummary` 配置变更，通过 `postMessageToWebview({ type: 'kiroSettingsUpdate', ... })` 实时同步到 Chat WebView。

---

## Config（src/extension/config）

`register35(context)` 初始化多工作区配置体系：每个工作区创建独立的 `WorkspaceConfig` 实例（基于 `ScopedWorkspaceResourceCollection`），并调用 `registerSettingsContextSync(configStore)` 将工作区配置变更同步到 VSCode 上下文键（`setContext`），供 `when` 表达式使用。

`config-keys.ts` 集中定义所有配置键名（`AUTONOMY_MODE`、`MODEL_SELECTION`、`USAGE_SUMMARY` 等）。`workspace-trust-warning.ts` 在工作区信任状态变更时弹出警告通知。`setting-context-sync.ts` 监听具体配置项并 `vscode.commands.executeCommand('setContext', key, value)` 同步到扩展上下文。

---

## ContextResolvers（src/extension/context-resolvers）

每个 resolver 是一个异步函数，接受字符串参数返回文本内容或 `Error`：

`fileContextResolver(path)`：用 `vscode.workspace.fs.readFile` 读取文件字节，`TextDecoder` 解码返回字符串；读取失败返回 `Error('Unable to read file: ...')`。`folderContextResolver(path)`：枚举目录下文件树。`steeringContextResolver`、`specContextResolver`、`mcpContextResolver` 分别对应 Steering 文档、Spec 文件和 MCP 上下文的解析器，注册到统一的 resolver registry 后由 ACP 消息处理层按类型分发调用。

---

## ContextLSP（src/extension/context-lsp）

`register44(context)` 为 `@provider-name` 语法提供完整的语言服务支持：

`ContextReferenceCompletionProvider.register(context, registry)`：在用户输入 `@` 时触发，列出所有已注册 Context Provider 的名称。`ContextReferenceHoverProvider.register(context, registry)`：hover 在 `@name` 上时展示 Provider 描述。`ContextReferenceLinkProvider.register(context, registry)`：将 `@name` 渲染为可跳转的 DocumentLink。`ContextProviderSemanticTokensProvider.register(context)`：对 `@name` 应用语义 token 着色（`TokenTypes.ContextProvider`），在编辑器中高亮显示。

---

## AgentEventPolling（src/extension/polling）

`AgentEventPollingService`：长轮询服务，通过 `MetricReporter(TelemetryNamespace.Application, 'agent-event-polling')` 上报指标。构造参数为 `agentController`，订阅其事件流；内部维护轮询间隔和重试逻辑，将后端推送的 Agent 事件（任务进度、工具调用结果、错误）转发给 WebView 和本地状态机。

---

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                         activate2 (Kiro 主入口)
│                                                                              │
│  setupCa()  → 平台证书注入                                                   │
│      │                                                                        │
│      ▼                                                                        │
│  registerExtension()                                                         │
│    telemetry / powers / profile / config 监听                                 │
│      │                                                                        │
│      ▼                                                                        │
│  dynamicImportAndActivate()                                                   │
│      │                                                                        │
│      ├──────────────────────────────────────────────────────────────────────► │
│      │                                                                        │
│      │          ┌──────────────────────────────────────────────────────┐      │
│      │          │ Continue Extension                                   │      │
│      │          │ activateExtension(context)                           │      │
│      │          └──────────────────────────────────────────────────────┘      │
│      │                                                                        │
│      │  ◄────────────────────────────────────────────────────────────────────┤
│      │   激活成功 / 失败提示 View Logs / Retry                                 │
│      ▼                                                                        │
│  initializeAvailableModelsCache()                                             │
│      │                                                                        │
│      ▼                                                                        │
│  remoteToolsDiscovery.discoverTools()  (async)                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Agent 执行引擎

执行队列事件状态机（`ExecutionQueue`）：

```
Queued → Began → Resumed ⇄ Yielded
                         ↓
              SaveState / SummarizeUsage
                         ↓
              Success / Failed / Aborted
                         ↓
              ContextUsageUpdate
```

内置 subagent 类型：`context-gatherer`（探索陌生代码库，每次查询只调用一次）、`general-task-execution`（并行子任务）、`custom-agent-creator`（创建自定义 agent）。

Workspace 工具调用层：`readFile` / `writeFile`（含 createFile 标志）/ `searchFiles`（支持 `path:line-range` 语法）/ `runTerminal`。

---

## Steering 控制器

`.kiro/steering/` 下的 `.md` 文件，front matter 由 `SteeringContextFrontMatterSchema`（Zod）解析：

- `inclusion: always` — 永远注入上下文（默认）
- `inclusion: fileMatch` + `fileMatchPattern: 'README*'` — 仅当匹配文件被读入上下文时触发
- `inclusion: manual` — 只能手动 @mention
- `inclusion: auto` — 由 agent 自动决策是否引入

`kiro.steering.createInitialSteering` 触发时新建 `.kiro/steering/` 目录，启动 `generate-steering` agent session，让 agent 分析工作区后自动写入初始规则。`listSteeringUris()` 枚举全局与工作区两处目录，合并返回所有有效 URI。

```text
用户触发 createInitialSteering
        │
        ▼
创建 .kiro/ + .kiro/steering/
        │
        ▼
chatAgent({ agentMode: 'generate-steering' })
        │
        ▼
agent 分析工作区 → 写入 .md 文件
        │
        ▼
FileSystemWatcher 检测变更 → listSteeringUris() 刷新
```

---

## Spec 控制器

Spec 文档组织在 `.kiro/specs/<featureName>/` 下，每个 feature 包含四种文档：

- `requirements.md` — 需求说明（bugfix 场景改为 `bugfix.md`）
- `design.md` — 设计文档
- `tasks.md` — 任务分解

`SpecDocumentManager` 通过 `FileSystemWatcher` 监听 `.kiro/specs/` 目录，`listFeatures()` 返回所有子目录名。Webview 与 Extension 之间通过 `readRangeInFile` / `saveFile` / `focusEditor` 消息通信。

```text
kiro.spec.createDocument (Webview 消息)
        │
        ▼
SpecDocumentManager
  ├─ listFeatures()        → 枚举 .kiro/specs/ 子目录
  ├─ readRangeInFile       → 读取 requirements/design/tasks.md
  ├─ saveFile              → 写入文档变更
  └─ focusEditor           → 在编辑器中定位到对应文件
        │
        ▼
RequirementsView WebviewPanel (requirements-webview/)
  └─ requirementAnalyzerEnabled → 触发 AI 分析
```

---

## Hook 控制器

Hook 配置文件存放在 `.kiro/hooks/` 下（`.json` 格式），文件路径即为 hook id。`HookController` 实现 `TreeDataProvider`，在侧边栏展示 hook 列表，支持增删改查。`HookRunner` 在 `extension.js` 中处理触发事件分发。

**IDEListenableEvent 枚举（when.type 取值）**
`fileEdited` — 文件被编辑后触发（目前唯一正式枚举值，`fileCreated`/`fileDeleted`/`fileRenamed` 来自 hook-editor 包的扩展枚举）。

**HookActionEvent 枚举（then.type 取值）**
`askAgent` — 触发 Agent 会话，执行器为 `AskAgentHookExecutor`，调用 `loadSessionWithPrompt` 命令发起新 session。
`runCommand` — 在新 terminal session 执行 shell 命令，执行器为 `RunCommandHookExecutor`，保存时自动调用 `approveHookCommand()` 记录信任状态。

**HookSchema（Zod，`validateHookStructure` 校验）**
```
{
  name: string (min 1),
  version: string (min 1),
  when: { type: IDEListenableEvent, ... },
  then: { type: HookActionEvent, timeout?: number (>=0), ...passthrough }
}
```

**Hook 命令列表**
`hooks.create` / `hooks.read` / `hooks.delete` / `hooks.setEnabled` / `hooks.getEnabled` / `hooks.trigger` / `hooks.openUI`

启用状态持久化在 workspaceState，key 格式为 `kiroAgent.hooks.{id}.enabled`，删除 hook 时同步清除该 key。

```text
文件系统事件 / agent 执行事件
        │
        ▼
HookRunner.dispatch(event)
        │


---

## RichExecutionLog 体系

`rich-execution-log/` 模块负责 Agent 执行过程中的文件活动追踪和 Session 持久化。

`AgentActivityFileDecorationProvider` 实现 `vscode.FileDecorationProvider`，维护 `fileActivityRecord`（`executionId → {filePath: activityType}` 映射）。当 Agent 修改文件时，通过 `updateFileActivityRecord(newRecord)` 触发 `_onDidChangeFileDecorations` 事件，使资源管理器中被修改的文件显示彩色角标。

`adaptActionToMessages(action, executionId)` 把 Agent Action 对象压缩为最小序列化格式（只保留 `executionId`、`actionId`、`actionType`、`actionState`、`startTime`、`endTime`、`errorMessage`），通过 `setActionToMessagesImpl` 注入 ExecutionLog V2 持久化层。初始化失败时只打 warn 日志，不中断流程。

`register46` 是该模块的注册入口，聚合 ExecutionLog View、FileDecorationProvider 和 Session 持久化的初始化。

---

## contextual-spec 模块

`contextual-spec/` 提供从编辑器上下文直接触发 Spec 生成的能力。

`invokeSpecAgent(newSpecProps, agents)` 检查 `vscode.workspace.workspaceFolders` 是否存在，调用 `agents.specAgent` 发起 Spec 会话。`generateInitialSpec` 是其包装层，额外调用 `Feature.reportUsage("spec-generation")` 上报用量。

`register5` 注册命令 `kiroAgent.initiateSpecCreation`，参数 `newSpecProps` 直接透传给 `generateInitialSpec`，是右键菜单「从选中内容生成 Spec」的入口点。

---

## utils 工具层


## spec-editor 模块

`spec-editor/` 是 extension.js 中体量最大的子模块，完整实现了 Kiro 的 Spec 驱动开发工作流。

**SpecDocumentManager**：以 `workspaceUri` 为根，扫描 `.kiro/specs/` 目录，维护 feature → document 映射。`setupFileSystemWatcher()` 监听 `.kiro/specs/**` 变化，自动同步内存状态。核心方法：`createFeature`、`createDocument`、`listFeatures`、`getDocumentsForFeature`、`getPairingDocumentForUri`。

**SpecExplorerView**：实现 `TreeDataProvider`，将 `.kiro/specs/` 以树状结构展示在侧边栏，Feature 节点可展开查看 requirements/design/tasks 三个文档。

**MarkdownTaskCodeLensProvider**：解析 tasks.md 的 CheckBox，在每个任务行上方注入 CodeLens（Run Task / Run All / Chat to Fix）。`updateTaskStatus` 通过 `taskService.associateTaskExecution` 和 `updateTaskMetadata` 写入持久化层，触发 `changeType: 'status_changed'` 通知 UI 刷新。

**TaskService**：以 `tasksPath.uri` 为 key，维护 `taskId → executionId → ExecutionStatus` 多层映射，`trackDocumentForExecution` 注册追踪。

**SpecTraceDatabase**：基于 `better-sqlite3` 存储 Spec 执行轨迹，每次 Agent 执行后写入，供 ExecutionLog 视图查询回放。

**注册命令**（`register47`）：`kiroAgent.spec.explorerCreateSpec`、`explorerDeleteSpec`、`explorerRenameSpec`、`generateSpecDocument`、`executeTask`、`runAllTasks`、`navigateDocuments`、`editorImplementSpec`、`chatToFixPbt`。

**Hover Providers**：`PbtHoverProvider`、`PropertyHoverProvider`、`RequirementsHoverProvider` 注册到 `*.md` 文件，提供标注说明。

**SpecConfigManager**：读写 `.kiro/specs/<feature>/.config.kiro`，存储 Feature 级配置（model override、并发数等）。

**SpecTelemetryService**：封装所有 Spec 埋点，`DimensionBuilder` 构建事件维度，`EventFilter` 过滤重复事件，`SpecSessionState` 追踪状态机转换。

## contextual-spec 模块

`contextual-spec/` 实现从右键菜单或命令面板快速生成 Spec 的入口。

**invokeSpecAgent**（`invoke-spec-agent.ts`）：接收 `newSpecProps` 和 `agents` 实例，取 `workspaceFolders[0]` 作根目录，将 `initialContext` 转换为 `mappedContextItems` 后调用 `agents.specAgent.invoke()`。无工作区时直接 return。

**generateInitialSpec**（`generate-initial-spec.ts`）：薄封装，先调 `Feature.reportUsage('spec-generation')` 上报 telemetry，再透传参数给 `invokeSpecAgent`。

**command.ts**：注册 `kiroAgent.generateInitialSpec`（通过 `showInputBox` 收集 featureName）和 `kiroAgent.contextualSpec.fromSelection`（把编辑器选中文本作为 initialContext）两条命令。

---

## platform 模块

`platform/vscode-file-system.ts` 实现 `VSCodeFileSystem` 类，是 Kiro 对 VSCode 文件系统 API 的统一封装，供 Agent 工具调用。

构造时接受可选 `workspaceRoot`，默认取 `vscode.workspace.workspaceFolders?.[0]?.uri ?? vscode.Uri.file(process.cwd())`。核心方法：`resolveUri` 解析相对路径、`readFile/writeFile` 包装 `vscode.workspace.fs`、`stat`、`readDirectory`、`delete`（支持 recursive）、`rename`、`exists`。所有操作走 VSCode 虚拟文件系统，天然支持远程工作区。

---

## q-custom-model 模块

`q-custom-model/q-developer-converse-factory.ts` 实现 Q Developer 自定义模型的 Converse 工厂。

**qChatLogger**：懒初始化 OutputChannel（名称 `Q Chat API`），维护 `logBuffer` 和 `conversationIds` 供调试。

**工厂函数**：接收 `credentialsProvider` 和 `modelId`，构建 `ConverseStreamCommand` 请求，将每个 streaming chunk 转换为统一 `ChatMessage` 格式后 yield，`conversationIds` 追踪每次对话 ID 供 telemetry 关联。

---

## utils 工具层

**get-agent-controller.ts**：维护模块级单例 `globalAgentController`，`setAgentController` 在 `activate2` 写入一次，`getAgentController` 在各处取用，未初始化时抛错。

**trigger-agent.ts**：`triggerAgent(definition, signal)` 是所有命令触发 Agent 的统一入口，通过 `ContextPropagation.withContextValues` 注入 `TelemetryAttributes.ConversationId`，再调 `agentController.trigger()`。

**trusted-tools.ts**：`trustTool(toolName)` 读取 `ConfigKey.TrustedTools` 配置，合并新工具名后写回；`isTrustedTool` 做 includes 检查。

**validate-file-access.ts**：`validateFileAccess(uri)` 检查路径是否在 workspace 根目录下，解析符号链接后前缀比较，超出范围返回 `AccessDenied`。

**scoped-workspace-resource-collection.ts**：`ScopedWorkspaceResourceCollection<T>` 以 workspaceFolder URI 为 key 管理资源，监听 `onDidChangeWorkspaceFolders`，folder 移除时自动 dispose 对应资源。

**activation-attempts.ts**：记录激活重试次数到 `globalStorageUri/activation-attempts.json`，超阈值时展示错误通知并停止重试。

**validate-file-ignore.ts**：读取 `.gitignore` / `.kiroignore`，用 `ignore` 包检查文件是否应被 Agent 忽略。

**with-abort.ts**：`withAbort(signal, fn)` 封装可中止异步操作，signal 已 abort 时立即 reject `AbortError`，否则执行 fn 并在 abort 时清理。


---

## commands 模块

`commands/` 是所有顶级命令的聚合注册中心，`register34` 依次调用子注册函数：

- `register9`（`agent/index`）：`kiroAgent.chat`、`kiroAgent.compactAgent`、`kiroAgent.retryAgent`、`kiroAgent.createHook`
- `register11`（`create-debug-log-zip`）：打包 logs 目录为 zip，调用 `showSaveDialog` 让用户选路径
- `register12`（`debug-capture-llm-log`）：捕获最近一次 LLM 请求/响应 payload 到剪贴板
- `register26`（`configuration/index`）：`kiroAgent.openSettings`、`kiroAgent.resetSettings`
- `register13`（`enable-shell-integration`）：引导用户开启 VSCode shell integration
- `register14`（`file-feedback`）：对当前文件发送正/负反馈到后端
- `register15`（`get-codewhisperer-config`）：返回 CodeWhisperer 兼容配置供第三方扩展查询

`commands/profiles/`、`commands/source-control/`、`commands/subscription-plans/`、`commands/telemetry/`、`commands/usage-limits/` 各自注册的命令已在对应章节覆盖。

`utils/` 下有三类工具：

`directory-sizer.ts`：`getDirSize(uri)` 递归遍历 `vscode.workspace.fs.readDirectory` 统计目录总大小和文件数，用于 Indexing 前的规模评估。

`file/file-utils.ts`：`fileExists(uri)` 用 `vscode.workspace.fs.stat` 检测文件存在性，捕获异常返回 `false`。`InteractionError` 继承自 `KiroError`，用于 Capabilities 工具执行失败时的错误分类。

`get-agent-controller.ts`：维护全局单例 `globalAgentController`，通过 `setAgentController` / `getAgentController` 提供跨模块访问入口，是 Agent 触发链路的核心依赖。

`get-execution-logger.ts`：通过 `vscode.extensions.getExtension(EXTENSION_FULL_NAME).exports.getExecutionLogger()` 获取 ExecutionLogger 实例，失败时抛异常。

`trigger-agent.ts`：`triggerAgent(definition, signal)` 读取当前模型 ID，通过 `ContextPropagation.withContextValues` 注入 `TelemetryAttributes.ConversationId` 后调用 `agentController.triggerAgent`，是所有外部触发 Agent 的统一入口。

---

## platform 模块

`platform/` 封装与操作系统和 VSCode 宿主环境的交互，提供平台类型检测（`isMac` / `isWindows` / `isLinux`）、VSCode 版本兼容判断、`ExtensionContext` 路径解析（`globalStorageUri`、`extensionUri`）。这些工具函数被 Auth、Storage、Telemetry 等多个模块直接 import 使用。

---

## q-custom-model 模块

`q-custom-model/` 支持 Amazon Q 自定义基座模型的配置和切换。

`QCustomModelManager` 维护用户配置的自定义模型列表，通过 `kiroAgent.qCustomModel.*` 配置键读取模型端点、API Key、modelId。注册以下命令：`kiroAgent.addCustomModel`（添加模型）、`kiroAgent.removeCustomModel`（删除模型）、`kiroAgent.selectCustomModel`（切换活跃模型）。模型切换后通过 `postMessageToWebview({ type: 'modelChanged' })` 通知 Chat WebView 刷新模型选择器。


## ACP Dev Inspector（src/extension/acp-dev-inspector）

调试工具，仅在开发模式下启用。`registerView(context)` 通过 `vscode.window.registerWebviewViewProvider(VIEW_ID, new AcpDevInspectorProvider(extensionUri))` 注册一个 WebView 侧边栏面板，用于实时展示 ACP 消息流。`initializeInspector()` 初始化消息拦截器，依赖 `acp-message-tap.ts` 在 ACP 客户端发送/接收每条消息时触发回调，将消息结构化后推送到 Inspector WebView，供开发者检查协议细节。

---

## SubscriptionPlans（src/extension/commands/subscription-plans）

三个计费命令，均通过 `registerInstrumentedCommand` 注册，用 `MetricReporter(TelemetryNamespace.Billing, ...)` 上报遥测：

`kiro.subscriptionPlans.getPortalSessionUrl`（`register20`）：调用 CodeWhisperer Runtime 获取计费门户的会话 URL，在浏览器中打开供用户管理订阅。`kiro.subscriptionPlans.getSubscriptionPlans`（`register22`）：拉取可用计划列表，`transformOutput2` 将响应规范化为 `{ subscriptionPlans, disclaimer }` 结构，返回给 WebView 展示。`kiro.subscriptionPlans.getCheckoutSessionUrl`（`register24`）：获取 Checkout 会话 URL，引导用户完成订阅购买流程。

---

## EditorAPI（src/extension/editor/editor-api）

`createEditorApi(document)` 工厂函数，为每个文档实例化编辑器操作 API，供 ACP 工具调用层使用。核心子模块：

`apply-to-code.ts`：`applyToCode(specDocument)` 调用 `vscode.commands.executeCommand('kiroAgent.spec.editorImplementSpec', Uri.parse(specDocument.documentUri))` 将 Spec 实现应用到编辑器。`create-workspace-edit.ts`：封装 `WorkspaceEdit` 创建和应用逻辑，支持多文件批量编辑。`open-file.ts`：`openFile(path)` 用 `vscode.window.showTextDocument` 在编辑器中打开指定文件并可选跳转到行号。`get-file-content.ts`、`list-directory.ts`、`get-workspace-structure.ts`：分别实现文件内容读取、目录枚举和工作区结构扫描，作为 ACP 工具的底层实现。

---

## Powers API（src/extension/powers）

Powers 是 Kiro 的扩展插件体系（类似 VS Code 扩展的扩展）。

`ui-interaction.ts`：`powerWebviewProviders` 注册表保存各 Power 的 WebView Provider，`setPowerWebviewProviders(providers)` 在 activate 时由主入口注入。`openExternal({ url })` 调用 `vscode.env.openExternal` 在系统浏览器中打开链接，供 Power 的 UI 使用。`powerDetailsPanel` 单例管理 Power 详情 WebView Panel 的生命周期。

`list-powers.ts`：`normalizePower2(power)` 规范化 Power 描述对象（补全缺失字段，默认 `description = 'No description available'`），返回供 UI 展示的标准化列表。

`powers/registry-v2/api/get-power-details.ts`：通过 `node:fs/promises` 读取 Power 的 manifest JSON（支持 `comment-json` 格式），返回完整的 Power 元数据（名称、版本、工具列表、权限声明等）。
        ├─ 匹配 .kiro/hooks/*.json 中的触发条件
        │
        ▼
执行 hook 动作（shell 命令 / agent 调用）
        │
        ▼
HookController (TreeDataProvider) 刷新侧边栏 UI
```

---

## Powers 控制器

三级 Registry 架构：

```text
查询 getPower(registryId, powerName)
        │
        ├─ KiroRecommended（官方推荐，内存缓存）
        ├─ UserAdded（用户手动添加）
        └─ UserRegistry（用户自定义仓库，远程请求）
                │
                ▼
        安装 / 卸载
                │
        triggerOnPowerInstalledEvent
        triggerOnPowerUninstalledEvent
                │
                ▼
        installedPowersListViewProvider 刷新 UI
```

V1→V2 迁移：读取旧 `registry.json`，按 source 类型重新分配，备份为 `.v1.backup`。

---

## MCP 控制器

`MCPConfigManager`（类名 `Tl`）负责读取和监听所有 MCP 配置文件，`MCPServerPool`（`Fl`）维护实际连接池。

配置文件路径优先级（低→高，后者覆盖前者）：

用户级：`~/.kiro/settings/mcp.json`，工作区级：`<workspaceRoot>/.kiro/settings/mcp.json`（多根工作区每个目录各一份，按顺序合并）。

`getActiveMcpConfigLocation(workspaceDirs)` 收集所有存在的路径，返回 `{ workspaceConfigPaths[], userConfigPath }`。`MCPConfigManager.setWorkspacePaths()` 在 session 创建时被调用，触发 FileSystemWatcher 重建；配置变更后通过 `onConfigChanged` 回调通知 agent，agent 调用 `mergeServers(fileBasedServers, clientMcpServers)` 合并文件配置与客户端传入的 MCP 服务器，再调用 `updateMcp()` 更新连接池并 emit MCP 状态。

每条服务器配置通过 `Yi2(name, config)` 校验，非法条目只 warn 不阻断。Powers 安装的 MCP 服务器写入 `~/.kiro/settings/mcp.json` 的 `powers.mcpServers` 段，`readPowersConfig()` 单独读取该段合并进全局列表。`autoApprove` 字段支持精确工具名或通配符 `*`，`isPowerToolApproved()` 读取用户级 mcp.json 验证。

```text
session 创建 / 配置文件变更
        │
        ▼
MCPConfigManager
  ├─ getConfigPaths()        → 用户级 + 各工作区 mcp.json
  ├─ readConfigFile(path)    → JSON.parse → Zod 校验 → Yi2 逐条验证
  ├─ readPowersConfig()      → powers.mcpServers 段
  └─ onConfigChanged(cb)     → 变更去重（序列化对比）
        │
        ▼
mergeServers(fileBasedServers, clientMcpServers)
        │
        ▼
MCPServerPool.updateMcp()   → 建立 / 复用 / 关闭连接
        │
        ▼
emitMcpStatus(sessionId)    → 推送状态到 WebView
```

---

## Skill 控制器

Skill 存放于 `.kiro/skills/<skillName>/SKILL.md`（工作区）或 `~/.kiro/skills/<skillName>/SKILL.md`（全局），文件名固定为 `SKILL.md`，目录名即 skill name。

`ProgressiveContextLoader` 在启动时调用 `scanAll()` 扫描全局与各工作区 skills 目录，并对 `**/SKILL.md` 和 `*` 建立 FileSystemWatcher（debounce 300ms）。每个 `SKILL.md` 必须有 YAML front matter 且包含 `name` 和 `description` 字段，缺失则 warn 跳过。

导入方式（`SKILL_IMPORT_METHOD_ITEMS`）：
- GitHub URL（指向含 SKILL.md 的子目录，不能是仓库根目录）：`importSkillByUrl()` → git clone → sparse-checkout 指定子路径
- 本地文件夹：`importSkillByFolder()` → 文件夹选择对话框 → 复制到 skills 目录

```text
扫描 / FileWatcher 触发
        │
        ▼
ProgressiveContextLoader.scanAll()
  ├─ getGlobalSkillsDirectory()     → ~/.kiro/skills/
  └─ getWorkspaceSkillsDirectories() → .kiro/skills/ × 每个工作区
        │
        ▼
读取 SKILL.md → 解析 frontMatter → 校验 name + description
        │
        ▼
注册到 ContextEngine（供 agent session 引用）
```

---

## Profile 选择链路

Profile 适用于 IAM Identity Center（Enterprise）账号，`supportsProfiles(token)` 检查 token 是否具备该能力。

`handleProfiles(token)` 在账号校验阶段被调用：执行 `kiro.profiles.listAvailableProfiles`（传 `accessToken` + `idcRegion`）获取 profile 列表；列表只有一条时直接执行 `kiro.profiles.selectProfile`；多条时执行 `kiro.profiles.showProfileSelector`（`isDismissible: true`）弹出选择器。

`ProfileStorage.initializeInstance(context)` 在 `registerProfileStorage()` 中初始化，遥测通过 `ProfileStorageMetrics`（`MetricReporter(TelemetryNamespace.Profiles, "ProfileStorage")`）上报读写成功/失败计数，`recordProfileStorageEvent(event, success, errorType, dimensions)` 是具体上报入口。

```text
登录后 handleProfiles(token)
        │
        ├─ supportsProfiles(token) → false → 跳过
        │
        ▼
kiro.profiles.listAvailableProfiles(accessToken, idcRegion)
        │
        ├─ 1 条 → kiro.profiles.selectProfile(profile)
        │
        └─ 多条 → kiro.profiles.showProfileSelector({ isDismissible: true })
                          │
                          ▼
                  用户选择 → selectProfile → ProfileStorage 持久化
```

---

## 遥测架构

`TelemetryNamespace` 枚举定义上报域：`kiro.application` / `kiro.feature` / `kiro.agent` / `kiro.tool` / `kiro.auth` / `kiro.billing` / `kiro.profiles` / `kiro.remote-tools` / `kiro.spec` 等。

`MetricReporter` 是通用上报封装，当前已实例化的 reporter：`ChatUIMetrics`、`PowersMetrics`、`ProfileStorageMetrics`、`McpRegistryMetrics`、`ToolUsage`。`initializeAgentTelemetry()` 在 `activate2()` 最早阶段调用，初始化 tracer、meter 和 journeyTracker。

---

## Steering 文档体系

Steering 是 Kiro 的「规则文件」机制，对应用户在 `.kiro/steering/` 目录下放置的 `.md` 文件，以及 workspace 根目录的 `AGENTS.md`。

`NodeSteeringDocumentSource`（`W14`）是 Node.js 侧的文件系统读取器，核心常量：

- `E10 = "steering"`、`I11 = ".md"`、`S12 = "AGENTS.md"`、`R6 = "skills"`、`_10 = "SKILL.md"`、`c12 = "specs"`

文件发现逻辑：`listSteeringFilePaths()` 合并三路来源——workspace `.kiro/steering/*.md`、全局 `~/.kiro/steering/*.md`、workspace 根目录 `AGENTS.md`。`workspaceTrusted` 为 false 时只保留 `scope === "global"` 的文档。

加载逻辑：`basename === "AGENTS.md"` 走 `loadAgentsMdDocument()`，固定 `inclusion: "always"`、`scope: "workspace"`；其余走 `loadSteeringDocument()`，读取文件 front-matter 决定 inclusion 和 scope。

文件监听：chokidar watcher 同时监听 home 和所有 workspace，fileFilter 只放行 `.kiro/steering/` 路径或文件名为 `AGENTS.md` 的变更。

执行时注入：`ae9()` 函数在每轮执行前调用 `getSteeringDocuments()`，过滤出 `included` 文档并附加到请求 `_meta.kiro.steeringDocuments`。

---

## Powers（MCP servers 用户层配置）

`loadPowersMcpConfig()` 读取 `~/.kiro/settings/mcp.json` 的 `powers.mcpServers` 段，经 `expandEnvironmentVariables()` 展开环境变量后与 workspace 级 MCP 配置合并，最终传给 MCP 注册表。`readPowersConfig()` 是同步版本，供 `NodeSteeringDocumentSource` 内部使用。

合并优先级：user-level powers.mcpServers → workspace mcp.json mcpServers，同名 key workspace 覆盖 user。

---

## VSCode Authentication Provider 注册

扩展在 activate 阶段调用 `vscode.authentication.registerAuthenticationProvider(PROVIDER_ID, "Kiro", extensionInstance)`，将 `AuthProviderExtension` 注册为 IDE 原生认证提供者。

`AuthProviderExtension` 内部：
- 持有 `SignInController` 实例，通过 `authProvider.onDidChangeLoginStatus` 监听登录状态变更。
- 登录成功时 fire `_onDidChangeSessions` 并携带 `added: [AuthProviderSession(token.provider)]`；登出时 fire `removed: [AuthProviderSession()]`。
- VSCode 借此感知认证状态，其他扩展可通过 `vscode.authentication.getSession(PROVIDER_ID, scopes)` 获取 token。

`AmazonQDeveloper`（provider: `qdev`）是 continuedev 层的 LLM 适配器，调用 `codewhisperer-streaming-client` 的 `generateAssistantResponse`，请求前注入三个中间件：`addPrivacyHeadersMiddleware`、`addAgentModeHeadersMiddleware`（标记为 `autocomplete`）、`addExternalIdpTokenTypeMiddleware`（携带 authMethod）。`Agent4`（provider: `kiro`）继承自 `AmazonQDeveloper`，用于 Agent 对话模式。

---

## Diff 体系

两种 Diff Manager 并存：

**DiffManager（水平 diff）**：通过 `kiroAgent.acceptDiff` / `kiroAgent.rejectDiff` 命令驱动，操作完整文件级 diff，内部调用 `verticalDiffManager.clearForFilepath(path, accept)` 联动清理。

**VerticalDiffManager（垂直 diff）**：`VerticalDiffHandler` 以行为单位流式插入/删除，维护 `editorToVerticalDiffCodeLens`（Map，key 为 filepath，value 为 `{start, numRed, numGreen}[]`）。每个 diff block 结束后调用 `refreshCodeLens` 触发 CodeLens 重绘。`kiroAgent.acceptVerticalDiffBlock` / `kiroAgent.rejectVerticalDiffBlock` 传入 `(filepath, index)` 逐块操作。

`streamInlineEdit(promptName, fallbackPrompt, onlyOneInsertion, range)` 是 Quick Edit 的入口：先读 `config.experimental.contextMenuPrompts[promptName]`，fallback 到内置 prompt，再调 `verticalDiffManager.streamEdit`。

---

## Indexing 与 Embeddings 体系

**EmbeddingsProvider 层级**：`BaseEmbeddingsProvider` 定义接口（`providerName`、`maxBatchSize`、`defaultOptions`）。默认内置实现是 `TransformersJsEmbeddingsProvider`，使用本地 ONNX 模型 `all-MiniLM-L6-v2`（384 维向量），通过 `transformers.js` 的 `EmbeddingsPipeline` 推理，每批最多 4 个 chunk，mean pooling + normalize。测试环境直接返回 mockVector 跳过推理。

**Docs 索引**：`DocsIndex` 类（LanceDB 表名和 SQLite 表名均为 `docs`），持有 `preIndexedDocsEmbeddingsProvider` 单例。预索引数据从 S3 拉取：`https://{bucket}.s3.{AWS_REGION}.amazonaws.com/{embeddingsProviderId}/{title}`。索引完成后通过 `addContextProvider` 把 `docs` provider 写入 `config.json`，并展示 toast 提示。`hasDocsContextProvider()` 检查当前 config 是否已含 docs provider。

**Auth Token 文件监听**：`BuilderIdTokenStorage` 在构造时用 `fs.watchFile` 监听 `~/.aws/sso/cache/<hash>` 文件变化，变化时清缓存并通过 `_onDidChange` 事件广播 `{ oldToken, newToken }`。`KiroAuthSession` 订阅该事件，登录状态变化时触发 `_onDidChangeLoginStatus`；窗口聚焦（`onDidChangeWindowState`）时重启 refresh loop。

---

## Context Provider 注册链路

`intermediateToFinalConfig()` 是配置实例化的核心函数，在每次 `loadConfig()` 时调用。流程如下：

默认内置两个 Context Provider：`FileContextProvider`（`@file`）和 `SpecContextProvider`（`@spec`），无论 workspace 是否受信任都会加载。

受信任 workspace（`vscode.workspace.isTrusted`）才会加载用户配置的 providers，规则：
- `codebase` provider 被跳过（内部处理）。
- `repo-map` 仅在 `ideSettings.enableRepositoryMapIndex === true` 时加载。
- 通过 `contextProviderClassFromName(name)` 按名称查找类，未知 name 打 warning 跳过。
- 实例化时传入 `provider.params`。

MCP 也在此处注入：workspace 受信任时执行 `kiroAgent.mcp.reloadConfig`（携带 `onConnectionChangeCommand = kiroAgent.refreshContextProviders`），然后 push `MCPContextProvider`；不受信任时执行 `kiroAgent.mcp.reset`。

`defaultConfig`（`config.json` 缺失时的 fallback）内置四个 provider：`code`、`docs`、`repo-map`、`diff`。`addContextProvider(provider)` 是运行时向 `config.json` 追加 provider 的工具函数，Docs 索引完成后会自动调用它。

---

## StatusBar 体系

扩展注册三个 StatusBar 项（均靠右对齐）：

**Experiments StatusBar**（priority -999）：显示实验性功能开关，点击触发 `showQuickPick()` 列出所有实验项，用户选择后持久化到配置。

**Feedback StatusBar**（priority -998）：固定显示反馈入口，点击触发 `FEEDBACK_ITEM_COMMAND`，引导用户提交反馈。

**UsageMeter StatusBar**（priority -999）：仅在 `usageBreakdowns.length === 1` 时显示（`shouldShowUsageMeterStatusBarItem`）。文本格式为 `currentUsage / usageLimit updated X ago`，通过 `usageLimitsEventEmitter.onDidUpdateUsageLimits` 订阅事件实时刷新，每隔 `USAGE_METER_UPDATE_INTERVAL` ms 更新「X ago」时间戳。

---

## QuickEdit 体系

`kiroAgent.quickEdit` → `quickEdit.show(args)` 弹出行内编辑输入框。`kiroAgent.defaultQuickAction` 是透传入口，内部直接执行 `kiroAgent.quickEdit`（用于遥测区分）。

`VerticalDiffManager.streamEdit(input, modelTitle, streamId, onlyOneInsertion, quickEdit, range)` 是核心流式编辑方法：
- 执行前设置 context `kiroAgent.diffVisible = true`。
- 检查当前文件是否已有 handler（`getHandlerForFile`），有则复用或根据 range 变化决定是否重置。
- QuickEdit 模式下比较新旧 range，决定是复用还是新建 `VerticalDiffHandler`。

CodeLens Provider（`getDefaultCommand`）在有选中代码时注入一个 `kiroAgent.defaultQuickAction` CodeLens，title 为「Continue」。

`ContextProvidersQuickPick` 在 QuickEdit 输入框中提供 `@` 触发的 context provider 选择器。

`kiroAgent.quickFix`：接收 `(range, diagnosticMessage)`，构造 prompt「How do I fix...」后调 `addCodeToContextFromRange` 发送到 Chat，再 focus `continueGUIView`。

---

## Autocomplete 提供者

扩展在激活时通过 `vscode.languages.registerInlineCompletionItemProvider` 注册 `ContinueCompletionProvider`，pattern 为 `**`（覆盖所有文件）。

两层 Provider 架构：
- 外层 `ContinueCompletionProvider`：负责状态栏联动（`setupStatusBar`）、`StatusBarStatus.Enabled` 检查、`selectedCompletionInfo` 前缀过滤、结果去重与 `markDisplayed`。
- 内层 `completionProvider`（`RuntimeServiceClient` 驱动）：实际调用 LLM 获取补全，返回 `{ completion, completionId }` 结构。

每次触发时生成唯一 UUID 存入 `lastUUID`，用于去重取消。`Battery` 对象管理生命周期（subscription 注册/释放）。`registerAutocompleteCommands` 额外注册 `logAutocompleteOutcome`、`toggleTabAutocompleteEnabled`、`openTabAutocompleteConfigMenu` 三个命令。

---

## WebView Provider 注册

扩展注册了三个 WebView：

**ContinueGUIWebviewViewProvider**（viewType: `kiroAgent.continueGUIView`）
注册为 `WebviewViewProvider`，`retainContextWhenHidden: true`。加载资源路径为 `packages/continuedev/gui/dist`。初始化时创建 `InProcessMessenger` 和 `OutputChannel`（名称：`Kiro - LLM Prompt/Completion`），通过 `resolveWebviewProtocol` 与 sidebar 双向通信。

**AcpChatViewProvider**（viewType: `kiroAgent.acpChatView`）
未认证时 `enableScripts: false`，显示「Initializing…」占位页；认证完成后调用 `renderWebview()` 加载完整 UI。

**Power Details Panel**（viewType: `kiro.views.power-details`）
用 `createWebviewPanel` 创建普通面板（非 sidebar），`enableScripts: true` + `retainContextWhenHidden: true`，标题格式为 `Power: {displayName}`，在 `ViewColumn.One` 打开。`powerWebviewProviders` 全局对象持有 `powerDetailsViewProvider` 引用，`setPowerWebviewProviders()` 在 activate2 阶段注入。

---

```

                    ┌─────────────────────────────────────────────────────────────────────────────────┐
                    │                       Kiro IDE  (VSCode fork + Electron)                        │
                    │                                                                                 │
                    │  ┌───────────────────────────────────────────────────────────────────────────┐ │
                    │  │                       kiro.kiro-agent  extension.js                       │ │
                    │  │                                                                           │ │
                    │  │  activate()                                                               │ │
                    │  │   ├─ setupCa()                  平台证书注入                              │ │
                    │  │   ├─ dynamicImportAndActivate() 延迟加载 Continue 核心                    │ │
                    │  │   └─ activate2()                                                          │ │
                    │  │       ├─ initializeAgentTelemetry()                                      │ │
                    │  │       ├─ registerExtension()                                              │ │
                    │  │       │   ├─ 80× kiroAgent.* commands                                   │ │
                    │  │       │   ├─ StatusBar (Right, priority -999)                            │ │
                    │  │       │   ├─ onDidChangeConfiguration                                    │ │
                    │  │       │   └─ TreeDataProvider ──► HookController                        │ │
                    │  │       ├─ Powers 初始化                                                   │ │
                    │  │       ├─ 账号/配置校验                                                   │ │
                    │  │       └─ 模型缓存 + 远端工具发现                                         │ │
                    │  │                                                                           │ │
 ┌──────────────────┼──┼──────────────────── 核心服务层 ──────────────────────────────────────────┼─┼──────────────────┐
 │  hook-editor/    │  │  HookController     SteeringController    SpecController                 │ │  requirements-   │
 │  (TreeView UI)───┼──┼──►                                        │                              │ │  webview/        │
 │                  │  │  McpController      SkillController        PowerController               │ │  (Spec WebView)◄─┼─┐
 │  kiro-shared/    │  │  AutoSwitchController                                                    │ │                  │ │
 │  (认证/遥测) ────┼──┼──► 基础设施                                                              │ │  kiro-ui-powers/ │ │
 └──────────────────┼──┼──────────────────────────┬────────────────────────────────────────────┼─┼──(Powers WV) ◄───┘ │
                    │  │                          │ dispatch                                    │ │                    │
 ┌──────────────────┼──┼──────────────────────────▼──────────────────── Agent 执行层 ──────────┼─┼────────────────────┤
 │  acp-type-       │  │  ExecutionQueue                                                        │ │                    │
 │  covenant/       │  │   Queued ──► Began ──► Resumed ◄──► Yielded                           │ │                    │
 │  (协议类型) ─────┼──┼──►                              │                                     │ │                    │
 │                  │  │                      SaveState / SummarizeUsage                       │ │                    │
 │                  │  │                              │                                        │ │                    │
 │                  │  │               Success / Failed / Aborted / ContextUsageUpdate         │ │                    │
 │                  │  │                                                                        │ │                    │
 │                  │  │  Subagents: context-gatherer  general-task  custom-creator             │ │                    │
 │                  │  │  Tools:     readFile  writeFile  searchFiles  runTerminal              │ │                    │
 └──────────────────┼──┼─────────────────────────────┬──────────────────────────────────────┼─┼────────────────────┘
                    │  │                             │ call                                   │ │
 ┌──────────────────┼──┼─────────────────────────────▼──────────────────── 通信与 UI 层 ────┼─┼──────────────────────┐
 │  kiro-client/    │  │  EditorApi  WebviewPanel ◄──────────► Extension                    │ │  kiro-ui-agent-chat/ │
 │  (ACP 协议层)────┼──┼──► KiroClient                         ChatSession (会话队列+历史) ◄─┼─┼──(Chat WebView)       │
 │                  │  │                                                                     │ │                      │
 │  autocomplete/   │  │  InlineCompletion (provideInlineCompletionItems)                   │ │  kiro-context-       │
 │  (补全引擎) ─────┼──┼──►                                                                 │ │  providers/          │
 │                  │  │                                                                     │ │  (ContextEngine) ────┼─┐
 └──────────────────┼──┼──────────────────────────────────────────────────────┬────────────┼─┼──────────────────────┘ │
                    │  └──────────────────────────────────────────────────────┼────────────┘ │                        │
                    └─────────────────────────────────────────────────────────┼──────────────┘                        │
                                                                              │ ACP (WebSocket / HTTP)                 │
                                                                 ┌────────────▼────────────┐                          │
                                                                 │      Kiro 后端服务        │ ◄────────────────────────┘
                                                                 │  q.{region}.amazonaws.com│   Context 补全请求
                                                                 └─────────────────────────┘
```

## custom-agent-loader 模块

\custom-agent-loader/\ 负责加载工作区自定义 Agent 定义文件并注册到全局 registry。

**custom-agent-registry-initializer.ts**：\createCustomAgentRegistry(agentProviders)\ 新建 \CustomAgentRegistry\ 实例，先调 \getBuiltinAgents()\ 注册三个内置 Agent（v20、A11、E19，分别对应 coding/spec/general agent），再遍历 \gentProviders\ 追加外部 provider。

**prompt-file-resolver.ts**：\
esolvePromptFileUri(fileUri, baseFileUri)\ 解析 Agent 定义中 \ile://\ 前缀的 prompt 路径。绝对路径直接转 \scode.Uri.file\，相对路径相对于 Agent 定义文件所在目录解析。

**custom-agent-loader.ts**：\CustomAgentFileLoader\ 类扫描 \.kiro/agents/*.yaml\，解析 YAML 后构造 \CustomAgentDefinition\ 并注册；用 \createFileSystemWatcher\ 监听文件变更实现热重载。

**commands/list-custom-agents.ts**：注册 \kiroAgent.customAgents.listCustomAgents\ 命令，调用 \customAgentRegistry.getAll()\ 过滤 \specOnly\ Agent 后返回列表供 WebView 渲染菜单。

## repos 模块

`repos/repo-uri-handler.ts`：`registerRepoUriHandler()` 订阅 `uriEventHandler.onUri`，过滤 `authority === 'kiro.repo'; path === '/clone'` 的 URI，从 query 取 `url` 和可选 `branch` 参数，调用 VSCode 内置 `git.clone` 命令完成克隆。

`repos/index.ts` 的 `register50` 只调一次 `registerRepoUriHandler()`。外部可通过 `kiro://kiro.repo/clone?url=...` 直接触发工作区克隆。
