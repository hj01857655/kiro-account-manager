# 文档迁移映射表

> 整理日期：2026-03-18
> 用途：记录 `docs/` 目录重组前后的路径映射，降低后续合并与 cherry-pick 的冲突成本。

## kiro-agent

- `docs/kiro-agent-extension-overview.md` -> `docs/kiro-agent/extension/overview.md`
- `docs/kiro-agent-extension.md` -> `docs/kiro-agent/extension/activation.md`
- `docs/kiro-agent-mcp.md` -> `docs/kiro-agent/extension/mcp.md`
- `docs/kiro-agent-q-custom-model.md` -> `docs/kiro-agent/extension/q-custom-model.md`
- `docs/kiro-agent-rich-execution-log.md` -> `docs/kiro-agent/extension/rich-execution-log.md`
- `docs/kiro-agent-modules.md` -> `docs/kiro-agent/runtime/packages-kiro-agent-dist.md`
- `docs/kiro-agent-kiro-shared.md` -> `docs/kiro-agent/runtime/packages-kiro-shared.md`
- `docs/kiro-agent-kiro-client.md` -> `docs/kiro-agent/runtime/packages-kiro-client.md`
- `docs/kiro-agent-acp-type-covenant.md` -> `docs/kiro-agent/runtime/acp-type-covenant.md`
- `docs/kiro-shared-modules.md` -> `docs/kiro-agent/runtime/shared-runtime-overview.md`
- `docs/kiro-agent-extension-resources.md` -> `docs/kiro-agent/assets/extension-resources.md`
- `docs/kiro-agent-bundled-webviews.md` -> `docs/kiro-agent/assets/bundled-webviews.md`
- `docs/kiro-agent-webviews.md` -> `docs/kiro-agent/assets/webviews.md`
- `docs/kiro-agent-models.md` -> `docs/kiro-agent/assets/models.md`
- `docs/kiro-agent-tree-sitter.md` -> `docs/kiro-agent/assets/tree-sitter.md`
- `docs/kiro-agent-kiricons.md` -> `docs/kiro-agent/assets/kiricons.md`
- `docs/kiro-agent-tests.md` -> `docs/kiro-agent/assets/tests.md`
- `docs/kiro-agent-node-modules.md` -> `docs/kiro-agent/assets/node-modules.md`
- `docs/kiro-agent-autocomplete.md` -> `docs/kiro-agent/assets/autocomplete.md`
- `docs/kiro-agent-context-providers.md` -> `docs/kiro-agent/assets/context-providers.md`
- `docs/kiro-agent-continuedev.md` -> `docs/kiro-agent/assets/continuedev.md`
- `docs/kiro-agent-architecture.md` -> `docs/kiro-agent/architecture.md`

## 其他

- `docs/kiro-app-structure.md` -> `docs/kiro-app/structure.md`
- `docs/kiro-app-node-modules.md` -> `docs/kiro-app/node-modules.md`
- `docs/kiro-ide/kiro-ide-overview.md` -> `docs/kiro-ide/overview.md`

## 新增文档

以下文件为本次整理新增，没有旧路径对应：

- `docs/kiro-agent/overview.md`
- `docs/kiro-agent/extension/modules-coverage.md`
- `docs/kiro-agent/doc-migration-map.md`

## 合并建议

- 如果另一条分支还在修改旧路径文档，优先按本映射表把变更手工迁移到新路径，再处理内容冲突。
- 目录重组后的后续分析，统一写入新路径，避免旧路径“复活”。
