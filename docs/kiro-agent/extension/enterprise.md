# enterprise 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/enterprise/**` 注释边界去重

## 结论

- `enterprise/**` 当前共识别 `4` 个唯一模块块。
- 主要负责企业侧 registry / settings / HTTP 拉取链路。

## 模块清单

```text
enterprise/enterprise-settings-manager.ts
enterprise/index.ts
enterprise/mcp-registry-loader.ts
enterprise/registry-http-fetcher.ts
```

## 分层

- `enterprise-settings-manager.ts`
  - 企业设置集中管理
- `mcp-registry-loader.ts`
  - 企业 MCP registry 加载
- `registry-http-fetcher.ts`
  - 远程 registry HTTP 拉取
- `index.ts`
  - 注册入口

## 判断

`enterprise/**` 是企业分发配置与 registry 的接入层。
