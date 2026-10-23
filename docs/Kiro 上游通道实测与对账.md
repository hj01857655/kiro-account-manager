# Kiro 上游通道实测与对账

本文记录 **Kiro 上游通道的实测结果**与**开源实现的适配情况对账**。

与《Kiro 官方 thinking 与 effort 事实》分工明确：

| 文档 | 内容 | 取证方式 |
| --- | --- | --- |
| 《Kiro 官方 thinking 与 effort 事实》 | Kiro 官方客户端源码 + 官方文档的事实（thinking / effort / AMRF 形状） | 只读官方 `extension.js` 与官方文档 |
| **本文** | 上游通道的**活体实测**、**开源实现**的适配情况、**社区报告**的复核 | 直连上游打真实请求 + 拉取 GitHub 仓库源码 |
| 《Kiro 开源项目索引》 | 各仓库的**完整索引**与**核验深度**记录 | 逐仓库拉源码核验 |

**本文的一切结论都不是"官方行为"**。凡涉及社区实现的部分，都是**社区的观察与实现**；凡涉及实测的部分，都受**账号类型、模型、上游灰度**影响，只能代表当次观测。

主文档中的官方事实结论仍以主文档为准；本文只在其基础上补充"真实上游如何响应"这一层。

---

## 1. 实测环境

| 项 | 值 |
| --- | --- |
| 日期 | 2026-10-21 |
| 账号类型 | **Free**（`authMethod=social`，`provider=Google`） |
| Region | `us-east-1` |
| 凭据来源 | `~/.aws/sso/cache/kiro-auth-token.json`（Kiro CLI 的 SSO 缓存） |
| 凭据字段 | `accessToken` / `refreshToken` / `profileArn` / `expiresAt` / `authMethod` / `provider` |
| 实拉模型清单 | `auto` / `claude-sonnet-4.5` / `claude-haiku-4.5` / `deepseek-3.2` / `minimax-m2.5` / `minimax-m2.1` / `glm-5` / `qwen3-coder-next`（8 个） |
| 模型清单来源 | `POST https://management.us-east-1.kiro.dev/` + `X-Amz-Target: KiroControlPlaneBearerService.ListAvailableModels` |

**Free 账号的关键前提**：8 个模型**全部没有 `additionalModelRequestFieldsSchema`**。因此任何 `reasoning` / `effort` 类字段都会被上游拒绝——这不是通道问题，是账号能力问题。

**令牌不打印、不硬编码**。所有脚本从 `~/.aws/sso/cache/kiro-auth-token.json` 读取，输出只含状态码、字段名、事件名。

---

## 2. 实测结论一：官方出站 CreateResponse 链路真实可用

官方扩展的 `pfd()` 走 `new URL("/v1/responses", krsEndpoint)` 裸 HTTP（主文档 §2.2.1 / §8.1.1）。本轮直连实测：

```text
POST https://runtime.us-east-1.kiro.dev/v1/responses
Content-Type: application/json
Authorization: Bearer <token>
User-Agent: KiroCLI/2.24.1 KAS/0.58.7
x-amzn-kiro-origin: AI_EDITOR
x-amzn-kiro-profile: arn:aws:codewhisperer:us-east-1:...:profile/EHGA3GRVQMUK

{"model":"claude-sonnet-4.5","input":"Reply with exactly CREATE_RESPONSE_OK.","stream":true}
→ 200  content-type: text/event-stream
```

响应是**标准 OpenAI Responses SSE**，事件名与主文档 §2.2.2 记录的一致：

```
response.created → response.in_progress → response.output_item.added
→ response.content_part.added → response.output_text.delta × N
→ response.output_text.done → response.content_part.done
→ response.output_item.done → response.completed
```

模型确实按指令回了 `CREATE_RESPONSE_OK`。`response.completed` 的 `response` 对象含 `status:"completed"`、`store:false`、`truncation:"disabled"`、`temperature:1.0`、`top_p:1.0`、`max_output_tokens:null`、`reasoning:null`、`output:["message"]`，以及 **Kiro 私有的 usage**：

```json
"usage":{"kiro:credits":{"value":0.0181,"unit":"credit","unit_plural":"credits"},
         "kiro:context_usage_percentage":2.058}
```

**`kiro:credits` 与 `kiro:context_usage_percentage` 是官方文档里没有的字段**，只在实测中出现。

---

## 3. 实测结论二：Free 账号下该端点拒绝一切扩展字段

| 请求字段 | 状态 | 错误 `param` |
| --- | --- | --- |
| `model` / `input` / `stream` | **200** | — |
| `metadata` | **200** | — |
| `origin` | **200** | — |
| `profileArn` | **200** | — |
| `stream: false` | 400 | `stream` |
| `store: true` | 400 | `store` |
| `reasoning` | 400 | `reasoning` |
| `reasoning.effort` | 400 | `reasoning.effort` |
| `instructions` | 400 | `instructions` |
| `temperature` | 400 | `temperature` |
| `top_p` | 400 | `top_p` |
| `tool_choice`（带 tools） | 400 | `tool_choice` |
| `max_output_tokens` | 400 | `max_output_tokens` |

典型错误体：

```json
{"error":{"message":"additionalModelRequestFields is not supported for this model",
          "type":"invalid_request","param":"reasoning.effort","code":"unsupported_parameter"}}
```

```json
{"error":{"message":"Non-streaming responses are not supported for this request. Please set stream=true.",
          "type":"invalid_request","param":"stream","code":"unsupported_parameter"}}
```

```json
{"error":{"message":"systemPrompt field is not enabled for this user.",
          "type":"invalid_request","param":"instructions","code":"unsupported_parameter"}}
```

**跨模型一致性**：`claude-sonnet-4.5` / `claude-haiku-4.5` / `deepseek-3.2` / `glm-5` 四个模型对 `temperature` / `top_p` 的响应**完全一致**（均 400），不存在"某些模型可用"的差异。

`reasoning` 的拒绝与主文档 §11.1 记录的 Free 4.5 原生链路拒绝是**同一条上游校验**（同为 `additionalModelRequestFields is not supported for this model`），只是报错落点不同：原生链路落在 `param:"reasoning.effort"`，CreateResponse 链路落在 `param:"reasoning"`。

---

## 4. 实测结论三：同一上游存在两套错误协议

同一个上游、同一批字段，两条路径的错误形状不同：

| 路径 | 错误体形状 |
| --- | --- |
| `/v1/responses`（OpenAI 风格） | `{"error":{"message","type":"invalid_request","param","code"}}` |
| `POST /` 根路径（Smithy AWS-JSON 风格） | `{"__type":"com.amazon.kiro.runtimeservice#ValidationException","message","reason"}` |

---

## 5. 实测结论四：两种绑定形态与三变体 content-type

### 5.1 `/v1/responses` 的三种 content-type 变体

| 变体 | 结果 |
| --- | --- |
| `application/json`（无 target，官方 `pfd` 形态） | **200** |
| `application/x-amz-json-1.0` + `X-Amz-Target` | **200** |
| `application/x-amz-json-1.0`（无 target） | **200** |

且**带 target 与官方裸 JSON 形态返回字节级相同的响应**（均 `text/event-stream`，均 4199 字节，`response.created` 开头）。即 `X-Amz-Target` 只是被接受，**不改变行为**。

### 5.2 `POST /` 根路径确实绑定了 `KiroRuntimeService.CreateResponse`，但不流式

`nguyen-duc-loc/kiro-gateway` 的探针设计文档假设 `POST /` + `X-Amz-Target: KiroRuntimeService.CreateResponse`。本轮实测：

