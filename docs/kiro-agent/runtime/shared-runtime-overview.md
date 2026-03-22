# kiro-shared / kiro-client / acp-type-covenant 模块分析

> 分析日期：2026-03-17

---

## kiro-client

路径：`packages/kiro-client/dist/`，共 2 个文件（client.js 12.6KB + index.js 0.1KB）。

### KiroClient 类（client.js）
基于 ACP（Agent Client Protocol）SDK 的客户端实现，是 VS Code 扩展侧和 agent 进程之间的通信桥梁。

核心设计：
- 构造时接收 `capabilities`（能力声明）、`clientInfo`、`stream`（通信流）
- `mapCapabilities()` 把能力列表映射成处理器函数和协议字段：fs（读/写文件）、terminal（create/kill/release/output/wait）、扩展方法
- `initialize()` 幂等初始化，并发安全（同一 promise 复用），连接断开后自动清理
- `prompt(request)` → 向 agent 发送用户输入
- `cancel(sessionId)` → 取消进行中的操作
- `onSessionUpdate / onPermissionRequest` → 注册 session 状态和权限请求回调
- `onExtNotification(method, handler)` → 监听 agent 发来的扩展通知（如 `_kiro/steering/documents_changed`），支持多 handler
- `extNotification(method, params)` → ACP SDK 回调入口，派发到所有注册 handler

能力字段结构（`capabilities.fs._meta.kiro`）用于向 agent 声明客户端支持的文件系统能力细节。

---

## kiro-shared

路径：`packages/kiro-shared/dist/`，共 17 个模块，总大小约 263KB。
这是扩展主入口（`dist/index.js`）的核心依赖库，包含认证、MCP 管理、遥测、工具计数等基础设施。

### index.js（顶层入口）
聚合导出所有子模块。关键导入链：
`external-idp-auth-provider` → `sso-oidc-client` → `mcp-manager` → `resolve-registry-entries`

顶层关键全局类：
- `TokenStorage`：认证 token 的磁盘持久化，路径 `~/.aws/sso/cache/kiro-auth-token.json`，`fs.watchFile` 监听变化，自动触发 `_onDidChange` 事件
- `ProfileStorage`：用户 profile（ARN + name）持久化到 VS Code `globalStorageUri`，单例，Sema(1) 并发保护
- token sanitize 逻辑：Google/Github → authMethod=social；Enterprise/BuilderId/Internal → authMethod=IdC

### external-idp-auth-provider-Bx1PFVkO.js（40KB）
OAuth2 / OIDC 外部身份提供商实现，处理 Social（Google/Github）和 IdC（企业 SSO）登录流程。

核心类：
- `AuthCallbackHandler`：深链接回调处理器，注册 `kiro://kiro.oauth/callback` URI 监听，waitForCallback 带双重超时（60s 警告 / 10min 放弃），Promise 映射到 state 参数
- `AuthSSOServer`：本地 HTTP 回调服务器（`http://127.0.0.1` 随机端口），备用回调方案（非深链接时使用）
- `authCallbackHandler`：全局单例

### sso-oidc-client-BhJdhafO.js（28KB）
完整认证错误类型体系 + AWS SSO OIDC 客户端封装。

错误类层级：
`AuthError` → `MissingTokenError / MalformedTokenError / InvalidAuthError / InvalidSSOAuthError / InvalidIdCAuthError`
`UserEnvironmentError` → `NetworkIssueError / FileSystemAccessError`
`ServerIssueError / UnexpectedIssueError / CanceledError / AbandonedError`
`InvalidAuthError` → `AuthProviderDeniedAccess / AuthProviderFailure / MissingCodeError / MissingStateError / InvalidStateError`

关键常量：AUTH_TOKEN_INVALIDATION_OFFSET=3min，REFRESH_BEFORE_EXPIRY=10min，REFRESH_LOOP=60s
依赖 `@aws-sdk/client-sso-oidc`，axios + axiosRetry 做 HTTP 重试

### mcp-manager-CVW9dPVa.js（69KB）
完整 MCP（Model Context Protocol）配置加载 + 连接管理器，这里是 VS Code 扩展层的 MCP，与 kiro-agent 包内的 mcp-config-manager 分工：
- kiro-shared/mcp-manager：负责从磁盘读取配置、启动 stdio/SSE/StreamableHTTP 连接、向 VS Code 展示错误
- kiro-agent/mcp-config-manager：负责 agent 运行时的 tool 注册和 JSON-RPC 调用

