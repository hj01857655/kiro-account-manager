# Kiro 1.4.5 协议面（kiro.kiro-agent 1.1.499）

本文只记录 **官方客户端源码里的事实**，全部取证自静态反编译产物 `docs/kiro-re-1.4.5/`，每条附产物文件名，可逐字复核。不做推测；社区/活体实测结论不在本文范围（见《Kiro 上游通道实测与对账》）。

> 2026-10-22 追加：混淆常量的**字面量**已从同一个 `extension.js` 静态还原（仍**未动态抓包**），产物 `consts-restored.txt`、`error-enums-restored.txt`、`stopreason-enum.txt`；汇总见 §13。

## 0. 版本基线（必须先对齐）

| 基线 | 值 | 记录位置 |
| --- | --- | --- |
| Kiro 桌面产品 | `1.4.5`（distro `d0fd3324a737f695bd14f2aee3ca92accd28870f`，data=`.kiro`） | `docs/kiro-re-1.4.5/version.txt` |
| 扩展 | `kiro.kiro-agent@1.1.499`，main `./dist/extension.js`，14.53MB | 同上 |
| 取证方式 | **static only（未动态抓包）** | 同上 |

**与其他文档的基线关系（不可混用结论）**：

- `docs/kiro-re/`（旧目录）= 产品 `1.0.89` / agent `1.0.165`，是**另一代基线**。其 EventStream / 事件名结论只在明确标注时使用。
- `docs/Kiro 1.1.14/` = agent `1.1.28`（手动更新），其中 G1–G5 与 workspace/Powers 类结论属账号管理器上下文，**不自动适用于 1.4.5**。
- `docs/Kiro 官方 thinking 与 effort 事实.md` 与 `docs/Kiro 上游通道实测与对账.md` 的**方向切分**在 1.4.5 仍成立，本文沿用（见 §1）。

## 1. 两条出站链路：`/v1/responses` 与 `generateAssistantResponse`

官方客户端 1.4.5 同时存在两条把对话发往后端的路径，字段形状**不等价**：

```
① 原生路径  POST https://runtime.{region}.kiro.dev/generateAssistantResponse
             body = { conversationState, profileArn?, additionalModelRequestFields? }
             响应 = Kiro EventStream（assistantResponseEvent / reasoningContentEvent / ...）

② CreateResponse 路径  POST {krsEndpoint}/v1/responses
             body = { model, input, instructions?, tools?, stream:true, origin,
                      profileArn?, metadata?, reasoning? }
             响应 = OpenAI Responses SSE（response.* 事件 + kiro:* 扩展事件）
```

- 路径①的 `additionalModelRequestFields`（AMRF）与 `conversationState` **同级**，位于请求体顶层。
- 路径②由 `cfd()` 构造、`pfd()` 裸 HTTP 发送；`krsEndpoint` 由 `SK({identity, regionOverride, endpointOverride}).endpoint` 解析，即 **runtime 域名**（产物 `getkrs.txt`、`krs-endpoint-callers.txt`、`sk-endpoint.txt`）。
- ⚠️ 这两个 `/v1/responses` **同名但方向相反**：路径②是官方**出站**；本项目的 `/v1/responses` 是**入站**兼容层（`src-tauri/src/gateway/mod.rs:1148` `openai_responses_handler`）。本文所有 `/v1/responses` 均指官方出站。

## 2. CreateResponse 请求体构造（`cfd`）

产物：`cfd-builder.txt`、`sfd-input.txt`、`i6i-items.txt`、`efd-envelope.txt`、`ifd-reasoning.txt`、`c6i-builder.txt`

```js
function cfd(t){
  let {conversationState:e, systemPrompt:r, modelId:n, origin:s, profileArn:a,
       agentMode:l, additionalModelRequestFields:u} = t,
      d = afd(e),   // tools
      p = {};       // metadata
  return l && (p["kiro:agent_mode"] = l),
         e.agentTaskType === Km.SPEC_TASK && (p["kiro:agent_task_type"] = Km.SPEC_TASK),
         e.conversationId !== void 0 && (p["kiro:conversation_id"] = e.conversationId),
         e.rootConversationId !== void 0 && (p["kiro:root_conversation_id"] = e.rootConversationId),
         e.agentContinuationId !== void 0 && (p["kiro:agent_continuation_id"] = e.agentContinuationId),
         { model:n, input:sfd(e), ...r !== void 0 && {instructions:r}, ...d && {tools:d},
           stream:!0, origin:s, ...a !== void 0 && {profileArn:a},
           ...Object.keys(p).length > 0 && {metadata:p}, ...lfd(u) }
}
```

