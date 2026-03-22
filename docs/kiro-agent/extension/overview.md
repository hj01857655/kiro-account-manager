# kiro.kiro-agent 扩展目录概览

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent`
> 分析日期：2026-03-17

本文件提供整体结构与索引，细节见对应专题文档。

---

## 目录结构

- `dist/`：VS Code 扩展入口与运行时脚本（主 bundle + tokenizer/worker）。
- `extension-resources/`：扩展内置资源（Hook JSON Schema）。
- `bundled-webviews/`：requirements webview 静态资源。
- `models/`：本地嵌入模型（sentence transformers/ONNX）。
- `tree-sitter/`：Tree-sitter 查询脚本（代码片段/导入/上下文路径）。
- `treesitter-wasm/`：Tree-sitter 多语言解析器 WASM。
- `packages/`：多包工作区（UI、autocomplete、context provider 等）。
- `package.json`：扩展声明（activation events、view、command、配置等）。

---

## 入口与核心资源

- `dist/extension.js`：扩展主入口（约 49MB）。
- `dist/llamaTokenizer.mjs`：Llama tokenizer（本地估算/处理）。
- `dist/llamaTokenizerWorkerPool.mjs`、`dist/tiktokenWorkerPool.mjs`：Tokenizer worker 池。
- `dist/xhr-sync-worker.js`：同步 XHR worker。
- `extension-resources/hook.json`：Hook JSON Schema（`fileCreated/fileDeleted/fileEdited/userTriggered` → `alert/askAgent`）。
- `models/all-MiniLM-L6-v2/`：transformers.js 使用的 ONNX 权重与 tokenizer 配置。

---

## tree-sitter 资源摘要

- `tree-sitter/`：code-snippet、import、root-path-context 查询脚本。
- `treesitter-wasm/`：bash/c/cpp/python/ts/tsx 等多语言解析器。

---

## packages 概览

- `acp-type-covenant`：ACP 共享类型包（dist-es/dist-cjs）。
- `autocomplete`：Inline Completion，详见 `../assets/autocomplete.md`。
- `kiro-context-providers`：Context providers，详见 `../assets/context-providers.md`。
- `kiro-ui-agent-chat` / `kiro-ui-powers` / `hook-editor` / `bundled-webviews`：Webview 资源，详见 `../assets/webviews.md`。
- `continuedev`：Continue GUI 与内置主题。
- `kiricons`：自定义图标字体。
- `kiro-agent-tests`：集成测试包。
- `kiro-agent`：详见 `../runtime/packages-kiro-agent-dist.md`（packages/kiro-agent/dist）。

---

## package.json 摘要

- `main`：`./dist/extension.js`
- `activationEvents`：`onUri`、`onFileSystem:kiro-*`、`onStartupFinished`、`onView:*`、`onLanguage:*`、`onCommand:*`。
- `viewsContainers`：`continue`（Chat）、`kiro`（Specs/Steering/Hooks/MCP）、`powers`（Powers UI）。
- `views`：`kiro.views.specExplorer`、`kiroAgent.views.hooksStatus`、`kiroAgent.views.steeringExplorer`、`kiroAgent.views.mcpServerStatus` 等。
- `jsonValidation`：`.hooks/*.json` → `extension-resources/hook.json`

---

## 文档索引

- `activation.md`
- `agent-chat.md`
- `auth.md`
- `background-processes.md`
- `checkpoints.md`
- `commands.md`
- `config.md`
- `context-lsp.md`
- `context-resolvers.md`
- `custom-agent-loader.md`
- `enterprise.md`
- `experiments.md`
- `final-status.md`
- `../architecture.md`
- `hooks.md`
- `mcp.md`
- `platform.md`
- `powers.md`
- `profiles.md`
- `../assets/context-providers.md`
- `../assets/autocomplete.md`
- `../assets/webviews.md`
- `../assets/bundled-webviews.md`
- `../runtime/packages-kiro-agent-dist.md`
- `../runtime/packages-kiro-shared.md`
- `../runtime/packages-kiro-client.md`
- `q-custom-model.md`
- `rich-execution-log.md`
- `notifications.md`
- `model-selection.md`
- `contextual-spec.md`
- `spec-editor.md`
- `steering.md`
- `storage.md`
- `telemetry.md`
- `terminal.md`
- `usage.md`
- `utils.md`
- `session-resume.md`
- `../runtime/acp-type-covenant.md`
- `../assets/continuedev.md`
- `../assets/kiricons.md`
- `../assets/tests.md`
- `../assets/extension-resources.md`
- `../assets/models.md`
- `../assets/tree-sitter.md`
- `../assets/node-modules.md`

---

## ASCII 架构图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                         kiro.kiro-agent Extension                            │
│                                                                              │
│  dist/extension.js                                                           │
│   activate() / activate2()                                                   │
│   setupCa() / initializeAgentTelemetry()                                     │
│   registerExtension() → commands / views / status bar / tree providers       │
│   powers 初始化 / profile 校验 / config 监听                                  │
│   dynamicImportAndActivate() → Continue 核心                                  │
│              │                                                              │
│              ▼                                                              │
│  packages/kiro-agent/dist  (Agent Core)                                      │
│   ExecutionQueue / Spec & Vibe flows / Controllers                           │
│   HookController / SteeringController / SpecController / McpController       │
│              │                                                              │
│              ▼                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ 依赖模块                                                               │  │
│  │  kiro-client  ── ACP 会话桥接 / prompt / onSessionUpdate                │  │
│  │  kiro-context-providers ─ file/spec/mcp/term 等 7 类 Context            │  │
│  │  kiro-shared ─ auth/token/profile + telemetry + MCP utilities           │  │
│  │  autocomplete ─ Inline Completion / CodeWhispererRuntime                │  │
│  │  webviews ─ hook-editor / requirements / chat / powers                  │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  资源侧路径                                                                   │
│   tokenizer workers / models(all-MiniLM-L6-v2) / tree-sitter queries         │
│                                                                              │
│  ACP 通道                                                                     │
│   kiro-client ─────────────────────────────────────────────────────────────► │
│                                                                              │
│  ◄────────────────────────────────────────────────────────────────────────── │
│    sessionUpdate / permissionRequest / extNotification                       │
└──────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                    Kiro 后端 (ACP / CodeWhisperer)                            │
│                    q.{region}.amazonaws.com                                   │
└──────────────────────────────────────────────────────────────────────────────┘
```
