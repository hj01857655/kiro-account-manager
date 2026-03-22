# context-lsp 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/context-lsp/**` 注释边界去重

## 结论

- `context-lsp/**` 当前共识别 `11` 个唯一模块块。
- 它为 `#[[...]]` 上下文引用语法提供语言服务能力：
  - 解析
  - completion
  - hover
  - definition/document link
  - semantic highlighting

## 模块清单

```text
context-lsp/completion/completion-provider.ts
context-lsp/definition/document-link-provider.ts
context-lsp/errors.ts
context-lsp/highlighting/semantic-provider.ts
context-lsp/hover/hover-provider.ts
context-lsp/index.ts
context-lsp/parser/syntax.ts
context-lsp/providers/continue/continue-adapter.ts
context-lsp/providers/continue/file-provider.ts
context-lsp/providers/registry.ts
context-lsp/utils/references.ts
```

## 分层

### 1. Provider Registry

- `context-lsp/providers/registry.ts`
  - 维护 provider 单例 registry
  - 支持按 path segment 逐级解析 provider / item

这是整套 LSP 能力的核心路由层。

### 2. Continue 适配

- `context-lsp/providers/continue/continue-adapter.ts`
- `context-lsp/providers/continue/file-provider.ts`

说明：

- context-lsp 不是直接硬编码所有上下文源
- 而是把 Continue/Context Provider 体系适配进来

### 3. 引用语法解析

- `context-lsp/parser/syntax.ts`
  - 解析 `#[[...]]`
  - 校验 path segment
  - 支持相对文件路径标准化
  - 提取全文所有 context references

### 4. 编辑器语言能力

- `context-lsp/completion/completion-provider.ts`
- `context-lsp/hover/hover-provider.ts`
- `context-lsp/definition/document-link-provider.ts`
- `context-lsp/highlighting/semantic-provider.ts`
- `context-lsp/utils/references.ts`

这几层共同构成 context reference 的“可编辑体验”。

### 5. 错误与入口

- `context-lsp/errors.ts`
- `context-lsp/index.ts`

## 判断

`context-lsp/**` 是 Kiro 上下文引用语法在 IDE 中成立的关键原因。没有它，`#[[provider:path]]` 只是文本约定；有了它，用户才能获得：

- 自动补全
- 悬停解释
- 跳转/链接
- 语法高亮

这让 context reference 真正变成语言级能力，而不是 prompt 小技巧。