| 探针 | 结果 |
| --- | --- |
| `POST /` + target（基线） | **200**，`content-type: application/x-amz-json-1.0`，**body 是空对象 `{}`** |
| + `maxOutputTokens` | 400 `ValidationException` |
| + `instructions` | 400 `systemPrompt field is not enabled for this user.` |
| + `Accept: application/vnd.amazon.eventstream` | 仍 200 + `{}` |
| + `Accept: text/event-stream` | 仍 200 + `{}` |
| + `x-amzn-kiro-agent-mode: vibe` | 仍 200 + `{}` |

**该 target 在根路径确实被绑定**（加字段会触发该 target 专属的校验错误），但**绑定了却不返回流**。`Accept` 协商救不回来。

即：社区那份探针假设**部分成立**（target 已绑定），但**未验证出可用响应**。

### 5.3 扩展方法

| 探针 | 结果 |
| --- | --- |
| `GET /v1/responses/{id}` | **404** + **XML** 体：`<UnknownOperationException><Message>The requested operation is not recognized by the service.</Message></UnknownOperationException>` |
| `GET /v1/responses/{id}/input_items` | 同上 404 + XML |
| `DELETE /v1/responses/{id}` | 同上 404 + XML |
| `POST /v1/responses/input_tokens` | **200**，body 为 160 字节 `{"Output":{"__type":"com.amazon.coral.service#UnknownOperationException","message":"The requested operation is not recognized by the service."},"Version":"1.0"}` |
| `POST /v1/responses/compact` | 同上，**完全相同的 160 字节 body** |

- 扩展方法返回 **XML** 错误体，主端点返回 JSON 错误体——**两种错误协议并存**。
- `input_tokens` 与 `compact` 返回**字节级相同**的 UnknownOperation 包装：这两个路径虽被路由，但**上游没有实现**。

### 5.4 `previous_response_id`

传一个**编造的** id（`resp_1234567890abcdef`）返回 **200**——不校验存在性。

---

## 6. 各开源实现实际适配的上游通道清单

本轮逐个拉取源码核实（GitHub 代码检索 + `gh api` 取原文）。**结论：所有 Rust / Go 实现适配的都是 Kiro 的 `generateAssistantResponse` / `SendMessage` 系通道（`conversationState` 请求体 + AWS event-stream 响应），没有一条走官方原生 CreateResponse。**

| 实现 | 通道数 | 端点 / target | 是否 CreateResponse |
| --- | --- | --- | --- |
| `hank9999/kiro.rs`（★1964，主仓库） | 1 | `ide`：`https://q.{api_region}.amazonaws.com/generateAssistantResponse` + MCP `.../mcp` | ❌ |
| `ZyphrZero/kiro.rs`（★489） | 2 | `ide` + `cli`：`https://{host}/`（根路径）、`application/x-amz-json-1.0`、origin `KIRO_CLI`、target `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` | ❌ |
| `Hoshino-Yumetsuki/kiro.rs`（★5） | 2 | `ide` + `cli`；cli 的 target 为 `AmazonQDeveloperStreamingService.SendMessage`、origin `KIRO_CLI` | ❌ |
| `seven7763/kiro.rs` | 1 | 仅 `ide` | ❌ |
| `Quorinex/Kiro-Go` | 4 | 见 §6.1 | ❌ |
| `RyanKung/rotom` | 1（Kiro） | `https://runtime.{region}.kiro.dev/`（根路径）、target `AmazonCodeWhispererStreamingService.GenerateAssistantResponse`、CT `application/x-amz-json-1.0`、Accept `application/vnd.amazon.eventstream`、请求体 `conversationState` | ❌ |
| `nguyen-duc-loc/kiro-gateway` | 1（生产） | `https://runtime.{region}.kiro.dev:443/generateAssistantResponse`、target `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` | ⚠️ 仅**测试探针文档** |
| `sunerpy/kiro-provider` | 2 | 原生 `/v1/responses` + GenerateAssistantResponse fallback | ✅ **唯一实现** |
| `jwadow/kiro-gateway` | 1 | `https://runtime.{region}.kiro.dev:443/generateAssistantResponse`、target `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` | ❌ |
| `doitcan-oiu/kiro.rs-admin` | **4** | `ide` / `runtime`（`runtime.{region}.kiro.dev/generateAssistantResponse`）/ `amazonq` / `codewhisperer` | ❌ |
| `nopperabbo/kiroxy` | 3（fail-over） | `q.{region}.amazonaws.com`（默认）/ `codewhisperer.*` / `amazonq`，legacy 模式走 `runtime.{region}.kiro.dev/` | ❌ |
| `freebattle/kiro.rs` | 2 | `ide` + `cli` | ❌ |
| `jwadow/kiro-gateway` / `aliom-v/KiroGate` / `freebattle/kiro.rs` / `doitcan-oiu/kiro.rs-admin` / `kiroxy` | — | 只做入站 OpenAI/Anthropic 兼容层，出站走 `generateAssistantResponse` | ❌ |

### 6.1 Kiro-Go 的 4 条通道

`proxy/kiro.go` 的端点表（按凭据类型选择，额度耗尽时自动回退）：

| 名称 | URL | Origin | `X-Amz-Target` |
| --- | --- | --- | --- |
| Kiro IDE | `https://q.us-east-1.amazonaws.com/generateAssistantResponse` | `AI_EDITOR` | （无） |
| CodeWhisperer | `https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse` | `AI_EDITOR` | `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` |
| AmazonQ | `https://q.us-east-1.amazonaws.com/generateAssistantResponse` | `AI_EDITOR` | `AmazonQDeveloperStreamingService.SendMessage` |
| Kiro CLI | `https://runtime.{region}.kiro.dev/` | `KIRO_CLI` | `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` |

### 6.2 `hank9999/kiro.rs` 的端点抽象

设计上支持多端点，但当前**只注册了一个**。

`src/kiro/endpoint/mod.rs`：

```rust
pub trait KiroEndpoint: Send + Sync {
    fn name(&self) -> &'static str;
    fn api_url(&self, ctx: &RequestContext<'_>) -> String;
    fn mcp_url(&self, ctx: &RequestContext<'_>) -> String;
    fn decorate_api(&self, req: RequestBuilder, ctx: &RequestContext<'_>) -> RequestBuilder;
    fn transform_api_body(&self, body: &str, ctx: &RequestContext<'_>) -> String;
    ...
}
```

`src/main.rs` 的注册表：

```rust
let mut endpoints: HashMap<String, Arc<dyn KiroEndpoint>> = HashMap::new();
let ide = IdeEndpoint::new();
endpoints.insert(ide.name().to_string(), Arc::new(ide));
```

`src/kiro/endpoint/ide.rs`：

```
API: https://q.{api_region}.amazonaws.com/generateAssistantResponse
MCP: https://q.{api_region}.amazonaws.com/mcp
```

README 亦明写：`defaultEndpoint` 当前支持：`ide`。**该仓库没有 cli / responses 端点实现**（fork 才补上 cli）。

### 6.3 rotom 的 `NativeResponses` 对 Kiro 是名不副实

```rust
fn response_creation_strategy(&self) -> ResponseCreationStrategy {
    ResponseCreationStrategy::NativeResponses      // KiroUpstream
}
```

但 Kiro 的实际请求走的是**另一条**函数：

```rust
async fn send_kiro_body(&self, body: &Value, credentials: &Credentials) -> Result<Response> {
    let upstream_body = kiro::to_kiro_payload(body, credentials)?;   // ← conversationState 形状
    let url = kiro::kiro_endpoint_url(&self.base_url, credentials)?;  // ← https://runtime.{region}.kiro.dev/
    ...
}
```

