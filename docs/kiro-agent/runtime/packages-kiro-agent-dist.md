# kiro-agent dist 模块分析

> 分析路径：C:/Users/12925/AppData/Local/Programs/Kiro/resources/app/extensions/kiro.kiro-agent/packages/kiro-agent/dist
> 分析日期：2026-03-17
> 2026-03-18 校对：按 `dist/extension.js` 中保留的源码注释边界重新去重统计，当前可识别为 `88` 个内部模块块。下文早期分批分析内容仍保留，但“81 个模块已全部读完”的旧结论已过时。

按当前统计口径，`packages/kiro-agent/dist/**` 共 `88` 个内部模块块。按职责分层如下。

## 模块清单（88）

```text
_commonjsHelpers-DaMA6jEr.js
acp-remote-mcp-client-DTe6uFFL.js
actions-Dng4pati.js
agent-context/compaction/index.js
async-delivered-object/index.js
async-stream/index.js
auth-DCPC05L9.js
autonomy-mode-B8cmGn9E.js
base-BCJFoMV_.js
cancellation-C60foDZK.js
chat-agent-IUIL54gd.js
clarification-handler-Zo3AuRjE.js
command-approval-MQd-5ajF.js
command-approval-_g4NkBJ8.js
config-constants-CsFJbjLP.js
config-file-watcher-DbLp_qKw.js
context-chat-message-DzjJbTBD.js
custom-agent-parser-DbpwC6qc.js
custom-agent-registry-DF7phCBA.js
dev-inspector-CF526NYm.js
disclose-context-bPcRLLYL.js
errors-1JFcQdts.js
execution-log-controller-BGxzu20b.js
file-context-ClFefQMc.js
file-lock-D4f90hJ2.js
get-user-input-DLJmPWtM.js
hooks-Dmyvf9cL.js
index-C-CV5vnh.js
index-DBxQQQD8.js
index-D_kbIg3m.js
index-Q4bc6qbj.js
index.js
intent-detection-service-DV14tv5n.js
logger-CTb8_yz9.js
mcp-config-manager-BVKB8dJE.js
message-analyzer-BAqw8PNp.js
message-parts-D8WKxpG9.js
message-replay-Cjyo5CAH.js
model-config-BdfVU6dY.js
model-provider-DwyIQZZf.js
node-background-process-manager-_8U2gTa-.js
node-progressive-context-source-C3Yk_3xF.js
number-coercion-CeAOJ7Po.js
orchestrator-prompt-Vzhr2QeL.js
parse-front-matter-CXai4UYs.js
pending-changes-BxcOZUqX.js
powers-manager-CbGmy5n_.js
prompt-DuC9mWRH.js
prompt-processor-Dw_f2vjD.js
prompt-template-C_Mn10zi.js
prompts-DfcXHzAI.js
pruning-service-DJ-mli7u.js
q-client-DsNNqt3G.js
q-developer-converse-BT9F76b_.js
range-utils-CHnKtlN2.js
remote-tools-C9QNKWUJ.js
remote-tools-discovery-DUpCRp4S.js
session-C-gUzvZx.js
session-update-utils-S4kmZ2as.js
session/schemas/index.js
shared-types-CIxCt9tj.js
spec-agent-OdZ7esxm.js
spec-platform-361SGdHa.js
spec/tasks/index.js
steering-Dcn_tjkT.js
stream-CtDBTPgX.js
streamed-data-object/index.js
string-BZO3_EzG.js
strip-json-comments-BVnY24gX.js
sub-agent-Dxs7IWdE.js
telemetry-Bf0GI6nJ.js
telemetry/index.js
terminal-dGJDGXey.js
token-estimator-B45EKs9J.js
token-monitor-QTYxqQ8j.js
tool-filter-CS5Fsu0N.js
tool-message-list-B1XEM-VL.js
tool-tags-NGeoUHCI.js
tool-usage-meter-BVm5olm7.js
transformers-BLTCfl6l.js
types-CkvOiUfs.js
types-DThkhnH-.js
types-core-DBgLqCOQ.js
unknown-error-7JAvTvbU.js
validator-ClHWtYX0.js
web-fetch-utils-C1Z4KMmp.js
workspace-connection-impl-Dee9nf40.js
workspace-object-CspubCq6.js
```

完整分类视图见 `../extension/modules-coverage.md`。

---

## 一、类型与枚举层

### actions-Dng4pati.js
动作类型判别辅助模块，本身很小，但被 `pending-changes`、execution log 回滚逻辑反复调用。

- `isFileChangeAction()`：识别 `create / append / write / replace / editCode / delete`
- `isMoveAction()`：识别 `move`
- `isSemanticRenameAction()`：识别 `semanticRename`

作用：把 execution action 的恢复/应用流程按动作类型分流，避免各模块重复写字符串判断。

### shared-types-CIxCt9tj.js
全局枚举中心：
- AutonomyMode：Autopilot / Supervised
- ActionState：NotStarted / Queued / InProgress / Completed
- TaskEventType：FileEdited 等
- AgentWorkflowType、ExecutionStatus

### types-core-DBgLqCOQ.js
- AgentType：mocked-agent / chat-agent / spec-generation / sub-agent / hook-command
- ExecutionStatus：queued / running / failed / aborted / completed

### session-C-gUzvZx.js
- 合法 session 类型：vibe / spec
- isSessionType() 校验函数

### types-CkvOiUfs.js
Zod v4 扩展，补充 ISO 时间格式（ZodISODateTime / ZodISODate / ZodISOTime / ZodISODuration）。

---

## 二、基础设施层

### index.js
运行时总出口文件，职责不是业务实现，而是把内部模块统一 re-export 成对外 API。

- 导出核心类与函数：`KiroAgent`、`AgentController`、`AgentExecution`、`RemoteToolsDiscovery`、`PowersManager`、`MCPConfigManager`、`SessionPersistence`
- 导出 ACP 侧工厂：`createACPWorkspaceConnection()`
- 导出运行时单例注入点：`initializeKiroAgent()`、`setModelConfigProvider()`、`setMetricReporterFactory()`、`setToolTrustManager()` 等
- 导出大量 schema、错误类、工具类和 helper，说明 runtime 被设计成“宿主扩展可按需装配”的模块化内核

关键判断：`index.js` 是 runtime 的“装配出口层”，不是逻辑中心，但它决定了哪些能力会暴露给宿主。

### q-client-DsNNqt3G.js
Q Developer 客户端工厂，全局单例：
- setQClientFactory(factory) 注入工厂
- createQDeveloperConverse(provider, modelId) 创建客户端
- 未注入前调用直接抛错

### model-provider-DwyIQZZf.js
模型实例缓存层：
- loadModel(provider, modelId, options) — Map 缓存，key 为 provider:modelId
- clearModelCache() 强制刷新

### model-config-BdfVU6dY.js
- 默认 provider qdev，默认任务 simple-task
- getModelIdentifier() → provider::modelId 格式
- getQDevFastModelProvider() / getQDevFastModelId()

### autonomy-mode-B8cmGn9E.js
- 默认 Supervised 模式
- setAutonomyModeProvider(p) 注入，getAutonomyMode() 读取

