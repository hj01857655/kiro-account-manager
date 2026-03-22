# runtime 模块状态清单

> 统计对象：`packages/kiro-agent/dist/**`
> 统计依据：`dist/extension.js` 中保留的源码注释边界，与 `packages-kiro-agent-dist.md` 中的独立小节交叉比对。
> 统计日期：2026-03-18

## 结论

- 当前共识别 `88` 个内部模块块。
- 其中 `88` 个已经在 [packages-kiro-agent-dist.md](./packages-kiro-agent-dist.md) 中有独立 `###` 小节，可视为已详细分析。
- 当前 `0` 个尚未独立展开，`packages/kiro-agent/dist/**` 已达到按模块块口径的全覆盖。

## 已详细分析

以下模块在 [packages-kiro-agent-dist.md](./packages-kiro-agent-dist.md) 中已有独立小节：

- `auth-DCPC05L9.js`
- `autonomy-mode-B8cmGn9E.js`
- `cancellation-C60foDZK.js`
- `chat-agent-IUIL54gd.js`
- `clarification-handler-Zo3AuRjE.js`
- `command-approval-MQd-5ajF.js`
- `command-approval-_g4NkBJ8.js`
- `config-constants-CsFJbjLP.js`
- `config-file-watcher-DbLp_qKw.js`
- `context-chat-message-DzjJbTBD.js`
- `custom-agent-parser-DbpwC6qc.js`
- `dev-inspector-CF526NYm.js`
- `disclose-context-bPcRLLYL.js`
- `errors-1JFcQdts.js`
- `execution-log-controller-BGxzu20b.js`
- `file-context-ClFefQMc.js`
- `file-lock-D4f90hJ2.js`
- `get-user-input-DLJmPWtM.js`
- `hooks-Dmyvf9cL.js`
- `index.js`
- `index-C-CV5vnh.js`
- `index-D_kbIg3m.js`
- `index-Q4bc6qbj.js`
- `intent-detection-service-DV14tv5n.js`
- `logger-CTb8_yz9.js`
- `mcp-config-manager-BVKB8dJE.js`
- `message-analyzer-BAqw8PNp.js`
- `message-parts-D8WKxpG9.js`
- `message-replay-Cjyo5CAH.js`
- `model-config-BdfVU6dY.js`
- `model-provider-DwyIQZZf.js`
- `powers-manager-CbGmy5n_.js`
- `node-progressive-context-source-C3Yk_3xF.js`
- `number-coercion-CeAOJ7Po.js`
- `orchestrator-prompt-Vzhr2QeL.js`
- `parse-front-matter-CXai4UYs.js`
- `pending-changes-BxcOZUqX.js`
- `prompt-DuC9mWRH.js`
- `prompt-processor-Dw_f2vjD.js`
- `prompt-template-C_Mn10zi.js`
- `prompts-DfcXHzAI.js`
- `pruning-service-DJ-mli7u.js`
- `q-client-DsNNqt3G.js`
- `q-developer-converse-BT9F76b_.js`
- `range-utils-CHnKtlN2.js`
- `remote-tools-C9QNKWUJ.js`
- `remote-tools-discovery-DUpCRp4S.js`
- `session/schemas/index.js`
- `session-C-gUzvZx.js`
- `shared-types-CIxCt9tj.js`
- `spec-agent-OdZ7esxm.js`
- `spec/tasks/index.js`
- `session-update-utils-S4kmZ2as.js`
- `spec-platform-361SGdHa.js`
- `steering-Dcn_tjkT.js`
- `stream-CtDBTPgX.js`
- `strip-json-comments-BVnY24gX.js`
- `sub-agent-Dxs7IWdE.js`
- `custom-agent-registry-DF7phCBA.js`
- `telemetry-Bf0GI6nJ.js`
- `terminal-dGJDGXey.js`
- `token-estimator-B45EKs9J.js`
- `token-monitor-QTYxqQ8j.js`
- `tool-filter-CS5Fsu0N.js`
- `tool-message-list-B1XEM-VL.js`
- `tool-tags-NGeoUHCI.js`
- `tool-usage-meter-BVm5olm7.js`
- `transformers-BLTCfl6l.js`
- `types-CkvOiUfs.js`
- `types-DThkhnH-.js`
- `types-core-DBgLqCOQ.js`
- `unknown-error-7JAvTvbU.js`
- `validator-ClHWtYX0.js`
- `workspace-connection-impl-Dee9nf40.js`
- `workspace-object-CspubCq6.js`
- `acp-remote-mcp-client-DTe6uFFL.js`
- `actions-Dng4pati.js`
- `index-DBxQQQD8.js`
- `node-background-process-manager-_8U2gTa-.js`
- `agent-context/compaction/index.js`
- `_commonjsHelpers-DaMA6jEr.js`
- `async-delivered-object/index.js`
- `async-stream/index.js`
- `base-BCJFoMV_.js`
- `streamed-data-object/index.js`
- `string-BZO3_EzG.js`
- `telemetry/index.js`
- `web-fetch-utils-C1Z4KMmp.js`

## 尚未独立展开

当前为空。

## 建议优先级

`packages/kiro-agent/dist/**` 这一层已经补齐。后续优先级建议转向两类：

- 深挖型：
  - 已有独立小节但仍可继续下钻的大块，如 `base-BCJFoMV_.js`、`string-BZO3_EzG.js`、`stream-CtDBTPgX.js`
- 回到 extension 层：
  - 继续补 `src/extension/**` 下尚未彻底收敛的 feature 目录，例如 hooks / spec-editor / steering / profiles / terminal / telemetry / experiments

## 使用方式

- 新增 runtime 模块时，先核对 `dist/extension.js` 的源码注释边界，再补独立 `###` 小节。
- 如果只是摘要里提到，但没有独立小节，不要提前标成“已详细分析”。
