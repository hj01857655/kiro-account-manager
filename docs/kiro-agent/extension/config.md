# config 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/config/**` 注释边界去重

## 结论

- `config/**` 当前共识别 `10` 个唯一模块块。
- 它不是单纯配置常量目录，而是 extension 的配置同步与 trust 管理中枢。

## 模块清单

```text
config/application-config.ts
config/autonomy-mode.ts
config/config-keys.ts
config/errors.ts
config/index.ts
config/package-manager.ts
config/setting-context-sync.ts
config/workspace-config.ts
config/workspace-state.ts
config/workspace-trust-warning.ts
```

## 分层

### 1. 配置读写抽象

- `config/workspace-config.ts`
  - 面向 workspace 的配置存取和监听
- `config/application-config.ts`
  - 面向全局 application 级配置存取和监听

这说明 Kiro 显式区分了：

- 当前 workspace 范围配置
- 用户全局配置

### 2. Context 同步

- `config/config-keys.ts`
  - 定义需要同步到 VS Code context 的关键配置项
- `config/setting-context-sync.ts`
  - 把配置值写到 `kiroAgent.settings.*` context key

关键点：菜单和 UI 显隐不是直接读取配置，而是走 `setContext`。

### 3. Trust / Workspace State

- `config/workspace-trust-warning.ts`
  - 未受信任 workspace 的提示与状态记录
- `config/workspace-state.ts`
  - 暴露 workspaceState 读写命令

### 4. 其它

- `config/autonomy-mode.ts`
  - workspace 不受信任时强制 `Supervised`
- `config/package-manager.ts`
  - 包管理器相关解析
- `config/errors.ts`
  - `InvalidModelIdentifierError`
- `config/index.ts`
  - 聚合注册入口

## 关键判断

### Trust 会直接影响 agent autonomy

`config/autonomy-mode.ts` 清楚表明：

- workspace 不受信任
- agent 自动降级为 `Supervised`

这不是 UI 提示级别，而是能力模式级别的硬约束。

### 配置同步是 UI 控制的一部分

通过 `setting-context-sync.ts`：

- 配置变化立即转成 `kiroAgent.settings.*`
- command palette / menu / view 的 `when` 子句可直接消费

所以 `config/**` 本质上也属于 UI 运行时的一部分。

## 结论

`config/**` 是 extension 的配置与安全策略中枢，连接：

- VS Code settings
- workspace trust
- autonomy mode
- UI context keys
- workspace state

它在架构重要性上明显高于“普通配置目录”。
