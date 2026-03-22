# powers 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/powers/**` 注释边界去重

## 结论

- `powers/**` 当前共识别 `48` 个唯一模块块。
- 这是 extension 层最大的单一 feature 目录之一。
- 它实现的是 Kiro 的“插件生态层”，不是普通工具列表。

## 模块清单

```text
powers/api/add-custom-power/add-custom-power.ts
powers/api/add-custom-power/add-custom-power-by-folder.ts
powers/api/add-custom-power/add-custom-power-by-url.ts
powers/api/add-custom-power/user-added-registry.ts
powers/api/check-power-updates.ts
powers/api/configure-power.ts
powers/api/get-mcp-json.ts
powers/api/index.ts
powers/api/list-powers.ts
powers/api/sync-kiro-repo.ts
powers/api/try-power.ts
powers/api/ui-interaction.ts
powers/api/uninstall-power.ts
powers/api/update-power.ts
powers/constants.ts
powers/index.ts
powers/mcp/config-watcher.ts
powers/mcp/config-writer.ts
powers/powers-commands.ts
powers/powers-view-provider.ts
powers/registry-v2/api/get-power-details.ts
powers/registry-v2/api/install-power.ts
powers/registry-v2/api/uninstall-power.ts
powers/registry-v2/auto-install-powers.ts
powers/registry-v2/init.ts
powers/registry-v2/installed-powers-manager.ts
powers/registry-v2/kiro-recommended-cache.ts
powers/registry-v2/migration.ts
powers/registry-v2/notifications.ts
powers/registry-v2/paths.ts
powers/registry-v2/registry-resolver.ts
powers/registry-v2/registry-watcher.ts
powers/registry-v2/schema.ts
powers/registry-v2/types.ts
powers/registry-v2/utils.ts
powers/repos/github-utils.ts
powers/repos/installer.ts
powers/repos/scanner.ts
powers/repos/validator.ts
powers/tools/configure-powers.ts
powers/tools/list-powers.ts
powers/tools/read-power-steering.ts
powers/tools/use-power.ts
powers/utils/parse-power-server.ts
powers/utils/paths.ts
powers/utils/power-helpers.ts
powers/utils/security.ts
powers/utils/types.ts
```

## 分层

### 1. Registry V2

- `powers/registry-v2/*`
  - schema / types / paths / utils
  - install / uninstall / get-power-details
  - registry watcher / resolver / notifications
  - recommended cache / migration / auto-install

### 2. Repo 处理

- `powers/repos/github-utils.ts`
- `powers/repos/scanner.ts`
- `powers/repos/installer.ts`
- `powers/repos/validator.ts`

其中 `powers/repos/validator.ts` 达到 3075 行，是整个 extension 层最重的大块之一，说明 power repo 校验是高风险、高复杂度入口。

### 3. MCP 配置桥

- `powers/mcp/config-writer.ts`
- `powers/mcp/config-watcher.ts`

这两块把 powers 和 MCP 强绑定起来：power 安装不是只写本地元数据，还会影响 MCP 配置落盘与监听。

### 4. API 层

- `powers/api/*`
  - list / update / configure / uninstall / try-power
  - add custom power by folder / by url
  - sync repo / check updates / get mcp json / ui interaction

### 5. Tool 层

- `powers/tools/list-powers.ts`
- `powers/tools/read-power-steering.ts`
- `powers/tools/use-power.ts`
- `powers/tools/configure-powers.ts`

### 6. Root / UI

- `powers/index.ts`
- `powers/powers-commands.ts`
- `powers/powers-view-provider.ts`
- `powers/constants.ts`

### 7. 通用工具

- `powers/utils/*`
  - paths / types / helpers / security / parse-power-server

## 判断

从目录结构看，powers 至少具备：

- registry
- repo 发现与验证
- install / uninstall / update
- custom power
- MCP 配置桥
- steering 读取
- tool 暴露

这已经是平台化能力，而不是单个 feature。

## 结论

`powers/**` 是 extension 层最接近“插件生态系统”的目录，也是目前最值得继续深挖的二期大块之一。
