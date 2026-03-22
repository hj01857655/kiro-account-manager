# extension-resources（Hook JSON Schema）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\extension-resources`
> 分析日期：2026-03-18

该目录仅包含 `hook.json`，用于 VS Code JSON 校验与 Hook 编辑器表单约束。

---

## hook.json 结构

- `when`（触发条件）可选类型：
- `fileEdited`：`pattern` + `scopeId`
- `fileCreated`：`pattern`
- `fileDeleted`：`pattern`
- `userTriggered`

- `then`（执行动作）可选类型：
- `alert`：`message`
- `askAgent`：`prompt` 或 `promptId`

- `id` / `name` / `comment` 为可选元数据。

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                          extension-resources                                 │
│                                                                              │
│  hook.json  (draft-07 JSON Schema)                                           │
│   when: fileEdited / fileCreated / fileDeleted / userTriggered               │
│   then: alert / askAgent                                                     │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │           ┌──────────────────────────────────────────┐         │
│             │           │ VS Code JSON Validation                  │         │
│             │           │ .kiro/hooks/*.json                        │         │
│             │           └──────────────────────────────────────────┘         │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  Hook 编辑器 / 命令读写 hook 配置                                │
└──────────────────────────────────────────────────────────────────────────────┘
```
