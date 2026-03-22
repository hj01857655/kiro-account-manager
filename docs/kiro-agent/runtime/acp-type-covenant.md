# acp-type-covenant（ACP 类型契约）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\acp-type-covenant\dist-es` / `dist-cjs`
> 分析日期：2026-03-18

本包提供 ACP 协议的类型契约与会话持久化 Zod Schema，供 `kiro-client` 与 `kiro-agent` 双端共享。

---

## 目录结构

- `dist-es/`：ESM 构建产物。
- `dist-cjs/`：CJS 构建产物。
- `capabilities/`：ACP 能力类型定义（fs/search/terminal 等）。
- `agent-capabilities/`：client → agent 扩展方法的能力契约入口。
- `session/schemas/`：会话持久化 Zod Schema（运行时校验 + 类型推导）。
- `config/`：模型选项元数据类型（类型导出）。

---

## 能力分类（capabilities）

- `fs`：`read-file` / `read-text-file` / `write-file` / `write-text-file` / `read-directory` / `stat` / `delete`
- `terminal`：`create` / `kill` / `release` / `output` / `wait-for-exit`
- `workspace`：`active-file` / `currently-open-files`
- `search`：`text-search` / `find-files`
- `steering`：`get-documents` / `documents-changed` / `progressive-context-changed`
- `tasks`：`list-tasks` / `get-task-metadata`
- `powers`：`powers-changed`

---

## Session 持久化 Schema（Zod）

`session/schemas/index.js` 提供运行时校验，核心结构包括：

- `AgentModeSchema`：`agent` / `spec`
- `ToolCallStatusSchema`：`pending` / `awaiting_approval` / `approved` / `denied` / `executing` / `completed` / `failed`
- `ToolKindSchema`：`read` / `edit` / `execute` / `search` / `delete` / `move` / `fetch` / `think` / `switch_mode` / `other`
- `MessagePayloadSchema`：`user` / `assistant` / `tool_call` / `tool_result` / `system` / `error` / `mode_change` / `session_event` / `sub_agent_*` / `steering_inclusion`
- `SessionMetadataSchema`：`id` / `title` / `agentMode` / `workspacePaths` / `createdAt` / `lastModifiedAt` / `parentSessionId` 等
- `PersistedSessionSchema`：`metadata` + `messages[]`
- `ListSessionsResponseSchema` / `ListSessionsOptionsSchema`

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                         acp-type-covenant                                     │
│                                                                              │
│  capabilities/*  fs / terminal / search / workspace / steering / tasks / powers │
│  agent-capabilities/*  client → agent 扩展能力契约                             │
│  session/schemas/*  Zod 运行时校验 + 类型推导                                  │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │              ┌──────────────────────────────────────────┐      │
│             │              │ kiro-client (ACP 调用侧)                 │      │
│             │              │  sendExtMethod / session / tool calls    │      │
│             │              └──────────────────────────────────────────┘      │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │              共享类型契约 / Session schema                      │
│             │                                                                │
│             │              ┌──────────────────────────────────────────┐      │
│             │              │ kiro-agent (ACP 处理侧)                  │      │
│             │              │  capabilityHandlers / session persistence │      │
│             │              └──────────────────────────────────────────┘      │
└──────────────────────────────────────────────────────────────────────────────┘
```