要点（全部有产物支撑）：

1. `stream` **恒为 `true`**，无条件分支。
2. `metadata` 的键全部带 `kiro:` 前缀；只有 `agentTaskType === SPEC_TASK` 才写 `kiro:agent_task_type`；`conversationId` / `rootConversationId` / `agentContinuationId` 三者只要 `!== undefined` 就写，空 metadata 对象整体省略。
3. `tools` 只从 **`currentMessage`** 取（`afd` 读 `conversationState.currentMessage.userInputMessage.userInputMessageContext.tools`），不看 history；每条要求 `toolSpecification.name` 存在，形状为扁平 `{type:"function", name, description, parameters: inputSchema.json}`（注意：`parameters` 直接放 JSON Schema 本体，不是嵌套 `{parameters:{...}}` 的 OpenAI tool 形状）。
4. `instructions` 只在 `systemPrompt !== undefined` 时出现。
5. `origin` 是**必发字段**（无 `?? undefined` 保护），值来自 `getOrigin()`，默认 `"AI_EDITOR"`（`ofd-incomplete.txt` 末尾 `x6i="AI_EDITOR"`，`Vfd={"kiro-ide":"AI_EDITOR", ...}`；`getkrs.txt` 中 `this.getOrigin = r.getOrigin ?? (() => "AI_EDITOR")`）。
6. `reasoning` 由 `lfd(additionalModelRequestFields)` 从 AMRF **重新投影**得到，不是原样透传 AMRF：

```js
function lfd(t){
  let e = Iui(t),   // t.thinking.type    -> reasoning.type
      r = Cui(t),   // t.output_config.effort 或 t.reasoning.effort -> reasoning.effort
      n = Tui(t);   // t.thinking.display -> reasoning.summary
  return e === void 0 && r === void 0 && n === void 0 ? {}
       : { reasoning: { ...e!==void 0&&{type:e}, ...r!==void 0&&{effort:r}, ...n!==void 0&&{summary:n} } };
}
```

也就是说：同一个 AMRF 在路径①里原样发（顶层 `additionalModelRequestFields`），在路径②里被压成 `reasoning.{type,effort,summary}`。`summary` 的来源是**输入侧的 `thinking.display`**（产物 `tui-summary.txt`），不是输出侧字段。

## 3. history / currentMessage → Responses `input`

产物：`sfd-input.txt`、`i6i-items.txt`、`efd-envelope.txt`、`c6i-builder.txt`

`input` 由 `sfd(conversationState)` 生成：先摊平 `history`，再摊平 `currentMessage`，每条走同一个 `I6i`。顺序即会话顺序，**不区分历史与当前**（当前消息只是最后一条）。

`I6i` 规则（逐条对应产物原文）：

| 来源 | 产出条目 |
| --- | --- |
| `userInputMessage.userInputMessageContext.toolResults[]` | `{type:"function_call_output", call_id: toolUseId, output: content[].text 以换行 join}` |
| `userInputMessage.images[]`（需 `source.bytes`） | `{type:"input_image", image_url:"data:image/{format小写};base64,{bytes}"}` |
| `userInputMessage.documents[]` | `ofd()` → `{type:"input_file", filename:"{name}.{format}", file_data:"data:{mime};base64,{bytes}"}`；`source.bytes` 缺失直接丢弃并打 `cr.request.document.dropped_no_bytes` |
| 上述多模态存在时 | 折叠成一个 `{role:"user", content:[ {type:"input_text",text}, ...images, ...files ]}`；文本块只在 `typeof content=="string" && length>0` 时插入 |
| 纯文本 user | `{role:"user", content: 字符串}` |
| `assistantResponseMessage.reasoningContent` | `ifd()` → `{type:"reasoning", summary:[], encrypted_content:<JSON 字符串>}` |
| `assistantResponseMessage.content`（非空字符串） | `{role:"assistant", content: 字符串}` |
| `assistantResponseMessage.toolUses[]` | `{type:"function_call", call_id: toolUseId, name, arguments: 字符串原样 / 否则 JSON.stringify(input ?? {})}` |

reasoning 信封（`encrypted_content` 的**内容**）由 `C6i` 构造、`efd` 反向解析，是两个固定形状的 JSON 字符串：

```js
// C6i：写侧
{ reasoningText: { text, signature? } }          // 普通思考
{ redactedContent: <base64> }                     // 被加密屏蔽的思考

// efd：读侧（response.output_item.done 的 item.encrypted_content）
"redactedContent" in obj  -> {kind:"redacted", redactedContentBase64}
"reasoningText" in obj    -> {kind:"text", text, signature?}
```

