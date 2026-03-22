# spec-editor 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/spec-editor/**` 注释边界去重

## 结论

- `spec-editor/**` 当前共识别 `42` 个唯一模块块。
- 它是 extension 层体量最大的 feature 子系统之一。
- 不是单一编辑器增强，而是完整的 Spec 工作流平台：
  - 文档解析
  - Spec Explorer
  - Task 执行
  - Trace 数据库存储
  - Hover / CodeLens
  - Telemetry

## 模块清单

```text
spec-editor/agent-task-connection.ts
spec-editor/auto-open-spec-artifacts.ts
spec-editor/commands/chat-to-fix-pbt.ts
spec-editor/commands/execute-task.ts
spec-editor/commands/explorer-create-spec.ts
spec-editor/commands/explorer-delete-spec.ts
spec-editor/commands/explorer-rename-spec.ts
spec-editor/commands/generate-spec-document-command.ts
spec-editor/commands/index.ts
spec-editor/commands/navigate-documents.ts
spec-editor/commands/run-all-tasks.ts
spec-editor/documents/spec-document-manager.ts
spec-editor/errors.ts
spec-editor/index.ts
spec-editor/markdown-parser.ts
spec-editor/markdown-task-codelens-provider.ts
spec-editor/pbt-hover-provider.ts
spec-editor/pbt-hover-utils.ts
spec-editor/property-hover-provider.ts
spec-editor/reasoning/requirements-analyzer.ts
spec-editor/requirements-hover-provider.ts
spec-editor/requirements-parser.ts
spec-editor/spec-config-manager.ts
spec-editor/spec-explorer-view.ts
spec-editor/spec-file-system-provider.ts
spec-editor/spec-trace-database-manager.ts
spec-editor/spec-trace-database.ts
spec-editor/storage.ts
spec-editor/task-format-validator.ts
spec-editor/tasks/task-metadata-storage.ts
spec-editor/tasks/task-service.ts
spec-editor/telemetry/dimension-builder.ts
spec-editor/telemetry/event-filters.ts
spec-editor/telemetry/spec-session-state.ts
spec-editor/telemetry/spec-telemetry-service.ts
spec-editor/utils.ts
spec-editor/utils/abort-pending-executions.ts
spec-editor/utils/pbt-parser.ts
spec-editor/utils/properties-parser.ts
spec-editor/utils/spec-document-utils.ts
spec-editor/utils/spec-session-tracking.ts
spec-editor/utils/task-utils.ts
```

## 分层

### 1. 文档与文件系统

- `spec-editor/documents/spec-document-manager.ts`
  - Spec 文档读写协调器
- `spec-editor/spec-file-system-provider.ts`
  - 规格文件系统视图与读写适配
- `spec-editor/spec-config-manager.ts`
  - Spec 相关配置集中管理
- `spec-editor/storage.ts`
  - 持久化入口

定位：这一层负责“Spec 文件如何存在、如何打开、如何更新”。

### 2. 解析器

- `spec-editor/markdown-parser.ts`
- `spec-editor/requirements-parser.ts`
- `spec-editor/utils/properties-parser.ts`
- `spec-editor/utils/pbt-parser.ts`
- `spec-editor/utils/spec-document-utils.ts`
- `spec-editor/reasoning/requirements-analyzer.ts`

职责：

- 从 markdown 中切出 requirements / task / property / pbt 结构
- 为 hover、执行和 trace 建立统一语法模型

关键点：Spec editor 不是把 markdown 当普通文本，而是先做结构化解析，再提供后续能力。

### 3. 任务执行

- `spec-editor/tasks/task-service.ts`
  - 任务执行主服务
- `spec-editor/tasks/task-metadata-storage.ts`
  - task 元数据持久化
- `spec-editor/task-format-validator.ts`
  - task 定义格式校验
- `spec-editor/agent-task-connection.ts`
  - task 与 agent 会话的桥接