- `send_body`（用 `responses_url` + `NativeResponses`）只服务 **Grok** 与 **Vercel**；Kiro 走 `send_kiro_body`。
- `to_kiro_payload` 把 OpenAI 风格请求体转成 Kiro 的 `conversationState`（`modelId` / `currentMessage` / `profileArn`），**不是**官方 `cfd` 产出的 OpenAI Responses 形状。
- 因此 rotom 的 Kiro 通道**等价于官方原生 `GenerateAssistantResponse`**，与 CreateResponse 无关。

### 6.4 `kiro-gateway` 的 CreateResponse 只是一份"发现探针"设计文档

`docs/specs/0004-claude-code-bridge/0004-createresponse-discovery.md`，自称假设 `createresponse_awsjson_v1`：

```text
Operation: POST /, no query, X-Amz-Target: KiroRuntimeService.CreateResponse
Destination: https://runtime.us-east-1.kiro.dev:443/
Headers: Content-Type: application/x-amz-json-1.0, X-Amzn-Codewhisperer-Optout: true, ...
Body: {model, input, origin:"KIRO_CLI", instructions, stream:true, maxOutputTokens:1024, profileArn}
```

- 文档明确写「This is a declared hypothesis, not a proven accepted request」，且入口默认关闭（`-response-discovery-launch`）。
- 仓库**非测试代码**里的 `CreateResponse` **命中 0**，只存在于 spec 文档与 `internal/kiro/RESPONSE_DISCOVERY.md`。
- 生产路径是 `internal/kiro/adapter.go`：

```go
req, err := http.NewRequestWithContext(ctx, http.MethodPost, "https://runtime."+region+".kiro.dev:443/generateAssistantResponse", ...)
"Content-Type": "application/x-amz-json-1.0", "X-Amz-Target": "AmazonCodeWhispererStreamingService.GenerateAssistantResponse",
```

- 该探针假设的实测结果见本文 §5.2：**target 已绑定，但不流式**。

---

## 7. 与社区实现的逐字对账

对账对象：`github.com/sunerpy/kiro-provider`（MIT，TypeScript/Bun）。

| 项 | 值 |
| --- | --- |
| 定位 | 本地网关，把 Kiro 账号以 OpenAI Responses / Anthropic Messages 暴露给 Codex CLI、Claude Code 等 |
| 关键文件 | `scripts/probe-v3-create-response.ts`（实探脚本）、`src/server/responses/native-transport.ts`（原生通道）、`src/kiro/constants.ts`（端点常量） |
| 取证文档 | `docs/audits/kiro-cli-provider-v3-wire-diff-2026-09-05.zh.md`、`docs/audits/kiro-provider-v3-openai-responses-validation-2026-09-05.md`、`docs/PROTOCOL_COMPATIBILITY.md` |
| 官方站点镜像 | `firlab.app/kiro-provider/reference/protocol` |
| 被测客户端 | Kiro CLI 2.21.1，KAS 0.58.7 |

把社区的 `native-transport.ts` / `probe-v3-create-response.ts` 与官方扩展原文（主文档 §2.2.1）**逐项对账**，以下全部吻合：

| 维度 | 官方扩展原文 | 社区实现 | 吻合 |
| --- | --- | --- | --- |
| host | `` `https://runtime.${n}.kiro.dev` ``（`resolveTarget`） | `RUNTIME_ENDPOINT: "https://runtime.{{region}}.kiro.dev"` | ✅ |
| 路径 | `new URL("/v1/responses", r)`（`pfd`） | `fetch(`${endpoint}/v1/responses`)` | ✅ |
| content-type | `"Content-Type":"application/json"` | `"Content-Type": "application/json"` | ✅ |
| 鉴权 | `Authorization: Bearer ${token}` | `Authorization: Bearer ${auth.access}` | ✅ |
| origin | `"x-amzn-kiro-origin":a`（`x6i="AI_EDITOR"`） | `"x-amzn-kiro-origin": "AI_EDITOR"` | ✅ |
| profile | `"x-amzn-kiro-profile":s`（有则带） | `"x-amzn-kiro-profile": auth.profileArn`（有则带） | ✅ |
| User-Agent | `"User-Agent":e.userAgent` | `"User-Agent": "KiroCLI/2.21.1 KAS/0.58.7 ..."` | ✅ |
| 请求体 | `cfd` 产出 `{model,input,instructions,tools,stream,origin,profileArn,metadata,reasoning}` | 转发 `model/input/instructions/tools/tool_choice/stream/max_output_tokens/temperature/top_p/truncation/reasoning/previous_response_id` | ✅ |

**官方 `pfd` 比社区实现多发两个请求头**（`x-amzn-codewhisperer-optout`、`TokenType`），社区均未带；反向则是官方 `pfd` **不带** `x-amzn-kiro-agent-mode`（该头在 Smithy 中间件里，只作用于 `GenerateAssistantResponse`）。

**`origin` 取值全集**（官方 `Vfd`）：

```js
Vfd={"kiro-ide":"AI_EDITOR","kiro-web":"KIRO_WEB","kiro-cli":"KIRO_CLI"}
```

### 7.1 一处社区与官方的**实质差异**

官方 `lfd`（主文档 §8）把 AMRF 的 `thinking.type` 映射为 `reasoning.type` 一并下发；而社区原生通道只设 `reasoning.effort` 与 `reasoning.summary`，**不发 `reasoning.type`**：

```ts
if (typeof request.reasoning.effort === "string") reasoning.effort = request.reasoning.effort;
if (typeof request.reasoning.summary === "string" && request.reasoning.summary !== "none") reasoning.summary = request.reasoning.summary;
```

- 官方：原生 AMRF 里 `thinking` 信封恒为 `{thinking:{type}}`（主文档 §7），而 `/v1/responses` 链路会把 `thinking.type` 摊平成 `reasoning.type`（主文档 §8）——**两条链路都发 type**。
- 社区：原生通道不发 `reasoning.type`。
- 这说明社区实现**没有逐字复刻官方 `lfd`**，只复刻了它自认为必要的子集。**若要实现这条链路，应以官方 `lfd` 为准，而不是以社区实现为参照。**

---

## 8. 社区报告 vs 本机实跑

社区的活体探针结果（来自其审计文档）：

| 探针 | 社区报告 |
| --- | --- |
| 原生非流式 | HTTP 200，标准 OpenAI `response` 对象 |
| 原生流式 | HTTP 200，标准 Responses SSE 事件序列 |
| function tool | HTTP 200，`function_call` 输出 |
| `previous_response_id` | HTTP 200，历史正确回放 |
| 原生 retrieve / input-items / delete | **HTTP 404** |
| `/v1/responses/input_tokens` | HTTP 200，body 是 `Output{__type,message}/Version`（160 字节） |
| `/v1/responses/compact` | HTTP 200，同上，body hash 相同 |
| `store: false` | 上游仍回报 `store: true` |
| `reasoning.effort: xhigh` | 原生可用 |
| `reasoning.effort: max` | **上游拒绝** |
| `top_p` | 上游 **500** |
| `temperature` | 模型相关（GPT 拒绝，Claude 可用） |
| Structured Outputs | 上游**静默忽略** |
| `instructions` | 原生可用 |

**与本机实跑的差异**（本机为 Free 账号，社区疑似付费账号）：

