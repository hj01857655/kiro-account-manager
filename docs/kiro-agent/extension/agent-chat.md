# agent-chat 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/agent-chat/**` 注释边界去重

## 结论

- `agent-chat/**` 当前共识别 `16` 个唯一模块块。
- 它是 Kiro Chat UI 在 extension 侧的主实现，负责 webview、session、permission request 和 chat API 桥接。

## 模块清单

```text
agent-chat/agent-chat-view-provider.ts
agent-chat/api/get-context-items.ts
agent-chat/api/get-context-providers.ts
agent-chat/api/index.ts
agent-chat/api/list-sessions.ts
agent-chat/api/load-session.ts
agent-chat/api/new-session.ts
agent-chat/api/open-text-document.ts
agent-chat/api/prompt.ts
agent-chat/api/resolve-permission-request.ts
agent-chat/api/set-session-config-option.ts
agent-chat/config-sync.ts
agent-chat/constants.ts
agent-chat/create-file-system-watchers.ts
agent-chat/index.ts
agent-chat/session-manager.ts
```

## 分层

- `agent-chat/index.ts`
  - 342 行，主入口
  - 集成 KiroClient / ACP 会话与 chat webview
- `agent-chat/agent-chat-view-provider.ts`
  - 注入 `sessionId`
  - 中转 session update / permission request / config update
- `agent-chat/session-manager.ts`
  - 维护 active session registration
- `agent-chat/api/*`
  - 构成 webview 到 extension 的 API 面
- `agent-chat/api/resolve-permission-request.ts`
  - 维护按 `sessionId + toolCallId` 映射的 Promise

## 判断

`agent-chat/**` 是 chat webview 与 agent core 之间的接缝层，不是简单页面壳。