注意 `ifd` 对 `redactedContent` 会先 `Buffer.from(t.redactedContent).toString("base64")` —— 内部表示是二进制，落到 `encrypted_content` 才是 base64。

`ofd` 的 MIME 表由扩展名白名单 `tfd` + 映射 `rfd` 决定，不在白名单时 MIME 退 `application/octet-stream` 并打 `cr.request.document.unsupported_extension`（**字段仍然发送**，只是类型不对）。

## 4. CreateResponse 请求头（`pfd`）

产物：`pfd-fn.txt`、`pfd-full.txt`、`origin-header.txt`、`origin-header-ctx.txt`、`kiro-headers-1.4.5.txt`

```js
let d = {
  "Content-Type": "application/json",
  "Authorization": `Bearer ${token}`,
  "x-amzn-kiro-origin": origin,
  "x-amzn-codewhisperer-optout": contentCollectionEnabled ? "false" : "true",
  ...profileArn ? { "x-amzn-kiro-profile": profileArn } : {},
  ...tokenType   ? { TokenType: tokenType } : {},
  ...userAgent   ? { "User-Agent": userAgent } : {},
  ...extraHeaders
};
```

与路径①（AWS SDK / EventStream）的**关键差别**：

- profile 头名不同：CreateResponse 用 `x-amzn-kiro-profile`，SDK 路径用 `x-amzn-kiro-profile-arn`（产物统计：`-profile` ×1、`-profile-arn` ×4）。
- optout 是**二值**：内容采集开启时显式发 `"false"`，而 SDK 中间件只在关闭时补 `"true"`、从不发 `false`（`xan-te-ctx2.txt` 的 `kAn`）。
- 没有 `amz-sdk-invocation-id` / `amz-sdk-request` / `x-amz-user-agent` / `host` —— 裸 `http.request`，不经 SDK；重试头改由 `x-kiro-attempt`（`LNr`，`rhd()` 写 `${retryCount+1};max=${attemptBudget()}`）承担。
- 非 https endpoint 会打 warn（`[create-response-raw] HTTP endpoint detected — bearer token will be sent over an unencrypted connection`），但**不阻止**发送。
- 超时：`requestTimeoutMs` 默认 60s（TTFB，触发后 `m.destroy`），`connectionTimeoutMs` 默认 `ufd = 15e3`（15s；>0 时生效，socket `connect` 后清除）。预读上限 `dfd = 1024*1024`（`ffd` 攒够 1MB 即提前返回）。
- 响应侧读取 `x-amzn-requestid` ?? `x-request-id` 作 requestId，并把**所有 string 型响应头**留存；`retryAfterMs` 从 **`x-amzn-kiro-ratelimit-retry-after`** 头解析（常量 `nHi` 已还原；`Number.isFinite && >= 0`）。

## 5. CreateResponse SSE 解析（`Dfd`）

产物：`dfd-parser.txt`、`ofd-incomplete.txt`

解析器逐行切 `\n`，只处理 `data:` 前缀行，`[DONE]` 与空 payload 直接忽略；单行缓冲上限 `Mfd = 4*1024*1024`（超出抛 `FH`）。按事件类型：

| SSE `type` | 内部产出 |
| --- | --- |
| `kiro:keep_alive` | `{ kiroKeepaliveEvent: {} }`（空心跳哨兵；`Rfd` 已还原为 `"kiroKeepaliveEvent"`） |
| `response.output_text.delta` | `assistantResponseEvent.content = delta` |
| `response.reasoning_text.delta` **或** `response.reasoning.delta` | `reasoningContentEvent.text = delta` |
| `response.output_item.done` && `item.type=="reasoning"` | 解 `item.encrypted_content`：`kind=="redacted"` → `reasoningContentEvent.redactedContent = Buffer.from(b64,"base64")`；`kind=="text" && signature!==undefined` → `reasoningContentEvent.signature`；否则打 `cr.sse.reasoning.envelope.undecodable` |
| `response.refusal.delta` | 置位 `l=true`，并把 delta **当正文**写入 `assistantResponseEvent.content` |
| `response.refusal.done` | 仅当 `l===false`（没收到过 delta）时写入 `refusal` 全文 |
| `response.output_item.added` && `item.type=="function_call"` | `call_id ?? id ?? ""` 入集合并记为当前 `n`，发 `toolUseEvent{toolUseId,name,input:arguments??"",stop:false}` |
| `response.function_call_arguments.delta` | 仅当已有当前 `n`；发 `toolUseEvent{input:delta,stop:false}`，并把 `n` 记入“已给过增量”集合 |
| `response.function_call_arguments.done` | 若 `n` 在 open 集合：从两个集合删除，发 `toolUseEvent{toolUseId:n,stop:true}`，清空 `n` |
| `response.output_item.done` && `item.type=="function_call"` | 不在 open 集合则**丢弃**；否则出 open，“给过增量”时补一帧 `input`，`item.status=="incomplete"` 则打 `cr.sse.tool_call.incomplete_item` 并**不**入 held-stops，否则 `a.push(call_id)` |
| `response.completed` / `response.incomplete` | 走终止子生成器（见下） |
| `error` / `response.failed` | 抛 `wfd(error, requestId)`，并清空 held-stops / open 集合 |
| `kiro:code_reference.added` | `Nfd(annotations)` 非空时发 `codeReferenceEvent.references` |