| 项 | 社区报告 | 本机实跑 |
| --- | --- | --- |
| **非流式** | 200，返回 OpenAI `response` 对象 | **400**：`Non-streaming responses are not supported for this request.` |
| `store: false` | 上游仍回报 `store: true` | **正常**，回报 `store:false` |
| `store: true` | （未测） | **400**：`Persisting responses (store=true) is not supported for this request.` |
| `reasoning.effort` | `xhigh` 可用 | **400**：`additionalModelRequestFields is not supported for this model` |
| `reasoning.type` | （未测） | **400**：同上报错，`param:"reasoning"` |
| `instructions` | 原生可用 | **400**：`systemPrompt field is not enabled for this user.` |
| `top_p` | 上游 **500** | **400**：`This request contains parameters or input that are not supported.` |
| `temperature` | 模型相关（Claude 可用） | **400**，且**跨模型一致**（四模型全拒） |
| function tool | 200，`function_call` 输出 | **400**：`This request contains parameters or input that are not supported.` |
| `max_output_tokens` | （未测） | **400**：`param:"max_output_tokens"` |
| `metadata` / `origin` / `profileArn` | （未测） | **均 200** |

**复现成功的社区结论**：扩展方法 404、160 字节 Smithy 包装、`input_tokens` 与 `compact` body 相同。

**未能复现的**：非流式 200、`store:false` 回报 true、`temperature` 模型相关、`top_p` 500、function tool 200。

> **结论：社区结论不可当作通用事实。** 差异很可能源于账号类型——社区用付费 / Pro 账号，本机为 Free。引用社区数据时必须标注账号前提。

---

## 9. 官方扩展里 CreateResponse 的位置（关键反证）

这一节是**官方源码事实**，主文档结论第 17 条曾以 §8.3.3 收录（该节已迁入本文），此处保留以便与对账结论对照。

官方扩展**没有** `CreateResponse` 的 Smithy command：

```js
.s("KiroRuntimeService","GenerateAssistantResponse")
.s("KiroRuntimeService","GetFeatureConfiguration")
.s("KiroRuntimeService","InvokeMCP")
.s("KiroRuntimeService","InvokeMCPStream")
```

- `KiroRuntimeService` 的 operation 只有这 4 个——**没有 `CreateResponse`**。
- 因此 CreateResponse 走的是**裸 HTTP**（`pfd` 用 `fetch`/`http` 直连），不走 Smithy RPC 序列化，**也就不带 `x-amz-target`**。
- 官方扩展里的 `x-amz-target` 只有 4 处，全部在 Smithy RPC 序列化器内自动拼接：`n.headers["x-amz-target"]=`${this.serviceTarget}.${t.name}``，`serviceTarget` 即 `KiroRuntimeService`。
- 扩展里另有两个相关键：`thd="kiro-runtime:CreateResponse"`（路由 / 遥测键）、`xfd=Ct(bt.Agent,"CreateResponseSse")`（遥测 metric 名）。
- 社区探针脚本显式试了三种 content-type 变体，其中包含 `"X-Amz-Target": "KiroRuntimeService.CreateResponse"` + `application/x-amz-json-1.0`——这是**社区在验证 target 形式是否可用**，而官方实际用的是裸 `application/json` 无 target。
- 本轮实测进一步确认：**加不加 target，`/v1/responses` 的响应字节级相同**（§5.1）；但 `POST /` 根路径的 target 绑定**不流式**（§5.2）。

---

## 10. 图像与 web 搜索：上游到底支持什么

社区实现普遍宣称支持「搜索 + 图片」。本轮对两者分别做了**上游直连实测**，结论与直觉差别很大。

### 10.1 图像：Kiro 上游原生支持视觉，实测通过

`generateAssistantResponse` 的 `userInputMessage.images` 字段是官方协议的**一部分**（主文档 §3 提到的 `fhd` 形状：`userInputMessage:{content,modelId,origin,images}`），每个图像块形状为：

```json
{"format": "png", "source": {"bytes": "<base64>"}}
```

**实测结果**（用纯 zlib 手写 PNG 编码器生成纯色图，无第三方依赖）：

| 输入 | 模型回答 |
| --- | --- |
| 64×64 纯红 PNG | `Red` |
| 64×64 纯蓝 PNG | `Blue` |
| 同样问题但**不带** `images` | `I don't see any image attached to your message. Could you share the image you'd like me to analyze?` |

- 三个探针都 **200**。对照组（不带图）明确说"没看到图片"，证明**模型确实读到了图像像素**，不是靠 prompt 猜。
- 结论：**图像支持是上游原生能力，不需要额外通道**。

**社区实现对照**：

| 实现 | 图像支持 | 备注 |
| --- | --- | --- |
| `aliom-v/KiroGate` | ✅ | `kiro_gateway/converters.py` 的 `extract_images_from_content()`，同时解析 OpenAI `image_url` 与 Anthropic `image` 两种入站格式 |
| `nopperabbo/kiroxy` | ✅ | 专门的 `internal/reqconv/images.go` + `docs/VISION.md` |
| `jwadow/kiro-gateway` | ❌ | 仓库无任何 image / base64 相关模块 |
| `hank9999/kiro.rs` | ❌ | 无图像相关文件 |
| `bestK/kiro2cc` | ❌ | 无图像相关文件 |

`aliom-v/KiroGate` 的转换逻辑与 Kiro 协议形状一致：

```python
# 提取 media_type，例如 "image/png"
media_type = header.split(":")[1].split(";")[0]
# 提取格式，例如 "png"
img_format = media_type.split("/")[1] if "/" in media_type else "png"
images.append({"format": img_format, "source": {"bytes": data}})
```

`kiroxy` 的 Go 实现同构：

```go
format := b.Source.MediaType
if idx := strings.LastIndex(format, "/"); idx >= 0 {
    format = format[idx+1:]
}
```

**两者都与本项目 `gateway/models/kiro.rs` 的 `ImageBlock` 定义一致**（`{format, source:{bytes}}`）——即本项目在协议层已对齐。

### 10.2 web 搜索：Kiro 上游**没有**这个能力，必须自己执行

这是本节最重要的结论。分三层说清。

**第一层：上游把 `web_search` 当普通工具规格处理，不提供搜索能力。**

实测（`generateAssistantResponse`，把 `web_search` 作为 `toolSpecification` 下发）：

| 下发的工具名 | 状态 | 上游行为 |
| --- | --- | --- |
| `getWeather`（自定义工具） | 200 | 模型返回 `tool_use`，`name:"getWeather"` |
| `webSearch` | 200 | 模型返回 `tool_use`，`name:"webSearch"` |
| `web_search` | 200 | 模型返回 `tool_use`，`name:"web_search"` |
| `My Weird Tool`（含空格） | **400** | `Invalid tool use format.` / `REQUEST_BODY_INVALID` |

- **上游对工具名不做白名单校验**（只校验格式：不能有空格等非法字符）。`web_search` 被完全当作普通自定义工具接受。
- 模型**发起了 tool_use，但没有执行它**——上游不做搜索。工具执行必须由调用方完成。
- 附带结论：上游会校验工具名字符集，含空格的直接 400。这也解释了本项目 `sanitize_tool_name` 的必要性。

**第二层：真正的搜索能力在 Kiro 的远程 MCP 工具上。**

实测 `POST https://runtime.us-east-1.kiro.dev/mcp`（JSON-RPC 2.0）：

```json
{"jsonrpc":"2.0","id":"web_search_tooluse_<hex>","method":"tools/call",
 "params":{"name":"web_search","arguments":{"query":"Kiro IDE"}}}
```