### hooks-Dmyvf9cL.js
前后置钩子系统：
- preHook / postHook 默认空实现（pass-through state）
- setPreHookProvider(p) / setPostHookProvider(p) 注入
- runPreHook() / runPostHook() 执行

### telemetry-Bf0GI6nJ.js
- MetricReporterFactory 单例，setMetricReporterFactory() 注入
- createMetricReporter(name, opts)
- setTelemetryContext(ctx) / getTelemetryContext()
- withSpan(name, fn) 追踪包装器

### token-estimator-B45EKs9J.js
Token 估算引擎（纯本地）：
- 四种策略：tiktoken / llama / claude / generic
- Kiro 当前所有模型均走 generic（映射表为空）
- generic 算法：ceil(len/4) + ceil(lines*0.5) + codeBlocks*2
- LRU 缓存（1000 条，5 分钟 TTL），key = md5(content)
- estimateMessageTokens(msg) / estimateContextTokens(ctx)

### logger-CTb8_yz9.js
普通 logger a 和带前缀 logger l。

---

## 三、上下文与消息层

### context-chat-message-DzjJbTBD.js
ContextChatMessage Builder 模式核心：
- fromHuman() / fromBot() / fromTool() 工厂方法
- .withText() / .withToolUse() / .withToolUseResponse() 链式构建
- .withDocumentAttachments(docs) 附加文档
- ContextChatHistory：.withNewMessage() 追加，.messages 只读

### token-monitor-QTYxqQ8j.js
ModelContext 核心类，管理单次执行上下文窗口：
- 持有消息历史、token 预算、当前使用量
- 依赖 stream / transformers / context-chat-message

### pruning-service-DJ-mli7u.js
上下文压缩服务，接近 token 上限时裁剪消息：
- PruningService 策略模式
- 错误体系：CompactionError / TokenEstimationError / StrategyNotFoundError
- 依赖 token-estimator

### stream-CtDBTPgX.js（748 行）
流式输出处理核心：
- HumanMessage / AIMessage LangChain 消息类
- AsyncStream 异步流包装
- 流转换器、过滤器、token 统计回调

### transformers-BLTCfl6l.js
消息转换管道，LangChain 消息与 Kiro 内部格式互转。
---

## 四、Prompt 层

### index-C-CV5vnh.js
大体量的通用依赖打包壳，不是 Kiro 特有业务逻辑入口。

- 开头可见 `kind-of`、`extend-shallow`、`section-matter`、YAML/异常处理等通用解析库
- 从模块内容特征看，主要承载 frontmatter、YAML、glob、路径/匹配等基础库的 bundle 聚合
- 更适合作为“runtime 内部依赖集合”看待，而不是独立业务能力模块

它之所以出现在 `packages/kiro-agent/dist/**`，更像是构建产物拆包后的 utility chunk。

### prompts-DfcXHzAI.js
系统 prompt 生成器：
- getBaseSystemPrompt() — 主系统 prompt，含 OS 信息和工作区描述
- getOsString() — OS 标识字符串

### prompt-DuC9mWRH.js
Prompt 组合与上下文序列化层，是系统 prompt、文件树、文件内容、忽略规则等内容的拼装中心。

- 内置 ignore 规则：
  - 文件级：锁文件、日志、图片、二进制、数据库、wasm、map、jsonl 等
  - 目录级：`.git/`、`.vscode/`、`node_modules/`、`dist/`、`build/`、`.kiro/` 等
- 提供 `DEFAULT_IGNORE`、`DEFAULT_IGNORE_DIRS`、`DEFAULT_IGNORE_FILETYPES`
- 用 `node-ignore` 风格规则处理文件排除
- 文件树序列化：
  - `serializeFileTree()`
  - 自动展开部分目录，支持 closed/expanded 状态
  - 对大目录生成 summary，避免上下文爆炸
- 模板片段：
  - tool usage 示例
  - file 内容包装
  - diagnostics 包装
  - fileTree 包装

定位：`prompts-DfcXHzAI.js` 更像“写什么 prompt”，`prompt-DuC9mWRH.js` 更像“怎么把工作区内容和上下文素材塞进 prompt”。

### orchestrator-prompt-Vzhr2QeL.js（616 行）
Spec 工作流 orchestrator prompt 工厂：
- getSpecWorkflowDecisionPrompt() — Mermaid 流程图格式决策树
- getSpecOrchestratorPrompt() — 完整 orchestrator 系统 prompt
- 内嵌 flowchart TD 决策逻辑

### intent-detection-service-DV14tv5n.js（410 行）
意图检测服务：
- 通过 LLM 判断用户意图：vibe coding vs spec generation
- 依赖 telemetry / shared-types / stream

---

## 五、Agent 执行层

### index-D_kbIg3m.js
Agent 图中的失败干预与循环检测节点集合。

- 检测连续失败模式：
  - 最近多条消息都含 tool failure
  - 同一工具重复使用形成 loop
  - 同一工具连续多次失败
- `FailureDetectionNode`：在继续模型调用前检查是否需要用户干预
- `UserInterventionNode`：将“取消 / 继续迭代 / 提供指导”问题注入执行队列，并通过 yield 等待用户选择
- 用户选择会被转写回上下文消息，影响后续 agent 行为

作用：这是 agent runtime 的“防卡死/防瞎试”控制层，避免工具连续失败时无限迭代。

### index-Q4bc6qbj.js
Spec 工具集合，集中实现和规格任务相关的内置工具。

- `prework` 工具：
  - 分析 requirements/bugfix 文档中的条目是否适合做 property-based testing
  - 生成 `properties` 列表并写回 execution output
- `updatePBTStatus` 工具：
  - 更新 `tasks.md` 中 PBT 任务状态
  - 要求 `failed` / `unexpected_pass` 时提供 failing example
- 同模块还承载 `taskStatus` 等 spec 相关工具实现

定位：这是“Spec workflow 的工具实现 chunk”，和 `spec/tasks/index.js`、`spec-platform-361SGdHa.js` 一起组成 spec 执行面。

### remote-tools-discovery-DUpCRp4S.js
远端工具发现缓存层，对 `mcpClient.listTools()` 做 lazy discovery 包装。

- `discoverTools(force = false)`：调用远端 `listTools()`，成功后缓存并包装为 `RemoteWrapper`
- `getCachedTools()`：优先返回缓存；若没有缓存且当前未在发现流程中，则后台触发一次 lazy discovery
- `triggerLazyDiscovery()`：用 `discoveryInProgress` 防止并发重复拉取
- `clearCache()`：模型或连接变化后清空工具缓存

它本身不处理协议细节，只负责“发现、缓存、包装”，并把远端工具转成 agent 可消费的工具对象。

### disclose-context-bPcRLLYL.js（154KB，核心）
工具注册 + Agent 执行引擎总入口。

#### AgentExecution 类
状态机：Queued → Running → (Yielded ↔ Running) → Completed / Aborted / Failed

关键字段：executionId / agentWorkflowType / status / workspace / model / definition / abortController / yieldSemaphore

