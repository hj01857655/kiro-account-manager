# hooks 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/hooks/**` 注释边界去重

## 结论

- `hooks/**` 当前共识别 `39` 个唯一模块块。
- 这一层不是单个“hook runner”文件，而是完整的 Hook 平台：
  - 配置存储
  - TreeView 展示
  - 命令增删改查
  - 文件事件监听
  - 触发器执行
  - contextual pre/post tool hooks
  - 命令审批与运行中断

## 模块清单

```text
hooks/contextual/hook-command-approval-storage.ts
hooks/contextual/hook-operation-registry.ts
hooks/contextual/contextual-hook-actions.ts
hooks/contextual/contextual-hook-triggers.ts
hooks/contextual/hooks-provider.ts
hooks/contextual/post-tool-use-hooks.ts
hooks/contextual/pre-tool-use-hooks.ts
hooks/contextual/task-execution-hooks.ts
hooks/contextual/tool-type-mapping.ts
hooks/commands/create-hook.ts
hooks/commands/delete-hook-context-menu.ts
hooks/commands/delete-hook.ts
hooks/commands/enable-hook.ts
hooks/commands/list-hooks.ts
hooks/commands/read-hook.ts
hooks/commands/sync-hook-running-state.ts
hooks/commands/trigger-hook.ts
hooks/errors.ts
hooks/hook-controller.ts
hooks/hook-singleton.ts
hooks/hook-storage.ts
hooks/hook-validation.ts
hooks/index.ts
hooks/listeners/hook-listener.ts
hooks/listeners/index.ts
hooks/listeners/when-file-created.ts
hooks/listeners/when-file-deleted.ts
hooks/listeners/when-file-edit.ts
hooks/triggers/hook-trigger.ts
hooks/triggers/index.ts
hooks/triggers/then-ask-agent.prompt.ts
hooks/triggers/then-ask-agent.ts
hooks/triggers/then-run-command.ts
hooks/types.ts
hooks/utils/request-command-approval.ts
hooks/utils/spawn-hook-command.ts
hooks/views/get-hook-tree-item.ts
hooks/views/hook-tree-item.ts
hooks/views/hooks-treeview.ts
```

## 分层

### 1. 配置与持久化

- `hooks/hook-storage.ts`
  - 枚举所有 workspace 的 `.kiro/hooks/` 目录
  - 负责读写、删除、列出 hook 文件
  - 包含旧 hook id 到 URI id 的迁移逻辑
- `hooks/types.ts`
  - `serializeHook()` / `deserializeHook()`
  - 把 `fileUri` 在持久化时转字符串
- `hooks/hook-validation.ts`
  - 用 Zod 校验 Hook JSON 的最小结构
  - `when.type` 绑定 `IDEListenableEvent`
  - `then.type` 绑定 `HookActionEvent`

### 2. 控制器与单例

- `hooks/hook-controller.ts`
  - 核心协调器
  - 负责 list/read/create/delete/enable/trigger
  - 驱动 TreeView 刷新与运行状态同步
- `hooks/hook-singleton.ts`
  - 提供全局单例接入点
- `hooks/index.ts`
  - 注册入口，聚合命令、监听器和视图

### 3. TreeView 与 UI

- `hooks/views/hooks-treeview.ts`
  - TreeDataProvider 外壳
- `hooks/views/get-hook-tree-item.ts`
  - 按 hook 状态生成树节点
  - 对 running / disabled / userTriggered 给不同图标与按钮
- `hooks/views/hook-tree-item.ts`
  - TreeItem 扩展，保存 `hookId`、按钮命令与图标

结论：Hooks 侧边栏不是纯静态列表，而是带运行态、启停态和手动触发入口的状态视图。

### 4. 命令层

- `hooks/commands/create-hook.ts`
- `hooks/commands/delete-hook.ts`
- `hooks/commands/delete-hook-context-menu.ts`
- `hooks/commands/enable-hook.ts`
- `hooks/commands/read-hook.ts`
- `hooks/commands/trigger-hook.ts`
- `hooks/commands/sync-hook-running-state.ts`
- `hooks/commands/list-hooks.ts`