| 主机 / 工具名 | 结果 |
| --- | --- |
| `runtime.*.kiro.dev/mcp` + `web_search` + `x-amzn-kiro-profile-arn` 头 | **200**，返回真实搜索结果（`results[].title/url/snippet`） |
| `runtime.*.kiro.dev/mcp` + 无 profile 头 | **400** `profileArn is required for this request.` |
| `runtime.*.kiro.dev/mcp` + `webSearch` | 400（工具名不匹配） |
| `q.us-east-1.amazonaws.com/mcp` + `web_search` | **200**，同样返回真实结果 |
| `q.us-east-1.amazonaws.com/mcp` + `webSearch` | 200 但 `{"error":{"code":-32602,"message":"Tool not found"}}` |

- **工具名必须精确是 `web_search`**（下划线），`webSearch` 报 "Tool not found"。
- `runtime` 主机需要 `x-amzn-kiro-profile-arn` 头（或 body 里带 `profileArn`）。
- 这是 **JSON-RPC 2.0**，不是 AWS JSON 协议——`X-Amz-Target` 不适用。
- 响应形状：`result.content[0].text` 是 **JSON 字符串**（不是对象），内含 `{"results":[...],"totalResults":N,"query":"..."}`。

**第三层：官方客户端也是这么做的。**

官方扩展里 `web_search` 是**客户端内置工具**（`$ti=[...,"web_search",...]`），配合别名表 `webSearch→web_search`、`webFetch→web_fetch`；官方扩展同时有 `InvokeMCP` operation（target `KiroRuntimeService.InvokeMCP`）。也就是说官方 IDE 同样是**调远程 MCP 拿搜索结果**，而不是指望模型端自带搜索。

### 10.3 社区实现的做法：两条路

| 实现 | 做法 | 是否绕过主链路 |
| --- | --- | --- |
| `jwadow/kiro-gateway` | 专门的 `kiro/mcp_tools.py`：`call_kiro_mcp_api()` 调 `{q_host}/mcp` 的 `tools/call`；`handle_native_web_search()` 注释明写「**This function bypasses /generateAssistantResponse entirely.** Direct MCP API call → SSE emulation → return to client.」 | ✅ 完全绕开主生成链路 |
| `kiroxy` | 端点 fail-over + 独立的 `internal/reqconv` 转换层 | 走主链路 |
| 本项目 | 专门的 `gateway/proxy/websearch.rs`（657 行） | 走主链路，但**改写**子请求 |

本项目 `websearch.rs` 的注释记录了完整协议取证结论（2026-09-28 实测），与本轮实测**完全一致**：

```text
POST https://runtime.{region}.kiro.dev/mcp
Authorization: Bearer <access_token>
content-type: application/json
accept: application/json, text/event-stream
x-amzn-kiro-profile-arn: <profile_arn>

{"jsonrpc":"2.0","id":"...","method":"tools/call",
 "params":{"name":"web_search","arguments":{"query":"..."}}}
```

本项目的常量也与实测一致：

```rust
const WEB_SEARCH_TOOL_NAME: &str = "web_search";        // MCP 侧用的名字
const WEB_SEARCH_SANITIZED_NAME: &str = "webSearch";    // 归一化后的名字
```

**本轮实测复核结论**：本项目的 `call_remote_web_search()` 走的正是 `runtime.{region}.kiro.dev/mcp` + JSON-RPC 2.0 + `params.name = "web_search"` + `x-amzn-kiro-profile-arn` 头——**实测 200 并返回真实搜索结果**。项目实现与上游现状一致，无需改动。

### 10.4 两个能力的性质对比

| | 图像 | web 搜索 |
| --- | --- | --- |
| 上游原生支持 | ✅ 是（`userInputMessage.images` 是协议字段） | ❌ 否（只是普通工具规格） |
| 需要额外通道 | 不需要 | 需要：远程 MCP `tools/call` |
| 上游会执行吗 | 会（模型直接看像素） | 不会（只发 tool_use，需调用方执行） |
| 本项目状态 | 已实现（`converter/images.rs` + `models/kiro.rs` 的 `ImageBlock`） | 已实现（`proxy/websearch.rs`） |

**一句话**：**图片是上游白送的，搜索是要自己接 MCP 的。** 社区项目宣传的"搜索 + 图片"其实是两件性质完全不同的事。

### 10.5 顺带验证的两条协议细节

**① `generateAssistantResponse` 的请求体必须带顶层 `profileArn`。**

实测错误信息的演进很有信息量：

| 请求体 | 上游响应 |
| --- | --- |
| 缺 `conversationState` 包装 | 400 `Improperly formed request.` / `REQUEST_BODY_INVALID` |
| 有 `conversationState` 但缺 `profileArn` | 400 `profileArn is required for this request.` |
| 两者齐全 | **200** |

- 早期排查时，**带工具规格**的请求报的是 `Invalid tool use format.`，而**不带工具**的报 `Improperly formed request.`——两种错误消息说明上游在**两个不同阶段**做了校验：先解析整体结构，再校验工具部分。这个差异可用于快速定位是哪一环出了问题。
- `conversationId` 必须用 UUID 格式（非 UUID 会 400）。
- `modelId` 必须是上游认的模型 id，`claude-sonnet-4-5`（连字符）会 400 `Invalid model ID.`，而清单里的 `claude-sonnet-4.5`（点）正常。

**② 工具名字符集受上游校验。**

含空格的 `My Weird Tool` → 400 `Invalid tool use format.`。合法字符集不含空格。这从上游侧印证了本项目 `sanitize_tool_name`（把 `_`/`-` 转 camelCase）的必要性。

---

## 11. 上游有没有 Anthropic Messages 协议

**没有。** Kiro 上游**不提供** Anthropic Messages 协议（`/v1/messages`）。本节给出三条独立证据。

### 11.1 官方扩展：Smithy service 穷举，没有 messages operation

官方扩展的 Smithy service/operation 定义可完整枚举（`grep -o '\.s("[^"]*","[^"]*")'`），全部结果：

| service | operation |
| --- | --- |
| `AmazonCodeWhispererService` | `CreateArtifactUploadUrl` / `CreateSubscriptionToken` / `CreateTaskAssistConversation` / `CreateUploadUrl` / `CreateUserMemoryEntry` / `CreateWorkspace` / `DeleteTaskAssistConversation` / `DeleteUserMemoryEntry` / `DeleteWorkspace` / `GenerateCompletions` / `GetCodeAnalysis` / `GetCodeFixJob` / `GetProfile` / `GetRetrievals` / `GetTaskAssistCodeGeneration` / `GetTestGeneration` / `GetTransformation` / `GetTransformationPlan` / `GetUsageLimits` / `ListAvailableCustomizations` / `ListAvailableModels` / `ListAvailableProfiles` / `ListAvailableSubscriptions` / `ListCodeAnalysisFindings` / `ListEvents` / `ListFeatureEvaluations` / `ListUserMemoryEntries` / `ListWorkspaceMetadata` / `PushTelemetryEvent` / `ResumeTransformation` / `SendTelemetryEvent` / `SetUserPreference` / `StartCodeAnalysis` / `StartCodeFixJob` / `StartTaskAssistCodeGeneration` / `StartTestGeneration` / `StartTransformation` / `StopTransformation` / `UpdateUsageLimits` |
| `AmazonCodeWhispererStreamingService` | `ExportResultArchive` / `GenerateAssistantResponse` / `GenerateTaskAssistPlan` / `InvokeMCP` / `InvokeMCPStream` / `SendMessage` |
| `KiroRuntimeService` | `GenerateAssistantResponse` / `GetFeatureConfiguration` / `InvokeMCP` / `InvokeMCPStream` |
| `AWSSSOOIDCService` | `CreateToken` / `CreateTokenWithIAM` / `RegisterClient` / `StartDeviceAuthorization` |

