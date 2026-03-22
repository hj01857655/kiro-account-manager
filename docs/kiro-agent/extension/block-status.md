# extension 逐块审计台账

> 统计来源：`dist/extension.js` 中 `// src/extension/...` 注释边界去重
> 审计日期：2026-03-18
> 判定规则：
> - `covered`：模块路径在某个专题文档中出现
> - `partial`：模块路径只在 `modules-coverage.md` / `overview.md` / `final-status.md` 这类索引文档出现
> - `missing`：模块路径在 extension 文档中完全未出现

## 汇总

- 总模块块：`474`
- `covered`：`474`
- `partial`：`0`
- `missing`：`0`

## 按一级目录汇总

- `acp-dev-inspector`: total `3`, covered `3`, partial `0`, missing `0`
- `(root)`: total `1`, covered `1`, partial `0`, missing `0`
- `checkpoints`: total `5`, covered `5`, partial `0`, missing `0`
- `storage`: total `5`, covered `5`, partial `0`, missing `0`
- `utils`: total `19`, covered `19`, partial `0`, missing `0`
- `capabilities`: total `47`, covered `47`, partial `0`, missing `0`
- `contextual-spec`: total `4`, covered `4`, partial `0`, missing `0`
- `commands`: total `38`, covered `38`, partial `0`, missing `0`
- `telemetry`: total `4`, covered `4`, partial `0`, missing `0`
- `config`: total `10`, covered `10`, partial `0`, missing `0`
- `experiments`: total `7`, covered `7`, partial `0`, missing `0`
- `onboarding`: total `10`, covered `10`, partial `0`, missing `0`
- `editor`: total `31`, covered `31`, partial `0`, missing `0`
- `q-custom-model`: total `1`, covered `1`, partial `0`, missing `0`
- `usage`: total `3`, covered `3`, partial `0`, missing `0`
- `profiles`: total `5`, covered `5`, partial `0`, missing `0`
- `hooks`: total `39`, covered `39`, partial `0`, missing `0`
- `spec-editor`: total `42`, covered `42`, partial `0`, missing `0`
- `rich-execution-log`: total `36`, covered `36`, partial `0`, missing `0`
- `powers`: total `48`, covered `48`, partial `0`, missing `0`
- `agent-chat`: total `16`, covered `16`, partial `0`, missing `0`
- `auth`: total `1`, covered `1`, partial `0`, missing `0`
- `terminal`: total `10`, covered `10`, partial `0`, missing `0`
- `steering`: total `16`, covered `16`, partial `0`, missing `0`
- `context-resolvers`: total `9`, covered `9`, partial `0`, missing `0`
- `first-time-project`: total `3`, covered `3`, partial `0`, missing `0`
- `import-steering`: total `12`, covered `12`, partial `0`, missing `0`
- `platform`: total `1`, covered `1`, partial `0`, missing `0`
- `background-processes`: total `3`, covered `3`, partial `0`, missing `0`
- `context-lsp`: total `11`, covered `11`, partial `0`, missing `0`
- `custom-agent-loader`: total `4`, covered `4`, partial `0`, missing `0`
- `enterprise`: total `4`, covered `4`, partial `0`, missing `0`
- `mcp`: total `7`, covered `7`, partial `0`, missing `0`
- `repos`: total `2`, covered `2`, partial `0`, missing `0`
- `session-resume`: total `4`, covered `4`, partial `0`, missing `0`
- `status-bar`: total `2`, covered `2`, partial `0`, missing `0`
- `model-selection`: total `3`, covered `3`, partial `0`, missing `0`
- `notifications`: total `5`, covered `5`, partial `0`, missing `0`
- `polling`: total `1`, covered `1`, partial `0`, missing `0`
- `context-usage`: total `2`, covered `2`, partial `0`, missing `0`

## Missing 清单


## Partial 清单


## 全量台账