- `spec-editor/utils/task-utils.ts`
- `spec-editor/utils/abort-pending-executions.ts`

结论：Spec task 执行链已经被单独抽成服务层，不是命令里直接拼 agent 调用。

### 4. 命令层

- `spec-editor/commands/execute-task.ts`
- `spec-editor/commands/run-all-tasks.ts`
- `spec-editor/commands/navigate-documents.ts`
- `spec-editor/commands/generate-spec-document-command.ts`
- `spec-editor/commands/explorer-create-spec.ts`
- `spec-editor/commands/explorer-delete-spec.ts`
- `spec-editor/commands/explorer-rename-spec.ts`
- `spec-editor/commands/chat-to-fix-pbt.ts`
- `spec-editor/commands/index.ts`

这些命令把 Explorer、文档和 Task Service 串起来，是整个 Spec UI 的交互面。

### 5. 编辑器增强

- `spec-editor/markdown-task-codelens-provider.ts`
  - 683 行，大块核心模块
  - 为 markdown task 区段提供可执行 CodeLens
- `spec-editor/pbt-hover-provider.ts`
- `spec-editor/property-hover-provider.ts`
- `spec-editor/requirements-hover-provider.ts`
- `spec-editor/pbt-hover-utils.ts`
- `spec-editor/spec-explorer-view.ts`
- `spec-editor/auto-open-spec-artifacts.ts`

定位：这一层把 Spec 从“文件”升级为“可执行工件”，用户可以直接在编辑器里运行、跳转、查看解释。

### 6. Trace 与数据库

- `spec-editor/spec-trace-database.ts`
  - 555 行，明显是底层核心
- `spec-editor/spec-trace-database-manager.ts`

作用：

- 记录每次 Spec / Task 执行轨迹
- 为 rich execution log、回放和诊断提供数据源

这意味着 Spec 工作流不是一次性生成，而是可追踪、可回放的长期状态机。

### 7. Telemetry

- `spec-editor/telemetry/event-filters.ts`
- `spec-editor/telemetry/spec-session-state.ts`
- `spec-editor/telemetry/dimension-builder.ts`
- `spec-editor/telemetry/spec-telemetry-service.ts`

说明：

- Spec 子系统有独立 telemetry，不只是复用通用 telemetry
- 会维护 spec session 状态和 dimension 构造
- 事件上报有过滤层，避免无效噪声

## 高价值模块判断

### `markdown-task-codelens-provider.ts`

- 683 行
- 直接决定 task 在 markdown 中的“可执行感”
- 是 Spec editor 最明显的 UI 入口之一

### `tasks/task-service.ts`

- 658 行
- 承担真正的 task 编排与运行状态管理
- 是 Spec 工作流的执行核心

### `spec-trace-database.ts`

- 555 行
- 把 task 执行结果持久化为可追踪历史
- 说明 Kiro 的 Spec 不是临时 prompt，而是长期资产

### `spec-telemetry-service.ts`

- 513 行
- 表明 Spec 功能在产品层面被当成重点可观测对象

## 关系图

```text
Spec Markdown
    │
    ▼
Parser 层
    ├─ requirements-parser
    ├─ markdown-parser
    ├─ properties-parser
    └─ pbt-parser
    │
    ▼
Document / Explorer / Hover / CodeLens
    │
    ▼
Commands
    ├─ execute-task
    ├─ run-all-tasks
    ├─ generate-spec-document
    └─ chat-to-fix-pbt
    │
    ▼
TaskService + AgentTaskConnection
    │
    ▼
SpecTraceDatabase
    │
    └─ Telemetry / Session Tracking
```

## 结论

`spec-editor/**` 是 Kiro “Spec 驱动开发”在 VS Code 扩展侧的主实现，不是附属 UI。它把：

- 文档结构化
- 任务执行
- 数据库存档
- 编辑器内交互
- telemetry

全部收拢成一个高耦合 feature 集群。
