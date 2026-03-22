# storage 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/storage/**` 注释边界去重

## 结论

- `storage/**` 当前共识别 `5` 个唯一模块块。
- 它负责 extension 自身元数据存储，而不是 session JSONL 主存储。

## 模块清单

```text
storage/commands/debug-open-metadata.ts
storage/commands/debug-purge-metadata.ts
storage/errors.ts
storage/index.ts
storage/storage-controller.ts
```

## 分层

### 1. StorageManager

- `storage/storage-controller.ts`
  - 核心体量最大
  - 负责初始化全局/工作区存储位置
  - 通过 workspace path hash 派生 workspaceId
  - 确保目录存在
  - 带重试写入
  - 采用 temp file + rename 的原子写策略

关键点：Kiro 在 extension 元数据写入上做了相对严谨的 durability 处理，不是直接裸写。

### 2. 错误分类

- `storage/errors.ts`
  - 针对写入失败做错误分类
  - 可区分 disk full、permission 等场景

### 3. 调试命令

- `storage/commands/debug-open-metadata.ts`
  - 打开存储目录
- `storage/commands/debug-purge-metadata.ts`
  - 清空元数据

### 4. 注册入口

- `storage/index.ts`
  - 初始化 `Storage`
  - 绑定 debug 命令

## 判断

`storage/**` 主要是宿主扩展自己的元数据与调试持久化层。它和 `message-replay` 那类 runtime session 持久化不同，更多承担：

- extension state file 存取
- 元数据目录生命周期
- 调试清理能力

但从 temp-file rename 和重试逻辑看，它仍然是认真设计过的基础层。
