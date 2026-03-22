# model-selection 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/model-selection/**` 注释边界去重

## 结论

- `model-selection/**` 当前共识别 `3` 个唯一模块块。
- 它负责 IDE 侧“当前选中模型”的缓存、配置和 provider 注入。

## 模块清单

```text
model-selection/index.ts
model-selection/model-cache.ts
model-selection/model-configuration.ts
```

## 分层

- `model-cache.ts`
  - 保存当前可用模型列表与默认模型
  - 刷新后触发 `kiro.updateModelsList`
- `model-configuration.ts`
  - 读取/写入 `kiroAgent.modelSelection`
  - `VSCodeModelConfigProvider`
  - 负责格式化 `provider::modelId`
  - 负责把选中模型注入 runtime
- `index.ts`
  - 注册模型相关命令

## 判断

`model-selection/**` 是宿主设置和 runtime model config 之间的桥接层。
