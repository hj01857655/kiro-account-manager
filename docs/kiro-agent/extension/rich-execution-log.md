# rich-execution-log 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/rich-execution-log/**` 注释边界去重

## 结论

- `rich-execution-log/**` 当前共识别 `36` 个唯一模块块。
- 这一层不是简单“日志展示”，而是 execution 生命周期、文件 diff、会话持久化、用户补充输入、sub-agent 延续、webview 同步的总控制面。
- 它把 runtime 侧的 `ExecutionLog` 数据结构变成 VS Code 可交互的控制器、SCM diff 视图、快照文件和 UI 指令。

## 模块清单

```text
rich-execution-log/controller/type-mapper.ts
rich-execution-log/types-actions.ts
rich-execution-log/errors.ts
rich-execution-log/index.ts
rich-execution-log/agent-activity-publisher/index.ts
rich-execution-log/agent-activity-publisher/agent-activity-publisher.ts
rich-execution-log/commands/accept-diff.ts
rich-execution-log/commands/accept-user-response.ts
rich-execution-log/commands/get-pending-questions.ts
rich-execution-log/commands/add-to-execution.ts
rich-execution-log/commands/cancel-active-execution.ts
rich-execution-log/commands/queue-user-message.ts
rich-execution-log/commands/consume-queue-now.ts
rich-execution-log/commands/clear-queued-message.ts
rich-execution-log/commands/diff-commands.ts
rich-execution-log/pending-changes-adapter.ts
rich-execution-log/session-file-snapshots.ts
rich-execution-log/session-persistence-integration.ts
rich-execution-log/commands/supervised-diff-sync.ts
rich-execution-log/utils/uri-matching.ts
rich-execution-log/commands/extract-last-execution-paths.ts
rich-execution-log/commands/get-active-execution.ts
rich-execution-log/commands/get-execution-by-id.ts
rich-execution-log/commands/get-execution-history.ts
rich-execution-log/commands/get-queued-executions.ts
rich-execution-log/commands/ui-control.ts
rich-execution-log/commands/subagent-continuation.ts
rich-execution-log/controller/agent-log-connection.ts
rich-execution-log/controller/execution-log-connection.ts
rich-execution-log/controller/execution-log-connection-to-webview.ts
rich-execution-log/diff/agent-activity-subscription.ts
rich-execution-log/diff/diff-controller.ts
rich-execution-log/diff/empty-text-document-provider.ts
rich-execution-log/session-snapshot-file-system-provider.ts
rich-execution-log/controller/execution-log-controller.ts
rich-execution-log/controller/execution-data-cache.ts
```

## 分组

### 1. Root / 类型与入口

- `rich-execution-log/controller/type-mapper.ts`
- `rich-execution-log/types-actions.ts`
- `rich-execution-log/errors.ts`
- `rich-execution-log/index.ts`

这组负责把底层 execution action 转成前端/UI 可消费的动作模型，并通过 `rich-execution-log/index.ts` 统一注册。

### 2. agent-activity-publisher

- `rich-execution-log/agent-activity-publisher/index.ts`
- `rich-execution-log/agent-activity-publisher/agent-activity-publisher.ts`

这一组负责把执行中的文件活动投影成编辑器装饰：

- `rich-execution-log/agent-activity-publisher/agent-activity-publisher.ts` 注册 file decoration provider。
- 它按 `executionId -> file -> action` 聚合写入动作，让活跃执行中的文件在 Explorer/编辑器里可见。

### 3. commands

- `rich-execution-log/commands/accept-diff.ts`
- `rich-execution-log/commands/accept-user-response.ts`
- `rich-execution-log/commands/get-pending-questions.ts`
- `rich-execution-log/commands/add-to-execution.ts`
- `rich-execution-log/commands/cancel-active-execution.ts`
- `rich-execution-log/commands/queue-user-message.ts`
- `rich-execution-log/commands/consume-queue-now.ts`
- `rich-execution-log/commands/clear-queued-message.ts`
- `rich-execution-log/commands/diff-commands.ts`
- `rich-execution-log/commands/supervised-diff-sync.ts`
- `rich-execution-log/commands/extract-last-execution-paths.ts`
- `rich-execution-log/commands/get-active-execution.ts`
- `rich-execution-log/commands/get-execution-by-id.ts`
- `rich-execution-log/commands/get-execution-history.ts`
- `rich-execution-log/commands/get-queued-executions.ts`
- `rich-execution-log/commands/ui-control.ts`
- `rich-execution-log/commands/subagent-continuation.ts`

这是 rich execution log 的命令表面：

