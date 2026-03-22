# telemetry 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/telemetry/**` 注释边界去重

## 结论

- `telemetry/**` 当前共识别 `4` 个唯一模块块。
- 这一层负责 extension 侧的 telemetry 接线，不是完整埋点实现本体。

## 模块清单

```text
telemetry/agent-telemetry-adapter.ts
telemetry/command.ts
telemetry/telemetry-config.ts
telemetry/workspace.ts
```

## 分层

### 1. 命令埋点包装

- `telemetry/command.ts`
  - `registerInstrumentedCommand()`
  - `registerInstrumentedTextEditorCommand()`

作用：统一给 VS Code 命令包上 trace，而不是每个命令手写 telemetry。

### 2. Agent Telemetry 适配

- `telemetry/agent-telemetry-adapter.ts`
  - 提供 `sharedMetricReporterFactory`
  - `ToolTelemetryAdapter` 把 runtime 侧 metric 调用接到 extension 侧 `ToolUsage`
  - `initializeAgentTelemetry()`：
    - 注入 metric reporter factory
    - 注入 tool telemetry reporter factory

关键判断：这里是 `packages/kiro-agent/dist/**` 与扩展宿主之间的 telemetry 桥接层。

### 3. Telemetry 端点配置

- `telemetry/telemetry-config.ts`
  - 按运行模式和 endpoint override 选 telemetry endpoint
  - 预置：
    - `beta`
    - `gamma`
    - `prod`
  - dev 模式优先 beta
  - 存在 codewhisperer endpoint override 时切 gamma

这说明 Kiro 的 telemetry 后端至少区分开发、灰度和正式环境。

### 4. Workspace 周期指标

- `telemetry/workspace.ts`
  - 定期采集用户工作区维度指标：
    - workspace folder 数
    - open editors 数
    - extension 总数 / active 数
    - 工作区大小
    - 文件数
    - `activeIde`

定位：这是产品使用环境的背景指标，不是单次命令事件。

## 关键机制

### “包装命令”而不是“命令里散落埋点”

`telemetry/command.ts` 说明：

- Kiro 优先用统一注册器包装命令
- trace 粒度与命令边界对齐

这比在每个 handler 里手写 `reportCountMetrics()` 更稳定。

### runtime telemetry 要通过宿主注入

从 `agent-telemetry-adapter.ts` 可以确定：

- runtime 自身并不直接依赖 extension 的 `MetricReporter`
- 而是通过注入 factory 的方式桥接

这符合 runtime 被设计成可嵌入内核的思路。

## 关系图

```text
VS Code commands
      │
      ▼
telemetry/command.ts
      │
      ▼
MetricReporter trace

packages/kiro-agent runtime
      │
      ▼
agent-telemetry-adapter.ts
      │
      ▼
ToolUsage / MetricReporter

workspace.ts
  └─ 周期性采集环境指标
```

## 结论

`telemetry/**` 不是“所有埋点都在这里”，而是 extension telemetry 的接线板：

- 命令 trace 包装
- runtime metric 桥接
- telemetry endpoint 选择
- workspace 周期性背景指标
