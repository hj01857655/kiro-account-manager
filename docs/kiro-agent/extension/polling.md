# polling 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/polling/**` 注释边界去重

## 模块清单

```text
polling/agent-event-polling-service.ts
```

## 结构

- `polling/agent-event-polling-service.ts`

`polling/agent-event-polling-service.ts` 提供 agent 事件轮询服务，默认 5 分钟轮询、2 分钟聚合延迟，负责订阅 agent 事件、安排后续命令执行、管理计时器和释放逻辑。

## 判断

- `polling/**` 是一个单模块调度服务，负责低频后台轮询与命令触发。