终止子生成器 `*p(reason, response)` 的固定行为：

1. `isIncomplete = reason==="response.incomplete" || response.status==="incomplete"`；`v = response.incomplete_details.reason`。
2. 不完整时若有未闭合工具，打 `cr.sse.tool_call.unstopped_on_incomplete`（**不补发 stop**）；完整时先给 held-stops 和残留 open 各补一帧 `toolUseEvent{stop:true}`。
3. `stopReason` 默认 `"END_TURN"`；不完整时查表 `Ofd`，**查不到则保持 `END_TURN`** 并上报 `createResponseUnknownIncompleteReason` 指标 + `cr.sse.unknown_incomplete_reason` 日志：

```js
Ofd = { max_output_tokens: "MAX_TOKENS",
        content_filter:    "CONTENT_FILTERED",
        compaction_required_for_model_handoff:
                           "COMPACTION_REQUIRED_FOR_MODEL_HANDOFF" }
```

（第三个键值由 `kfd` / `Tfd` 还原，见 §13.1。`max_output_tokens` 与 `content_filter` 止是 OpenAI 标准 `incomplete_details.reason` 取值；**`compaction_required_for_model_handoff` 是 Kiro 自定义值**，官方把它映射成一个同样非标准的 stopReason `COMPACTION_REQUIRED_FOR_MODEL_HANDOFF`，上游用它要求客户端先压缩上下文再交接模型，网关不能当 `end_turn` 吞掉——见对照文档 G-8b。

4. 依次发 `metadataEvent{stopReason}` → `contextUsageEvent` → `meteringEvent`：
   - `usage["kiro:context_usage_percentage"]`（**带 `kiro:` 前缀**，有限数才发）。
   - `usage["kiro:credits"]` → `meteringEvent{usage:value, unit?, unit_plural?→unitPlural}`。

=> 两个 usage 字段都在 `response.completed/incomplete` 的 `response.usage` 里，**都是 `kiro:` 命名空间扩展**，不是 OpenAI 标准字段。

## 6. 上游事件 → 内部消息（`Zfd`）

产物：`zfd-mapper.txt`

`Zfd` 把一个事件对象映射为 kind 数组（均对字段做类型守卫）：

| 事件 | kind | 备注 |
| --- | --- | --- |
| `assistantResponseEvent` | `assistant_response` | 需 `content` 为非空字符串；可选 `modelId` |
| `reasoningContentEvent` | `reasoning_content` | `text` / `signature` / `redactedContent`；`redactedContent` 为字符串直接用，为 `Uint8Array` 则 `toString("base64")`；三者全空则不产出 |
| `toolUseEvent` | `tool_use` | `stop` 取 `=== true` |
| `meteringEvent` | `metering` | `{usage,unit,unitPlural}` |
| `codeReferenceEvent` | `code_reference` | `references` 非数组时退空数组 |
| `metadataEvent` | `stop_reason` | 仅 `stopReason` 非空字符串时产出；`stopDetails.refusal{category,explanation,recommendedModel}` 同步映射 |
| `contextUsageEvent` | `context_usage` | 需 `contextUsagePercentage` 为 number |

## 7. thinking / effort 的 schema 推导

产物：`effort-parsers.txt`、`zyu-paths.txt`、`cbt-thinking.txt`、`tui-summary.txt`、`m4e-excluded.txt`、`jyu-listmodels.txt`、`jyu-modelmap.txt`

官方**不硬编码档位表**，而是从 `ListAvailableModels` 每模型的 `additionalModelRequestFieldsSchema` 推导：

```js
Zyu = [ { path:"output_config", extract: t => t.properties?.output_config?.properties?.effort },
        { path:"reasoning",     extract: t => t.properties?.reasoning?.properties?.effort } ];
// eAu：首个命中且 enum 非空的，返回 { levels: enum, schemaPath: path, defaultLevel: default }
```

