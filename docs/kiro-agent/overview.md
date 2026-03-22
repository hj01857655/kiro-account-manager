# kiro-agent 文档总览

> 整理日期：2026-03-18
> 目标：按分析对象拆分 `kiro.kiro-agent` 文档，区分宿主扩展层、agent runtime 层和资源层。

## 目录说明

- `architecture.md`
  - 整体架构说明。
- `extension/`
  - 对应 `dist/extension.js` 中的 `src/extension/**` 宿主扩展逻辑。
- `runtime/`
  - 对应 `packages/kiro-agent/dist/**` 和共享 runtime 相关模块。
- `assets/`
  - Webview、资源文件、模型、tree-sitter、测试与 node_modules 等非核心入口资产。

## 索引

### 宿主扩展层

- `extension/overview.md`
- `extension/activation.md`
- `extension/mcp.md`
- `extension/q-custom-model.md`
- `extension/rich-execution-log.md`
- `extension/modules-coverage.md`

### Agent Runtime 层

- `runtime/packages-kiro-agent-dist.md`
- `runtime/modules-status.md`
- `runtime/packages-kiro-shared.md`
- `runtime/packages-kiro-client.md`
- `runtime/acp-type-covenant.md`
- `runtime/shared-runtime-overview.md`

### 资源与附属模块

- `assets/autocomplete.md`
- `assets/bundled-webviews.md`
- `assets/context-providers.md`
- `assets/continuedev.md`
- `assets/extension-resources.md`
- `assets/kiricons.md`
- `assets/models.md`
- `assets/node-modules.md`
- `assets/tests.md`
- `assets/tree-sitter.md`
- `assets/webviews.md`

## 说明

- 后续若继续补 `dist/extension.js` 的穷尽式分析，优先更新 `extension/modules-coverage.md`。
- 若继续补 `packages/kiro-agent/dist/**` 的文件级分析，优先更新 `runtime/packages-kiro-agent-dist.md`，并修正其中的模块总数结论。