关键方法：
- invoke() — 启动执行（只允许从 Queued 状态调用）
- run() — 实际执行循环，调用 invokeHandler
- pushUserMessage(text, docs) — 中断当前执行，注入新消息后重新 run
- queueUserMessage(text) — 软发送，下一个 turn 边界消费
- consumeQueueNow() — 立即消费队列消息（Send Now 功能）
- acquireYield(toolId) / releaseYield(toolId) — 工具 yield 信号量管理
- modelStream2(context, tools) — 调用 LLM 流式输出，绑定工具、处理 abort
- saveState(graphState) — 持久化图状态
- abort(err) / abortAndWait() — 终止执行
- handleDanglingToolCall() — 清理悬空 toolUse，保证消息链合法

#### YieldSemaphore
基于 async-sema 的单槽信号量，管理工具 yield 状态：
- acquire(toolId) — 占用，进入 Yielded 状态
- release(toolId) — 释放，恢复 Running
- forceRelease() — abort 场景强制释放
- waitForRelease() — 等待释放信号

#### 内置工具
文件操作：readFile / readMultipleFiles / fsWrite / create / append / replace / editCode / deleteFile / moveFile / semanticRename

命令执行：executePwsh（Windows）/ executeBash（Unix），内置长命令检测（dev-server / watcher / interactive / daemon），controlPwshProcess / controlBashProcess 管理后台进程

搜索：grepSearch / fileSearch / semanticSearch

Web：webFetch（plain / rendered / selective / truncated 四种模式）/ remote_web_search

#### RemoteTool 类
MCP 远程工具通用包装器：
- 未信任工具先 acquireYield 等待用户审批
- 审批结果：accept（本次）/ trust（永久）/ reject（拒绝）
- JSON Schema → Zod schema 自动转换

#### 错误体系
- GoError — 用户面向会话错误基类
- InvalidExecutionStateError — 状态机非法转换
- UnknownExecutionError — 未知错误兜底
- NoQuestionsToYieldError — yield 无问题时抛出

---

## 六、子 Agent 与编排层

### custom-agent-registry-DF7phCBA.js
自定义 agent 定义注册表，结构非常简单，但它是宿主加载器与 runtime 之间的共享容器。

- 内部存储：`Map<id, definition>`
- `register(definition)`：按 id 注册，已存在则覆盖
- `get(id)`：读取单个定义
- `getAll()`：返回全部已注册定义
- `unregister(id)`：删除定义

作用：把 workspace/global custom agents 的发现结果汇总成运行时可查询注册表，供 `InvokeSubAgent` 一类工具读取。

### chat-agent-IUIL54gd.js（740 行）
主 chat agent 实现，管理多轮对话循环，处理工具调用返回。

### sub-agent-Dxs7IWdE.js（592 行）
子 agent（嵌套执行），导出 createSubAgent() 工厂和 AgentController。

### get-user-input-DLJmPWtM.js
用户交互输入工具，基于 zod-stream 流式 JSON 解析，isPathComplete 校验。

### spec-agent-OdZ7esxm.js
Spec agent 适配层（极简）：
- getLastUserMessage() 默认返回 undefined
- mapChatMessagesToExecutionIds() / mapExecutionIdsToMessages() 默认空实现
- setSpecAgentProvider(p) 注入真实实现

### custom-agent-parser-DbpwC6qc.js（207 行）
自定义 agent 配置解析器：
- 解析 .md/.json 格式的自定义 agent 定义文件
- 依赖 parse-front-matter / strip-json-comments

---

## 七、工作区层

### spec/tasks/index.js
Spec 任务文件的解析与状态持久化层，对 `tasks.md` 和对应 `.meta.json` 负责。

- Markdown 任务解析：
  - 识别 checkbox 状态：`[ ]`、`[-]`、`[~]`、`[x]`
  - 支持编号任务和非编号任务
  - 构建层级任务树与 `subTasks`
- 元数据文件：
  - `tasks.md` 对应 `tasks.meta.json`
  - 存 `pbtResults` 与 `executionHistory`
- `updateTaskStatus()`：
  - 用文件锁更新 markdown checkbox
  - 记录 `executionId` / `chatSessionId` / 时间戳
- `validateTaskStatusChange()`：
  - 完成父任务前，要求非可选子任务先完成
  - 启动任务时检查前序 sibling 是否未完成，并给 warning
- `updatePBTStatus()`：
  - 只允许对 PBT 任务写结果
  - 持久化 `passed / failed / unexpected_pass / not_run`
- `getTaskMetadata()` / `getTask()`：
  - 提供叶子任务判断和任务行号范围

这是 spec workflow 中最实际的“任务状态真源”，扩展层和工具层最终都要落到这里。

### validator-ClHWtYX0.js
会话消息合法性校验与修复层，面向 Q/CodeWhisperer 风格的对话结构。

- 规则校验：
  - 会话必须以 user message 开始
  - 会话必须以 user message 结束
  - user / assistant 交替出现
  - assistant 的 tool uses 后必须跟带对应 tool results 的 user message
  - user message 不能既无文本也无 tool result
- 自动修复：
  - 必要时插入默认 user/assistant message 占位
  - 缺失 tool result 时自动伪造错误 toolResults
  - 重新排序 tool result，使之跟对应 tool use 对齐
  - 删除空 user message
- 主要函数：
  - `sanitizeConversation()`
  - `validateConversation()`

作用：在把内部消息发给模型前做结构修正，避免 conversation 格式不合法导致上游 API 拒绝。

### session-update-utils-S4kmZ2as.js
Execution action 与 session 持久化消息之间的转换层。

- `actionToMessages()`：
  - 把 `AgentExecutionAction` 转成可持久化的 session message payload
  - 覆盖 `user`、`assistant`、`tool_call`、`tool_result`、`sub_agent_*`、`steering_inclusion`
- `createUserMessage()`：构造标准化 user payload
- 状态映射：
  - actionState -> session status
  - tool actionType -> `read/edit/delete/move/search/execute/fetch/other`
- 位置提取：
  - 从 `file`、`path`、`files[]`、`paths[]`、`targetDirectories[]` 等字段中推导 locations
- checkpoint 元数据提取：
  - 对 edit 类结果提取 `original / modified / local`

它是 runtime 的“消息语义桥”，把内部执行记录转换成回放、持久化、UI 会话流都能消费的统一格式。

### acp-remote-mcp-client-DTe6uFFL.js
ACP 远程工作区装配层，是 runtime 与 IDE 扩展宿主之间的桥接器。

- `createACPWorkspaceConnection()`：用 ACP `extMethod`、`requestPermission`、sessionId、workspacePaths 等依赖组装 `WorkspaceConnectionImpl`
- 把 ACP 侧能力映射为本地工具：
  - 文件搜索：`_kiro/search/find_files`
  - 文本搜索：`_kiro/search/text_search`
  - 终端/后台进程：`ACP terminal manager` + `ACP background process manager`
  - 远端工具：`RemoteToolsDiscovery`
  - MCP 工具：从 `mcpManager.getTools()` 包装成 runtime 可执行工具
- 命令审批在 ACP 模式下统一走 `requestPermission()`，只暴露 `accept/reject`
- `K10()` / `getTools()` 负责为不同 agent 类型组装不同工具集：`chat`、`specAgent`、`specOrchestrator`、`subAgent`

这个模块的定位很关键：它不是 MCP 管理器本身，而是“把 IDE 端、ACP 会话、Remote tools、MCP tools、Powers、Progressive Context”装配进同一个运行时工作区。

### powers-manager-CbGmy5n_.js
Powers 运行时管理器，负责扫描 `~/.kiro/powers/installed` 并向活跃 session 广播 powers 变化。