- `rich-execution-log/commands/accept-diff.ts` 允许在 diff hunk 工具栏里直接接受变更并回写文件。
- `rich-execution-log/commands/accept-user-response.ts` 把用户对提问/审批的响应送回当前执行。
- `rich-execution-log/commands/get-pending-questions.ts`、`rich-execution-log/commands/get-active-execution.ts`、`rich-execution-log/commands/get-execution-by-id.ts`、`rich-execution-log/commands/get-execution-history.ts`、`rich-execution-log/commands/get-queued-executions.ts` 提供查询接口。
- `rich-execution-log/commands/add-to-execution.ts`、`rich-execution-log/commands/queue-user-message.ts`、`rich-execution-log/commands/consume-queue-now.ts`、`rich-execution-log/commands/clear-queued-message.ts` 处理执行中的追加消息与队列消费。
- `rich-execution-log/commands/cancel-active-execution.ts` 负责中断活跃执行。
- `rich-execution-log/commands/diff-commands.ts`、`rich-execution-log/commands/supervised-diff-sync.ts`、`rich-execution-log/commands/extract-last-execution-paths.ts` 负责 diff/文件变更相关命令。
- `rich-execution-log/commands/ui-control.ts` 控制日志视图/UI 状态。
- `rich-execution-log/commands/subagent-continuation.ts` 处理 sub-agent 执行延续。

### 4. 控制器与连接层

- `rich-execution-log/controller/agent-log-connection.ts`
- `rich-execution-log/controller/execution-log-connection.ts`
- `rich-execution-log/controller/execution-log-connection-to-webview.ts`
- `rich-execution-log/controller/execution-log-controller.ts`
- `rich-execution-log/controller/execution-data-cache.ts`

这是整个目录的控制中枢：

- `rich-execution-log/controller/execution-log-controller.ts` 是主控制器，负责拉取 execution、管理订阅、协调 diff/UI。
- `rich-execution-log/controller/execution-data-cache.ts` 提供 write-back cache，把 execution 数据做缓存、延迟 flush、LRU 管理和命中统计。
- `rich-execution-log/controller/execution-log-connection.ts` 和 `rich-execution-log/controller/execution-log-connection-to-webview.ts` 负责把 execution 数据桥接到 Webview。
- `rich-execution-log/controller/agent-log-connection.ts` 更偏向 agent 事件流接入。

### 5. diff

- `rich-execution-log/diff/agent-activity-subscription.ts`
- `rich-execution-log/diff/diff-controller.ts`
- `rich-execution-log/diff/empty-text-document-provider.ts`

这一组把 execution action 映射成 SCM / multi-diff 视图：

- `rich-execution-log/diff/diff-controller.ts` 创建 `SourceControl` 和 resource group，按 execution 构造多文件 diff 视图。
- `rich-execution-log/diff/agent-activity-subscription.ts` 把 agent 活动订阅到 diff 侧刷新逻辑。
- `rich-execution-log/diff/empty-text-document-provider.ts` 处理空文件/虚拟文档场景。

### 6. 快照与持久化

- `rich-execution-log/pending-changes-adapter.ts`
- `rich-execution-log/session-file-snapshots.ts`
- `rich-execution-log/session-persistence-integration.ts`
- `rich-execution-log/session-snapshot-file-system-provider.ts`
- `rich-execution-log/utils/uri-matching.ts`

这一组负责 execution 的文件状态留痕：

- `rich-execution-log/pending-changes-adapter.ts` 应用或回滚 pending diff，把执行中的改动投影到真实文件系统。
- `rich-execution-log/session-file-snapshots.ts` 保存单次会话下的文件快照。
- `rich-execution-log/session-snapshot-file-system-provider.ts` 暴露快照文件系统访问能力。
- `rich-execution-log/session-persistence-integration.ts` 在用户消息和 execution action 到来时追加持久化消息，而不是重写整个日志。
- `rich-execution-log/utils/uri-matching.ts` 负责路径/URI 匹配，给 diff 和快照恢复做辅助。

## 关键链路

### 执行日志控制链

- `rich-execution-log/index.ts`
- `rich-execution-log/controller/execution-log-controller.ts`
- `rich-execution-log/controller/execution-log-connection.ts`
- `rich-execution-log/controller/execution-log-connection-to-webview.ts`
- `rich-execution-log/controller/execution-data-cache.ts`

这条链路完成 execution 数据获取、缓存、连接和 UI 投递，是整个 feature 的主干。

### Diff 接受与文件回写

- `rich-execution-log/commands/accept-diff.ts`
- `rich-execution-log/diff/diff-controller.ts`
- `rich-execution-log/pending-changes-adapter.ts`
- `rich-execution-log/session-file-snapshots.ts`
- `rich-execution-log/session-snapshot-file-system-provider.ts`

这条链路把“执行动作”变成“用户可接受的文件变更”，并支持后续快照/回滚。

### 用户交互与排队

- `rich-execution-log/commands/accept-user-response.ts`
- `rich-execution-log/commands/get-pending-questions.ts`
- `rich-execution-log/commands/add-to-execution.ts`
- `rich-execution-log/commands/queue-user-message.ts`
- `rich-execution-log/commands/consume-queue-now.ts`
- `rich-execution-log/commands/clear-queued-message.ts`
- `rich-execution-log/commands/subagent-continuation.ts`

这条链路说明 rich execution log 不只看历史记录，还参与执行中的人机交互调度。

## 关键判断

- `rich-execution-log/**` 实际上承担了 execution orchestration 的一部分，不只是日志可视化。
- 其核心价值在于把 agent 运行态转成可查询、可恢复、可接受 diff、可追踪文件活动的 IDE 体验。
- 如果缺失这一层，Kiro 仍可执行 agent，但用户会失去执行历史、问题队列、diff 接受、快照恢复、webview 联动这些关键操作面。
