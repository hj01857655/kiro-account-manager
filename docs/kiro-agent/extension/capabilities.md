# capabilities 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/capabilities/**` 注释边界去重

## 结论

- `capabilities/**` 当前共识别 `47` 个唯一模块块。
- 这一层不是单个 feature，而是 extension 暴露给 agent/runtime 的“工具能力装配层”。
- 它负责把 VS Code 文件系统、终端、搜索、工作区、Powers、MCP、编辑器能力拼装成可调用工具。

## 模块清单

```text
capabilities/errors.ts
capabilities/tool-factories/remote.ts
capabilities/fs/delete.ts
capabilities/fs/read-directory.ts
capabilities/utils/convert-file-type.ts
capabilities/fs/read-file.ts
capabilities/fs/stat.ts
capabilities/fs/write-file.ts
capabilities/fs/read-text-file.ts
capabilities/fs/write-text-file.ts
capabilities/terminal/create.ts
capabilities/terminal/kill.ts
capabilities/terminal/release.ts
capabilities/terminal/output.ts
capabilities/terminal/wait-for-exit.ts
capabilities/search/find-files.ts
capabilities/search/text-search.ts
capabilities/client-tools.ts
capabilities/tools/get-diagnostics.ts
capabilities/tools/semantic-rename.ts
capabilities/tools/errors.ts
capabilities/tools/smart-relocate.ts
capabilities/tools/read-code.ts
capabilities/tools/edit-code.ts
capabilities/tools/create-hook.ts
capabilities/tools/disclose-context.ts
capabilities/tools/kiro-powers.ts
capabilities/workspace/active-file.ts
capabilities/workspace/currently-open-files.ts
capabilities/bindings/checkpoint-file.ts
capabilities/bindings/resolve-workspace-uri.ts
capabilities/create-workspace-connection.ts
capabilities/workspace-providers.ts
capabilities/bindings/read-file.ts
capabilities/bindings/write-file.ts
capabilities/bindings/errors.ts
capabilities/bindings/update-task-status.ts
capabilities/bindings/validate-task-status-change.ts
capabilities/bindings/get-task-metadata.ts
capabilities/bindings/update-pbt-status.ts
capabilities/bindings/as-relative-path.ts
capabilities/tool-registry.ts
capabilities/tools/mcp-wrapper.ts
capabilities/tool-factories/fs.ts
capabilities/utils/protected-path-checker.ts
capabilities/tool-factories/shell.ts
capabilities/tool-factories/search.ts
```

## 分组

### 1. Root / 装配入口

- `capabilities/errors.ts`
- `capabilities/client-tools.ts`
- `capabilities/create-workspace-connection.ts`
- `capabilities/workspace-providers.ts`
- `capabilities/tool-registry.ts`

这一组决定 capability 的整体装配方式：

- `capabilities/create-workspace-connection.ts` 创建工作区连接，把 `VSCodeFileSystem`、终端管理、后台进程、steering/task/checkpoint/file ops provider 一次性注入。
- `capabilities/workspace-providers.ts` 组织工作区侧 provider，让 capability 可以绑定当前 workspace 语义。
- `capabilities/tool-registry.ts` 是中心注册表，向 chat/spec/sub-agent/system 暴露工具集合。
- `capabilities/errors.ts` 定义基础错误，例如 `InteractionError`、`NoWorkspaceError`。
- `capabilities/client-tools.ts` 负责把一组客户端工具打包成交付给上层 agent 的集合。

### 2. bindings

- `capabilities/bindings/checkpoint-file.ts`
- `capabilities/bindings/resolve-workspace-uri.ts`
- `capabilities/bindings/read-file.ts`
- `capabilities/bindings/write-file.ts`
- `capabilities/bindings/errors.ts`
- `capabilities/bindings/update-task-status.ts`
- `capabilities/bindings/validate-task-status-change.ts`
- `capabilities/bindings/get-task-metadata.ts`
- `capabilities/bindings/update-pbt-status.ts`
- `capabilities/bindings/as-relative-path.ts`

这层是 capability 和 workspace/task/spec 数据之间的适配胶水：

- `capabilities/bindings/resolve-workspace-uri.ts` 负责把相对路径、绝对路径、`~/.kiro` 路径、多工作区名称前缀映射到真实 `Uri`。
- `capabilities/bindings/read-file.ts` / `capabilities/bindings/write-file.ts` 封装绑定态文件访问。
- `capabilities/bindings/checkpoint-file.ts` 连接 checkpoint 文件接口。
- `capabilities/bindings/update-task-status.ts`、`capabilities/bindings/validate-task-status-change.ts`、`capabilities/bindings/get-task-metadata.ts`、`capabilities/bindings/update-pbt-status.ts` 把任务状态/PBT 状态变更接到 spec 体系。
- `capabilities/bindings/as-relative-path.ts` 负责路径标准化。
- `capabilities/bindings/errors.ts` 收口绑定层错误。

### 3. fs

- `capabilities/fs/delete.ts`
- `capabilities/fs/read-directory.ts`
- `capabilities/fs/read-file.ts`
- `capabilities/fs/stat.ts`
- `capabilities/fs/write-file.ts`
- `capabilities/fs/read-text-file.ts`
- `capabilities/fs/write-text-file.ts`

这些模块对应底层文件系统工具：

- `capabilities/fs/read-file.ts`、`capabilities/fs/read-text-file.ts` 支持整文件或行区间读取。
- `capabilities/fs/write-file.ts`、`capabilities/fs/write-text-file.ts` 处理整文件写入或文本写入。
- `capabilities/fs/delete.ts` 删除文件。
- `capabilities/fs/read-directory.ts` 枚举目录项。
- `capabilities/fs/stat.ts` 返回 size/type。
- 共同特征是都会先做 URI 访问校验，避免越权读写。