- 监听路径：`~/.kiro/powers/installed`，通过 `ConfigFileWatcher` 监控安装结果变化
- `onSessionCreated()`：首次 session 创建时懒加载 watcher，并立即做一次初始扫描
- `scanInstalledPowers()`：读取 `~/.kiro/powers/installed.json`，提取 `installedPowers`
- `extractPowerMetadata(name)`：从每个 power 目录中读取：
  - `POWER.md` frontmatter：`description`、`displayName`、`keywords`
  - `mcp.json`：提取 `mcpServers` 名称
  - `steering/*.md`：判断是否存在 steering 文件
- `getPowerDocumentation()`：读取并去掉 `POWER.md` frontmatter
- `getSteeringFiles()` / `getSteeringContent()`：读取某个 power 的 steering markdown
- 变化通知事件：`_kiro/powers/items_changed`

它的职责是“把安装在本地磁盘上的 powers 暴露成运行时可查询、可通知的数据源”，和扩展层 `src/extension/powers/**` 形成上下两层分工。

### workspace-connection-impl-Dee9nf40.js（272 行）
工作区连接实现：
- 依赖 vscode-uri / workspace-object / shared-types
- WorkspaceConnection 类，管理文件系统访问权限
- checkpointFile() / deleteFile() / withContext() 等

### workspace-object-CspubCq6.js
Workspace 值对象，封装工作区路径和 URI 转换。

### index-DBxQQQD8.js
Spec workflow 定义与内置 sub-agent 定义集合。

- 生成 `.kiro/specs/{feature}/.config.kiro` 初始化片段
- 提供多种 spec workflow definition：
  - `feature design-first`
  - `requirements-first`
  - `bugfix`
- `createSpecAgentProvider()`：返回 spec 专用 sub-agent 定义集合
- 还包含：
  - `spec-task-execution`
  - `custom-agent-creator`
  - `context-gatherer`
  这些内置 agent 的 definition / prompt / tool 范围

定位：这是“内置 agent definition 注册源”，负责把 prompt、tool 集和 workflow presets 拼成可调用的 sub-agent 定义。

### node-background-process-manager-_8U2gTa-.js
Node 环境下的文件系统、终端和后台进程管理实现，主要给 ACP / headless 场景使用。

- `NodeFileSystem`
  - `readTextFile` / `writeTextFile` / `delete` / `exists` / `stat` / `readDirectory`
  - 所有路径都相对 `basePath` 解析
- `NodeTerminal`
  - 通过 ACP client 创建 terminal，执行 `/bin/bash -c <cmd>`
  - 支持超时、收集输出、读取尾部若干行
- `NodeTerminalManager`
  - 按 `cwd:name` 复用 terminal
- `NodeBackgroundProcessManager`
  - `startProcess()`：同命令同 cwd 可复用已有 running 进程
  - `stopProcess()`：kill terminal command
  - `getProcessOutput()`：读取后台进程输出
  - `listProcesses()`：列出 runtime 管理的后台进程

作用：这是 runtime 在非 VS Code 原生终端环境下的 process adapter，实现“最小可用的文件系统 + 命令 + 后台进程”能力。

### agent-context/compaction/index.js
上下文压缩系统初始化与简单压缩策略实现。

- `initializeCompactionSystem()`：
  - 注册 compaction strategy
  - 设置默认 strategy
- 默认策略 `simple-strategy`
  - 用 token estimator 估算上下文
  - 截取会话中要压缩的中间段
  - 组织 structured summary prompt
  - 调用 summarization model 生成“Previous conversation summary”
  - 把 summary 重新注入消息流，替换原始冗长上下文
- 会提取：
  - 会话主题
  - 工具执行结果
  - 相关文件
  - 已实现代码
  - 已解决问题

定位：`pruning-service` 解决“裁剪”，这个模块解决“摘要式压缩并重建上下文”。

### session/schemas/index.js
Session 持久化与回放的 Zod schema 汇总层，给 `message-replay`、ACP session update 映射、历史会话列表等能力提供统一数据契约。

- `agentMode` 枚举：`agent` / `spec`
- `tool_call.status` 枚举：`pending` / `awaiting_approval` / `approved` / `denied` / `executing` / `completed` / `failed`
- `tool_call.kind` 枚举：`read` / `edit` / `execute` / `search` / `delete` / `move` / `fetch` / `think` / `switch_mode` / `other`
- `MessagePayload` 判别联合：
  - `user`
  - `assistant`
  - `tool_call`
  - `tool_result`
  - `system`
  - `error`
  - `mode_change`
  - `session_event`
  - `sub_agent_start`
  - `sub_agent_complete`
  - `sub_agent_progress`
  - `steering_inclusion`
- `SessionMetadata`：`id`、`title`、`agentMode`、`workspacePaths`、`createdAt`、`lastModifiedAt`、父 session / execution 关联
- `PersistedSession`：`metadata + messages`
- `ListSessionsResponseSchema`：`{ sessions: [...] }`
- `ListSessionsOptionsSchema`：`limit`、`sortBy(createdAt|lastModifiedAt)`、`sortOrder(asc|desc)`

作用：把 runtime 的 session 存储、恢复、列表查询和前端展示绑定到同一套运行时校验 schema 上，避免历史数据结构漂移。

---

## 八、模块依赖总图（简化）

```
q-client
  └─ model-provider
       └─ AgentExecution (disclose-context)
            ├─ chat-agent
            ├─ sub-agent
            ├─ spec-agent
            └─ 内置工具集

shared-types / types-core  ←  被几乎所有模块引用

token-estimator
  └─ pruning-service
       └─ token-monitor (ModelContext)
            └─ AgentExecution

context-chat-message
  ├─ token-monitor
  └─ AgentExecution

prompts / orchestrator-prompt
  └─ chat-agent / spec workflow

telemetry  ←  横切所有 agent 和工具
```

---

## 九、关键发现

1. 所有模块通过工厂注入（setXxxProvider / setXxxFactory）实现解耦，核心逻辑与 VSCode 宿主完全分离。
2. Token 估算全部用 generic 策略（len/4 公式），不依赖 tiktoken/tiktoken-lite。
3. YieldSemaphore 是 Supervised 模式审批交互的底层机制，工具执行中断等待用户确认靠它实现。
4. disclose-context.js 单文件承载所有内置工具注册 + AgentExecution 运行时，是逆向分析的核心入口。
5. 长命令检测硬编码了 dev-server / watcher / interactive / daemon 四类模式白名单。
6. MCP 远程工具（RemoteTool）走统一的 yield 审批流 ，信任状态持久化，web_search 有专属格式化器。

## 第四批模块分析（2026-03-17）

### pending-changes-BxcOZUqX.js
Supervised 模式变更管理核心类 PendingChangesService，三个方法：
- applyPendingChanges：遍历 executionLog 中 PendingAction 操作，按 move/semanticRename/fileChange 类型 apply，先 checkpoint 再生成 kiro-diff:// URI 作为 modified 引用
- restorePendingChanges：逆序 restore 所有 PendingAction 操作
- restoreAllChanges：逆序 restore 所有非 Rejected 操作（强制回滚）
操作 ID 格式：{prefix}_{Date.now()}_{random8}，依赖 checkpoint + fs + executionLog 三个注入服务。