- **没有任何 `messages` 类 operation**。唯一名字里带 "Message" 的是 `AmazonCodeWhispererStreamingService.SendMessage`——但那是 **CodeWhisperer 的流式对话 operation**（与 `GenerateAssistantResponse` 并列的私有协议），不是 Anthropic Messages API。
- 官方扩展里 `v1/messages` 字面量命中 **0**。
- 官方扩展里 `anthropic` 只有 4 处，**全部与 Anthropic 协议无关**：
  - `tbr=xn([Et("anthropic"),Et("openai"),Et("azure"),Et("vertex"),Et("bedrock"),pe()])` —— ACP（Agent Client Protocol）的 `apiType` 枚举
  - `globalThis.lc_block_translators_registry` 里注册 `"anthropic"` → `Npi` —— **langchain_core** 的 block translator
  - langchain 的 "New LangChain packages are available…" 提示
  - langchain OTel 的 `setGenAiSystem`（按模型名推断 telemetry 字段命名）

  即：`anthropic` 在官方扩展里是 **langchain 的多供应商适配 + ACP 协议枚举**，与 Kiro 上游协议无关。
- 官方扩展里的 `/v1/` 路径只有 5 个：`/v1/`（根）、`/v1/metrics`、`/v1/responses`、`/v1/token`、`/v1/traces`。**没有 `/v1/messages`**。

### 11.2 实跑探测：15 个路径全部未实现

**判据很关键**：上游对**未实现的路径**返回 **200 + 160 字节**的固定兜底体：

```json
{"Output":{"__type":"com.amazon.coral.service#UnknownOperationException",
           "message":"The requested operation is not recognized by the service."},
 "Version":"1.0"}
```

所以**不能只看状态码**。本轮用已知可用的路径做对照：

| 对照项 | 状态 | 响应体 |
| --- | --- | --- |
| `/generateAssistantResponse`（AWS-JSON） | 200 | 3050 字节，**真实 eventstream** |
| `/v1/responses`（OpenAI Responses） | 200 | 7014 字节，**真实 SSE** |
| `/mcp`（JSON-RPC） | 200 | 5908 字节，**真实搜索结果** |

待判定的 15 个组合（3 主机 × 5 路径）**全部**返回同一个 160 字节兜底体：

| 路径 | `runtime.*.kiro.dev` | `q.*.amazonaws.com` | `codewhisperer.*.amazonaws.com` |
| --- | --- | --- | --- |
| `/v1/messages` | 未实现 | 未实现 | 未实现 |
| `/messages` | 未实现 | 未实现 | 未实现 |
| `/v1/chat/completions` | 未实现 | 未实现 | 未实现 |
| `/v1/complete` | 未实现 | 未实现 | 未实现 |
| `/anthropic/v1/messages` | 未实现 | 未实现 | 未实现 |

补充验证：
- 加 `anthropic-version: 2023-06-01` 头 → 仍是未实现。
- 加 `x-amzn-kiro-profile` 头 → 仍是未实现。
- `/v1/responses` 加同样两个头 → **200 真实响应**（说明头不是变量）。

**结论**：Kiro 上游**只有两种对话协议**——Kiro 私有 `generateAssistantResponse`（`conversationState` + eventstream）与 OpenAI Responses（`/v1/responses`）。**没有 Anthropic Messages。**

### 11.3 社区实现：messages 全是入站

社区项目清一色把 `/v1/messages` 当**入站兼容层**（让 Claude Code 能接进来），出站一律打上游私有协议：

| 实现 | `/v1/messages` | 出站端点 |
| --- | --- | --- |
| `jwadow/kiro-gateway` | **入站**路由 | `url = f"{auth_manager.api_host}/generateAssistantResponse"` |
| `aliom-v/KiroGate` | **入站**路由（`/v1/messages` + `/v1/chat/completions` + `/v1/messages/count_tokens`） | `generateAssistantResponse` |
| `nopperabbo/kiroxy` | **入站**（`internal/convert/anthropic.go`） | `generateAssistantResponse` / `/mcp` |
| 本项目 | **入站**路由（`anthropic_messages_handler`） | `build_generate_assistant_response_url()` |

`jwadow` 的路由文件头注释写得很清楚：

```python
Contains the /v1/messages endpoint compatible with Anthropic's Messages API.
```

它后面 `url = f"{auth_manager.api_host}/generateAssistantResponse"`——**入站是 Anthropic 协议，出站是 Kiro 私有协议**，中间做转换。

### 11.3.1 复核（2026-10-22）：OpenAI Chat 协议的适配情况

问题：「社区的 Kiro 反代适配了 chat 协议吗」。对 6 个仓库按**当前 HEAD**逐字核验路由注册与出站端点：