| module | feature | status | doc |
|---|---|---|---|
| `acp-dev-inspector/acp-message-tap.ts` | `acp-dev-inspector` | `covered` | acp-dev-inspector.md |
| `acp-dev-inspector/acp-dev-inspector-provider.ts` | `acp-dev-inspector` | `covered` | acp-dev-inspector.md |
| `acp-dev-inspector/index.ts` | `acp-dev-inspector` | `covered` | acp-dev-inspector.md |
| `index.ts` | `(root)` | `covered` | acp-dev-inspector.md, activation.md, agent-chat.md, background-processes.md, checkpoints.md, commands.md, config.md, context-lsp.md, context-usage.md, editor.md, enterprise.md, first-time-project.md, hooks.md, import-steering.md, mcp.md, model-selection.md, notifications.md, onboarding.md, powers.md, profiles.md, rich-execution-log.md, session-resume.md, spec-editor.md, steering.md, storage.md, terminal.md |
| `checkpoints/index.ts` | `checkpoints` | `covered` | checkpoints.md |
| `storage/index.ts` | `storage` | `covered` | storage.md |
| `storage/storage-controller.ts` | `storage` | `covered` | storage.md |
| `storage/errors.ts` | `storage` | `covered` | storage.md |
| `utils/directory-sizer.ts` | `utils` | `covered` | utils.md |
| `storage/commands/debug-open-metadata.ts` | `storage` | `covered` | storage.md |
| `storage/commands/debug-purge-metadata.ts` | `storage` | `covered` | storage.md |
| `checkpoints/fs-checkpoint-controller.ts` | `checkpoints` | `covered` | checkpoints.md |
| `checkpoints/errors.ts` | `checkpoints` | `covered` | checkpoints.md |
| `utils/file/file-utils.ts` | `utils` | `covered` | utils.md |
| `capabilities/errors.ts` | `capabilities` | `covered` | capabilities.md |
| `checkpoints/checkpoint-file-system-provider.ts` | `checkpoints` | `covered` | checkpoints.md |
| `checkpoints/meta-file-system-provider.ts` | `checkpoints` | `covered` | checkpoints.md |
| `contextual-spec/command.ts` | `contextual-spec` | `covered` | contextual-spec.md |
| `contextual-spec/invoke-spec-agent.ts` | `contextual-spec` | `covered` | contextual-spec.md |
| `contextual-spec/generate-initial-spec.ts` | `contextual-spec` | `covered` | contextual-spec.md |
| `commands/agent/chat-agent-command.ts` | `commands` | `covered` | commands.md |
| `commands/agent/utils.ts` | `commands` | `covered` | commands.md |
| `telemetry/command.ts` | `telemetry` | `covered` | telemetry.md |
| `config/workspace-trust-warning.ts` | `config` | `covered` | config.md |
| `utils/get-session-details.ts` | `utils` | `covered` | utils.md |
| `utils/get-execution-logger.ts` | `utils` | `covered` | utils.md |
| `experiments/experiments-activation.ts` | `experiments` | `covered` | experiments.md |
| `experiments/experiments-config-provider.ts` | `experiments` | `covered` | experiments.md |
| `experiments/is-experiment-visible.ts` | `experiments` | `covered` | experiments.md |
| `experiments/experiments.ts` | `experiments` | `covered` | experiments.md |
| `experiments/experiments-service.ts` | `experiments` | `covered` | experiments.md |
| `experiments/experiments-telemetry.ts` | `experiments` | `covered` | experiments.md |
| `experiments/experiments-status-bar.ts` | `experiments` | `covered` | experiments.md |
| `utils/trigger-agent.ts` | `utils` | `covered` | utils.md |
| `utils/get-agent-controller.ts` | `utils` | `covered` | utils.md |
| `commands/agent/retry-agent-command.ts` | `commands` | `covered` | commands.md |
| `commands/agent/compact-agent-command.ts` | `commands` | `covered` | commands.md |
| `commands/agent/index.ts` | `commands` | `covered` | commands.md |
| `commands/configuration/index.ts` | `commands` | `covered` | commands.md |
| `onboarding/commands.ts` | `onboarding` | `covered` | onboarding.md |
| `onboarding/onboarding-service.ts` | `onboarding` | `covered` | onboarding.md |
| `onboarding/steps/alias-code.ts` | `onboarding` | `covered` | onboarding.md |
| `editor/editor-api/set-cli-alias.ts` | `editor` | `covered` | editor.md |
| `onboarding/steps/cli-command.ts` | `onboarding` | `covered` | onboarding.md |
| `utils/spawn.ts` | `utils` | `covered` | utils.md |
| `config/package-manager.ts` | `config` | `covered` | config.md |
| `onboarding/steps/cli-integration-setup.ts` | `onboarding` | `covered` | onboarding.md |
| `onboarding/errors.ts` | `onboarding` | `covered` | onboarding.md |
| `editor/editor-api/import-settings.ts` | `editor` | `covered` | editor.md |
| `editor/errors.ts` | `editor` | `covered` | editor.md, spec-editor.md |
| `utils/with-abort.ts` | `utils` | `covered` | utils.md |
| `onboarding/steps/import-settings.ts` | `onboarding` | `covered` | onboarding.md |
| `onboarding/steps/pin-to-dock.ts` | `onboarding` | `covered` | onboarding.md |
| `onboarding/steps/install-extensions.ts` | `onboarding` | `covered` | onboarding.md |
| `onboarding/index.ts` | `onboarding` | `covered` | onboarding.md |
| `commands/debug-capture-log.ts` | `commands` | `covered` | commands.md |
| `q-custom-model/q-developer-converse-factory.ts` | `q-custom-model` | `covered` | activation.md |
| `usage/usage-limits-availability-cache.ts` | `usage` | `covered` | usage.md |
| `commands/record-references.ts` | `commands` | `covered` | commands.md |
| `commands/utils/kiro-version.ts` | `commands` | `covered` | commands.md |
| `commands/debug-capture-llm-log.ts` | `commands` | `covered` | commands.md |
| `commands/delete-account.ts` | `commands` | `covered` | commands.md |
| `commands/enable-shell-integration.ts` | `commands` | `covered` | commands.md |
| `commands/file-feedback.ts` | `commands` | `covered` | commands.md |
| `commands/create-debug-log-zip.ts` | `commands` | `covered` | commands.md |
| `commands/open-execution-logs.ts` | `commands` | `covered` | commands.md |
| `commands/source-control/generate-commit-message.ts` | `commands` | `covered` | commands.md |
| `commands/source-control/errors.ts` | `commands` | `covered` | commands.md |
| `commands/source-control/index.ts` | `commands` | `covered` | commands.md |
| `commands/usage-limits/enable-overages.ts` | `commands` | `covered` | commands.md |
| `commands/subscription-plans/get-portal-session-url.ts` | `commands` | `covered` | commands.md |
| `commands/usage-limits/get-usage-limits.ts` | `commands` | `covered` | commands.md |
| `commands/utils/get-currency.ts` | `commands` | `covered` | commands.md |
| `commands/utils/extract-subscription-name.ts` | `commands` | `covered` | commands.md |
| `usage/usage-limits-event-emitter.ts` | `usage` | `covered` | usage.md |
| `commands/subscription-plans/get-subscription-plans.ts` | `commands` | `covered` | commands.md |
| `commands/subscription-plans/get-checkout-session-url.ts` | `commands` | `covered` | commands.md |
| `commands/list-models/get-available-models.ts` | `commands` | `covered` | commands.md |
| `commands/get-codewhisperer-config.ts` | `commands` | `covered` | commands.md |
| `commands/profiles/list-available-profiles.ts` | `commands` | `covered` | commands.md |
| `profiles/profile-transformer.ts` | `profiles` | `covered` | profiles.md |
| `commands/profiles/select-profile.ts` | `commands` | `covered` | commands.md |
| `commands/profiles/get-profile.ts` | `commands` | `covered` | commands.md |
| `commands/profiles/index.ts` | `commands` | `covered` | commands.md |
| `commands/telemetry/can-enable-telemetry.ts` | `commands` | `covered` | commands.md |
| `commands/refresh-remote-tools.ts` | `commands` | `covered` | commands.md |
| `capabilities/tool-factories/remote.ts` | `capabilities` | `covered` | capabilities.md |
| `utils/trusted-tools.ts` | `utils` | `covered` | utils.md |
| `editor/editor-api/get-settings.ts` | `editor` | `covered` | editor.md |
| `commands/supervised-diff/index.ts` | `commands` | `covered` | commands.md |
| `commands/cancel-hook-command.ts` | `commands` | `covered` | commands.md |
| `hooks/contextual/hook-operation-registry.ts` | `hooks` | `covered` | hooks.md |
| `commands/index.ts` | `commands` | `covered` | commands.md, spec-editor.md |
| `utils/scoped-workspace-resource-collection.ts` | `utils` | `covered` | utils.md |
| `config/workspace-config.ts` | `config` | `covered` | config.md |
| `config/setting-context-sync.ts` | `config` | `covered` | config.md |
| `config/config-keys.ts` | `config` | `covered` | config.md |
| `config/workspace-state.ts` | `config` | `covered` | config.md |
| `config/index.ts` | `config` | `covered` | config.md |
| `editor/editor-api/index.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/apply-to-code.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/delete-hook.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/get-full-spec-document.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/get-text-document-content.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/load-context-provider-items.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/read-hook.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/regenerate-spec-document.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/save-hook.ts` | `editor` | `covered` | editor.md |
| `hooks/contextual/hook-command-approval-storage.ts` | `hooks` | `covered` | hooks.md |
| `editor/editor-api/set-hook-enabled.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/show-message.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/spec-actions.ts` | `editor` | `covered` | editor.md |
| `contextual-spec/prompts/examples/project-spec-example.ts` | `contextual-spec` | `covered` | contextual-spec.md |
| `spec-editor/utils.ts` | `spec-editor` | `covered` | spec-editor.md |
| `hooks/index.ts` | `hooks` | `covered` | editor.md, hooks.md |
| `hooks/hook-controller.ts` | `hooks` | `covered` | hooks.md |
| `hooks/hook-storage.ts` | `hooks` | `covered` | hooks.md |
| `hooks/types.ts` | `hooks` | `covered` | hooks.md |
| `hooks/hook-validation.ts` | `hooks` | `covered` | hooks.md |
| `hooks/views/hooks-treeview.ts` | `hooks` | `covered` | hooks.md |
| `hooks/views/get-hook-tree-item.ts` | `hooks` | `covered` | hooks.md |
| `hooks/views/hook-tree-item.ts` | `hooks` | `covered` | hooks.md |
| `hooks/errors.ts` | `hooks` | `covered` | hooks.md |
| `hooks/hook-singleton.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/create-hook.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/delete-hook.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/delete-hook-context-menu.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/enable-hook.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/read-hook.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/trigger-hook.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/sync-hook-running-state.ts` | `hooks` | `covered` | hooks.md |
| `hooks/commands/list-hooks.ts` | `hooks` | `covered` | hooks.md |
| `hooks/listeners/when-file-created.ts` | `hooks` | `covered` | hooks.md |
| `hooks/listeners/hook-listener.ts` | `hooks` | `covered` | hooks.md |
| `hooks/listeners/when-file-deleted.ts` | `hooks` | `covered` | hooks.md |
| `hooks/listeners/when-file-edit.ts` | `hooks` | `covered` | hooks.md |
| `hooks/listeners/index.ts` | `hooks` | `covered` | hooks.md |
| `hooks/triggers/then-ask-agent.ts` | `hooks` | `covered` | hooks.md |
| `hooks/triggers/hook-trigger.ts` | `hooks` | `covered` | hooks.md |
| `hooks/triggers/then-ask-agent.prompt.ts` | `hooks` | `covered` | hooks.md |
| `hooks/triggers/then-run-command.ts` | `hooks` | `covered` | hooks.md |
| `utils/get-execution-log-controller.ts` | `utils` | `covered` | utils.md |
| `utils/get-operation-id.ts` | `utils` | `covered` | utils.md |
| `rich-execution-log/controller/type-mapper.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/types-actions.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/errors.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `hooks/utils/spawn-hook-command.ts` | `hooks` | `covered` | hooks.md |
| `hooks/utils/request-command-approval.ts` | `hooks` | `covered` | hooks.md |
| `hooks/triggers/index.ts` | `hooks` | `covered` | hooks.md |
| `editor/editor-api/trigger-hook.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/update-spec-document.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/update-text-document.ts` | `editor` | `covered` | editor.md |
| `editor/update-hook-page.ts` | `editor` | `covered` | editor.md |
| `editor/sync-hook-running-state.ts` | `editor` | `covered` | editor.md |
| `editor/hooks/open-active-hook-execution.ts` | `editor` | `covered` | editor.md |
| `editor/editor-api/analyze-requirements.ts` | `editor` | `covered` | editor.md |
| `spec-editor/reasoning/requirements-analyzer.ts` | `spec-editor` | `covered` | spec-editor.md |
| `powers/registry-v2/schema.ts` | `powers` | `covered` | powers.md |
| `powers/utils/types.ts` | `powers` | `covered` | powers.md |
| `powers/utils/paths.ts` | `powers` | `covered` | powers.md |
| `powers/utils/power-helpers.ts` | `powers` | `covered` | powers.md |
| `powers/utils/security.ts` | `powers` | `covered` | powers.md |
| `powers/mcp/config-writer.ts` | `powers` | `covered` | powers.md |
| `powers/mcp/config-watcher.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/installed-powers-manager.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/types.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/paths.ts` | `powers` | `covered` | powers.md |
| `powers/tools/list-powers.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/init.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/registry-watcher.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/auto-install-powers.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/registry-resolver.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/utils.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/api/install-power.ts` | `powers` | `covered` | powers.md |
| `powers/repos/github-utils.ts` | `powers` | `covered` | powers.md |
| `powers/api/ui-interaction.ts` | `powers` | `covered` | powers.md |
| `agent-chat/index.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/agent-chat-view-provider.ts` | `agent-chat` | `covered` | agent-chat.md |
| `editor/custom-webview-provider.ts` | `editor` | `covered` | editor.md |
| `editor/base-custom-webview-provider.ts` | `editor` | `covered` | editor.md |
| `config/application-config.ts` | `config` | `covered` | config.md |
| `agent-chat/constants.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/resolve-permission-request.ts` | `agent-chat` | `covered` | agent-chat.md |
| `auth/ide-auth-provider.ts` | `auth` | `covered` | auth.md |
| `agent-chat/session-manager.ts` | `agent-chat` | `covered` | agent-chat.md |
| `capabilities/fs/delete.ts` | `capabilities` | `covered` | capabilities.md |
| `utils/validate-file-access.ts` | `utils` | `covered` | utils.md |
| `capabilities/fs/read-directory.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/utils/convert-file-type.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/fs/read-file.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/fs/stat.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/fs/write-file.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/fs/read-text-file.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/fs/write-text-file.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/terminal/create.ts` | `capabilities` | `covered` | capabilities.md |
| `terminal/index.ts` | `terminal` | `covered` | terminal.md |
| `terminal/shell-detection.ts` | `terminal` | `covered` | terminal.md |
| `terminal/basic/terminal.ts` | `terminal` | `covered` | terminal.md |
| `terminal/terminal-errors.ts` | `terminal` | `covered` | terminal.md |
| `terminal/utils/strip-ansi.ts` | `terminal` | `covered` | terminal.md |
| `terminal/utils/output-sanitizer.ts` | `terminal` | `covered` | terminal.md |
| `terminal/basic/interactive-prompt-patterns.ts` | `terminal` | `covered` | terminal.md |
| `terminal/basic/terminal-manager.ts` | `terminal` | `covered` | terminal.md |
| `terminal/adapters/background-process-adapter.ts` | `terminal` | `covered` | terminal.md |
| `terminal/adapters/terminal-manager-adapter.ts` | `terminal` | `covered` | terminal.md |
| `capabilities/terminal/kill.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/terminal/release.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/terminal/output.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/terminal/wait-for-exit.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/search/find-files.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/search/text-search.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/client-tools.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/get-diagnostics.ts` | `capabilities` | `covered` | capabilities.md |
| `utils/find-similar-file-name.ts` | `utils` | `covered` | utils.md |
| `capabilities/tools/semantic-rename.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/errors.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/smart-relocate.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/read-code.ts` | `capabilities` | `covered` | capabilities.md, utils.md |
| `utils/shared-parser.ts` | `utils` | `covered` | utils.md |
| `capabilities/tools/edit-code.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/create-hook.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/disclose-context.ts` | `capabilities` | `covered` | capabilities.md |
| `steering/steering-controller.ts` | `steering` | `covered` | steering.md |
| `utils/product-config-directory.ts` | `utils` | `covered` | utils.md |
| `steering/types.ts` | `steering` | `covered` | import-steering.md, steering.md |
| `steering/file-utils.ts` | `steering` | `covered` | steering.md |
| `steering/progressive-context-registry.ts` | `steering` | `covered` | steering.md |
| `capabilities/tools/kiro-powers.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/workspace/active-file.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/workspace/currently-open-files.ts` | `capabilities` | `covered` | capabilities.md |
| `context-resolvers/currently-open-files.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `context-resolvers/folder.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `context-resolvers/utils/walk-workspace-tree.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `context-resolvers/utils/walk-file-tree.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `context-resolvers/file.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `context-resolvers/mcp.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `context-resolvers/steering.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `agent-chat/create-file-system-watchers.ts` | `agent-chat` | `covered` | agent-chat.md |
| `context-resolvers/utils/to-cache-item.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `agent-chat/config-sync.ts` | `agent-chat` | `covered` | agent-chat.md |
| `context-resolvers/spec.ts` | `context-resolvers` | `covered` | context-resolvers.md |
| `spec-editor/documents/spec-document-manager.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/spec-config-manager.ts` | `spec-editor` | `covered` | spec-editor.md |
| `powers/registry-v2/notifications.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/kiro-recommended-cache.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/migration.ts` | `powers` | `covered` | powers.md |
| `powers/tools/read-power-steering.ts` | `powers` | `covered` | powers.md |
| `powers/tools/use-power.ts` | `powers` | `covered` | powers.md |
| `powers/tools/configure-powers.ts` | `powers` | `covered` | powers.md |
| `powers/repos/validator.ts` | `powers` | `covered` | powers.md |
| `powers/repos/scanner.ts` | `powers` | `covered` | powers.md |
| `powers/repos/installer.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/api/uninstall-power.ts` | `powers` | `covered` | powers.md |
| `powers/api/list-powers.ts` | `powers` | `covered` | powers.md |
| `powers/registry-v2/api/get-power-details.ts` | `powers` | `covered` | activation.md, powers.md |
| `powers/api/get-mcp-json.ts` | `powers` | `covered` | powers.md |
| `powers/api/update-power.ts` | `powers` | `covered` | powers.md |
| `powers/api/configure-power.ts` | `powers` | `covered` | powers.md |
| `powers/api/uninstall-power.ts` | `powers` | `covered` | powers.md |
| `powers/api/add-custom-power/add-custom-power.ts` | `powers` | `covered` | powers.md |
| `powers/api/add-custom-power/add-custom-power-by-folder.ts` | `powers` | `covered` | powers.md |
| `powers/api/add-custom-power/user-added-registry.ts` | `powers` | `covered` | powers.md |
| `powers/api/add-custom-power/add-custom-power-by-url.ts` | `powers` | `covered` | powers.md |
| `powers/api/check-power-updates.ts` | `powers` | `covered` | powers.md |
| `powers/api/sync-kiro-repo.ts` | `powers` | `covered` | powers.md |
| `powers/api/try-power.ts` | `powers` | `covered` | powers.md |
| `powers/api/index.ts` | `powers` | `covered` | powers.md |
| `agent-chat/api/get-context-items.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/get-context-providers.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/list-sessions.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/load-session.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/new-session.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/open-text-document.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/prompt.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/set-session-config-option.ts` | `agent-chat` | `covered` | agent-chat.md |
| `agent-chat/api/index.ts` | `agent-chat` | `covered` | agent-chat.md |
| `commands/agent/create-hook.ts` | `commands` | `covered` | commands.md |
| `editor/editor-api/save-new-hook.ts` | `editor` | `covered` | editor.md |
| `spec-editor/utils/spec-session-tracking.ts` | `spec-editor` | `covered` | spec-editor.md |
| `editor/hooks/index.ts` | `editor` | `covered` | editor.md |
| `editor/hooks/hooks-ui-provider.ts` | `editor` | `covered` | editor.md |
| `editor/requirements-viewer/index.ts` | `editor` | `covered` | editor.md |
| `editor/requirements-viewer/requirements-viewer-provider.ts` | `editor` | `covered` | editor.md |
| `editor/custom-editor-provider.ts` | `editor` | `covered` | editor.md |
| `first-time-project/first-time-project-controller.ts` | `first-time-project` | `covered` | first-time-project.md |
| `first-time-project/first-time-project-detection-service.ts` | `first-time-project` | `covered` | first-time-project.md |
| `import-steering/ai-assistant-import-service.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/generic-steering-scanner.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/types.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/parsers/cursor-types.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/parsers/cursor-parser.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/parsers/parser-utils.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/ai-assistant-configs.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/generic-steering-parser.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/generic-steering-file-generator.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/util.ts` | `import-steering` | `covered` | import-steering.md |
| `first-time-project/index.ts` | `first-time-project` | `covered` | first-time-project.md |
| `import-steering/commands/import-all-steering.ts` | `import-steering` | `covered` | import-steering.md |
| `import-steering/index.ts` | `import-steering` | `covered` | import-steering.md |
| `rich-execution-log/index.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/agent-activity-publisher/index.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/agent-activity-publisher/agent-activity-publisher.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/accept-diff.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/accept-user-response.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/get-pending-questions.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/add-to-execution.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/cancel-active-execution.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/queue-user-message.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/consume-queue-now.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/clear-queued-message.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/diff-commands.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/pending-changes-adapter.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `capabilities/bindings/checkpoint-file.ts` | `capabilities` | `covered` | capabilities.md |
| `rich-execution-log/session-file-snapshots.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/session-persistence-integration.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `capabilities/bindings/resolve-workspace-uri.ts` | `capabilities` | `covered` | capabilities.md |
| `rich-execution-log/commands/supervised-diff-sync.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/utils/uri-matching.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/extract-last-execution-paths.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/get-active-execution.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/get-execution-by-id.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/get-execution-history.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/get-queued-executions.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/ui-control.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/commands/subagent-continuation.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/controller/agent-log-connection.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/controller/execution-log-connection.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/controller/execution-log-connection-to-webview.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/diff/agent-activity-subscription.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/diff/diff-controller.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/diff/empty-text-document-provider.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `capabilities/create-workspace-connection.ts` | `capabilities` | `covered` | capabilities.md |
| `platform/vscode-file-system.ts` | `platform` | `covered` | activation.md, platform.md |
| `background-processes/index.ts` | `background-processes` | `covered` | background-processes.md |
| `background-processes/background-process-terminal-manager.ts` | `background-processes` | `covered` | background-processes.md |
| `background-processes/background-process-manager.ts` | `background-processes` | `covered` | background-processes.md |
| `capabilities/workspace-providers.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/bindings/read-file.ts` | `capabilities` | `covered` | capabilities.md |
| `context-lsp/providers/registry.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/providers/continue/continue-adapter.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/errors.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/parser/syntax.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/completion/completion-provider.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/hover/hover-provider.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/utils/references.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/definition/document-link-provider.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/index.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/highlighting/semantic-provider.ts` | `context-lsp` | `covered` | context-lsp.md |
| `context-lsp/providers/continue/file-provider.ts` | `context-lsp` | `covered` | context-lsp.md |
| `utils/validate-file-ignore.ts` | `utils` | `covered` | utils.md |
| `capabilities/bindings/write-file.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/bindings/errors.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/bindings/update-task-status.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/bindings/validate-task-status-change.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/bindings/get-task-metadata.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/bindings/update-pbt-status.ts` | `capabilities` | `covered` | capabilities.md |
| `spec-editor/errors.ts` | `spec-editor` | `covered` | spec-editor.md |
| `capabilities/bindings/as-relative-path.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tool-registry.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tools/mcp-wrapper.ts` | `capabilities` | `covered` | capabilities.md |
| `custom-agent-loader/custom-agent-registry-initializer.ts` | `custom-agent-loader` | `covered` | custom-agent-loader.md |
| `capabilities/tool-factories/fs.ts` | `capabilities` | `covered` | capabilities.md |
| `utils/file/list-directory-recursive.ts` | `utils` | `covered` | utils.md |
| `capabilities/utils/protected-path-checker.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tool-factories/shell.ts` | `capabilities` | `covered` | capabilities.md |
| `capabilities/tool-factories/search.ts` | `capabilities` | `covered` | capabilities.md |
| `enterprise/index.ts` | `enterprise` | `covered` | enterprise.md |
| `enterprise/enterprise-settings-manager.ts` | `enterprise` | `covered` | enterprise.md |
| `mcp/mcp-availability.ts` | `mcp` | `covered` | mcp.md |
| `enterprise/mcp-registry-loader.ts` | `enterprise` | `covered` | enterprise.md |
| `enterprise/registry-http-fetcher.ts` | `enterprise` | `covered` | enterprise.md |
| `rich-execution-log/session-snapshot-file-system-provider.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `spec-editor/spec-file-system-provider.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/storage.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/execute-task.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/utils/spec-document-utils.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/navigate-documents.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/explorer-create-spec.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/explorer-delete-spec.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/explorer-rename-spec.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/agent-task-connection.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/generate-spec-document-command.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/chat-to-fix-pbt.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/pbt-hover-utils.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/spec-trace-database.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/utils/pbt-parser.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/utils/properties-parser.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/markdown-parser.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/requirements-parser.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/spec-trace-database-manager.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/run-all-tasks.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/utils/abort-pending-executions.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/commands/index.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/markdown-task-codelens-provider.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/pbt-hover-provider.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/property-hover-provider.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/requirements-hover-provider.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/spec-explorer-view.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/auto-open-spec-artifacts.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/telemetry/event-filters.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/index.ts` | `spec-editor` | `covered` | spec-editor.md |
| `mcp/index.ts` | `mcp` | `covered` | mcp.md |
| `mcp/mcp-tree-data-provider.ts` | `mcp` | `covered` | mcp.md |
| `powers/utils/parse-power-server.ts` | `powers` | `covered` | powers.md |
| `mcp/registry-mode-utils.ts` | `mcp` | `covered` | mcp.md |
| `mcp/mcp-utility-commands.ts` | `mcp` | `covered` | mcp.md |
| `mcp/registry-picker-command.ts` | `mcp` | `covered` | mcp.md |
| `mcp/registry-sync-controller.ts` | `mcp` | `covered` | mcp.md |
| `powers/index.ts` | `powers` | `covered` | powers.md |
| `powers/powers-commands.ts` | `powers` | `covered` | powers.md |
| `powers/powers-view-provider.ts` | `powers` | `covered` | powers.md |
| `powers/constants.ts` | `powers` | `covered` | powers.md |
| `repos/repo-uri-handler.ts` | `repos` | `covered` | activation.md |
| `repos/index.ts` | `repos` | `covered` | activation.md |
| `session-resume/resume-session-uri-handler.ts` | `session-resume` | `covered` | session-resume.md |
| `session-resume/session-zip-extractor.ts` | `session-resume` | `covered` | session-resume.md |
| `session-resume/errors.ts` | `session-resume` | `covered` | session-resume.md |
| `session-resume/index.ts` | `session-resume` | `covered` | session-resume.md |
| `status-bar/status-bar-feedback-item.ts` | `status-bar` | `covered` | status-bar.md |
| `status-bar/status-bar-usage-meter-item.ts` | `status-bar` | `covered` | status-bar.md |
| `steering/index.ts` | `steering` | `covered` | import-steering.md, steering.md |
| `steering/commands/create-initial-steering.ts` | `steering` | `covered` | steering.md |
| `steering/commands/create-steering-and-skills.ts` | `steering` | `covered` | steering.md |
| `steering/commands/import-skills.ts` | `steering` | `covered` | steering.md |
| `steering/commands/create-default-steering.ts` | `steering` | `covered` | steering.md |
| `steering/commands/delete-steering.ts` | `steering` | `covered` | steering.md |
| `steering/commands/delete-skill.ts` | `steering` | `covered` | steering.md |
| `steering/commands/get-steerings.ts` | `steering` | `covered` | steering.md |
| `steering/commands/refine-steering.ts` | `steering` | `covered` | steering.md |
| `steering/views/steering-treeview.ts` | `steering` | `covered` | steering.md |
| `steering/progressive-context-loader.ts` | `steering` | `covered` | steering.md |
| `steering/commands/get-skills.ts` | `steering` | `covered` | steering.md |
| `config/autonomy-mode.ts` | `config` | `covered` | config.md |
| `usage/usage-monitoring.ts` | `usage` | `covered` | usage.md |
| `profiles/services/profile-approval-service.ts` | `profiles` | `covered` | profiles.md |
| `custom-agent-loader/custom-agent-loader.ts` | `custom-agent-loader` | `covered` | custom-agent-loader.md |
| `custom-agent-loader/prompt-file-resolver.ts` | `custom-agent-loader` | `covered` | custom-agent-loader.md |
| `custom-agent-loader/commands/list-custom-agents.ts` | `custom-agent-loader` | `covered` | custom-agent-loader.md |
| `spec-editor/tasks/task-metadata-storage.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/tasks/task-service.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/utils/task-utils.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/task-format-validator.ts` | `spec-editor` | `covered` | spec-editor.md |
| `rich-execution-log/controller/execution-log-controller.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `rich-execution-log/controller/execution-data-cache.ts` | `rich-execution-log` | `covered` | rich-execution-log.md |
| `model-selection/model-cache.ts` | `model-selection` | `covered` | model-selection.md |
| `notifications/notification-service.ts` | `notifications` | `covered` | notifications.md |
| `notifications/notification-settings.ts` | `notifications` | `covered` | notifications.md |
| `polling/agent-event-polling-service.ts` | `polling` | `covered` | polling.md |
| `spec-editor/telemetry/spec-telemetry-service.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/telemetry/spec-session-state.ts` | `spec-editor` | `covered` | spec-editor.md |
| `spec-editor/telemetry/dimension-builder.ts` | `spec-editor` | `covered` | spec-editor.md |
| `notifications/resource-notification-service.ts` | `notifications` | `covered` | notifications.md |
| `notifications/resource-notification-utils.ts` | `notifications` | `covered` | notifications.md |
| `notifications/index.ts` | `notifications` | `covered` | notifications.md |
| `model-selection/model-configuration.ts` | `model-selection` | `covered` | model-selection.md |
| `config/errors.ts` | `config` | `covered` | config.md |
| `model-selection/index.ts` | `model-selection` | `covered` | model-selection.md |
| `profiles/services/profile-file-watcher.ts` | `profiles` | `covered` | profiles.md |
| `profiles/errors.ts` | `profiles` | `covered` | profiles.md |
| `profiles/index.ts` | `profiles` | `covered` | commands.md, profiles.md |
| `context-usage/index.ts` | `context-usage` | `covered` | context-usage.md |
| `context-usage/initial-context-estimator.ts` | `context-usage` | `covered` | context-usage.md |
| `hooks/contextual/contextual-hook-triggers.ts` | `hooks` | `covered` | hooks.md |
| `hooks/contextual/contextual-hook-actions.ts` | `hooks` | `covered` | hooks.md |
| `hooks/contextual/pre-tool-use-hooks.ts` | `hooks` | `covered` | hooks.md |
| `hooks/contextual/tool-type-mapping.ts` | `hooks` | `covered` | hooks.md |
| `hooks/contextual/post-tool-use-hooks.ts` | `hooks` | `covered` | hooks.md |
| `hooks/contextual/hooks-provider.ts` | `hooks` | `covered` | hooks.md |
| `hooks/contextual/task-execution-hooks.ts` | `hooks` | `covered` | hooks.md |
| `commands/telemetry/telemetry.ts` | `commands` | `covered` | commands.md |
| `telemetry/telemetry-config.ts` | `telemetry` | `covered` | telemetry.md |
| `telemetry/workspace.ts` | `telemetry` | `covered` | telemetry.md |
| `utils/activation-attempts.ts` | `utils` | `covered` | utils.md |
| `telemetry/agent-telemetry-adapter.ts` | `telemetry` | `covered` | telemetry.md |
