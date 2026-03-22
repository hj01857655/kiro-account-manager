# checkpoints 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/checkpoints/**` 注释边界去重

## 结论

- `checkpoints/**` 当前共识别 `5` 个唯一模块块。
- 这层负责文件系统 checkpoint 的抽象与 provider。

## 模块清单

```text
checkpoints/checkpoint-file-system-provider.ts
checkpoints/errors.ts
checkpoints/fs-checkpoint-controller.ts
checkpoints/index.ts
checkpoints/meta-file-system-provider.ts
```

## 分层

- `fs-checkpoint-controller.ts`
- `checkpoint-file-system-provider.ts`
- `meta-file-system-provider.ts`
- `errors.ts`
- `index.ts`

## 判断

`checkpoints/**` 是文件系统级回滚/状态快照基础设施，位于执行与存储之间。