### errors-1JFcQdts.js
统一错误类体系，全部继承 AgentError（extends Error）：
- McpConnectionError / McpToolError / PermissionDeniedError（code -32000）
- ResourceNotFoundError（-32002）、InvalidClientError（-32600）
- TokenExpiredError / TokenInvalidError / SessionNotFoundError（-32000）
- ContextWindowExceededError：userFacingMessage = "Context limit exceeded unexpectedly. Please start a new session to continue."

### steering-Dcn_tjkT.js
Zod schema 定义：
- steeringDocConfig：inclusion（always/fileMatch/manual/auto）+ fileMatchPattern（string 或数组）
- skillMetadata：name/description/license/compatibility/metadata
- 类型守卫：isSkill(doc)、isSteering(doc)

### remote-tools-C9QNKWUJ.js
极简工厂注入，两个注册槽：getRemoteToolsProvider()、getApprovalService()，均无默认实现，未注册返回 undefined。

### tool-usage-meter-BVm5olm7.js
内嵌 OpenTelemetry API v1.9.0，通过 Symbol.for("opentelemetry.js.api.1") 全局注册防止重复。提供 Gauge/Counter/Histogram/UpDownCounter/Observable 系列指标，用于工具调用次数、token 消耗遥测。

### message-replay-Cjyo5CAH.js（Session 持久化层）
SessionPersistence 类，基于文件系统存储：
- 路径：{basePath}/{workspaceHash}/{sessionId}/，workspaceHash = SHA256(多工作区路径排序拼接) 前16位
- 文件：session.json（元数据）+ messages.jsonl（每条消息一行）
- 原子写入：write tmp 文件后 rename，失败时删除 tmp
- 消息加载：逐行解析 JSONL，跳过损坏行继续
- 清理策略：超限时按 lastModifiedAt 删最旧 session，保留最新 5 个
- _global 代表无工作区的全局 session

### node-progressive-context-source-C3Yk_3xF.js（Steering 文档加载器）
NodeSteeringDocumentSource 加载优先级：AGENTS.md（工作区根）> .kiro/steering/（工作区级）> ~/.kiro/steering/（全局级）
- AGENTS.md 强制 inclusion=always
- .md 文件读取 frontmatter 决定 inclusion 策略，fileMatch 模式用 minimatch 匹配
- 工作区不可信时只加载 global scope 文档
SteeringManager 封装 source，用 chokidar 监听文件变化，变更时 push 到所有活跃 session，排除 node_modules/.git/dist/out 目录。
## 第五批模块分析（2026-03-17）

### clarification-handler-Zo3AuRjE.js
意图澄清处理器。acquireYield("clarification") 挂起执行，弹出 userInput action 等待用户选择，收到答案后 releaseYield 恢复。

### command-approval-_g4NkBJ8.js
写文件安全过滤层。路径黑名单（mcp.json/.kiroignore/.vscode/.git），二重 decodeURIComponent+NFC/NFKC 规范化防路径穿越，检测 $schema 字段防 RemoteJsonSchema 注入。

### command-approval-MQd-5ajF.js
Shell 命令白/黑名单工厂，getTrustedCommands/getCommandDenylist 默认空数组，由外部策略注入。

### logger-CTb8_yz9.js
双通道日志。普通日志内存无限队列，LLM 日志循环队列上限 500 条，capture() 清空返回，供 telemetry 上报。

### tool-tags-NGeoUHCI.js
工具标签枚举：read/write/shell/web/spec/@mcp/@powers/@builtin，每个标签带人类可读描述。

### tool-filter-CS5Fsu0N.js
工具筛选器，支持 glob 匹配。filterToolsWithOptions 可额外包含 MCP 和 Powers 工具，mergeTools 去重合并。

### tool-message-list-B1XEM-VL.js
LangChain ToolMessageList 扩展，_getType()=tool，打印只输出 toolMessages 数量防日志爆炸。

### message-parts-D8WKxpG9.js
消息内容分解，分离 text/imageUrl/documentUrl。contextProjection() 估算 token 占用率，>=95% truncate，>=80% summarize。

### prompt-template-C_Mn10zi.js
用 UUID 占位符做二阶替换防键名碰撞，p() 有键模板，r() 无键模板，j() 多段拼接。

### config-file-watcher-DbLp_qKw.js
chokidar 封装，监视 ~/.kiro 和 workspace/.kiro，300ms 防抖，感知 steering/skills/mcp.json 实时变更。

### parse-front-matter-CXai4UYs.js
gray-matter 封装，禁用 JS 引擎，Zod schema 校验 frontMatter 字段，支持自定义 YAML 解析器注入。
### cancellation-C60foDZK.js
Promise + AbortSignal 绑定，abort 时用 UnknownError 包装 reject。

### execution-log-controller-BGxzu20b.js
执行日志控制器单例工厂，未初始化调用直接抛错。

### config-constants-CsFJbjLP.js
路径常量：.kiro / steering / .md / AGENTS.md / skills / SKILL.md / specs，全系统目录规范来源。

### dev-inspector-CF526NYm.js
开发调试钩子，notifyDevInspector 静默调用，异常被吞不影响主流程。

### unknown-error-7JAvTvbU.js
UnknownError：把任意非 Error 值包装成 SessionError，userFacingMessage 固定返回 "An unexpected error occurred"。

### terminal-dGJDGXey.js
终端单例工厂，setShellTypeProvider 注入 shell 类型检测，默认 bash。

### file-context-ClFefQMc.js
编辑器文件上下文适配器，loadCurrentFile/loadOpenEditorFiles 由 VSCode 扩展侧注入。

### range-utils-CHnKtlN2.js
LSP Range 构造工具，lineRange() 把 1-based 行号转 0-based LSP Range。

### spec-platform-361SGdHa.js
Spec agent 平台适配层，parseMarkdownRequirements/validateFileAccess/hooks 全部外部注入，mapContextReferencesToFiles() 转 fileTree 结构（target:500）。

### message-analyzer-BAqw8PNp.js
消息质量检测，发现 AI 回复末尾冒号但无 toolUse 时上报 MissingExpectedToolUse metric，let's/start/create 等词豁免。

### number-coercion-CeAOJ7Po.js
递归遍历工具 input，把 ZodNumber 字段对应的字符串转数字，上报 stringToNumberCoercions metric，防 schema 校验失败。

### file-lock-D4f90hJ2.js
基于 Promise 链的文件锁单例，acquireLock 等待已有锁释放后重试，withLock 保证 finally 释放。

### prompt-processor-Dw_f2vjD.js
解析 prompt 中的 {{providerId:query}} 占位符，调用 @kiro/context-providers 逐个替换，provider 缺失只 warn 不报错。

### strip-json-comments-BVnY24gX.js
清洗 JSON with comments（// 和 /* */），支持 trailingCommas，用于解析 mcp.json 等允许注释的配置。

### workspace-object-CspubCq6.js
工作区路径安全层。fromRelativePath() 校验路径不逃逸 workspace，InvalidWorkspacePathError 提示只能编辑 workspace 内文件。~/.kiro 路径被特殊允许。

