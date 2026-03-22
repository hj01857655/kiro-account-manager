# kiro-client 模块概览

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\kiro-client\dist`
> 分析日期：2026-03-17

本文件聚焦 `KiroClient`（ACP 通信桥梁）的初始化、会话与扩展方法。

---

## 角色定位

`KiroClient` 负责 VS Code 扩展侧与 agent 进程之间的 ACP 通信，封装初始化、会话管理、权限请求与扩展通知。

---

## 初始化

- `initialize()` 幂等：若已初始化或正在初始化，复用结果。
- `ClientSideConnection` 建立后，关闭时自动清理 `connection` / `initializeResponse` / `initializePromise`。

---

## 会话与请求

- `newSession()` / `loadSession()`：创建或加载 session。
- `prompt()`：发送请求并等待 agent 响应。
- `cancel(sessionId)`：取消进行中任务。
- `setSessionConfigOption()`：通知 agent 配置变更（例如 MCP servers 更新）。

---

## 权限与通知

- `onSessionUpdate(sessionId, handler)`：处理 session 更新。
- `onPermissionRequest(sessionId, handler)`：处理权限请求，默认返回 cancelled。
- `onExtNotification(method, handler)`：监听 agent → client 扩展通知（支持多 handler）。

---

## 文件与终端能力

- `readTextFile()` / `writeTextFile()`：代理到注册的 fs handler。
- `createTerminal()` / `killTerminal()` / `releaseTerminal()` / `terminalOutput()` / `waitForTerminalExit()`：代理到 terminal handler。
- 若未注册 handler 会抛错（如 `fs/read_text_file`）。

---

## 架构流程图

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                               KiroClient (ACP)                                 │
│                                                                              │
│  VS Code 扩展侧                                                              │
│    initialize() → ClientSideConnection                                       │
│    registerFsHandler / registerTerminalHandler                               │
│             │                                                               │
│             ▼                                                               │
│  ACP 会话                                                                    │
│    newSession / loadSession / prompt / cancel                                │
│             │                                                               │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │                     ┌──────────────────────────────────────┐   │
│             │                     │ Agent 进程 (ACP Server)              │   │
│             │                     │  接收 prompt                         │   │
│             │                     │  发送流式更新                         │   │
│             │                     │  触发权限请求                         │   │
│             │                     └──────────────────────────────────────┘   │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │   onSessionUpdate / onPermissionRequest / onExtNotification   │
│             ▼                                                               │
│  文件与终端能力                                                              │
│    readTextFile / writeTextFile                                              │
│    createTerminal / killTerminal / terminalOutput                            │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 扩展方法调用

- `extMethod(method, params)`：调用已注册的扩展方法。
- `sendExtMethod(method, params)`：向 agent 发起扩展方法调用。
- `sendExtNotification(method, params)`：向 agent 发送通知（fire-and-forget）。
