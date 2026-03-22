# extension.js 模块覆盖清单

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 整理日期：2026-03-18
> 说明：本清单按 bundle 中保留的源码注释边界统计，只统计 Kiro 自研模块，不包含第三方 `node_modules/**`。

## 一、宿主扩展层 `src/extension/**`

按源码注释去重后，共识别出 `474` 个唯一模块块。按一级目录拆分如下：

- `powers`：48
- `capabilities`：47
- `spec-editor`：42
- `hooks`：39
- `commands`：38
- `rich-execution-log`：36
- `editor`：31
- `utils`：19
- `agent-chat`：16
- `steering`：16
- `import-steering`：12
- `context-lsp`：11
- `config`：10
- `onboarding`：10
- `terminal`：10
- `context-resolvers`：9
- `experiments`：7
- `mcp`：7
- `checkpoints`：5
- `storage`：5
- `profiles`：5
- `notifications`：5
- `contextual-spec`：4
- `telemetry`：4
- `custom-agent-loader`：4
- `enterprise`：4
- `session-resume`：4
- `acp-dev-inspector`：3
- `usage`：3
- `first-time-project`：3
- `background-processes`：3
- `model-selection`：3
- `repos`：2
- `status-bar`：2
- `context-usage`：2
- `(root)`：1
- `q-custom-model`：1
- `auth`：1
- `platform`：1
- `polling`：1

### 当前文档覆盖判断

已形成专题文档：

- `activation.md`
- `agent-chat.md`
- `auth.md`
- `background-processes.md`
- `checkpoints.md`
- `commands.md`
- `config.md`
- `context-lsp.md`
- `context-resolvers.md`
- `custom-agent-loader.md`
- `enterprise.md`
- `experiments.md`
- `hooks.md`
- `mcp.md`
- `platform.md`
- `powers.md`
- `profiles.md`
- `q-custom-model.md`
- `rich-execution-log.md`
- `notifications.md`
- `model-selection.md`
- `contextual-spec.md`
- `session-resume.md`
- `spec-editor.md`
- `steering.md`
- `storage.md`
- `telemetry.md`
- `terminal.md`
- `usage.md`
- `utils.md`
- `overview.md`

按当前口径，extension 层已形成覆盖主要 feature 目录的专题索引；后续若继续深挖，主要是对已成专题目录做更细的文件级展开，而不是补大块缺口。

## 二、Agent Runtime 层 `packages/kiro-agent/dist/**`

按源码注释去重后，共识别出 `88` 个内部模块块。

### 入口与运行时核心

- `index.js`
- `index-C-CV5vnh.js`
- `index-DBxQQQD8.js`
- `index-D_kbIg3m.js`
- `index-Q4bc6qbj.js`
- `actions-Dng4pati.js`
- `base-BCJFoMV_.js`
- `chat-agent-IUIL54gd.js`
- `sub-agent-Dxs7IWdE.js`
- `session-C-gUzvZx.js`
- `session-update-utils-S4kmZ2as.js`
- `session/schemas/index.js`
- `shared-types-CIxCt9tj.js`
- `types-CkvOiUfs.js`
- `types-core-DBgLqCOQ.js`
- `errors-1JFcQdts.js`
- `unknown-error-7JAvTvbU.js`
- `cancellation-C60foDZK.js`

### Prompt / Context / Memory

- `prompt-DuC9mWRH.js`
- `prompts-DfcXHzAI.js`
- `prompt-template-C_Mn10zi.js`
- `prompt-processor-Dw_f2vjD.js`
- `orchestrator-prompt-Vzhr2QeL.js`
- `context-chat-message-DzjJbTBD.js`
- `file-context-ClFefQMc.js`
- `node-progressive-context-source-C3Yk_3xF.js`
- `message-analyzer-BAqw8PNp.js`
- `message-parts-D8WKxpG9.js`
- `message-replay-Cjyo5CAH.js`
- `pruning-service-DJ-mli7u.js`
- `token-estimator-B45EKs9J.js`
- `token-monitor-QTYxqQ8j.js`
- `transformers-BLTCfl6l.js`
- `parse-front-matter-CXai4UYs.js`
- `range-utils-CHnKtlN2.js`
- `workspace-object-CspubCq6.js`
- `workspace-connection-impl-Dee9nf40.js`

### Tools / MCP / Powers / Capability Plumbing

- `disclose-context-bPcRLLYL.js`
- `remote-tools-C9QNKWUJ.js`
- `remote-tools-discovery-DUpCRp4S.js`
- `tool-filter-CS5Fsu0N.js`
- `tool-message-list-B1XEM-VL.js`
- `tool-tags-NGeoUHCI.js`
- `tool-usage-meter-BVm5olm7.js`
- `mcp-config-manager-BVKB8dJE.js`
- `powers-manager-CbGmy5n_.js`
- `acp-remote-mcp-client-DTe6uFFL.js`
- `command-approval-MQd-5ajF.js`
- `command-approval-_g4NkBJ8.js`
- `execution-log-controller-BGxzu20b.js`
- `terminal-dGJDGXey.js`
- `dev-inspector-CF526NYm.js`
- `config-file-watcher-DbLp_qKw.js`
- `config-constants-CsFJbjLP.js`
- `validator-ClHWtYX0.js`
- `web-fetch-utils-C1Z4KMmp.js`

### Spec / Hooks / Models / Auth

- `spec-agent-OdZ7esxm.js`
- `spec-platform-361SGdHa.js`
- `spec/tasks/index.js`
- `hooks-Dmyvf9cL.js`
- `custom-agent-parser-DbpwC6qc.js`
- `custom-agent-registry-DF7phCBA.js`
- `auth-DCPC05L9.js`
- `q-client-DsNNqt3G.js`
- `q-developer-converse-BT9F76b_.js`
- `model-config-BdfVU6dY.js`
- `model-provider-DwyIQZZf.js`
- `autonomy-mode-B8cmGn9E.js`
- `clarification-handler-Zo3AuRjE.js`
- `intent-detection-service-DV14tv5n.js`
- `steering-Dcn_tjkT.js`

### Infra / Async / Utility

- `_commonjsHelpers-DaMA6jEr.js`
- `async-delivered-object/index.js`
- `async-stream/index.js`
- `agent-context/compaction/index.js`
- `stream-CtDBTPgX.js`
- `streamed-data-object/index.js`
- `string-BZO3_EzG.js`
- `file-lock-D4f90hJ2.js`
- `number-coercion-CeAOJ7Po.js`
- `strip-json-comments-BVnY24gX.js`
- `get-user-input-DLJmPWtM.js`
- `node-background-process-manager-_8U2gTa-.js`
- `telemetry-Bf0GI6nJ.js`
- `telemetry/index.js`
- `pending-changes-BxcOZUqX.js`

### 当前文档覆盖判断

- `runtime/packages-kiro-agent-dist.md` 已覆盖大量核心模块，但其中“`81` 个模块已全部读完”的结论已经过时。
- 按当前 bundle 实扫结果，应修正为 `88` 个内部模块块。
- 后续补文档时，建议以上述四级分类为目录继续核对，避免重复分析与漏项并存。

## 三、排除项

以下内容不计入 Kiro 自研逻辑的“全量分析”范围：

- `packages/kiro-agent/node_modules/**`
- `node_modules/**`

这些模块应作为依赖层单独分析，不能与 `src/extension/**` 或 `packages/kiro-agent/dist/**` 的业务结论混用。
