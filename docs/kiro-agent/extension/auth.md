# auth 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/auth/**` 注释边界去重

## 结论

- `auth/**` 当前只有 `1` 个模块块：
  - `auth/ide-auth-provider.ts`
- 虽然体量小，但位置关键：它是 extension 向 runtime/commands 提供 IDE token 的桥接层。

## 模块清单

```text
auth/ide-auth-provider.ts
```

## 职责

- 读取 IDE 侧当前 token
- 包装成 runtime 可消费的 auth provider
- 作为 extension 宿主向 agent core 注入认证能力的边界

## 判断

这层之所以单独存在，而不是把认证逻辑内嵌到 activation 里，说明 Kiro 仍然坚持：

- 认证由宿主扩展提供
- runtime 只消费抽象 provider

也就是说，`auth/**` 的体量小，不代表它不重要；它是职责明确的接缝模块。