MCP 配置 Schema（`MCPOptionsSchema`）字段：
command/args/env/cwd（STDIO）、url/headers（HTTP/SSE）、type、timeout、disabled、autoApprove、disabledTools

配置加载优先级：workspace config > user config，Powers MCP 独立加载（`~/.kiro/settings/mcp.json` 的 powers 字段）

环境变量安全机制：`${VAR}` 占位符需用户在 `kiroAgent.mcpApprovedEnvVars` 中预先审批，未审批变量保留原样并弹出 VS Code 警告

传输层支持：`StdioClientTransport`（本地进程）、`SSEClientTransport`、`StreamableHTTPClientTransport`

---

## acp-type-covenant

路径：`packages/acp-type-covenant/dist-cjs/` 和 `dist-es/`，纯类型定义包（<1KB 每文件），无运行时逻辑。

定义了 ACP 协议的能力契约（TypeScript 接口），分为以下命名空间：
- `capabilities/fs`：read-file / read-text-file / write-file / write-text-file / delete / read-directory / stat
- `capabilities/terminal`：create / kill / release / output / wait-for-exit
- `capabilities/workspace`：active-file / currently-open-files
- `capabilities/search`：text-search / find-files
- `capabilities/steering`：get-documents / documents-changed / progressive-context-changed
- `capabilities/powers`：powers-changed
- `capabilities/tasks`：list-tasks / get-task-metadata
- `config/model-option-meta`：模型选项元数据类型
- `session/schemas`：session 协议 schema（8.7KB，最大文件）

作用：kiro-client 和 kiro-agent 两侧共享同一套能力类型定义，保证 ACP 协议类型安全。

---

## 三包关系总结

```

---

## kiro-shared 剩余模块（第二批）

### portal-auth-provider-5fy62fEs.js（17KB）
完整的 Portal 登录流程实现（AWS Builder ID / Identity Center 网页登录）。

核心类：
- `PortalAuthServer`：本地 HTTP 回调服务器，固定端口列表 `[3128, 4649, 6588, 8008, 9091, 49153-53153]`，`/oauth/callback` 和 `/signin/callback` 双路径，10min 超时 + 1min 警告，CSRF state 校验
- `isMwinitToolAvailable()`：检测 Midway（Amazon 内网 SSO 工具 `mwinit`）是否可用
- `ERROR_MAPPER`：错误分类遥测映射（blocked/abort/abandon/badInput/environmentIssue/unauthorized/failure）

### resolve-registry-entries-DSUmtYNT.js（12KB）
MCP 注册表解析器，处理从 Kiro 注册表 JSON 到本地可用 MCP 配置的转换。

核心结构：
- `RegistryStore` 单例：`Map<name, ServerDetail>` 存储注册表，`onDidChange` VS Code 事件，`setRegistry()` 原子替换
- `ServerDetailSchema`（Zod）：name 3-200 字符、packages/remotes 互斥、版本号不允许范围
- `PackageSchema`：registryType（npm/pypi/oci）+ transport（stdio/streamable-http/sse）
- `deriveCommandFromPackage()`：npm → `npx -y`，pypi → `uvx`，oci → `docker run`，自动注入 `NPM_CONFIG_REGISTRY` 环境变量
- `RegistryServerConfigEntrySchema`：type=registry 的配置项（timeout/headers/env/disabled/autoApprove）

### errors-C5wYZWCn.js（5.6KB）
三个独立 logger：
- `logger`：VS Code Output Channel "Kiro Logs"，带本地内存缓冲（`capture()` 返回并清空）
- `mcpLogger`：VS Code Output Channel "Kiro - MCP Logs"，按 serverName 分桶存储，`getLogsForServer(name)` 按服务器查询
- `powersLogger`：VS Code Output Channel "Kiro - Powers"

三个 logger 都支持 trace/debug/info/warn/error + capture() + show()。

错误辅助函数：`isAbortError`（名字或消息含 Abort）、`isBlockedAccessError`（NewUserAccessPausedError 或 access not available）、`mapUnknownToErrorType`、`TrustedError`（可信错误基类）

### paths-D3AmlBbI.js（1KB）
路径工具：
- `getHomeKiroPath()` → `~/.kiro`
- `getWorkspaceKiroPath(dir)` → `<dir>/.kiro`
- `getActiveMcpConfigLocation(workspaceDirs)` → 返回 `{ workspaceConfigPaths, userConfigPath }`，优先检查各 workspace 的 `.kiro/settings/mcp.json`，回退到 `~/.kiro/settings/mcp.json`

### timer-Ck322162.js（2.8KB）
三个工具：
- `IntervalTimer` 类：`start(cb, ms)` / `cancel()`，幂等取消
- `addPrivacyHeadersMiddleware(client)`：telemetry.dataSharing 未开启时自动注入 `x-amzn-codewhisperer-optout: true`
- `addAgentModeHeadersMiddleware(client, mode)`：注入 `x-amzn-kiro-agent-mode` 请求头
- `addExternalIdpTokenTypeMiddleware(client, authMethod)`：外部 IdP 时注入 `TokenType: EXTERNAL_IDP`
- `updateResolvedIDESetting(section, setting, value, scope)`：智能写入 VS Code 配置（有 workspace 就写 workspace 级，否则写 global 级）

---

## kiro-context-providers 包分析

> 路径：`packages/kiro-context-providers/dist`，总大小约 25KB，8 个 provider + 工具类

### 统一接口设计
每个 provider 实现三个方法：
- `resolveContent(query)` — 按 query 字符串解析并返回内容字符串
- `getItems()` — 返回可供 UI 选择的 ContextItem 列表（刷新缓存）
- `search(query)` — 同步子串搜索（基于上次缓存）

### 语法规则：`#[[providerId:query]]`
`parseContextReferences()` 用 `/\#\[\[([^\]]+)\]\]/g` 提取，只在第一个冒号处分割，query 内部可含冒号（路径友好）。

