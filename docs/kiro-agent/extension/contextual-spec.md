# contextual-spec 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/contextual-spec/**` 注释边界去重

## 结论

- `contextual-spec/**` 当前共识别 `4` 个唯一模块块。
- 它是“从上下文直接发起 spec 生成”的薄入口层。

## 模块清单

```text
contextual-spec/command.ts
contextual-spec/generate-initial-spec.ts
contextual-spec/invoke-spec-agent.ts
contextual-spec/prompts/examples/project-spec-example.ts
```

## 分层

- `invoke-spec-agent.ts`
  - 真正调用 spec agent
- `generate-initial-spec.ts`
  - 生成初始 spec 的薄封装
- `command.ts`
  - 命令入口
- `prompts/examples/project-spec-example.ts`
  - 示例 prompt / spec 文本

## 判断

`contextual-spec/**` 是连接 `commands`、`spec-editor` 与 runtime spec agent 的小桥接层。