- thinking 可开关的**必要条件**：`properties.thinking.properties.type.enum` 同时包含 `"disabled"` 和 `"adaptive"`（`gSe="disabled"`, `g4e="adaptive"`）；否则 `tAu` 直接返回 `undefined`（模型不可开关思考）。默认开启与否看 `type.default !== "disabled"`。
- `bui(effort, schemaPath)` 回写 AMRF：`output_config` → `{output_config:{effort}}`，`reasoning` → `{reasoning:{effort}}`。**两路径互斥**。
- `Cui` 从 AMRF 读 effort 时也按 `{output_config, reasoning}` 顺序查找。
- 思考关闭时档位被**降级**（`Cbt` + `ySe`）：`m4e = new Set(["xhigh","max"])`，当前档在这集合里就改取 `effortLevels` 中**最后一个不属于 m4e 的档**，并打 `llm.request.effort.capped_for_thinking_off`。
- `thinkingType` 本身另走 `rAu(t) = {thinking:{type:t}}`，与 effort 并列写入 AMRF；`display`（`Tui`）**只属于输入侧**，在 CreateResponse 路径被映射为 `reasoning.summary`。
- `ListAvailableModels` 返回的每模型字段（`jyu-modelmap.txt`）：`modelId, modelName, description, provider(modelProvider), rateMultiplier, rateUnit, maxInputTokens(tokenLimits.maxInputTokens), refusalFallbackModels[].modelId, effortLevels, effortSchemaPath, defaultEffortLevel, thinkingToggleable, defaultThinkingEnabled`；分页 `nextToken`，超页上限 `Sui = 10` 时打 warn 并**丢弃后续页**；`defaultModel.modelId` 只取首次非空。

## 8. 其余请求头（SDK / BFF 中间件）

产物：`new-headers-ctx.txt`、`kiro-headers-1.4.5.txt`、`client-attribution*.txt`、`jsu-authheaders.txt`、`xkiro-headers.txt`、`qsu-idp.txt`、`bae-tokentype.txt`

Agent 上下文中间件（`Eai`）逐请求注入：

```js
headers = {
  ...existing,
  ...(presentation !== "synthetic" && { "x-amzn-kiro-execution-id": executionId }),
  ...(agentName !== void 0 && { "x-amzn-kiro-agent-name": agentName }),
  "x-amzn-kiro-session-id": sessionId,          // Gsu，**无条件**写
  "x-amzn-kiro-agent-presentation": presentation // Vsu，**无条件**写
}
```

其余已取证头（1.4.5 全量清单，`kiro-headers-1.4.5.txt`）：

`x-amzn-kiro-agent-mode`（×4）、`x-amzn-kiro-agent-name`、`x-amzn-kiro-agent-presentation`、`x-amzn-kiro-client-attribution`、`x-amzn-kiro-conversation-id`（×2，**响应头**）、`x-amzn-kiro-execution-id`、`x-amzn-kiro-origin`、`x-amzn-kiro-profile`、`x-amzn-kiro-profile-arn`（×4）、`x-amzn-kiro-ratelimit-retry-after`（×3）、`x-amzn-kiro-session-id`。

- `agent-mode`：中间件 `xAn(t,e)` 直接写值；已取证调用例 `xAn(d, "autocomplete")`（补全）；会话路径取 `modeId ?? "vibe"`，Spec 下为 `"spec"`；`Km = {SPEC_TASK:"spectask", VIBE:"vibe"}`（`vibe-ctx.txt`）。
- `X-Kiro-Idp` / `X-Kiro-Profile-Arn` / `TokenType` 由 `jsu` 统一注入（**仅 `kind==="sign-in"` 时写 Idp 与 Profile-Arn**）；Idp 枚举映射 `qsu`：`Google|Github|BuilderId` 原样，`Enterprise`/`Internal` → `"AWSIdC"`，`ExternalIdp` → `"ExternalOIDC"`。
- TokenType 常量表 `bAe`：`external_idp→"EXTERNAL_IDP"`，`machine_token→"KIRO_MACHINE_TOKEN"`，`api_key→"API_KEY"`，`IdC→"SSO_OIDC"`（`VDl` 已还原）。机器身份集 `Zsu = {"api_key","machine_token"}`，`eau` 用它判定 `{kind:"machine"}`。
- 评测/归因头：`x-kiro-eval-treatments` / `x-kiro-eval-strategy`（变量名 `Fou`/`$ou`，各有正则约束）；`x-amzn-kiro-client-attribution` 为常量 `Hsu`。
- `GenerateAssistantResponseCommand` 的响应会读 `x-amzn-kiro-conversation-id` 头并放入 `output.conversationId`（`conv-id-resp.txt`）。