### auth-DCPC05L9.js
AuthorizationError 类型守卫（isAuthorizationError），Signal 类实现 Promise-like 的可取消异步原语（pending/fulfilled/rejected 三态，resolveAfter/tryCancel/tryReject 等方法），web-fetch-utils 用它控制请求超时。

## 关键依赖关系补充

- **@kiro/context-providers**：prompt-processor 调用，提供 context reference 解析能力
- **@agentclientprotocol/sdk**：index.js 的 ACP session 底层协议
- **@aws/codewhisperer-streaming-client**：q-developer-converse 和 validator 使用，定义 ToolResultStatus 等枚举
- **@mozilla/readability + jsdom**：web-fetch-utils 网页正文提取
- **chokidar**：config-file-watcher 文件系统监听
- **gray-matter**：parse-front-matter YAML 解析
- **zod-stream**：get-user-input 流式 JSON 解析
- **async-sema**：mcp-config-manager 并发信号量
- **minimatch**：node-progressive-context-source glob 文件匹配
- **axios + axios-retry**：web-fetch-utils HTTP 客户端
- **dedent**：prompt-template 模板字符串格式化
## 第五批模块分析（2026-03-17）

### q-developer-converse-BT9F76b_.js（完整）
错误类型体系（全部继承 KiroQError）：
- OutputParseError / NoResponseError / UnsupportedMessageTypeError / InvalidToolFormatError
- ModelThrottleError / UnsupportedClientVersionError / ContextWindowExceededError
- ClientThrottleError / ServiceThrottleError / UsageLimitReachedError（HOURLY/DAILY/WEEKLY/MONTHLY）
- NewUserAccessPausedError / OverageLimitReachedError / TemporarilySuspendedError / AccountNotSupportedError
- InvalidModelError / DocumentValidationError / GenericValidationError / ClientNetworkError / AccessDeniedError

mapError() 把 AWS SDK 异常映射到上述类型。

重试策略：
- StandardRetryStrategy：maxAttempts=3，exponential backoff，THROTTLING/TRANSIENT 可重试
- AdaptiveRetryStrategy：叠加 CubicRateLimiter（token bucket + cubic 算法）
- KiroRetryStrategy：retryDelay 乘以 5，>500ms 展示 info，>3000ms 展示 warn

后段打包了 LangChain InMemoryCache（SHA1 key + Map）。

### mcp-config-manager-BVKB8dJE.js（完整）
前段：JSON-RPC 2.0 McpServer，TaskStore 异步队列，requestResolvers Map，轮询默认 1000ms。
后段：打包 AJV 完整源码（CodeGen/Scope/ValueScope），将 JSON Schema 编译为验证函数，用于校验 MCP 工具 inputSchema。

### stream-CtDBTPgX.js（中段）
LangChain Core 消息系统：
- BaseMessage：content 支持 string | ContentBlock[]，ContentBlock 有 text/image_url/base64/id 四种 source_type
- ToolMessage：status + tool_call_id，lc_direct_tool_output=true
- mergeContent()：streaming chunk 按 index 对齐合并，id/name/model_provider 取后值不累加

## 第五批模块分析（2026-03-17）

### clarification-handler-Zo3AuRjE.js
意图澄清处理器。acquireYield("clarification") 挂起执行，弹出 userInput action 等待用户选择，收到答案后 releaseYield 恢复。

### command-approval-_g4NkBJ8.js
写文件安全过滤层。路径黑名单（mcp.json/.kiroignore/.vscode/.git），二重 decodeURIComponent+NFC/NFKC 规范化防路径穿越，检测 $schema 字段防 RemoteJsonSchema 注入。

### command-approval-MQd-5ajF.js
Shell 命令白/黑名单工厂，getTrustedCommands/getCommandDenylist 默认空数组，由外部策略注入。

### logger-CTb8_yz9.js
双通道日志。普通日志内存无限队列，LLM 日志循环队列上限 500 条，capture() 清空返回，供 telemetry 上报。

### tool-tags-NGeoUHCI.js
工具标签枚举：read/write/shell/web/spec/@mcp/@powers/@builtin，每个标签带人类可读描述。

### tool-filter-CS5Fsu0N.js
工具筛选器，支持 glob 匹配。filterToolsWithOptions 可额外包含 MCP 和 Powers 工具，mergeTools 去重合并。

### tool-message-list-B1XEM-VL.js
LangChain ToolMessageList 扩展，_getType()=tool，打印只输出 toolMessages 数量防日志爆炸。

### message-parts-D8WKxpG9.js
消息内容分解，分离 text/imageUrl/documentUrl。contextProjection() 估算 token 占用率，>=95% truncate，>=80% summarize。

### prompt-template-C_Mn10zi.js
用 UUID 占位符做二阶替换防键名碰撞，p() 有键模板，r() 无键模板，j() 多段拼接。

### config-file-watcher-DbLp_qKw.js
chokidar 封装，监视 ~/.kiro 和 workspace/.kiro，300ms 防抖，感知 steering/skills/mcp.json 实时变更。

### parse-front-matter-CXai4UYs.js
gray-matter 封装，禁用 JS 引擎，Zod schema 校验 frontMatter 字段，支持自定义 YAML 解析器注入。
### cancellation-C60foDZK.js
Promise + AbortSignal 绑定，abort 时用 UnknownError 包装 reject。

### execution-log-controller-BGxzu20b.js
执行日志控制器单例工厂，未初始化调用直接抛错。

### config-constants-CsFJbjLP.js
路径常量：.kiro / steering / .md / AGENTS.md / skills / SKILL.md / specs，全系统目录规范来源。

### dev-inspector-CF526NYm.js
开发调试钩子，notifyDevInspector 静默调用，异常被吞不影响主流程。

### unknown-error-7JAvTvbU.js
UnknownError：把任意非 Error 值包装成 SessionError，userFacingMessage 固定返回 "An unexpected error occurred"。

### terminal-dGJDGXey.js
终端单例工厂，setShellTypeProvider 注入 shell 类型检测，默认 bash。

### file-context-ClFefQMc.js
编辑器文件上下文适配器，loadCurrentFile/loadOpenEditorFiles 由 VSCode 扩展侧注入。

### range-utils-CHnKtlN2.js
LSP Range 构造工具，lineRange() 把 1-based 行号转 0-based LSP Range。

### spec-platform-361SGdHa.js
Spec agent 平台适配层，parseMarkdownRequirements/validateFileAccess/hooks 全部外部注入，mapContextReferencesToFiles() 转 fileTree 结构（target:500）。

### message-analyzer-BAqw8PNp.js
消息质量检测，发现 AI 回复末尾冒号但无 toolUse 时上报 MissingExpectedToolUse metric，let's/start/create 等词豁免。

### number-coercion-CeAOJ7Po.js
递归遍历工具 input，把 ZodNumber 字段对应的字符串转数字，上报 stringToNumberCoercions metric，防 schema 校验失败。

### file-lock-D4f90hJ2.js
基于 Promise 链的文件锁单例，acquireLock 等待已有锁释放后重试，withLock 保证 finally 释放。

### prompt-processor-Dw_f2vjD.js
解析 prompt 中的 {{providerId:query}} 占位符，调用 @kiro/context-providers 逐个替换，provider 缺失只 warn 不报错。

### strip-json-comments-BVnY24gX.js
清洗 JSON with comments（// 和 /* */），支持 trailingCommas，用于解析 mcp.json 等允许注释的配置。

