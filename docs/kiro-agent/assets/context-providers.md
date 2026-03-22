# Context Providers（kiro.kiro-agent）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\kiro-context-providers\dist`
> 分析日期：2026-03-17

本文件聚焦 context providers 的语法、缓存与各 provider 行为。

---

## 通用语法

- Context reference 语法：`#[[provider:query]]`。
- 解析规则：只在第一个冒号处分割 provider 与 query（允许 query 内包含冒号）。

---

## FileSystemCache

`FileSystemCache` 负责单次树遍历与后续变更更新，`file` 与 `folder` provider 共享该缓存。

---

## Providers 一览

- `currentlyOpenFiles`：支持 `active` 与 `all`，返回当前活跃文件或全部打开文件列表。
- `fileTree`：返回工作区文件树视图。
- `file`：支持 `file:path` 与 `file:path:10-20` 行号范围解析，输出带行号范围的内容块。
- `folder`：列出目录内容（区分 `[dir]` 与文件）。
- `spec`：`#[[spec:type:path]]`，附带 workflow 警告；bugfix 模式使用 `bugfix.md` 替代 `requirements.md`。
- `steering`：`#[[steering:scope:path]]`，返回 `<user-rule>` 包裹的规则文本，workspace 优先级高于 global。
- `mcp`：`#[[mcp:type:server:identifier]]`，支持 `?` 参数与 `resourceTemplate` URI 模板展开，详见 `../extension/mcp.md`。

---

## 架构流程图

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                            Context Providers                                         │
│                                                                                      │
│  用户输入 #[[provider:query]]                                                        │
│             │                                                                        │
│             ▼                                                                        │
│  ContextReferenceParser                                                              │
│   按第一个 ":" 分割 provider / query                                                  │
│             │                                                                        │
│             ├──────────────────────────────────────────────────────────────────────► │
│             │                                                                        │
│             │             ┌──────────────────────────────────────────┐               │
│             │             │ 不需要文件系统的 Provider                 │               │
│             │             │  currentlyOpenFiles (active/all)          │               │
│             │             │  fileTree (工作区树)                       │               │
│             │             │  spec / steering / mcp                     │               │
│             │             └──────────────────────────────────────────┘               │
│             │                                                                        │
│             │  ◄────────────────────────────────────────────────────────────────────┤
│             │                                                                        │
│             ▼                                                                        │
│  需要文件系统的 Provider                                                             │
│   FileSystemCache → 单次树遍历 + watchFile 增量更新                                   │
│   file / folder 共享此缓存                                                           │
│             │                                                                        │
│             ▼                                                                        │
│  file / folder                                                                        │
│   file: path:10-20 支持行号范围 + substring 搜索                                       │
│   folder: 区分 [dir] 与文件                                                          │
│                                                                                      │
│  spec / steering / mcp 细节                                                          │
│   spec: 缺 requirements.md 或 design.md 时注入强制流程提示                            │
│   steering: workspace 优先于 global                                                    │
│   mcp: #[[mcp:type:server:id]] + resourceTemplate + ? 参数                            │
│             │                                                                        │
│             ├──────────────────────────────────────────────────────────────────────► │
│             │                                                                        │
│             │                 ┌────────────────────────────────────┐                 │
│             │                 │ MCP Manager / MCP Servers           │                 │
│             │                 │ tools / resources / prompts         │                 │
│             │                 └────────────────────────────────────┘                 │
│             │                                                                        │
│             │  ◄────────────────────────────────────────────────────────────────────┤
│             │                 context references / resources                          │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 其他要点

- `file` / `folder` 提供 substring 搜索与 context item 列表。
- `spec` 会在缺少 `requirements.md` 或 `design.md` 时注入强制工作流提示。
- `steering` 输出包含明确优先级提示，方便模型理解规则覆盖关系。
