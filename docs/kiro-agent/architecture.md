┌──────────────────────────────────────────────────────────────────────────┐
│                     Kiro IDE  (VSCode fork + Electron)                   │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                  kiro.kiro-agent  extension.js                     │  │
│  │                                                                    │  │
│  │  activate()                                                        │  │
│  │   ├─ setupCa()                    平台证书注入                     │  │
│  │   ├─ dynamicImportAndActivate()   延迟加载 Continue 核心           │  │
│  │   └─ activate2()                 Kiro 主入口                       │  │
│  │       ├─ initializeAgentTelemetry()                                │  │
│  │       ├─ registerExtension()                                       │  │
│  │       │   ├─ 80× kiroAgent.* commands                             │  │
│  │       │   ├─ StatusBar  (Right, priority -999)                    │  │
│  │       │   ├─ onDidChangeConfiguration                              │  │
│  │       │   └─ TreeDataProvider ──► HookController                  │  │
│  │       ├─ Powers 初始化                                             │  │
│  │       ├─ 账号/配置校验                                             │  │
│  │       └─ 模型缓存 + 远端工具发现                                   │  │
│  │                                                                    │  │
│  │  ┌─────────────────────── 核心服务层 ───────────────────────────┐  │  │
│  │  │  HookController    SteeringController    SpecController       │  │  │
│  │  │  McpController     SkillController       PowerController      │  │  │
│  │  │  AutoSwitchController                                         │  │  │
│  │  └──────────────────────────┬────────────────────────────────────┘  │  │
│  │                             │ dispatch                               │  │
│  │  ┌──────────────────────────▼────────────────────────────────────┐  │  │
│  │  │                    Agent 执行层                                │  │  │
│  │  │                                                                │  │  │
│  │  │  ExecutionQueue                                                │  │  │
│  │  │   Queued ──► Began ──► Resumed ◄──► Yielded                  │  │  │
│  │  │                                  │                            │  │  │
│  │  │                        SaveState / SummarizeUsage             │  │  │
│  │  │                                  │                            │  │  │
│  │  │                   Success / Failed / Aborted                  │  │  │
│  │  │                                  │                            │  │  │
│  │  │                       ContextUsageUpdate                      │  │  │
│  │  │                                                                │  │  │
│  │  │  Subagents: context-gatherer  general-task  custom-creator    │  │  │
│  │  │  Tools:     readFile  writeFile  searchFiles  runTerminal     │  │  │
│  │  └──────────────────────────┬────────────────────────────────────┘  │  │
│  │                             │ call                                   │  │
│  │  ┌──────────────────────────▼────────────────────────────────────┐  │  │
│  │  │                   通信与 UI 层                                 │  │  │
│  │  │                                                                │  │  │
│  │  │  EditorApi   WebviewPanel ◄──────────► Extension              │  │  │
│  │  │  ChatSession (会话队列 + 历史)                                 │  │  │
│  │  │  InlineCompletion  (provideInlineCompletionItems)             │  │  │
│  │  │  KiroClient  ──────────────────────────────────────────────►  │  │  │
│  │  └────────────────────────────────────────────────────────────── ┘  │  │
│  └────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────┬───────────────────────────────────┘
                                       │ ACP (WebSocket / HTTP)
                          ┌────────────▼────────────┐
                          │      Kiro 后端服务        │
                          │  q.{region}.amazonaws.com│
                          └─────────────────────────┘

packages/ 对应关系
  autocomplete/          ──► InlineCompletion
  kiro-context-providers/ ──► ContextEngine (7 providers)
  kiro-client/           ──► KiroClient / ACP 协议层
  kiro-shared/           ──► 认证 / 遥测 / 工具基础类
  hook-editor/           ──► HookController TreeView UI
  kiro-ui-agent-chat/    ──► ChatSession WebviewPanel
  kiro-ui-powers/        ──► PowerController WebviewPanel
  requirements-webview/  ──► SpecController WebviewPanel
  acp-type-covenant/     ──► ACP 协议类型定义
