# usage 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/usage/**` 注释边界去重

## 结论

- `usage/**` 当前共识别 `3` 个唯一模块块。
- 它负责 usage limits 的缓存、事件分发和注册时机控制。

## 模块清单

```text
usage/usage-limits-availability-cache.ts
usage/usage-limits-event-emitter.ts
usage/usage-monitoring.ts
```

## 分层

- `usage-limits-availability-cache.ts`
  - usage 可用性缓存
  - 登录态变化时清理
  - disabled 场景带重试窗口
- `usage-limits-event-emitter.ts`
  - usage limits 更新事件总线
- `usage-monitoring.ts`
  - 登录后主动拉取 usage limits
  - enterprise profile 未 ready 时延后注册

## 判断

`usage/**` 是配额信息的宿主协调层，负责“什么时候拉、什么时候发、什么时候缓存”。
