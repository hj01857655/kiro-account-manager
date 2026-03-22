# notifications 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/notifications/**` 注释边界去重

## 结论

- `notifications/**` 当前共识别 `5` 个唯一模块块。
- 它负责通知设置与资源通知分发，不是简单 toast 封装。

## 模块清单

```text
notifications/index.ts
notifications/notification-service.ts
notifications/notification-settings.ts
notifications/resource-notification-service.ts
notifications/resource-notification-utils.ts
```

## 分层

- `notification-settings.ts`
- `notification-service.ts`
- `resource-notification-service.ts`
- `resource-notification-utils.ts`
- `index.ts`

## 判断

`notifications/**` 是通知策略层，拆开了偏好、资源判断和通知服务。