### MCPContextProvider（5.5KB）
语法：`#[[mcp:type:server-name:identifier]]`
- type = resource / resourceTemplate / prompt
- prompt 走 `mcpPromptResolver(serverName, promptName, args)`
- resourceTemplate 先展开 URI 模板（`{key}` 替换为 URL-encoded 参数）再调用 `mcpResourceResolver`
- blob 类型资源：text/* 和 application/json 自动 base64 解码为 UTF-8
- 缓存：`Map<"type:server:id", ContextItem>`，每次 `getItems()` 全量刷新

### FileContextProvider（3.3KB）
语法：`#[[file:path/to/file.ts]]` 或 `#[[file:path:10-20]]` 或 `#[[file:path:5]]`
- 从 `FileSystemCache` 读文件列表（不维护自己的缓存）
- `parseQuery()` 从末尾 `:` 解析行范围，格式 `start-end` 或单行号，解析失败则整个字符串视为路径
- 返回格式：`` File: path (lines N-M)\n\n```\ncontent\n``` ``

### FolderContextProvider（1.3KB）
语法：`#[[folder:path]]`
- 从 `FileSystemCache` 读文件夹列表
- 返回格式：`[dir] name` 或 `      name`

### FileSystemCache（3.8KB）
文件和文件夹的共享缓存，file/folder provider 共用同一个 tree walk 结果。
- 初始化：`walkWorkspace()` 异步单次扫描，结果存 `Map<path, item>`
- 增量更新：`addFile/removeFile/addFolder/removeFolder/removeTree(dirPath)` — `removeTree` 递归删除所有以该路径开头的子项
- `searchFiles/searchFolders(query)` 同步子串搜索（不等待初始化）

### SteeringContextProvider（2.4KB）
语法：`#[[steering:scope:path]]`，scope = global / workspace
- 注入格式：包含 `<user-rule id=query>` XML 标签，workspace 规则附加「workspace 规则优先于 global」提示
- `getItems()` 每次全量从扩展侧刷新

### SpecContextProvider（2.3KB）
语法：`#[[spec:type:path]]`
- 同 Steering 结构，从扩展侧获取 spec 文件列表

### CurrentlyOpenFilesContextProvider（1.5KB）
语法：`#[[currentlyOpenFiles:active]]` 或 `#[[currentlyOpenFiles:all]]`
- active → `activeFileResolver()`，all → `currentlyOpenFilesResolver()`
- 固定返回两个 item（Active File / All Open Files），无搜索

### ContextProviderRegistry（1KB）
简单 Map 包装，大小写不敏感的 id 索引，`register/get/getAll/has/unregister`。

  ├── kiro-shared（基础设施：认证/MCP配置/遥测）
  │     ├── TokenStorage → ~/.aws/sso/cache/kiro-auth-token.json
  │     ├── ProfileStorage → globalStorageUri/profile.json
  │     ├── ExternalIdpAuthProvider → OAuth2/OIDC 登录
  │     └── MCP Manager → 加载配置 + 管理连接
  └── kiro-client（ACP通信桥梁）
        ├── KiroClient.initialize() → ACP 握手
        ├── KiroClient.prompt() → 发送用户输入
        ├── KiroClient.onSessionUpdate() → 接收 agent 状态推送
        └── KiroClient.onExtNotification() → 接收 steering/powers 等通知

acp-type-covenant（类型契约，被两侧共享）
```

## packages/autocomplete

单文件 bundle（884KB），底层调用 `@aws/codewhisperer-runtime`。

核心配置（`DEFAULT_AUTOCOMPLETE_OPTS`）：debounce 350ms，FIM prefix 75% / suffix 25%，sliding window 500行，文件上限 10KB，最近编辑相似度阈值 0.3，启用 cache / otherFiles / recentlyEdited。

`provideInlineCompletionItems` 用 UUID 竞争取消：每次触发生成新 UUID，debounce 结束后比较 UUID，不匹配直接丢弃，避免乱序响应。

## packages/hook-editor

单文件 React Webview（648KB），纯 UI 层，业务逻辑在主扩展 `extension.js` 里。搜索 `onSave`、`kiro-pill` 等 UI 组件关键词可以确认这是 hook 编辑器的前端界面。

## packages/kiro-ui-agent-chat

`dist/` 只有空 `styles.js`，实际 bundle 在 `bundled-webviews/requirements-webview.js`（1MB React app）。

## 主扩展 dist/extension.js（47MB）

单行压缩，用 StreamReader 定位关键偏移。

**激活流程（activate2）**：
`register61(context)` → `initializeAgentTelemetry()` → `MetricReporter.startPeriodicReporterLoop()` → `register60()` → `initializePowersRegistry()` → `enforceProfileSelection()` → `activate()`（continuedev）→ `initializeAvailableModelsCache()` → `remoteToolsDiscovery()`

**核心对象初始化**：`agentController = new Ct5()` → `initializeKiroAgent(Gt7)` 注入全部依赖（authProvider、modelConfig、telemetry、mcpManager 等）。

**认证链**：`authProvider.readToken()` → `AuthProviderSession.isLoggedIn()` / `waitForSignIn()` / `logout()`，三种 provider（BuilderId / Enterprise / Social）共用同一接口，统计分别计入 `builderIdLoginCount` / `enterpriseLoginCount` / `socialLoginCount` 遥测。

**遥测端点**：beta → `kiro.aws.dev`，gamma → `kiro.aws.dev`，prod → `kiro.dev`，三环境硬编码。

**autocomplete**：底层 `@aws/codewhisperer-runtime`，FIM 模式，350ms debounce + UUID 竞争取消。

## packages/continuedev

无独立 dist，全部打包进主 `extension.js`。源码路径保留在 bundle 注释里（`packages/continuedev/core/...`）。

**defaultConfig contextProviders**：code / docs / repo-map / diff / terminal / problems / folder / codebase 等 8 个内置 provider。

**ContinueGUIWebviewViewProvider**：注册 Webview panel，处理 `webviewProtocol` 消息，调用 `sanitize` 过滤输出。

**recentlyEditedFilesCache**：QuickLRU 最多缓存 100 个最近编辑文件，用于 context retrieval。

**getContextItems**：支持 `file:path#startLine-endLine` 语法读取文件片段，返回带 `relativePath` 的 context item。

## packages/kiro-ui-powers

`dist/` 只有空 `styles.js`，Powers UI 逻辑打包进主 `extension.js`。

## extension-resources/hook.json

Hook 系统的 JSON Schema（draft-07）。触发条件（when）和执行动作（then）完全解耦：

触发条件四种：`fileEdited`（glob pattern + scopeId）、`fileCreated`、`fileDeleted`、`userTriggered`。

执行动作两种：`alert`（弹消息）、`askAgent`（发 prompt 给 agent，支持 promptId 引用预置 prompt）。

## models/all-MiniLM-L6-v2

本地打包的 sentence-transformers 向量模型（22MB ONNX 量化版），用于 codebase 语义检索 embedding，完全离线运行，不需要外部 API。

## extension.js — Powers 系统

三级 Registry：`KIRO_RECOMMENDED`（官方推荐）→ `USER_ADDED`（用户自添加）→ `USER_REGISTRY`（自定义仓库）。

V1→V2 迁移：读取旧 `registry.json`，将已安装的 powers 按 source 类型分配到新 registry，备份旧文件为 `.v1.backup`。

`RegistryResolver.getPower(registryId, powerName)` 是统一查询入口，优先走 KiroRecommended 缓存（内存），其次 UserAdded，最后远程 UserRegistry。

`triggerOnPowerInstalledEvent` / `triggerOnPowerUninstalledEvent` 通知 `installedPowersListViewProvider` 刷新 UI。

## extension.js — Hook 系统

Hook 运行时（HookRunner）在 `extension.js` 中处理事件分发，`kiro.hooks.*` 命令族负责增删改查 hook 配置文件（存储在 `.kiro/hooks/` 目录下），hook id 默认为文件路径。

## extension.js — Workspace 目录结构

Kiro 所有配置文件统一存放在 `.kiro/` 目录下：

- `.kiro/specs/<featureName>/` — Spec 文档，子目录按 feature 名称组织，每个 feature 包含 `requirements.md`、`design.md`、`tasks.md`、`bugfix.md` 四种文档类型
- `.kiro/steering/` — Steering 规则文档（`.md`），front matter 控制 inclusion 模式：`always`、`fileMatch`、`manual`、`auto`
- `.kiro/hooks/` — Hook 配置文件（`.json`），文件路径即为 hook id
- `AGENTS.md` — 根目录 steering 文件（与 `.kiro/steering/` 并列）

`SpecDocumentType` 枚举：`requirements`、`design`、`tasks`、`bugfix`。

`SteeringContextFrontMatterSchema`（Zod）：`inclusion`（always/fileMatch/manual/auto）+ `fileMatchPattern`（string 或 string[]）+ `name` + `description`。

## extension.js — Steering 命令

`kiro.steering.createInitialSteering` 命令触发时：创建 `.kiro/` 和 `.kiro/steering/` 目录，新建 `vibe` session，调用 `chatAgent({ agentMode: 'generate-steering' })` 让 agent 自动分析工作区并生成初始 steering 规则。

## extension.js — Spec 命令

`kiro.spec.createDocument` 通过 Webview 消息传递触发，`SpecDocumentManager` 监听 `.kiro/specs/` 目录变化（`FileSystemWatcher`），`listFeatures()` 返回所有子目录名，每个 feature 的文档通过 `readRangeInFile` / `saveFile` / `focusEditor` 消息与 Webview 通信。

## product.json — Kiro 主程序关键配置

- Kiro 版本：`0.11.34`，基于 VSCode `1.107.1`，commit `7b506f30`，quality `stable`
- `dataFolderName`：`.kiro`，URL 协议：`kiro://`
- 更新地址：`https://prod.download.desktop.kiro.dev`
- 扩展市场：Open VSX（`https://open-vsx.org`）
- 内置扩展：`kiro.kiro-agent 0.1.1`、`ms-vscode.js-debug 1.105.0`、`markdown-mermaid-aws 1.0.0`
- 明确禁用：`amazonwebservices.amazon-q-vscode`（竞品屏蔽）
- 信任域名：`*.kiro.dev`、`*.aws.dev`、`*.awsapps.com`、`billing.stripe.com`（含 Stripe 计费）
- SSO 端点：`https://device.sso.*.amazonaws.com`
- 远程服务器下载模板：`https://prod.download.desktop.kiro.dev/releases/remotes/${commit}/kiro-reh-${os}-${arch}.tar.gz`

## extension.js — 完整命令表

按功能域分组：

**认证/账号**：`kiro.profiles.getProfile` / `listAvailableProfiles` / `selectProfile` / `showProfileSelector`、`kiro.signIn.portalUrlGenerated`、`kiro.accountDashboard.showDashboard`

**Agent**：`kiro.agent`、`kiro.agentModels.getAvailableModels` / `getModelsList` / `getSelectedModelId` / `setSelectedModelId`、`kiro.resume-session`、`kiro.session`、`kiro.updateModelsList`

**Spec**：`kiro.spec.createDocument` / `explorerCreateSpec` / `explorerDeleteSpec` / `explorerRenameSpec` / `navigateToBugfix` / `navigateToDesign` / `navigateToRequirements` / `navigateToTasks` / `nextDocument` / `previousDocument` / `runAllTasks` / `startOptionalTask` / `toggleOptionalTask` / `updateDocument`

**Steering**：`kiro.steering.createInitialSteering` / `createSteeringOrImportSkills` / `deleteSteering` / `refineSteeringFile` / `canRefine`

**Powers**：`kiro.powers.configure` / `enable` / `focus`、`kiro.views.installedPowersList` / `recommendedPowersList` / `power-details` / `powerDetails`

**MCP/远程工具**：`kiro.mcp`、`kiro.tools.refreshRemoteTools`、`kiro.remote_tool`、`kiro.remote-tools`

**订阅/计费**：`kiro.subscriptionPlans.getCheckoutSessionUrl` / `getPortalSessionUrl` / `getSubscriptionPlans` / `showSubscriptionPlans`、`kiro.billing`、`kiro.usageLimits.enableOverages` / `getUsageLimits`、`kiro.resourceNotifications.usageState`

**状态/UI**：`kiro.status.autocomplete` / `experiments` / `feedback` / `usageMeter`、`kiro.views.agentChat` / `specExplorer`、`kiro.contextUsage.getEstimate`

**其他**：`kiro.skills.deleteSkill`、`kiro.webContentExtractor.extract`、`kiro.config.getWorkspaceState` / `setWorkspaceState`、`kiro.getCanEnableTelemetry`、`kiro.reloadConfig`

## extension.js 分析（主扩展激活层）

> 路径：`kiro.kiro-agent/dist/extension.js`，47MB 单行压缩，包含所有子包的 bundle。

### 激活流程

`activate2(context)` 是入口，调用顺序：

`register61`（遥测初始化）→ `initializeAgentTelemetry` → `register60`（主服务注册）→ `initializePowersRegistry` → `enforceProfileSelection` → `activate`（continuedev 激活）→ `initializeAvailableModelsCache` → `remoteToolsDiscovery`

### register60 注册的服务

`NotificationService` / `SpecTelemetryService` / `AgentEventPollingService`（轮询 `kiro.usageLimits.getUsageLimits`）/ `Storage` / `ExecutionLogController` / Webview 依赖注入

### 认证层

`authProvider` 通过依赖注入传入 `QDeveloperConverse`，核心方法：
- `readToken()` 读取当前 token
- `isLoggedIn()` 判断登录状态
- `waitForSignIn()` 异步等待登录完成
- `logout()` 登出并跳转 SignIn 页

VSCode 认证会话（`AuthProviderSession`）通过 `createSession` / `removeSession` 实现标准 VSCode Auth API 接入。

### Profile 选择流程

登录后调用 `handleProfiles(token)`：
- `supportsProfiles(token)` 判断是否支持 profile（Enterprise IDC token 才支持）
- 单 profile 直接执行 `kiro.profiles.selectProfile`
- 多 profile 弹出 `kiro.profiles.showProfileSelector`
- `isEnterpriseProfileReady()` 检查 `ProfileStorage` 是否已有选中的 profile

### 遥测端点

三个环境：beta（`kiro.aws.dev`）/ gamma / prod（`kiro.dev`），通过 `NODE_ENV` 或配置切换。

### 依赖注入模式

`extension.js` 把所有子包（kiro-agent、kiro-shared、continuedev 等）的模块全部 bundle 进来，通过 `setXxxFactory` / `setXxxProvider` 系列函数在激活时完成工厂注入，各子包自身不持有单例，全部通过注入获取。