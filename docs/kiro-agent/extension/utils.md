# utils 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/utils/**` 注释边界去重

## 结论

- `utils/**` 当前共识别 `19` 个唯一模块块。
- 它不是杂项目录，而是 extension 共享基础设施：agent 单例、执行日志入口、进程取消、文件访问校验、workspace 资源集合、树解析器缓存都在这里。

## 模块清单

```text
utils/directory-sizer.ts
utils/file/file-utils.ts
utils/get-session-details.ts
utils/get-execution-logger.ts
utils/trigger-agent.ts
utils/get-agent-controller.ts
utils/spawn.ts
utils/with-abort.ts
utils/trusted-tools.ts
utils/scoped-workspace-resource-collection.ts
utils/get-execution-log-controller.ts
utils/get-operation-id.ts
utils/validate-file-access.ts
utils/find-similar-file-name.ts
utils/shared-parser.ts
utils/product-config-directory.ts
utils/validate-file-ignore.ts
utils/file/list-directory-recursive.ts
utils/activation-attempts.ts
```

## 分层

### 1. Agent / Execution 入口辅助

- `utils/get-session-details.ts`
- `utils/get-execution-logger.ts`
- `utils/trigger-agent.ts`
- `utils/get-agent-controller.ts`
- `utils/get-execution-log-controller.ts`
- `utils/get-operation-id.ts`

这一组负责运行时单例、session 细节、execution logger 和操作 ID 生成。

### 2. 中断与进程执行

- `utils/spawn.ts`
- `utils/with-abort.ts`

这组提供子进程执行和统一 abort 语义。

### 3. 文件与路径安全

- `utils/file/file-utils.ts`
- `utils/validate-file-access.ts`
- `utils/validate-file-ignore.ts`
- `utils/find-similar-file-name.ts`
- `utils/file/list-directory-recursive.ts`
- `utils/product-config-directory.ts`

这组负责文件存在性、路径辅助、访问校验、忽略规则和目录遍历。

### 4. Workspace 与工具信任

- `utils/trusted-tools.ts`
- `utils/scoped-workspace-resource-collection.ts`
- `utils/directory-sizer.ts`
- `utils/activation-attempts.ts`

这组负责 trusted tools、多工作区资源集合、目录大小统计和激活尝试追踪。

### 5. 共享解析器

- `utils/shared-parser.ts`

`utils/shared-parser.ts` 用 `web-tree-sitter` 做 parser 共享缓存，按语言懒加载 `treesitter-wasm/tree-sitter-*.wasm`，为 `capabilities/tools/read-code.ts` 这类 AST 能力提供公共 parser 底座。

## 判断

- `utils/**` 是 extension shared infrastructure。
- 尤其 `utils/shared-parser.ts` 说明语法树解析器也被提升到了公共基础层，而不是塞在单个 code tool 内部。
