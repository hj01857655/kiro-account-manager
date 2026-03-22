# onboarding 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/onboarding/**` 注释边界去重

## 结论

- `onboarding/**` 当前共识别 `10` 个唯一模块块。
- 这一层实现新用户接入流程，把 CLI、设置导入、扩展安装、Dock 固定等步骤统一建模为 step service。

## 模块清单

```text
onboarding/commands.ts
onboarding/onboarding-service.ts
onboarding/steps/alias-code.ts
onboarding/steps/cli-command.ts
onboarding/steps/cli-integration-setup.ts
onboarding/errors.ts
onboarding/steps/import-settings.ts
onboarding/steps/pin-to-dock.ts
onboarding/steps/install-extensions.ts
onboarding/index.ts
```

## 分组

### 1. 服务与入口

- `onboarding/commands.ts`
- `onboarding/onboarding-service.ts`
- `onboarding/errors.ts`
- `onboarding/index.ts`

其中 `onboarding/onboarding-service.ts` 是中心服务，负责 step 注册、状态缓存、状态变更事件和 step 生命周期管理。

### 2. 具体步骤

- `onboarding/steps/alias-code.ts`
- `onboarding/steps/cli-command.ts`
- `onboarding/steps/cli-integration-setup.ts`
- `onboarding/steps/import-settings.ts`
- `onboarding/steps/pin-to-dock.ts`
- `onboarding/steps/install-extensions.ts`

这些模块把 onboarding 拆成明确步骤，而不是硬编码成单次向导。

## 判断

- `onboarding/**` 是“分步接入状态机”。
- 它的设计重点不是一次性弹窗，而是可查询、可重试、可跟踪状态变化的 onboarding step 系统。
