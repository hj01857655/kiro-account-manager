# commands 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/commands/**` 注释边界去重

## 结论

- `commands/**` 当前共识别 `38` 个唯一模块块。
- 它是 extension 对外公开能力的总调度层，而不是简单命令集合。

## 模块清单

```text
commands/agent/chat-agent-command.ts
commands/agent/compact-agent-command.ts
commands/agent/create-hook.ts
commands/agent/index.ts
commands/agent/retry-agent-command.ts
commands/agent/utils.ts
commands/cancel-hook-command.ts
commands/configuration/index.ts
commands/create-debug-log-zip.ts
commands/debug-capture-llm-log.ts
commands/debug-capture-log.ts
commands/delete-account.ts
commands/enable-shell-integration.ts
commands/file-feedback.ts
commands/get-codewhisperer-config.ts
commands/index.ts
commands/list-models/get-available-models.ts
commands/open-execution-logs.ts
commands/profiles/get-profile.ts
commands/profiles/index.ts
commands/profiles/list-available-profiles.ts
commands/profiles/select-profile.ts
commands/record-references.ts
commands/refresh-remote-tools.ts
commands/source-control/errors.ts
commands/source-control/generate-commit-message.ts
commands/source-control/index.ts
commands/subscription-plans/get-checkout-session-url.ts
commands/subscription-plans/get-portal-session-url.ts
commands/subscription-plans/get-subscription-plans.ts
commands/supervised-diff/index.ts
commands/telemetry/can-enable-telemetry.ts
commands/telemetry/telemetry.ts
commands/usage-limits/enable-overages.ts
commands/usage-limits/get-usage-limits.ts
commands/utils/extract-subscription-name.ts
commands/utils/get-currency.ts
commands/utils/kiro-version.ts
```

## 分组

### 1. Agent 命令

- `commands/agent/*`
  - chat / retry / compact / create-hook / utils / index

其中 `commands/agent/compact-agent-command.ts` 达到 379 行，是这个目录最重的单模块之一，说明上下文压缩是显式的一等命令能力。

### 2. 账号 / 订阅 / 配额

- `commands/profiles/*`
- `commands/subscription-plans/*`
- `commands/usage-limits/*`
- `commands/delete-account.ts`
- `commands/list-models/get-available-models.ts`

### 3. Source Control / Diff

- `commands/source-control/*`
- `commands/supervised-diff/index.ts`

### 4. 调试 / 日志 / 支持

- `commands/debug-capture-log.ts`
- `commands/debug-capture-llm-log.ts`
- `commands/create-debug-log-zip.ts`
- `commands/open-execution-logs.ts`
- `commands/file-feedback.ts`
- `commands/record-references.ts`

### 5. 配置 / Telemetry / 其它

- `commands/configuration/index.ts`
- `commands/telemetry/*`
- `commands/get-codewhisperer-config.ts`
- `commands/enable-shell-integration.ts`
- `commands/refresh-remote-tools.ts`
- `commands/cancel-hook-command.ts`
- `commands/utils/*`
- `commands/index.ts`

## 判断

`commands/**` 的价值在于定义“产品表面”：

- 哪些能力可直接触发
- 哪些能力被 webview / 菜单 / quick pick 间接调用
- 哪些 feature 真正进入 VS Code 命令系统

## 结论

`commands/**` 可以视为 extension 的公开能力总表。它不是业务内核，但决定了哪些业务内核真正可用。
