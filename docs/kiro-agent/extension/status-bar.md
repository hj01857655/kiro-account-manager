# status-bar 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/status-bar/**` 注释边界去重

## 模块清单

```text
status-bar/status-bar-feedback-item.ts
status-bar/status-bar-usage-meter-item.ts
```

## 结构

- `status-bar/status-bar-feedback-item.ts`
- `status-bar/status-bar-usage-meter-item.ts`

`status-bar/status-bar-usage-meter-item.ts` 负责 usage meter 状态栏项，按时间刷新显示最近 usage 更新时间；`status-bar/status-bar-feedback-item.ts` 负责反馈入口。

## 判断

- `status-bar/**` 体量很小，但直接决定了使用量和反馈入口如何挂到 VS Code 状态栏。