| 仓库 | `/v1/chat/completions` 入站 | 出站端点 | 核验 |
| --- | --- | --- | --- |
| [`xu-yuanhao/KiroProxy`](https://github.com/xu-yuanhao/KiroProxy) | **有**（`kiro_proxy/main.py:148`） | `handlers/openai.py` → `convert_openai_messages_to_kiro` + `build_kiro_request` → `generateAssistantResponse` | **深** |
| [`aliom-v/KiroGate`](https://github.com/aliom-v/KiroGate) | **有**（`main.ts:889`） | `handleChatCompletions` → `request_handler.py:472` `f"{api_host}/generateAssistantResponse"`（converters 源自 `Jwadow/kiro-openai-gateway`） | **深** |
| [`justlovemaki/AIClient2API`](https://github.com/justlovemaki/AIClient2API) | 服务器同时声明 OpenAI-compatible（`chat/completions` + `responses`）与 Claude 两套路由族（`api-server.js:338-340`） | Kiro provider `providers/claude/claude-kiro.js:43-45`：`q.{region}.amazonaws.com` / `codewhisperer.{region}.amazonaws.com` 的 `generateAssistantResponse` | **中**（chat 入站与 Kiro provider 的绑定关系未逐字核） |
| [`TsinHzl/kiro2cc-proxy`](https://github.com/TsinHzl/kiro2cc-proxy) | **无**（master 路由仅 `/v1/messages` + `/v1/messages/count_tokens` + `/v1/models`，converter 只有 `src/anthropic/`） | `generateAssistantResponse` | **深** |
| [`hank9999/kiro.rs`](https://github.com/hank9999/kiro.rs) | **无**（仅 `src/anthropic/router.rs`） | `generateAssistantResponse`（`ide` 端点） | **深** |
| [`GreyGunG/Kiro-RS-Tool`](https://github.com/GreyGunG/Kiro-RS-Tool) | **无**（仅 `src/anthropic/router.rs`） | `generateAssistantResponse` | **深** |

分型结论：

- **Python / TS 多协议项目**（KiroProxy、KiroGate、AIClient2API）：chat 协议只是**入站兼容层**——收到 OpenAI Chat 请求后转成 `conversationState` 打私有协议，与 §11.3 的 messages 形态相同。
- **Rust 系（kiro.rs 血统）**：只有 Anthropic Messages 入站，没有 chat 入站。
- **没有任何实现把 chat 协议打向上游**——上游本就不提供（§11.2 实测 `/v1/chat/completions` 全部未实现）。
- 出站也没有走其它对话通道：全部收敛到 `generateAssistantResponse`（AIClient2API 的 `codewhisperer.*` URL 同样指向 `generateAssistantResponse`，只是 host 不同）。
- 本项目（`gateway/mod.rs:1150`）同样提供 `/v1/chat/completions` 入站（`openai_chat_handler`），形态与社区一致。

### 11.3.2 复核（2026-10-22）：chat 入站的 thinking 映射

三家带 chat 入站的项目都支持 thinking，但机制分两代：

| 项目 | chat 入站的 thinking 字段 | 出站机制 | 档位 |
| --- | --- | --- | --- |
| `xu-yuanhao/KiroProxy` | `extra_body.anthropic.thinking`（Anthropic 形态嵌在 OpenAI 请求体内，`handlers/openai.py:49`） | **XML 提示词前缀**注入 system 文本（`converters.py:116-148`：`inject_thinking_prefix`） | enabled → `<thinking_mode>enabled</thinking_mode><max_thinking_length>{budget}</max_thinking_length>`；adaptive → `<thinking_mode>adaptive</thinking_mode><thinking_effort>{effort}</thinking_effort>`（low/medium/high，默认 high） |
| `aliom-v/KiroGate` | Anthropic 形态 `thinking`（dict/bool/str 三形态，`converters.py:57-80`） | **XML 提示词标签**注入 system prompt（`generate_thinking_hint`，`converters.py:106-137`） | `<thinking_mode>enabled</thinking_mode><max_thinking_length>{budget}</max_thinking_length>`；具体 budget 默认值未核到 |
| `justlovemaki/AIClient2API` | OpenAI 原生 `reasoning_effort`（另认 `body.thinking` / `body.output_config`，`claude-kiro.js:2002/2607`） | **additionalModelRequestFields（AMRF）**（`kiro-effort.js`） | GPT（gpt-5.6-sol/terra/luna）→ `{reasoning:{effort}}`，档位 none/low/medium/high/xhigh/max，`thinking.type==='disabled'→none`；Claude → `{output_config:{effort}}`，档位按模型静态枚举（4.6 系无 xhigh），enabled/adaptive 无显式档 → medium；**模型不在枚举内整个字段省略**（注释明说：Kiro 不忽略未知字段，发未知 AMRF 会 400） |

分代结论：

- KiroProxy / KiroGate 用的是**旧代机制**——`<thinking_mode>` / `<max_thinking_length>` 提示词标签。官方客户端**不发送**这些 XML 标签（官方事实见《Kiro 官方 thinking 与 effort 事实》），这是社区对早期上游行为的遗留适配。
- AIClient2API 用的是**新代机制**——AMRF 是否发送按模型 schema 决定、档位按模型枚举，`reasoning.effort`（GPT 系）/ `output_config.effort`（Claude 系）的嵌套位置与官方 `pfd` 取证一致。其"不支持 AMRF 的模型整个省略字段"与本项目的实现规则相同（schema 存在才发，下限保护一致思路）。
- 响应侧：AIClient2API 把上游 `reasoningContentEvent` 流映射回 chat 的 thinking/reasoning 输出通道（`claude-kiro.js:2504/2957/3287`），并在流尾显式关闭 thinking 块（原生事件不带 `</thinking>` 标签）。KiroProxy / KiroGate 的 chat **响应侧** reasoning 输出未核到证据（本轮 grep 仅命中请求侧）。
- KiroGate 的 chat 路径**不读** OpenAI 原生 `reasoning_effort`（`request_handler.py` 无该字段引用）——OpenAI 风格客户端要用 thinking 只能传 Anthropic 形态字段；KiroProxy 同理只认 `extra_body.anthropic.thinking`。

### 11.4 汇总：上游协议清单

| 协议 | 端点 | 请求体 | 响应 | 谁在用 |
| --- | --- | --- | --- | --- |
| **Kiro 私有** | `/generateAssistantResponse` | `conversationState`（AWS-JSON 1.0 + `X-Amz-Target`） | AWS eventstream | 官方客户端、**所有**社区实现、本项目 |
| **OpenAI Responses** | `/v1/responses` | OpenAI Responses（裸 JSON） | OpenAI SSE | 官方客户端（flag 开启时）、`sunerpy/kiro-provider` |
| **JSON-RPC** | `/mcp` | JSON-RPC 2.0 `tools/call` | JSON-RPC 响应 | 官方客户端（`InvokeMCP`）、本项目 `websearch.rs`、社区 `mcp_tools.py` |
| **Anthropic Messages** | — | — | — | **上游不提供** |

**对本项目的意义**：项目的 `/v1/messages` 与 `/v1/responses` 都是**入站**兼容层，出站只有 `generateAssistantResponse`（+ `/mcp` 供 WebSearch 用）——**与上游协议现状完全吻合**，无需为「上游不支持 messages」做任何适配。

---

## 12. 结论汇总

> 以下均为**非官方事实**：来自社区实现源码与本机活体实测。官方事实见《Kiro 官方 thinking 与 effort 事实》。

1. **官方出站 CreateResponse 链路真实可用**（§2）：Python 直打 `https://runtime.us-east-1.kiro.dev/v1/responses` 返回 200 + 标准 OpenAI Responses SSE，事件名与官方 §2.2.2 完全一致。**不是"仅存代码"。**
2. **该端点只支持流式**（§3）：`stream:false` → 400 `unsupported_parameter`（`param:"stream"`）。官方 `pfd` 恒发 `stream:!0`，与此一致。
3. **该端点同时接受 Smithy RPC 与裸 JSON 两种形态**（§5.1）：三种 content-type/target 组合均返回 200，且**带 target 与官方形态响应字节级相同**。
4. **Free 账号下该端点拒绝一切推理/采样类扩展字段**（§3）：`reasoning` / `instructions` / `temperature` / `top_p` / `tool_choice` / `max_output_tokens` 全部 400；`metadata` / `origin` / `profileArn` 被接受。
5. **`reasoning` 的拒绝与原生链路 Free 拒绝是同一条上游校验**（§3）：同为 `additionalModelRequestFields is not supported for this model`，只是报错落点不同（原生落 `param:"reasoning.effort"`，CreateResponse 落 `param:"reasoning"`）。
6. **同一上游存在两套错误协议**（§4）：`/v1/responses` 报 OpenAI 风格 `{"error":{message,type,param,code}}`，根路径 AWS-JSON 绑定报 Smithy 风格 `{"__type":"com.amazon.kiro.runtimeservice#ValidationException",message,reason}`。扩展方法则返回 XML。
7. **`POST /` 根路径确实绑定了 `KiroRuntimeService.CreateResponse`，但不流式**（§5.2）：返回 200 + `application/x-amz-json-1.0` + body `{}`；`Accept` 协商 `application/vnd.amazon.eventstream` / `text/event-stream` 均不改变结果。`kiro-gateway` 的 `createresponse_awsjson_v1` 假设**部分成立**（target 已绑定），但未验证出可用响应。
8. **扩展方法未实现**（§5.3）：retrieve / input-items / delete 均 404 + XML；`input_tokens` 与 `compact` 返回**字节级相同**的 UnknownOperation 包装（160 字节）。
9. **`previous_response_id` 不校验存在性**（§5.4）：传编造 id 仍返回 200。
10. **所有 Rust / Go 开源实现适配的都是 `generateAssistantResponse` / `SendMessage` 系通道，没有一条走官方原生 CreateResponse**（§6）。唯一实现 CreateResponse 的是 `sunerpy/kiro-provider`（TypeScript）。`hank9999/kiro.rs` 只注册了 `ide` 一个端点；`ZyphrZero` / `Hoshino-Yumetsuki` 两个 fork 补了 `cli` 端点；`Quorinex/Kiro-Go` 有 4 条通道；`rotom` 的 Kiro 走 `send_kiro_body`（`conversationState` 形状），其 `ResponseCreationStrategy::NativeResponses` 标注只对 Grok / Vercel 生效。
11. **社区实现与官方源码逐字吻合**（§7）：host、路径、content-type、鉴权、`x-amzn-kiro-origin`/`x-amzn-kiro-profile`、`User-Agent`、请求体字段集全部一致。社区没有虚构端点。
12. **一处社区与官方的实质差异**（§7.1）：官方 `lfd` 会把 `thinking.type` 摊平为 `reasoning.type` 一并下发，社区原生通道**不发** `reasoning.type`。**若要实现这条链路，应以官方 `lfd` 为准，不要以社区实现为参照。**
13. **社区报告的若干结论在本机未复现**（§8）：非流式 200、`store:false` 回报 `true`、`temperature` 模型相关（实测四模型一致拒绝）、`top_p` 500（实测 400）、function tool 200。**社区结论不可当作通用事实**；差异很可能源于账号类型（社区用付费/Pro 账号，本机为 Free）。
14. **上游不提供 Anthropic Messages 协议**（§11）：官方扩展的 Smithy operation 全集里没有任何 messages 类 operation；15 个路径组合（3 主机 × 5 路径）实测全部返回 `UnknownOperation` 兜底体。上游只有三种协议——Kiro 私有 `generateAssistantResponse`、OpenAI Responses `/v1/responses`、JSON-RPC `/mcp`。
15. **社区与本项目的 `/v1/messages` 都是入站兼容层**（§11.3）：出站一律打 `generateAssistantResponse`。本项目出站端点集合与上游协议现状完全吻合，无需为「上游不支持 messages」做适配。
16. **图像是上游原生能力，web 搜索不是**（§10）：图像走 `userInputMessage.images` 协议字段，模型直接读像素（实测纯红图回 `Red`、纯蓝图回 `Blue`、无图对照组明确说没看到）；web 搜索上游只当普通工具规格、只发 `tool_use` 不执行，真正的搜索在远程 MCP `tools/call`（工具名必须精确为 `web_search`）。
17. **上游对未实现路径返回 200 + 160 字节 `UnknownOperation` 兜底体**（§11.2）：**不能只看状态码判断端点是否存在**，必须看响应体。本项目所有出站探测都应遵守此判据。
18. **社区对 OpenAI Chat 协议的适配是「入站转换」而非「上游通道」**（§11.3.1，2026-10-22 复核）：Python/TS 多协议项目（`xu-yuanhao/KiroProxy`、`aliom-v/KiroGate`、`justlovemaki/AIClient2API`）提供 `/v1/chat/completions` 入站，收到后转 `conversationState` 打私有协议；Rust 系（kiro.rs 血统：`hank9999/kiro.rs`、`GreyGunG/Kiro-RS-Tool`、`TsinHzl/kiro2cc-proxy`）只有 Anthropic Messages 入站、没有 chat 入站。**没有任何实现把 chat 协议打向上游**——上游本就不提供（§11.2）。本项目 `/v1/chat/completions` 入站（`gateway/mod.rs:1150`）与社区形态一致。
19. **chat 入站的 thinking 映射分两代**（§11.3.2，2026-10-22 复核）：KiroProxy / KiroGate 走**旧代**——把 Anthropic 形态 `thinking` 转成 `<thinking_mode>` / `<max_thinking_length>` XML 提示词标签注入 system（官方客户端不发送这些标签，属社区遗留机制）；`justlovemaki/AIClient2API` 走**新代**——认 OpenAI 原生 `reasoning_effort`，按模型 schema 构造 AMRF（GPT 系 `{reasoning:{effort}}`，Claude 系 `{output_config:{effort}}`），模型不在枚举内则整个省略字段。KiroGate / KiroProxy 的 chat 路径**不读** OpenAI 原生 `reasoning_effort`。

---

## 13. 复核方法

### 13.1 本机实测

凭据取自 `~/.aws/sso/cache/kiro-auth-token.json`（Kiro CLI 的 SSO 缓存）。令牌过期时需先在 Kiro CLI 里刷新登录。脚本不打印令牌。

```bash
# 拉上游模型清单（management 端点）
python .git/kam-tmp/list_models.py

# 主探针：流式 + 各字段变体
python .git/kam-tmp/probe_create_response.py

# 分组探针（避免单次超时）
python .git/kam-tmp/probe2.py A   # previous_response_id
python .git/kam-tmp/probe2.py D   # content-type / target 三变体
python .git/kam-tmp/probe2.py E   # 跨模型 temperature/top_p
```

`ListAvailableModels` 的请求形态（与项目 `kiro_client.rs` 一致）：

```text
POST https://management.us-east-1.kiro.dev/
Content-Type: application/x-amz-json-1.0
X-Amz-Target: KiroControlPlaneBearerService.ListAvailableModels
x-amz-user-agent: aws-sdk-js/1.0.0
amz-sdk-invocation-id: <uuid>
amz-sdk-request: attempt=1; max=3
body: {"origin":"AI_EDITOR","profileArn":"..."}
```

### 13.2 社区源码复核（需要 `gh` 登录）

```bash
gh api repos/sunerpy/kiro-provider/contents/scripts/probe-v3-create-response.ts --jq .content | base64 -d
gh api repos/sunerpy/kiro-provider/contents/src/server/responses/native-transport.ts --jq .content | base64 -d
gh api repos/sunerpy/kiro-provider/contents/docs/audits/kiro-cli-provider-v3-wire-diff-2026-09-05.zh.md --jq .content | base64 -d

gh api repos/hank9999/kiro.rs/contents/src/kiro/endpoint/mod.rs --jq .content | base64 -d
gh api repos/ZyphrZero/kiro.rs/contents/src/kiro/endpoint/cli.rs --jq .content | base64 -d
gh api repos/Quorinex/Kiro-Go/contents/proxy/kiro.go --jq .content | base64 -d
gh api repos/RyanKung/rotom/contents/src/codex/upstream.rs --jq .content | base64 -d
gh api repos/RyanKung/rotom/contents/src/codex/kiro.rs --jq .content | base64 -d
gh api repos/nguyen-duc-loc/kiro-gateway/contents/internal/kiro/adapter.go --jq .content | base64 -d
```

注意 `hank9999/kiro.rs` 等仓库的默认分支是 **`master`**（不是 `main`）。

### 13.3 取证片段留存

`.git/kam-tmp/` 下保留了本轮与往轮的逐字取证片段：

| 文件 | 内容 |
| --- | --- |
| `probe_create_response.py` / `probe2.py` / `list_models.py` | 本机实测脚本 |
| `probe.ts` / `nt.ts` / `wirediff.md` | 社区 `kiro-provider` 的探针脚本、原生通道、wire diff 审计 |
| `rotom_up.rs` / `rotom_kiro.rs` / `rotom_kreq.rs` / `rotom_client.rs` | rotom 的上游适配源码 |
| `kirors_ep.txt` / `kirors_cli.txt` | kiro.rs 的端点抽象与 cli 端点 |
| `fn_*.txt` / `gmr.txt` / `schema.txt` / `cache.txt` / `ase*.txt` / `at*.txt` / `batch1.txt` / `flag.txt` / `urcp.txt` / `shapecode.txt` | 官方 `extension.js` 的逐字取证（主文档沿用） |

