# acp-dev-inspector 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/acp-dev-inspector/**` 注释边界去重

## 模块清单

```text
acp-dev-inspector/acp-message-tap.ts
acp-dev-inspector/acp-dev-inspector-provider.ts
acp-dev-inspector/index.ts
```

## 结构

- `acp-dev-inspector/acp-message-tap.ts`
- `acp-dev-inspector/acp-dev-inspector-provider.ts`
- `acp-dev-inspector/index.ts`

`acp-dev-inspector/acp-dev-inspector-provider.ts` 对应 `kiroAgent.views.acpInspector`，把 ACP 消息流转发到 webview，用于开发态检查 ACP 通道消息。

## 判断

- `acp-dev-inspector/**` 是纯开发调试面板，不属于正式用户功能主路径。
