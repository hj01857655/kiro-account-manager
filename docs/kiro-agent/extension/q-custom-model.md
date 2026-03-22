# q-custom-model（Q Developer 会话适配）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 关注模块：`src/extension/q-custom-model/`
> 分析日期：2026-03-18

该模块把 Kiro 扩展侧的对话请求封装成 Q Developer Streaming API 调用，并在本地注入认证、隐私与 agent mode 等头部，同时接入日志与指标。

---

## 核心入口

- `q-developer-converse-factory.ts`：构造 `createQDeveloperConverse(modelType, agentMode, fields)`
- `qChatLogger`：`OutputChannel`（标题 `Q Chat API`），带缓冲区用于调试输出

`createQDeveloperConverse` 返回的会话执行器负责：
- 获取 CodeWhisperer Streaming Client（Q Developer 底层）
- 注入 `authProvider` 认证头
- 注入 `addPrivacyHeadersMiddleware` / `addAgentModeHeadersMiddleware`
- 连接 `metrics` / `qLogger` / `qChatLogger`
- 读取 `isUsageEnabled`（usageSummary 配置缓存）
- 调用 `recordReferences` 记录文件引用

---

## 关键链路

- `createQDeveloperConverse` 初始化 streaming client
- client 中间件层叠加隐私与 agent-mode headers
- 发送 streaming 请求并返回 chunk 流
- chunk 侧同时写入 `qLogger`/`qChatLogger` 以及指标
- 通过 `recordReferences` 把引用的文件上下文回写给扩展层

---

## 架构图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ q-custom-model (createQDeveloperConverse)                                    │
│                                                                              │
│  Input: modelType / agentMode / fields                                       │
│         │                                                                    │
│         ▼                                                                    │
│  getCodeWhispererStreamingClient                                             │
│   + authProvider                                                            │
│   + addPrivacyHeadersMiddleware                                              │
│   + addAgentModeHeadersMiddleware                                            │
│         │                                                                    │
│         ▼                                                                    │
│  Streaming invoke (Q Developer)  ─────────────────────────────────────────►  │
│         │                                                                    │
│         ▼                                                                    │
│  qLogger / qChatLogger (OutputChannel)                                       │
│  metrics / usageEnabled / recordReferences                                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 与其他模块的关系

- 认证：依赖 `authProvider` 提供访问 token
- 计量：与 `UsageSummary` 机制一致，读取 usage 开关配置
- 引用：与 `recordReferences` 同通道，便于前端展示引用文件