### workspace-object-CspubCq6.js
工作区路径安全层。fromRelativePath() 校验路径不逃逸 workspace，InvalidWorkspacePathError 提示只能编辑 workspace 内文件。~/.kiro 路径被特殊允许。

### auth-DCPC05L9.js
AuthorizationError 类型守卫（isAuthorizationError），Signal 类实现 Promise-like 的可取消异步原语（pending/fulfilled/rejected 三态，resolveAfter/tryCancel/tryReject 等方法），web-fetch-utils 用它控制请求超时。

## 关键依赖关系补充

- **@kiro/context-providers**：prompt-processor 调用，提供 context reference 解析能力
- **@agentclientprotocol/sdk**：index.js 的 ACP session 底层协议
- **@aws/codewhisperer-streaming-client**：q-developer-converse 和 validator 使用，定义 ToolResultStatus 等枚举
- **@mozilla/readability + jsdom**：web-fetch-utils 网页正文提取
- **chokidar**：config-file-watcher 文件系统监听
- **gray-matter**：parse-front-matter YAML 解析
- **zod-stream**：get-user-input 流式 JSON 解析
- **async-sema**：mcp-config-manager 并发信号量
- **minimatch**：node-progressive-context-source glob 文件匹配
- **axios + axios-retry**：web-fetch-utils HTTP 客户端
- **dedent**：prompt-template 模板字符串格式化
## 第六批模块分析（2026-03-17）

### tool-tags-NGeoUHCI.js
工具分类枚举。READ/WRITE/SHELL/WEB/SPEC/MCP(@mcp)/POWERS(@powers)/BUILTIN(@builtin)，每类附带描述字符串。hook 配置的 toolTypes 字段就是用这套枚举。

### config-constants-CsFJbjLP.js
硬编码路径常量：.kiro 目录名、steering/specs/skills 子目录、AGENTS.md 和 SKILL.md 文件名。整个系统凡是涉及这些路径的都从这里取值。

### terminal-dGJDGXey.js
终端工厂注入点。setTerminalProvider() + getTerminalProvider()，另有 shellType 查询（默认 bash）。

### unknown-error-7JAvTvbU.js
兜底错误类 UnknownError，继承 KiroBaseError，userFacingSessionErrorMessage 返回固定字符串。

### execution-log-controller-BGxzu20b.js
执行日志控制器注入点，全局单例，未初始化时 get 直接 throw。

### dev-inspector-CF526NYm.js
开发调试用 inspector，setInspector + notifyInspector，调用出错静默吞掉。

### message-analyzer-BAqw8PNp.js
检测 AI 回复末尾以冒号结尾但未调用工具的异常模式，上报 MissingExpectedToolUse 指标。

### file-context-ClFefQMc.js
注入 loadCurrentFile / loadOpenEditorFiles，供 agent 感知当前打开文件。

### range-utils-CHnKtlN2.js
行列范围辅助函数，createPosition / createRange / createLineRange，行号从 1 开始转为 0-based。

### number-coercion-CeAOJ7Po.js
工具参数数字类型兼容层。LLM 有时把数字输出成字符串，此模块按 Zod schema 递归把 string 强转 number，并上报 stringToNumberCoercions 指标。

### strip-json-comments-BVnY24gX.js
完整实现，支持 // 和 /* */ 注释，可选 trailingCommas 容错，用于解析 mcp.json 等带注释的配置文件。

### file-lock-D4f90hJ2.js
基于 Promise chain 的文件级互斥锁单例（FileLock）。acquireLock / releaseLock / withLock，写文件前必须先拿锁，防止并发竞争。

### parse-front-matter-CXai4UYs.js
解析 steering/spec/skill 文件的 YAML front matter，JavaScript engine 被显式禁用防注入，用 Zod schema 校验 front matter 字段。

### command-approval-MQd-5ajF.js
getCommandApprovalProvider 注入点，提供 getTrustedCommands / getCommandDenylist，默认都返回空数组。

### command-approval-_g4NkBJ8.js
命令白名单/黑名单核心逻辑。路径黑名单：mcp.json / .kiroignore / .vscode/ / .git/ 等敏感路径禁写。Unicode NFC/NFKC 规范化 + 双重 decodeURIComponent 防路径穿越攻击。

### message-parts-D8WKxpG9.js
解析消息 parts 数组（text/mention/imageUrl/documentUrl），计算 token 使用率并给出 truncate/summarize/proceed 三档决策（95%/80% 阈值）。

### prompt-processor-Dw_f2vjD.js
自定义 agent prompt 中的 #[[providerId:query]] 引用解析，通过 ContextProviderRegistry 解析后替换为实际内容。

### spec-platform-361SGdHa.js
Spec 平台注入点：parseMarkdownRequirements / validateFileAccess / loadOpenEditorFiles / runPreTaskExecutionHooks / runPostTaskExecutionHooks。任务文件列表转换为 fileTree（expandedPaths + target:500）。

### workspace-object-CspubCq6.js
WorkspaceObject 类封装 URI+relativePath，fromRelativePath 做路径安全检查（必须在 workspace 内或 ~/.kiro/），解析失败抛 InvalidWorkspacePathError / WorkspaceUriNotFoundError。

### config-file-watcher-DbLp_qKw.js
基于 chokidar 的配置文件监听器，300ms debounce，监听 home + workspace 下的 .kiro/<subPath> 目录。addWorkspacePaths 支持运行时动态追加监听路径。

### types-DThkhnH-.js
Custom agent 配置 Zod schema。SubAgentConfig（工具列表 string|array，includeMcpJson，includePowers）和 CustomAgentConfig（增加 prompt 字段，tools 支持 "*"）。

### prompts-DfcXHzAI.js（完整内容）
这是 Kiro system prompt 的完整来源，buildSystemPrompt() 输出的内容包含：
- <key_kiro_features>：详细描述 specs/hooks/steering/mcp/internet_access 五大功能，包含 Hook 完整 JSON schema 和示例
- <current_date_and_time>：动态注入日期
- <system_information>：OS/Platform/Shell 信息
- <model_information>：模型名称和描述
- <platform_specific_command_guidelines>：win32 vs 其他平台命令规范
- <goal> + <subagents>：任务执行目标和 sub-agent 使用规范
chat mode 与 agentic mode 分别调用 buildChatSystemPrompt() 和 buildSystemPrompt()。

## 第七批模块分析（2026-03-18）

### _commonjsHelpers-DaMA6jEr.js
极小的 CommonJS 兼容辅助块，只有两件事：

- `l3()`：把 CommonJS 默认导出规范化成 ESM 风格 default
- 初始化全局对象引用：优先 `globalThis`，再回退 `window/global/self`

定位：纯 bundler helper，不承载 Kiro 业务语义，但被多个旧式依赖块复用。

### async-delivered-object/index.js
单值异步交付原语，和 `streamed-data-object` 这种增量流不同，它只维护“当前值 + 完成承诺”。

- `getCurrentValue()`：同步读取当前快照
- `update()`：流未结束前可多次更新；结束后抛 `Cannot update a completed AsyncDeliveredObject`
- `complete()`：可在收尾时覆盖最终值，并 resolve 内部 Promise
- `waitForCompletion()`：等待最终值