这些模块都很薄，主要把 VS Code 命令参数适配到 `HookController`。

### 5. 事件监听层

- `hooks/listeners/hook-listener.ts`
  - 通用监听器基类/桥接层
- `hooks/listeners/when-file-created.ts`
- `hooks/listeners/when-file-deleted.ts`
- `hooks/listeners/when-file-edit.ts`
- `hooks/listeners/index.ts`

定位：把 IDE 文件系统事件映射成 Hook 的 `when.*` 条件。

### 6. 触发器执行层

- `hooks/triggers/hook-trigger.ts`
  - 统一触发器接口
- `hooks/triggers/then-ask-agent.prompt.ts`
  - `askAgent` 的大 prompt 模板
- `hooks/triggers/then-ask-agent.ts`
  - 把 hook 上下文交给 agent
- `hooks/triggers/then-run-command.ts`
  - 在终端中执行 shell 命令
- `hooks/utils/spawn-hook-command.ts`
  - 生成运行命令的具体执行流程
- `hooks/utils/request-command-approval.ts`
  - 命令审批交互

关键判断：真正危险的是 `then-run-command`，因此 Hooks 系统专门拆出了审批存储和运行中断 registry，不把命令执行混在普通 trigger 逻辑里。

### 7. Contextual Hooks

- `hooks/contextual/pre-tool-use-hooks.ts`
- `hooks/contextual/post-tool-use-hooks.ts`
- `hooks/contextual/task-execution-hooks.ts`
- `hooks/contextual/contextual-hook-actions.ts`
- `hooks/contextual/contextual-hook-triggers.ts`
- `hooks/contextual/tool-type-mapping.ts`
- `hooks/contextual/hooks-provider.ts`
- `hooks/contextual/hook-operation-registry.ts`
- `hooks/contextual/hook-command-approval-storage.ts`

这一层把 hooks 从“IDE 文件事件”扩展到了“agent/tool 生命周期事件”：

- pre tool use
- post tool use
- task execution hooks
- tool type 到 hook 触发类型的映射
- operationId 到 `AbortController` 的注册/取消
- 已批准命令的 workspaceState 持久化

## 关键实现点

### Hook 文件与 ID

- Hook 文件位于 `.kiro/hooks/`
- 当前 ID 以 `fileUri` 为准
- 老版本短 id 会迁移到 URI 形式

这意味着 Kiro 已从“按名字标识 hook”切换为“按文件实体标识 hook”，更适合多工作区和重命名场景。

### 启用状态

- 启用状态不直接写回 hook 文件
- 而是写入 workspaceState
- 删除 hook 时要同步清理状态 key

这是一种“配置文件只存定义，编辑器状态单独存”的设计。

### askAgent 与 runCommand 的风险级别不同

- `askAgent` 主要风险在 prompt 注入和上下文范围
- `runCommand` 主要风险在 shell 执行和阻塞

所以 bundle 里只对 `runCommand` 额外配了审批存储、命令运行包装和取消控制。

## 关系图

```text
文件事件 / Tool 事件
        │
        ▼
listeners + contextual hooks
        │
        ▼
HookController
        │
        ├─ HookStorage 读写 .kiro/hooks/*.json
        ├─ HookValidation 校验结构
        ├─ HookTreeView 刷新侧边栏
        └─ Trigger 分发
             ├─ then-ask-agent
             └─ then-run-command
                    ├─ request-command-approval
                    ├─ spawn-hook-command
                    └─ hook-operation-registry
```

## 结论

`hooks/**` 已经是一个完整的“本地自动化编排层”，不是简单事件监听。它同时覆盖：

- 文件系统事件自动化
- 工具调用前后拦截
- task 生命周期扩展点
- 带审批的本地命令执行
- TreeView 管理界面

这也是为什么它在 extension 层属于高耦合核心模块，而不是边缘功能。