### 4. search

- `capabilities/search/find-files.ts`
- `capabilities/search/text-search.ts`

这两个模块分别提供文件搜索和文本 grep 能力，给 agent 的“先搜索再编辑”链路提供低成本探测。

### 5. terminal

- `capabilities/terminal/create.ts`
- `capabilities/terminal/kill.ts`
- `capabilities/terminal/release.ts`
- `capabilities/terminal/output.ts`
- `capabilities/terminal/wait-for-exit.ts`

这一组把终端会话抽象成可控资源：

- 创建、终止、释放、读取输出、等待退出全部拆成独立模块。
- 上层 shell tool factory 会把这些原语组合成 `execute bash`、进程控制、输出流查询能力。

### 6. tool-factories

- `capabilities/tool-factories/remote.ts`
- `capabilities/tool-factories/fs.ts`
- `capabilities/tool-factories/shell.ts`
- `capabilities/tool-factories/search.ts`

这层不是直接实现工具，而是批量生成工具定义：

- `capabilities/tool-factories/remote.ts` 负责把远端 MCP 工具包装进本地工具体系，串上 `mcpClient`、`toolTrustManager`、命令审批和远程工具发现。
- `capabilities/tool-factories/fs.ts` 统一生成文件系统相关 tool。
- `capabilities/tool-factories/shell.ts` 组合出执行 bash、控制进程、拉取输出、列进程等工具。
- `capabilities/tool-factories/search.ts` 组合文件搜索与文本搜索工具。

### 7. tools

- `capabilities/tools/get-diagnostics.ts`
- `capabilities/tools/semantic-rename.ts`
- `capabilities/tools/errors.ts`
- `capabilities/tools/smart-relocate.ts`
- `capabilities/tools/read-code.ts`
- `capabilities/tools/edit-code.ts`
- `capabilities/tools/create-hook.ts`
- `capabilities/tools/disclose-context.ts`
- `capabilities/tools/kiro-powers.ts`
- `capabilities/tools/mcp-wrapper.ts`

这是 capability 目录最重的业务工具层：

- `capabilities/tools/edit-code.ts` 约 `1340` 行，是 AST 级代码编辑器，支持 `replace_node` / `insert_node` / `delete_node` 和多语言 selector 解析。
- `capabilities/tools/kiro-powers.ts` 约 `884` 行，负责列出、激活、使用、读取 steering、配置 Powers。
- `capabilities/tools/mcp-wrapper.ts` 统一包装 MCP 工具调用，处理 consent、execution event、错误归一化和 response 回传。
- `capabilities/tools/read-code.ts` 负责读取代码并服务于上下文提取。
- `capabilities/tools/smart-relocate.ts` 负责结构化搬移/重定位编辑目标。
- `capabilities/tools/create-hook.ts` 直接接入 Hook 创建能力。
- `capabilities/tools/get-diagnostics.ts` 读取诊断信息。
- `capabilities/tools/semantic-rename.ts` 触发语义级 rename。
- `capabilities/tools/disclose-context.ts` 负责向模型显式暴露当前上下文。
- `capabilities/tools/errors.ts` 收敛工具侧错误模型。

### 8. utils

- `capabilities/utils/convert-file-type.ts`
- `capabilities/utils/protected-path-checker.ts`

这层提供小而关键的安全辅助能力：

- `capabilities/utils/convert-file-type.ts` 在 VS Code `FileType` 和 capability 输出类型之间做映射。
- `capabilities/utils/protected-path-checker.ts` 用于识别受保护路径，避免 agent 误伤敏感区域。

### 9. workspace

- `capabilities/workspace/active-file.ts`
- `capabilities/workspace/currently-open-files.ts`

这两项是“编辑器现场感知”入口：

- `capabilities/workspace/active-file.ts` 返回当前活动文件。
- `capabilities/workspace/currently-open-files.ts` 返回当前打开文件集合。

## 关键链路

### 工作区连接装配

- `capabilities/create-workspace-connection.ts`
- `capabilities/workspace-providers.ts`
- `capabilities/bindings/resolve-workspace-uri.ts`

这一链路把 workspace root、路径解析、文件系统、任务状态、checkpoint/provider 全接到 capability 容器里。

### 工具注册装配

- `capabilities/tool-registry.ts`
- `capabilities/tool-factories/fs.ts`
- `capabilities/tool-factories/search.ts`
- `capabilities/tool-factories/shell.ts`
- `capabilities/tool-factories/remote.ts`

这一链路决定 chat/spec/sub-agent 到底能看到哪些工具，以及每类工具如何被批量注入。

### 高价值工具

- `capabilities/tools/edit-code.ts`
- `capabilities/tools/kiro-powers.ts`
- `capabilities/tools/mcp-wrapper.ts`
- `capabilities/tools/read-code.ts`
- `capabilities/tools/smart-relocate.ts`

这些是 capability 层最重的执行模块，分别覆盖 AST 编辑、Powers 调用、MCP 代理、代码读取和结构化迁移。

## 判断

- `capabilities/**` 是 extension 和 agent runtime 的桥。
- 如果没有这一层，commands/views 仍然存在，但 agent 无法安全地拿到文件、终端、搜索、MCP、Powers、workspace 上下文这些执行能力。
- 因此它本质上是“工具装配总线”，不是单一 feature。
