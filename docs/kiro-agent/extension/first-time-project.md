# first-time-project 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/first-time-project/**` 注释边界去重

## 模块清单

```text
first-time-project/first-time-project-controller.ts
first-time-project/first-time-project-detection-service.ts
first-time-project/index.ts
```

## 结构

- `first-time-project/first-time-project-controller.ts`
- `first-time-project/first-time-project-detection-service.ts`
- `first-time-project/index.ts`

这里把“首次进入项目”的检测与控制拆成独立 controller + detection service，用于判断何时触发首次项目体验。

## 判断

- `first-time-project/**` 属于首次项目进入体验的触发层。
