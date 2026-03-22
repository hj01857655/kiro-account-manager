# continuedev（Continue GUI 与内置配置）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\continuedev`
> 分析日期：2026-03-18

本包为 Kiro 侧集成的 Continue GUI 与默认配置。核心逻辑打包进 `dist/extension.js`，目录下保留 GUI 静态资源与内置主题。

---

## 目录结构

- `extension/builtin-themes/`：内置主题 JSON（dark/light/hc 系列）。
- `gui/dist/`：Continue GUI Webview 静态资源。
- `gui/dist/assets/index.js`：前端 bundle。
- `gui/dist/assets/index.css`：样式。
- `gui/dist/logos/`：模型供应商 logo（OpenAI/Anthropic/Gemini/DeepSeek 等）。

---

## extension.js 内置模块摘要

- `defaultConfig.contextProviders`：默认 Provider 列表。
- `ContinueGUIWebviewViewProvider`：WebviewViewProvider，负责 GUI 面板与消息协议。
- `recentlyEditedFilesCache`：QuickLRU，最大 100 个最近编辑文件。

`defaultConfig.contextProviders` 实际值：

- `code`
- `docs`
- `repo-map`
- `diff`
- `terminal`
- `problems`
- `folder`
- `codebase`
- `url`
- `currentFile`
- `steering`
- `spec`

---

## ContinueGUIWebviewViewProvider 关键行为

- `viewType = "kiroAgent.continueGUIView"`。
- `webviewProtocol` 负责 request/response 通道。
- 监听配置：
- `kiroAgent.enableDebugLogs` → 控制输出通道显示。
- `kiroAgent.agentAutonomy` / `kiroAgent.usageSummary` / `kiroAgent.experiments` → 推送 `kiroSettingsUpdate`。
- `kiroAgent.modelSelection` → `updateModelSelection`。
- `workbench.colorTheme` → `setTheme`。
- Webview 消息处理：
- `chatFocus` → `setContext('kiroAgent.chatFocus', ...)`。
- `log` → 输出到 OutputChannel（debug 受 enableDebugLogs 控制）。

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                         continuedev (bundled)                                │
│                                                                              │
│  defaultConfig.contextProviders                                              │
│  ContinueGUIWebviewViewProvider (viewType: kiroAgent.continueGUIView)         │
│  recentlyEditedFilesCache (QuickLRU, 100)                                    │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────────┐      │
│             │          │ Continue GUI Webview (gui/dist)              │      │
│             │          │ index.html + assets/index.js + index.css     │      │
│             │          └──────────────────────────────────────────────┘      │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  webviewProtocol request/response + log + chatFocus             │
│             │                                                                │
│             │          ┌──────────────────────────────────────────────┐      │
│             │          │ builtin-themes/*.json                        │      │
│             │          └──────────────────────────────────────────────┘      │
└──────────────────────────────────────────────────────────────────────────────┘
```