## 9. region / endpoint 解析

产物：`pyu-region.txt`、`hli-resolver.txt`、`nse-profile-region.txt`、`xsu-region.txt`、`sk-endpoint.txt`、`krs-endpoint-callers.txt`

```js
Hli(service, cfg) = { region, endpoint }   // region = cfg.regionOverride ?? regionOf(identity) ?? "us-east-1"
                                          // endpoint = cfg.endpointOverride || `https://${service}.${region}.kiro.dev`
E3(cfg) = Hli("management", cfg)   // 控制面
SK(cfg) = Hli("runtime", cfg)      // 数据面 = krsEndpoint
```

- region 来源优先级（`rSe` / `pyu`）：`regionOverride` → **profileArn 第 4 段**（`nSe`：`arn:...:region:...` 且通过 `xai` 合法性校验）→ `identity.region`（同样过 `xai`）。`xai = /^[a-z]{2,4}(-[a-z]+)+-\d{1,2}$/`——即 `us-east-1` / `eu-central-1` / `us-gov-west-1` 这类形式，不匹配单段名（如 `us1`）。
- 因此 **region 不是配置项而是 ARN 派生值**；profile ARN 的区域与请求区域不一致时，ARN 赢。
- 身份类型分类 `eau`：`authMethod` 在 `Zsu` 集合内 → `{kind:"machine"}`；否则有 ARN region → `{kind:"region",region}`；再否则 `{kind:"sign-in-without-region"}`。`Abt`：`external_idp→"EXTERNAL_IDP"`、`IdC→"SSO_OIDC"`。
- SDK 客户端统一 `maxAttempts:3`，`requestHandler: Hie(endpoint)`（连接/请求超时各 15s）。
- CreateResponse 的 endpoint 就是 `SK({identity, regionOverride, endpointOverride}).endpoint`（`krs-endpoint-callers.txt`），**没有单独域名**。

## 10. 域名与操作清单

产物：`kiro-hosts-1.4.5.txt`、`all-hosts-1.4.5.txt`、`ops.txt`

**Kiro 域名（按服务族）**

- `runtime.{us-east-1|eu-central-1|us-gov-east-1|us-gov-west-1}.kiro.dev` —— 数据面（generateAssistantResponse / CreateResponse / mcp）
- `management.{us-east-1|eu-central-1|us-gov-east-1|us-gov-west-1}.kiro.dev` —— 控制面
- `prod.us-east-1.auth.desktop.kiro.dev` —— 认证
- `*.telemetry.desktop.kiro.dev`（prod / gamma / beta）与 `telemetry.{region}.kiro.dev`、`prod.download.desktop.kiro.dev`、`app.kiro.dev`、`management.us-gov-west-1.kiro.dev` —— 遥测/下载/Web
- AWS 侧仍有 `q.us-east-1.amazonaws.com`、`q.eu-central-1.amazonaws.com`、`q-fips.*`、政务/闭源域（`c2s.ic.gov` / `sc2s.sgov.gov` / `hci.ic.gov`）

**已取证操作名**：`CreateToken`、`RefreshToken`、`GenerateAssistantResponse`、`ListAvailableModels`、`GetUsageLimits`、`GetProfile`、`GetFeatureConfiguration`、`InvokeMCP`、`InvokeMCPStream`、`SendMessage`。

## 11. 错误码映射

产物：`pfd-full.txt`（`hfd` / `vfd` / `yfd` / `wfd`）

- HTTP 层 `hfd(status, code)`：`413 → CONTENT_LENGTH_EXCEEDS_THRESHOLD`；否则**先查覆盖表 `mfd`**，未命中再把 `kiro:` 前缀去掉并大写，落在 `gfd` 内则用该枚举名，否则**原样返回 code**（`mfd` / `gfd` 字面量见 §13.3）。
- 流内错误 `vfd(type, code)`（无 HTTP 状态时合成）：

| 条件 | 映射状态 |
| --- | --- |
| `code == "context_length_exceeded"` | 413 |
| `code == "model_not_found"` 或 `type=="not_found"` | 404 |
| `type=="too_many_requests"` 或 `code=="rate_limit_exceeded"` | 429 |
| `type=="insufficient_quota"` 或 `code=="credit_balance_exhausted"` | 402 |
| `code∈{access_denied, "kiro:model_requires_user_opt_in"}` 或大写后属 `Sfd` | 403 |
| `type=="invalid_request"` | 400 |
| `code=="server_is_overloaded"` | 503 |

- 重试提示：`yfd` 会从 body 的 `retryAfterMilliseconds` 取值，并从 `x-amzn-kiro-ratelimit-retry-after` 头算 `retryAfterHintMs`；预响应重试策略 `rhd` 用令牌桶 + `x-kiro-attempt` 头。

## 12. 与旧基线（1.0.89 / agent 1.0.165）的差异

对比 `docs/kiro-re/`（旧）与 `docs/kiro-re-1.4.5/`（新）：

| 项 | 1.0.89 / 1.0.165 | 1.4.5 / 1.1.499 |
| --- | --- | --- |
| 事件集 | `AssistantResponseEvent` / `ReasoningContentEvent` / `ToolUseEvent` / `ToolResultEvent` / `ContextUsageEvent` / `MeteringEvent` / `MessageMetadataEvent` / `GenerateAssistantResponse` | 同一内部事件族仍在；**新增一整层 OpenAI Responses SSE（`response.*` + `kiro:*`）**，由 `Dfd` 翻译成旧事件 |
| 出站端点 | 仅 `generateAssistantResponse`（SDK / EventStream） | 双路：SDK 与裸 HTTP `POST {runtime}/v1/responses` |
| profile 头 | `x-amzn-kiro-profile-arn` | 仍用 `-profile-arn`（SDK）**+** `x-amzn-kiro-profile`（CreateResponse） |
| usage 字段 | `contextUsageEvent` 直接给百分比 | CreateResponse 路径改为 `response.usage["kiro:context_usage_percentage"]` 与 `usage["kiro:credits"]` |
| stop 语义 | `metadataEvent.stopReason` 枚举 | 新增 `incomplete_details.reason → stopReason` 映射（`Ofd`） |
| kiro 头 | 无 | 新增 `x-amzn-kiro-session-id` / `-agent-name` / `-agent-presentation` / `-execution-id` / `-client-attribution` / `x-kiro-attempt` / `x-kiro-eval-*` |

## 13. 已还原的混淆常量与枚举

静态还原自同一个 `extension.js`（仍**未动态抓包**）；产物：`consts-restored.txt`、`error-enums-restored.txt`、`stopreason-enum.txt`。本节只给字面量，不作推论。旧版本里本节列的是「未还原」，已全部取到值。

### 13.1 标量常量

| 符号 | 字面量 | 用途 |
| --- | --- | --- |
| `nHi` | `"x-amzn-kiro-ratelimit-retry-after"` | CreateResponse 读的重试等待头 |
| `ufd` | `15e3` | `connectionTimeoutMs` 默认 |
| `dfd` | `1024*1024` | 首帧预读上限（`ffd`） |
| `Mfd` | `4*1024*1024` | SSE 单行缓冲上限 |
| `kfd` | `"compaction_required_for_model_handoff"` | `Ofd` 第三键（incomplete reason） |
| `Tfd` | `"COMPACTION_REQUIRED_FOR_MODEL_HANDOFF"` | 对应 stopReason |
| `Rfd` | `"kiroKeepaliveEvent"` | 心跳内部键 |
| `oHi` | `"MODEL_REQUIRES_USER_OPT_IN"` | `kiro:model_requires_user_opt_in` 的枚举值 |
| `Sui` | `10` | `ListAvailableModels` 分页上限 |
| `xai` | `/^[a-z]{2,4}(-[a-z]+)+-\d{1,2}$/` | region 合法性校验 |
| `VDl` | `"SSO_OIDC"` | `bAe.IdC` 的 TokenType |
| `Zsu` | `{"api_key","machine_token"}` | `eau` 判机器身份 |
| `Hsu` / `Gsu` / `Vsu` | `"x-amzn-kiro-client-attribution"` / `"x-amzn-kiro-session-id"` / `"x-amzn-kiro-agent-presentation"` | 归因 / agent 中间件头 |
| `LNr` / `zsu` | `"x-kiro-attempt"` / `"amz-sdk-request"` | 重试计数头

### 13.2 stopReason 全枚举（`ldi`）

```
CONTENT_FILTERED | END_TURN | MAX_TOKENS | MODEL_CONTEXT_WINDOW_EXCEEDED
| PAUSE_TURN | TOOL_USE | UNKNOWN
```

另有 `cdi = wDr = { MODEL_TEMPORARILY_UNAVAILABLE }`。这 7 个值是上游 stop 语义的**完整上界**：Anthropic 侧只能直接表达 `end_turn` / `max_tokens` / `tool_use`（`CONTENT_FILTERED` 需映射成 `refusal`），`PAUSE_TURN` 与 `MODEL_CONTEXT_WINDOW_EXCEEDED` **无对应标准值**，必须显式定策——详见对照文档 G-8 / G-8b。

### 13.3 错误码集合

- **`gs`**（26 项，请求校验类；即 `hfd` 大写后的合法集）：`CONTENT_LENGTH_EXCEEDS_THRESHOLD`、`DISALLOWED_FILE_TYPE`、`DOCUMENT_COUNT_EXCEEDED`、`DOCUMENT_DUPLICATE_NAME`、`DOCUMENT_MAXIMUM_PAGES_EXCEEDED`、`DOCUMENT_MODEL_NOT_SUPPORTED`、`DOCUMENT_PASSWORD_PROTECTED`、`DOCUMENT_SIZE_EXCEEDED`、`IMAGE_COUNT_EXCEEDED`、`IMAGE_DIMENSION_EXCEEDED`、`IMAGE_FORMAT_UNSUPPORTED`、`IMAGE_MIME_MISMATCH`、`IMAGE_SIZE_EXCEEDED`、`INVALID_CONVERSATION_ID`、`INVALID_DOCUMENT_NAME`、`INVALID_KMS_GRANT`、`INVALID_MODEL_ID`、`PROMPT_TOO_LONG`、`REQUEST_BODY_INVALID`、`REQUEST_BODY_INVALID_JSON`、**`THINKING_SIGNATURE_INVALID`**、`TOOL_CONFIG_MISSING`、`TOOL_DUPLICATE`、`TOOL_SCHEMA_INVALID`、**`TOOL_USE_RESULT_MISMATCH`**、`UNSUPPORTED_LANGUAGE`。
  其中 `THINKING_SIGNATURE_INVALID` 与 `TOOL_USE_RESULT_MISMATCH` 直接对应网关的 thinking signature 透传（G-7）与 toolResults 排序（`order_tool_results_like_previous_tool_uses`）——这两处写错会被上游拒收而不是静默失败。
- **`Kbt`**（限流/额度 6 项）：`CREDIT_CONSUMPTION_RATE_EXCEEDED`、`DAILY_REQUEST_COUNT`、`INSUFFICIENT_MODEL_CAPACITY`、`MONTHLY_REQUEST_COUNT`、`SERVICE_REQUEST_RATE_EXCEEDED`、`USER_REQUEST_RATE_EXCEEDED`。
- **`vDr`**（3 项）：`CONVERSATION_LIMIT_EXCEEDED`、`MONTHLY_REQUEST_COUNT`、`OVERAGE_REQUEST_LIMIT_EXCEEDED`。
- **`yDr`**（4 项，403 类；即 `Sfd = Object.values(yDr)`）：`FEATURE_NOT_SUPPORTED`、`TEMPORARILY_SUSPENDED`、`UNAUTHORIZED_CUSTOMIZATION_RESOURCE_ACCESS`、`UNAUTHORIZED_WORKSPACE_CONTEXT_FEATURE_ACCESS`。
- **`mfd`**（`hfd` 首查的覆盖表）：`context_length_exceeded→gs.CONTENT_LENGTH_EXCEEDS_THRESHOLD`、`model_not_found→gs.INVALID_MODEL_ID`、`invalid_function_parameters→gs.TOOL_SCHEMA_INVALID`、`invalid_json→gs.REQUEST_BODY_INVALID_JSON`、`"kiro:model_requires_user_opt_in"→oHi`、`server_is_overloaded→wDr.MODEL_TEMPORARILY_UNAVAILABLE`、`rate_limit_exceeded→Kbt.USER_REQUEST_RATE_EXCEEDED`。
- **`gfd`** = `union(gs, Kbt, vDr, yDr, cdi, wDr)`；**`Afd`** = `{invalid_request, not_found, too_many_requests, insufficient_quota, server_error, model_error}`（`wfd` 在 `type` 缺失时用它的 `code` 升级为 `type`）。

仍未还原的只剩 `FH`（SSE 行溢出所抛异常类）的具体定义与部分埋名常量。

另：`CreateResponse` 分支在官方代码里受开关控制（文档中称“服务端 flag 开启后改用 Responses 协议”）；本文只记录**已取证的代码形状**，不推断当前账号/区域是否已命中该分支。

另：`CreateResponse` 分支在官方代码里受特开关控制（文档中称“服务端 flag 开启后改用 Responses 协议”）；本文只记录**已取证的代码形状**，不推断当前账号/区域是否已命中该分支。

