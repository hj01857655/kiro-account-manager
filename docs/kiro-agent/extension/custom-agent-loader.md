# custom-agent-loader 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/custom-agent-loader/**` 注释边界去重

## 结论

- `custom-agent-loader/**` 当前共识别 `4` 个唯一模块块。
- 这层负责把 `.kiro/agents/` 下的用户/工作区自定义 agent 文件加载进运行时 registry，并支持热重载。

## 模块清单

```text
custom-agent-loader/commands/list-custom-agents.ts
custom-agent-loader/custom-agent-loader.ts
custom-agent-loader/custom-agent-registry-initializer.ts
custom-agent-loader/prompt-file-resolver.ts
```

## 分层

### 1. Registry 初始化

- `custom-agent-loader/custom-agent-registry-initializer.ts`
  - 建立全局 `customAgentRegistry`
  - 先注册 builtin agents
  - 再追加外部 provider 返回的 agents
  - 支持 `reloadBuiltinCustomAgents()`

关键点：自定义 agent 不是替代 builtin，而是叠加到同一 registry。

### 2. Prompt 文件解析

- `custom-agent-loader/prompt-file-resolver.ts`
  - 判断 `prompt` 是否是 `file://`
  - 支持绝对路径与相对当前 agent 文件的 prompt 路径
  - 从外部 prompt 文件读取实际 prompt 内容

这意味着 JSON agent 定义可以把 prompt 主体拆到单独文件，不必全部内联。

### 3. 文件加载与热重载

- `custom-agent-loader/custom-agent-loader.ts`
  - 383 行，主核心
  - `CustomAgentFileLoader` 负责：
    - 加载用户级 `~/.kiro/agents/`
    - 加载工作区 `<workspace>/.kiro/agents/`
    - 支持 `.md` 与 `.json`
    - JSON 模式可引用外部 prompt 文件
    - 文件变化时 debounce 重载
    - 文件删除时自动 unregister
    - workspace folder 新增/删除时同步增删 agents
    - workspace trust 获得后才加载 workspace agents

### 4. 命令层

- `custom-agent-loader/commands/list-custom-agents.ts`
  - 对外列出当前 registry 中的 custom agents

## 关键机制

### 双来源

- 用户级：`~/.kiro/agents/`
- 工作区级：`<workspace>/.kiro/agents/`

而且 workspace agents 受 `workspace.isTrusted` 保护，说明 Kiro 明确把工作区 agent 定义视为可执行配置。

### 双格式

- `.json`
  - 先 `JSON.parse`
  - 走 schema 校验
  - `prompt` 可为 `file://...`
- `.md`
  - 走 front matter 解析
  - prompt 直接取正文

### 热重载与外部 prompt watcher

除了监听 agent 定义文件本身，loader 还会为 JSON agent 的外部 prompt 文件建立独立 watcher：

- prompt 文件变更：重新加载 agent
- prompt 文件删除：注销 agent

这块实现比较细，说明 Kiro 把“prompt 是外部文件”当一等场景处理，而不是附带能力。

### CLI-only 字段过滤

bundle 里可以看到有一段专门检查 unsupported fields，并提示：

- 某些字段只支持 CLI
- IDE 中发现后直接拒绝加载该 agent

这说明 IDE custom agent 和 CLI custom agent 的能力边界并不完全相同。

## 关系图

```text
~/.kiro/agents/            <workspace>/.kiro/agents/
       │                            │
       └──────────────┬─────────────┘
                      ▼
             CustomAgentFileLoader
                      │
          ├─ 解析 .md / .json
          ├─ 解析 file:// prompt
          ├─ watch agent files
          ├─ watch external prompt files
          └─ trust / workspace folder 变化处理
                      │
                      ▼
               customAgentRegistry
```

## 结论

`custom-agent-loader/**` 是 IDE 侧“用户自定义 agent 扩展点”的落地点。它已经具备：

- 用户级与工作区级加载
- 热重载
- 外部 prompt 文件依赖
- trust 边界控制
- builtin + custom 同 registry 共存

这不是简单读配置，而是完整的 agent 插件装载层。
