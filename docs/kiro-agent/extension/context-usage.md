# context-usage 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/context-usage/**` 注释边界去重

## 模块清单

```text
context-usage/index.ts
context-usage/initial-context-estimator.ts
```

## 结构

- `context-usage/index.ts`
- `context-usage/initial-context-estimator.ts`

`context-usage/initial-context-estimator.ts` 估算系统 prompt、MCP tools、always-included steering 文档占用的初始上下文 token，并根据阈值给出 usage 百分比判断。

## 判断

- `context-usage/**` 是上下文预算预估层，主要服务于 token/上下文容量感知。
