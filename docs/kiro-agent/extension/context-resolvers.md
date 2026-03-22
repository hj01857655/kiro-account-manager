# context-resolvers 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/context-resolvers/**` 注释边界去重

## 结论

- `context-resolvers/**` 当前共识别 `9` 个唯一模块块。
- 它负责把 IDE 里的不同上下文源统一解析成 agent 可消费内容。

## 模块清单

```text
context-resolvers/currently-open-files.ts
context-resolvers/file.ts
context-resolvers/folder.ts
context-resolvers/mcp.ts
context-resolvers/spec.ts
context-resolvers/steering.ts
context-resolvers/utils/to-cache-item.ts
context-resolvers/utils/walk-file-tree.ts
context-resolvers/utils/walk-workspace-tree.ts
```

## 分层

### 1. 基础 resolver

- `context-resolvers/file.ts`
- `context-resolvers/folder.ts`
- `context-resolvers/currently-open-files.ts`

负责最基本的文件、目录、当前打开文件上下文。

### 2. Kiro 专有上下文

- `context-resolvers/spec.ts`
- `context-resolvers/steering.ts`
- `context-resolvers/mcp.ts`

对应 Kiro 自己的高阶上下文源：

- spec 文档
- steering 文档
- MCP 相关上下文

### 3. 遍历与缓存辅助

- `context-resolvers/utils/walk-file-tree.ts`
- `context-resolvers/utils/walk-workspace-tree.ts`
- `context-resolvers/utils/to-cache-item.ts`

说明：

- resolver 不是简单读一次文件
- 还要把目录树遍历结果和缓存项格式统一起来

## 判断

`context-resolvers/**` 是“扩展宿主世界”到“agent prompt/context 世界”的转换层。没有这一层，runtime 只能拿到抽象接口，拿不到 VS Code 里的真实工作区视角。