定位：适合保存 agent 原始回复、最终聚合文本、延后可用的单对象结果。

### async-stream/index.js
最底层异步消息流实现，`streamed-data-object` 等上层流式能力都建立在它之上。

- 错误类型：
  - `StreamError`
  - `StreamCompleteError`：关闭后继续监听/读取
  - `StreamAlreadyHasListenerError`：同一流只允许一个等待中的 listener
- `fromArray()`：把静态数组包成已结束流
- 消费接口：
  - `Symbol.asyncIterator`
  - `next()`
  - `all()`
  - `last()`
- 生产接口：
  - `send()` / `trySend()`
  - `close()` / `tryClose()`
  - `error()`
  - `onEnd()`

关键点：它内部维护的是带类型标记的事件队列（`message/error/close`），不是裸数组，所以可以把异常和结束信号也纳入同一条异步通道。

### base-BCJFoMV_.js
超大基础 vendor chunk，主要不是 Kiro 自研业务，而是 schema / tool / runnable 相关底座的打包集合。

- 包含大量 Zod 3/4 兼容实现与错误映射逻辑
- 包含 schema 到 JSON Schema 的转换辅助
- 包含 retry/backoff 逻辑
- 末尾还能看到 `RunnableToolLike` 这类 LangChain 风格 runnable/tool 适配器

从实际内容看，它更像“验证与工具协议基础库的公共底盘”，被 `validator`、`get-user-input`、tool schema 相关链路复用；分析时不应把它误判成单一业务模块。

### streamed-data-object/index.js
结构化流式对象层，用 keypath 增量构建 JSON-like 数据，并向订阅者发出对象级事件。

- 根对象必须是 `array` 或 `object`
- 支持三类增量写入：
  - `extendArrayAtKeypath()`
  - `mergeObjectAtKeypath()`
  - `extendStringAtKeypath()`
- 内部维护：
  - `dataByKeypath`
  - `incompleteDataByKeypath`
  - `completedDataByKeypath`
  - `subscribersByKeypath`
- 订阅接口：
  - `subscribe(keyPath)`
  - `subscribeAll()`
- 事件类型包括：
  - `ObjectSubscribed`
  - `ArrayExtended`
  - `ObjectUpdated`
  - `StringExtended`
  - `ObjectCompleted`
  - `ObjectCompletedWithoutData`

关键判断：这是 Kiro 处理“LLM 边生成、边组装结构化对象”的核心数据层；`async-stream` 负责通道，这个模块负责 keypath 级状态机和事件语义。

### string-BZO3_EzG.js
另一个大型 vendor chunk，主体是 Zod v4 的 string/format/check 实现与相关 JSON Schema 映射逻辑。

- 内置大量字符串格式规则：
  - `email`
  - `uuid`
  - `url`
  - `emoji`
  - `datetime/date/time/duration`
  - `ipv4/ipv6/cidr`
  - `base64/base64url`
  - `jwt`
- 含字符串正规化、长度检查、regex 检查、format 到 schema 约束的转换
- 尾部还有一组格式 regex registry 与 metadata registry

定位：这块本质上是 schema 验证基础设施，不是 Kiro 独有逻辑；它解释了为什么 runtime 在工具参数、流式对象、配置文件上能做很细的字符串格式校验。

### telemetry/index.js
remote tool 专用 telemetry 聚合层，建立在 `telemetry-Bf0GI6nJ.js` 提供的基础 reporter 之上。

- `setRemoteToolTelemetryReporterFactory()`：允许宿主替换 reporter 工厂
- `getRemoteToolTelemetryReporter()`：拿到 reporter，没有注入时返回 no-op 版本
- `initializeRemoteToolMetrics()`：注册 remote tool 相关 counters / histogram
- `RemoteToolTelemetryRecorder.start()`：
  - 记录 invoked
  - 后续记录 success / error / fault / rejection / acceptance / trust
  - 同时记录 duration
- `WebFetchTelemetryRecorder.start()`：
  - 额外统计 `contentSize`、`fetchTime`、`matchCount`
  - 跟踪 mode 使用（`full/truncated/selective`）
  - 跟踪 URL 清洗、非 HTTPS 拦截、unsafe redirect、用户 accept/trust/reject

定位：`telemetry-Bf0GI6nJ.js` 是通用 span/metric 注入层，`telemetry/index.js` 是 remote tool 尤其 `web_fetch` 的业务指标封装层。

### web-fetch-utils-C1Z4KMmp.js
网页抓取工具的底层实现，涵盖取消信号、HTTP 请求、内容筛选、HTML 正文抽取与结果格式化。

- URL 与内容处理：
  - `re6()`：清洗 URL，去 query/hash
  - `se7()`：限制仅允许 HTTPS
  - `ne6()` / `oe5()`：判定是否可读文本 / HTML 内容类型
  - `ce6()` / `he5()`：selective mode，按关键短语截取上下文片段
  - `ae6()`：用 `jsdom + readability` 提取正文
  - `le5()`：超长内容截断
  - `ue4()`：拼成最终返回文本
- `Signal`（Promise-like 可取消异步原语）：
  - `resolve/reject/cancel`
  - `resolveAfter`
  - `tryResolve/tryReject/tryCancel/tryResolveAfter`
- `WebFetchClient`：
  - 基于 `axios + axios-retry`
  - 默认 `30s` 超时、最多 `1` 次重试、最大 `10MB`
  - 5xx 与网络错误可重试，4xx 默认不重试
  - 把 axios 错误映射为 `Timeout / ContentTooLarge / Http / Network` 等专用错误
- 还定义了 `UnsupportedContentType`、`UnsafeRedirect`、`Rejected`、`InvalidInput` 等错误类型

关键判断：这是 `web_fetch` 工具的真正实现底座，不只是“抓网页”，还把审批、取消、超时、正文抽取、selective mode 和错误语义都封在同一层。

---

## 总体结论（packages/kiro-agent/dist 全量分析）

按 2026-03-18 的重新去重统计，`packages/kiro-agent/dist/**` 当前共识别 `88` 个内部模块块。本文已覆盖大部分核心模块，但不再使用“81 个模块已全部读完”这一旧结论。整个 kiro-agent 的架构可归纳为：

**运行时核心**：disclose-context（工具注册+AgentExecution状态机）→ chat-agent/sub-agent（执行入口）→ q-developer-converse（AWS Bedrock LLM调用）

**上下文管理**：context-chat-message（消息构建）→ pruning-service（token裁剪，hash去重）→ token-monitor（ModelContext窗口监控）→ node-progressive-context-source（steering文件加载）

**工具体系**：tool-tags定义8大分类 → disclose-context注册30+内置工具 → remote-tools注入MCP远程工具 → pending-changes管理Supervised审批

**配置体系**：config-file-watcher监听 → mcp-config-manager解析验证 → powers-manager管理Powers → autonomy-mode控制Autopilot/Supervised

**持久化**：message-replay（JSONL session存储）→ file-lock（写锁）→ execution-log-controller（执行日志）

**通信层**：index.js（ACP KiroAgent顶层）→ acp-remote-mcp-client（远程MCP连接）→ session-update-utils（状态同步到UI）

**安全**：command-approval白名单/黑名单 → workspace-object路径安全检查 → file-lock并发保护 → number-coercion/strip-json-comments容错
