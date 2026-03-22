# profiles 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/profiles/**` 注释边界去重

## 结论

- `profiles/**` 当前共识别 `5` 个唯一模块块。
- 这一层不负责“所有账号逻辑”，而是负责 profile 文件存在性、切换准备态和 usage/subscription 结果变换。

## 模块清单

```text
profiles/errors.ts
profiles/index.ts
profiles/profile-transformer.ts
profiles/services/profile-approval-service.ts
profiles/services/profile-file-watcher.ts
```

## 分层

### 1. 数据变换

- `profiles/profile-transformer.ts`
  - 校验 profile 基本字段：`arn`、`profileName`
  - 过滤掉缺少关键字段的 profile
  - 从 usage limits 结果中提取：
    - `currentUsage`
    - `usageLimit`
    - `subscriptionName`

关键判断：UI 或命令层拿到的 profile 数据，不是后端原样透传，而是先在这里被标准化。

### 2. Profile Ready 校验

- `profiles/services/profile-approval-service.ts`
  - `isEnterpriseProfileReady()`
  - 如果 token 不支持 profiles，直接视为 ready
  - 如果支持 profiles，则检查本地 profile 是否已写入存储

这说明 enterprise / profile 选择是扩展激活链路里的前置门槛之一。

### 3. 文件监听

- `profiles/services/profile-file-watcher.ts`
  - 300ms debounce
  - 监听 profile 文件 create / change
  - 文件变化后重新处理 profile 初始化或刷新逻辑

关键点：profile 变化不是只靠命令触发，Kiro 会监听磁盘层配置文件变化。

### 4. 错误层

- `profiles/errors.ts`
  - `ProfileError`
  - `ProfileFileWatcherError`
  - `InitializationRefreshError`

### 5. 注册入口

- `profiles/index.ts`
  - 启动 `ProfileFileWatcher`
  - 把 watcher 注册进 extension lifecycle

## 关系图

```text
profile file / token / usage limits
          │
          ├─ profile-transformer
          ├─ profile-approval-service
          └─ profile-file-watcher
                  │
                  ▼
             profiles/index.ts
```

## 结论

`profiles/**` 是 profile 生命周期的“边界层”：

- 把 profile 数据变成 UI 可消费结构
- 监听本地 profile 文件变化
- 为 enterprise/profile 选择流程提供 ready 判定

它体量不大，但位于激活和账号流的关键路径上。
