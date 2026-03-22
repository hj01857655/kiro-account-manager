# experiments 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/experiments/**` 注释边界去重

## 结论

- `experiments/**` 当前共识别 `7` 个唯一模块块。
- 它实现的是一套完整的 feature flag 子系统，而不只是状态栏开关。

## 模块清单

```text
experiments/experiments-activation.ts
experiments/experiments-config-provider.ts
experiments/experiments-service.ts
experiments/experiments-status-bar.ts
experiments/experiments-telemetry.ts
experiments/experiments.ts
experiments/is-experiment-visible.ts
```

## 分层

### 1. 实验定义

- `experiments/experiments.ts`
  - 定义 `EXPERIMENT_DEFINITIONS`
  - 每个实验项带：
    - `name`
    - `owner`
    - `description`
    - `default`
    - `visibility`

bundle 中能看到的实验示例包括：

- `filePruning`
- `enqueueMessage`
- `subagentContinuation`
- `mergeVibeSpec`
- `acpChat`
- `sessionManagerV2`

### 2. 可见性过滤

- `experiments/is-experiment-visible.ts`
  - 根据 `visibility` 和 `vscode.env.buildQuality` 判断是否显示

可见性至少有：

- `none`
- `stable`
- `insider`
- `dev`

这说明实验开关不是简单全量暴露，而是和构建渠道绑定。

### 3. 配置提供层

- `experiments/experiments-config-provider.ts`
  - 读取 `kiroAgent.experiments`
  - 只保留当前 buildQuality 可见的 definitions
  - 读取当前用户设置
  - 更新多项实验值时会与已有配置 merge，避免覆盖不可见实验
  - 监听配置变化并回调订阅者

### 4. 服务层

- `experiments/experiments-service.ts`
  - 对外暴露 `isEnabled()`、`experiments`、`definitions`
  - 负责 listeners 管理
  - 特殊兼容：
    - `sessionManagerV2` 开启逻辑会兼容 `acpChat`

关键点：这里已经出现 feature 之间的兼容逻辑，而不只是纯配置读写。

### 5. 状态栏与 Quick Pick

- `experiments/experiments-status-bar.ts`
  - 创建 `$(beaker) Experiments` 状态栏项
  - 点击后弹出 `showQuickPick(canPickMany: true)`
  - 多选结果回写 experiments 配置

### 6. Telemetry

- `experiments/experiments-telemetry.ts`
  - 上报 feature toggled
  - 周期性统计：
    - totalFeatures
    - enabledFeatures
    - disabledFeatures
    - 每个 feature 的 enabled 状态

### 7. 激活层

- `experiments/experiments-activation.ts`
  - 初始化 config provider + service
  - 同步 `setContext("kiroAgent.experiments.<id>")`
  - 注册 `kiroAgent.experiments.getExperiments`
  - 若存在可见实验，则打开 `kiroAgent.showExperiments`
  - 启动 telemetry 和状态栏

## 关键机制

### buildQuality 感知

Experiments 系统不是单纯看用户设置，它先按：

- stable
- rc
- insider
- dev

做一轮可见性过滤。也就是说，同一份代码在不同发行渠道暴露的实验列表会不同。

### 设置合并而不是整表覆盖

`updateExperiments()` 会：

- 读取当前用户设置
- 只覆盖本次操作涉及的 key
- merge 回原配置

这样可以保留不可见实验或旧版本实验的状态。

### VS Code context 同步

激活后会为每个实验写入：

- `kiroAgent.experiments.<id>`

这意味着菜单、命令、按钮显隐并不直接读配置，而是通过 VS Code `when` context 联动。

## 关系图

```text
EXPERIMENT_DEFINITIONS
        │
        ▼
is-experiment-visible
        │
        ▼
ExperimentsConfigProvider
        │
        ▼
ExperimentsService
    ├─ ExperimentsStatusBar
    ├─ ExperimentsTelemetry
    └─ experiments-activation
           ├─ setContext(kiroAgent.experiments.*)
           └─ register getExperiments command
```

## 结论

`experiments/**` 已经是标准 feature flag 平台，具备：

- 定义表
- 渠道可见性控制
- 配置持久化
- VS Code context 同步
- UI 开关入口
- telemetry

不是零散的“实验按钮”集合。
