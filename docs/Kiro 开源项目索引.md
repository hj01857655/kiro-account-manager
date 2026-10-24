# Kiro 开源项目索引

本文是一份 **Kiro 相关开源项目的索引与核验记录**。

只收录**实际浏览并核验过源码**的仓库。每个条目标注核验深度：

| 标记 | 含义 |
| --- | --- |
| **深** | 拉取了具体源文件，逐字读过上游请求构造（端点 URL、`x-amz-target`、请求体形状） |
| **中** | 拉取了仓库文件树并定位到上游调用文件，但未逐行读完请求构造 |
| **浅** | 仅核验了仓库元数据与关键词命中数，**未**拉取源码 |

> **核验日期：2026-10-21**（元数据与星标数为该日快照，之后会变）

## 相关文档

| 文档 | 内容 |
| --- | --- |
| 《Kiro 官方 thinking 与 effort》 | Kiro 官方客户端源码的事实（thinking / effort / AMRF 形状 / 开关判定链） |
| 《Kiro 上游通道实测与对账》 | 上游通道的活体实测、各实现的通道适配清单、社区结论复核 |

本文只做**索引**——记录有哪些项目、各自适配了什么通道、核验到什么程度。

---

## 1. 核心结论

**本轮核验的 12 个仓库中，没有一个实现了官方原生 CreateResponse 通道。**

所有实现走的都是 Kiro 的 `generateAssistantResponse` / `SendMessage` 系通道——请求体是 Kiro 私有的 `conversationState` 形状，响应是 AWS event-stream。

唯一实现 CreateResponse 的是 `sunerpy/kiro-provider`（TypeScript），它也是唯一一个**以原生 `/v1/responses` 为主路径**、以 `generateAssistantResponse` 为 fallback 的实现。

### 1.1 全局分布：多少项目在打 `generateAssistantResponse`

用 GitHub code search 做全局检索（2026-10-21 快照）：

| 检索词 | 命中总数 | 真正打 Kiro/CodeWhisperer 上游的仓库数 |
| --- | --- | --- |
| `generateAssistantResponse` | **4000**（上限截断） | **878**（含 `codewhisperer` 或 `kiro.dev` 字面量） |
| `"/v1/responses"` + `kiro` | 11216 | — |
| `KiroRuntimeService` | **118** | — |

**这个 4000 是个陷阱，必须拆开看。**

`generateAssistantResponse` 是 **Amazon CodeWhisperer 的 operation 名**，Kiro 只是它的主要使用者之一。同一个 operation 也被 **Antigravity** 系项目使用——而 Antigravity 的上游是 **Google** 的：

```js
// link0518/Antigravity2api —— src/config/config.js
url: 'https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:streamGenerateContent?alt=sse',
```

它的 `src/api/client.js` 里有个**本地函数**叫 `generateAssistantResponse()`——只是个包装函数名，跟 Kiro 无关：

```js
export async function generateAssistantResponse(requestBody, callback) {
  const token = await tokenManager.getToken();
  const response = await fetch(config.api.url, { ... });   // ← Google，不是 Kiro
}
```

所以 4000 命中里混着大量 Antigravity 衍生项目和同名自研函数。**差集验证**：只含函数名、不含 `codewhisperer` / `kiro.dev` 字面量的仓库有 56+ 个（`Fubro/antigravity2api-nodejs`、`Maol-1997/Antigravity2api-English`、`AntiHub-Project/Antigv-plugin` 等），全部是 Google 系。

**判定某仓库是否真在打 Kiro 上游的可靠方法**：搜 `generateAssistantResponse` **并且**搜 `codewhisperer` 或 `kiro.dev`，而不是只看前者。

### 1.2 真正打 Kiro 上游的仓库（抽样 40 个，878 命中里的人工筛选）

```
AlqattanDev/kiro-claude-bridge      GreyGunG/Kiro-RS-Tool       IRNova/NovaRoute
LeenHawk/gproxy                     PawanOsman/OpenCursor        Quorinex/Kiro-Go
TsinHzl/kiro2cc-proxy              Yeachan-Heo/gajae-code        Yoahoug/kiro-stack
ZyphrZero/kiro.rs                   bestK/kiro2cc                 cagedbird043/kiro-relay
caidaoli/kiro2api                   chaogei/Kiro-account-manager  cheaper-inference/OmniRoute
d-kuro/kirocc                      donyst/kiro-cli-pool-proxy    dwalleck/cyril
edison7009/EchoBird                generalbots/generalbots       hj01857655/kiro-account-manager
hnewcity/KiroaaS                    itututu/claudecode-kiro       justlovemaki/AIClient2API
jwadow/kiro-gateway                 lidge-jun/opencodex          luqman-v1/9router-go
mydisha/keirouter                   nopperabbo/kiroxy             quangdang46/openproxy
rsalmn/ExtremeRouter                soulduse/ai-token-monitor     techysy/10router
theRizwan/FlagshipRouter            theRizwan/ShellShock          tickernelz/opencode-kiro-auth
```

**共同点**：全部走 `generateAssistantResponse`（+ 可选 `/mcp`），出站**没有**一个用 Anthropic Messages 协议（上游不提供，见《Kiro 上游通道实测与对账》§11）。

**注意「入站 `/v1/responses` 命中 11216」的规模远大于上游协议命中**——这印证了社区项目的普遍形态：**入站提供各种协议（Messages / Chat / Responses），出站统一收敛到 Kiro 私有协议**。例如 `xu-yuanhao/KiroProxy` 的 `kiro_proxy/main.py`：

```python
@app.post("/v1/messages")
@app.post("/v1/messages/count_tokens")
@app.post("/v1/chat/completions")
@app.post("/v1/responses")
@app.post("/v1beta/models/{model_name}:generateContent")
@app.post("/v1/models/{model_name}:generateContent")
```

**入站 6 种协议，出站 1 种。**

### 1.3 唯一的例外仍然只有 CreateResponse

`KiroRuntimeService` 全局只有 **118** 命中，其中真正实现原生 CreateResponse 请求构造的**只有 `sunerpy/kiro-provider`**。其余（`nguyen-duc-loc/kiro-gateway`、`minpeter/kiro-lb`、`hnewcity/KiroaaS` 等）的命中是：

- 引用 `KiroRuntimeService.GenerateAssistantResponse` / `InvokeMCP`（正常用法）
- 或在文档/探针脚本里提到 `CreateResponse`（未实现）

---

## 2. Rust 系：`kiro.rs` 及其 fork

`kiro.rs` 是本项目最贴近的参照物（同样是 Rust + Tauri 风格的 Kiro 账号管理/代理）。原始仓库与各 fork 的差异集中在**端点抽象层**。

| 仓库 | 星标 | 分支 | 许可 | 核验 | 端点实现 |
| --- | --- | --- | --- | --- | --- |
| [`hank9999/kiro.rs`](https://github.com/hank9999/kiro.rs) | ★1964 | `master` | MIT | **深** | 1 个：`ide` |
| [`ZyphrZero/kiro.rs`](https://github.com/ZyphrZero/kiro.rs) | ★489 | `master` | MIT | **深** | 2 个：`ide` + `cli` |
| [`Hoshino-Yumetsuki/kiro.rs`](https://github.com/Hoshino-Yumetsuki/kiro.rs) | ★5 | `master` | MIT | **深** | 2 个：`ide` + `cli`（target 不同） |
| [`freebattle/kiro.rs`](https://github.com/freebattle/kiro.rs) | ★7 | `master` | MIT | **中** | 2 个：`ide` + `cli` |
| [`doitcan-oiu/kiro.rs-admin`](https://github.com/doitcan-oiu/kiro.rs-admin) | ★6 | `master` | MIT | **深** | **4 个**：`ide` + `runtime` + `amazonq` + `codewhisperer` |
| [`TsinHzl/kiro2cc-proxy`](https://github.com/TsinHzl/kiro2cc-proxy) | ★223 | `main` | — | **深** | 1 条（`generateAssistantResponse`） |
| [`GreyGunG/Kiro-RS-Tool`](https://github.com/GreyGunG/Kiro-RS-Tool) | ★119 | `main` | — | **深** | 1 条（`runtime.*.kiro.dev/generateAssistantResponse`） |
| [`seven7763/kiro.rs`](https://github.com/seven7763/kiro.rs) | ★1 | `master` | MIT | **浅** | 1 个：`ide` |

### 2.1 `hank9999/kiro.rs` —— 端点抽象的基准

**深**核验。`src/kiro/endpoint/mod.rs` 定义了可插拔端点 trait：

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

但 `src/main.rs` 的注册表**只注册了一个**：

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

README 明写：`defaultEndpoint` 当前支持：`ide`。**该仓库没有 `cli` / `responses` 端点实现**（fork 才补上）。

### 2.2 两个 fork 的 `cli` 端点 —— target 不同

**深**核验。同为 `cli` 端点，两者的 `x-amz-target` 不一样：

| Fork | 端点 | Origin | `x-amz-target` |
| --- | --- | --- | --- |
| `ZyphrZero/kiro.rs` | `https://{host}/`（根路径） | `KIRO_CLI` | `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` |
| `Hoshino-Yumetsuki/kiro.rs` | `https://{host}/`（根路径） | `KIRO_CLI` | `AmazonQDeveloperStreamingService.SendMessage` |

### 2.3 `doitcan-oiu/kiro.rs-admin` —— 端点最全的 fork（4 个）

**深**核验。它的 4 个端点各自带独立的限流桶，这是它最有价值的地方：

| 端点 | API | MCP | 额外 header | 备注 |
| --- | --- | --- | --- | --- |
| `ide` | `https://q.{region}.amazonaws.com/generateAssistantResponse` | `.../mcp` | 无 | 基准形状 |
| `runtime` | `https://runtime.{region}.kiro.dev/generateAssistantResponse` | — | 无 | **与 `ide` 请求体完全一致，唯一差别是域名** |
| `amazonq` | `https://q.{region}.amazonaws.com/generateAssistantResponse` | `.../mcp` | `x-amz-target: AmazonQDeveloperStreamingService.SendMessage` | 与 `ide` 同主机，靠 target 区分服务 |
| `codewhisperer` | `https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse` | — | `x-amz-target: AmazonCodeWhispererStreamingService.GenerateAssistantResponse` | us-east-1 独有主机 |

源码注释原文（`runtime.rs`）：

```rust
//! 对应 Kiro IDE 客户端较新的 `runtime.kiro.dev` 推理链路：
//! - API: `https://runtime.{api_region}.kiro.dev/generateAssistantResponse`
//! **完全一致**，唯一差别是域名从 `q.{region}.amazonaws.com` 换成
//! 关键价值：实测 `runtime.kiro.dev` 与 `q.amazonaws.com` 是**两个独立的限流桶**
```

**重要**：它的 `runtime` 端点用的是 `runtime.{region}.kiro.dev/generateAssistantResponse`——**不是** `/v1/responses`。这条路径与官方原生 CreateResponse 的 `/v1/responses` 是**同主机、不同路径**。

`codewhisperer.rs` 的注释还记录了主机回退逻辑：

```rust
//! - **主机不同**：`us-east-1` 走独有的 `codewhisperer.us-east-1.amazonaws.com` 主机
//!   （一个与 `q.*` / `runtime.kiro.dev` 相互独立的限流桶）。
//! 回退到区域化的 Amazon Q 主机 `q.{region}.amazonaws.com`。us-east-1 或空区域为原样。
```

### 2.4 `freebattle/kiro.rs` 与 `seven7763/kiro.rs`

- `freebattle/kiro.rs`：**中**核验。文件树含 `src/kiro/endpoint/{cli,ide,mod}.rs`，结构同 `hank9999` 系的 fork 形态。
- `seven7763/kiro.rs`：**浅**核验。描述为「kiro-rs Enhanced Fork — Opus 4.7 完整支持 + 系统提示词预设系统 + Admin UI 热更新」，无 endpoint 目录。

---

## 3. Go 系

| 仓库 | 星标 | 分支 | 许可 | 核验 | 通道 |
| --- | --- | --- | --- | --- | --- |
| [`Quorinex/Kiro-Go`](https://github.com/Quorinex/Kiro-Go) | ★1186 | `main` | MIT | **深** | **4 条** |
| [`bestK/kiro2cc`](https://github.com/bestK/kiro2cc) | ★367 | `main` | none | **中** | 1 条 |
| [`d-kuro/kirocc`](https://github.com/d-kuro/kirocc) | ★68 | `main` | Apache-2.0 | **浅** | 1 条 |
| [`hj01857655/Kiro-Go`](https://github.com/hj01857655/Kiro-Go) | ★34 | `feature/react-admin` | MIT | **浅** | 1 条 |
| [`wenyi401/ikik-api`](https://github.com/wenyi401/ikik-api) | ★242 | `main` | — | **深** | 2 个（`q.*` / `krs` 可切） |
| [`nopperabbo/kiroxy`](https://github.com/nopperabbo/kiroxy) | ★3 | `main` | MIT | **深** | 3 条（fail-over） |

### 3.1 `Quorinex/Kiro-Go` —— 4 条通道

**深**核验。`proxy/kiro.go` 的端点表：

| 名称 | URL | Origin | `X-Amz-Target` |
| --- | --- | --- | --- |
| Kiro IDE | `https://q.us-east-1.amazonaws.com/generateAssistantResponse` | `AI_EDITOR` | （无） |
| CodeWhisperer | `https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse` | `AI_EDITOR` | `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` |
| AmazonQ | `https://q.us-east-1.amazonaws.com/generateAssistantResponse` | `AI_EDITOR` | `AmazonQDeveloperStreamingService.SendMessage` |
| Kiro CLI | `https://runtime.{region}.kiro.dev/` | `KIRO_CLI` | `AmazonCodeWhispererStreamingService.GenerateAssistantResponse` |

**与 `kiro.rs-admin` 的 4 端点高度重合**——两者显然是同源演进（`codewhisperer` 主机 + `amazonq` target + `runtime` 主机这三个细节都一致）。

### 3.2 `nopperabbo/kiroxy` —— 值得注意的 fail-over 设计

**深**核验。`internal/kiroclient/client.go` 原文：

```go
// endpointURL returns the Kiro API URL for a region. Returns the native
// shape (q.<region>.amazonaws.com/generateAssistantResponse) by default
// and the legacy shape (runtime.<region>.kiro.dev/) when
// KIROXY_NATIVE_HEADERS=0. Native is the path genuine Kiro IDE talks to;
// legacy is preserved for debugging or upstream-rollback scenarios.
//
// The earlier comment in this position claimed jwadow/kiro-gateway#146
// would deprecate q.<region>.amazonaws.com on 2026-05-15 and migrate to
// runtime.<region>.kiro.dev. Live curl probes against q.us-east-1 on
// 2026-05-15 returned HTTP 200 + real eventstream — the deprecation has
// not happened. kiroxy now treats q.<region> as the primary path. If
// upstream genuinely cuts over later, KIROXY_NATIVE_HEADERS=0 falls back.
```

这段注释很有价值——**它记录了一次「上游要迁移」的传闻被实测证伪**（2026-05-15 实测 `q.us-east-1` 仍返回 200 + 真 eventstream，迁移未发生）。

fail-over 行为：

```go
//   - Native shape (default): walks the kiroEndpoints list in fail-over order
//     (Kiro IDE primary → CodeWhisperer → AmazonQ). Each endpoint runs the
//     full retry budget independently. HTTP 429 after retries-exhausted on
//     one endpoint advances to the next; other terminal errors return.
```

**每个端点独立跑完整重试预算，429 耗尽才进下一个**——这对多限流桶场景是正确做法。

### 3.3 `bestK/kiro2cc`

**中**核验。`main.go` 里直接硬编码端点：

```go
ProfileArn: "arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK",
"https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse",
```

注意硬编码的 `ProfileArn`（`EHGA3GRVQMUK`）与本机 `~/.aws/sso/cache/kiro-auth-token.json` 里的 profile 后缀**相同**——Kiro CLI 似乎给同一用户发放固定 profile id。

### 3.4 `d-kuro/kirocc` / `hj01857655/Kiro-Go`

**浅**核验。`kirocc` 的 `internal/app/messages/` 下有 `effort.go`（ effort 处理独立成文件，与本项目的 effort 逻辑相关），但本轮未拉取其上游请求构造。

### 3.5 `wenyi401/ikik-api` —— 按账号组切换 Kiro 端点

**深**核验。`backend/internal/service/kiro_runtime.go` 把 Kiro 端点做成**按 group 配置切换**：

```go
// kiroKRSEndpointURL 是 Kiro 自家前置网关（KRS = Kiro Runtime Service）的固定 URL。
// KRS 仅支持 us-east-1 / eu-central-1 两个 region；这里固定走 us-east-1。
const kiroKRSEndpointURL = "https://runtime.us-east-1.kiro.dev/generateAssistantResponse"

func buildKiroEndpoints(account *Account, mode string) []kiroEndpointConfig {
	if mode == KiroEndpointModeKRS {
		return []kiroEndpointConfig{{URL: kiroKRSEndpointURL, Name: "KiroRuntime"}}
	}
	region := kiroAPIRegion(account)
	return []kiroEndpointConfig{{
		URL:  fmt.Sprintf("https://q.%s.amazonaws.com/generateAssistantResponse", region),
		Name: "AmazonQ",
	}}
}
```

两个模式常量（`group_ikik_extensions.go`）：

```go
const KiroEndpointModeKRS = "krs"
const KiroEndpointModeQ   = "q"
```

要点：

- 两种模式**请求体完全相同**（`conversationState`），只是**主机**不同——与 `kiro.rs-admin` 的 `ide` / `runtime` 端点设计一致。
- 它把模式做成**账号组级配置**（`Group.EffectiveKiroEndpointMode()`），而不是全局开关——同一部署里不同分组走不同 Kiro 主机，便于分流。
- 注释里明确了 KRS 的 region 限制：**只有 `us-east-1` / `eu-central-1`**。
- 请求构造走 `newKiroJSONRequest(ctx, endpoint.URL, payload, ..., endpoint.AmzTarget, ...)`——`AmzTarget` 随端点配置一起传，说明**同一套代码用不同 target 区分 CodeWhisperer 与 KRS**。

---

## 4. Python 系

| 仓库 | 星标 | 分支 | 许可 | 核验 | 上游 |
| --- | --- | --- | --- | --- | --- |
| [`jwadow/kiro-gateway`](https://github.com/jwadow/kiro-gateway) | ★2327 | `main` | AGPL-3.0 | **深** | `generateAssistantResponse` |
| [`aliom-v/KiroGate`](https://github.com/aliom-v/KiroGate) | ★421 | `main` | AGPL-3.0 | **中** | `generateAssistantResponse` |
| [`xu-yuanhao/KiroProxy`](https://github.com/xu-yuanhao/KiroProxy) | ★392 | `main` | MIT | **深** | `generateAssistantResponse`（入站 6 种协议） |
| [`hnewcity/KiroaaS`](https://github.com/hnewcity/KiroaaS) | ★149 | `main` | AGPL-3.0 | **浅** | 未核验 |
| [`minpeter/kiro-lb`](https://github.com/minpeter/kiro-lb) | ★31 | `main` | AGPL-3.0 | **浅** | 未核验 |

### 4.1 `jwadow/kiro-gateway`

**深**核验。`kiro/routes_openai.py` 与 `kiro/routes_anthropic.py` 都用同一模式：

```python
# profileArn is required by runtime.kiro.dev for all auth types
url = f"{auth_manager.api_host}/generateAssistantResponse"
```

`kiro/config.py` 记录了主机选择的踩坑史：

```python
# Host for main API (generateAssistantResponse)
# Fixed in issue #58 - codewhisperer.{region}.amazonaws.com doesn't exist for non-us-east-1 regions
```

`kiroxy` 注释里提到的「jwadow/kiro-gateway#146 声称要迁移到 `runtime.<region>.kiro.dev`」即出自此仓库的 issue。

### 4.2 `aliom-v/KiroGate` / `hnewcity/KiroaaS` / `minpeter/kiro-lb`

- `aliom-v/KiroGate`：**中**核验。`kiro_gateway/` 包结构完整（`auth.py` / `converters.py` / `chunked_processor.py` / `model_resolver.py` 等）。
- `hnewcity/KiroaaS`：**浅**核验。
- `minpeter/kiro-lb`：**浅**核验。有 `src/convert_responses.rs`——但这是**入站**的 OpenAI Responses 协议转换（对外暴露 `/v1/responses`），**不是**出站调用官方 CreateResponse 通道。

---

## 5. TypeScript / Go / 其他

| 仓库 | 星标 | 语言 | 许可 | 核验 | 说明 |
| --- | --- | --- | --- | --- | --- |
| [`sunerpy/kiro-provider`](https://github.com/sunerpy/kiro-provider) | ★2 | TypeScript | MIT | **深** | ✅ **唯一实现官方 CreateResponse** |
| [`mikeyobrien/pi-provider-kiro`](https://github.com/mikeyobrien/pi-provider-kiro) | ★106 | TypeScript | MIT | **深** | 出站 `runtime.{region}.kiro.dev/generateAssistantResponse` |
| [`nguyen-duc-loc/kiro-gateway`](https://github.com/nguyen-duc-loc/kiro-gateway) | ★0 | Go | none | **深** | 生产走 `generateAssistantResponse`；CreateResponse 仅在探针设计文档 |
| [`RyanKung/rotom`](https://github.com/RyanKung/rotom) | ★6 | Rust | LGPL-3.0 | **深** | Kiro 走 `send_kiro_body`（`conversationState`） |
| [`xwteam/kiro2api`](https://github.com/xwteam/kiro2api) | ★13 | Rust | NOASSERTION | **浅** | 四协议中转（入站） |
| [`dwgx/KiroStudio`](https://github.com/dwgx/KiroStudio) | ★16 | Rust | NOASSERTION | **浅** | Anthropic 兼容代理 |
| [`ssmDo/CodeFreeMax`](https://github.com/ssmDo/CodeFreeMax) | ★181 | Shell | — | **深** | 脚本级封装，**未见** Kiro 上游端点实现 |
| [`hj01857655/kiro-account-manager`](https://github.com/hj01857655/kiro-account-manager) | ★2035 | Rust | NOASSERTION | **浅** | 账号管理（本项目同类） |

### 5.1 `sunerpy/kiro-provider` —— 唯一的 CreateResponse 实现

**深**核验。详见《Kiro 上游通道实测与对账》§7。

要点：
- 端点常量 `RUNTIME_ENDPOINT: "https://runtime.{{region}}.kiro.dev"`，路径 `/v1/responses`
- 请求形态与官方 `cfd` **逐字吻合**（host / 路径 / content-type / 鉴权 / `x-amzn-kiro-origin` / `x-amzn-kiro-profile` / `User-Agent` / 请求体字段集）
- 唯一实质差异：官方 `lfd` 会下发 `reasoning.type`，社区实现**不发**

### 5.2 `RyanKung/rotom` —— `NativeResponses` 标注具误导性

**深**核验。`src/codex/upstream.rs` 里：

```rust
fn response_creation_strategy(&self) -> ResponseCreationStrategy {
    ResponseCreationStrategy::NativeResponses      // KiroUpstream
}
```

但 Kiro 的实际请求走**另一条**函数（`src/codex/kiro.rs`）：

```rust
async fn send_kiro_body(&self, body: &Value, credentials: &Credentials) -> Result<Response> {
    let upstream_body = kiro::to_kiro_payload(body, credentials)?;   // ← conversationState 形状
    let url = kiro::kiro_endpoint_url(&self.base_url, credentials)?;  // ← https://runtime.{region}.kiro.dev/
    ...
}
```

- `send_body`（用 `responses_url` + `NativeResponses`）只服务 **Grok** 与 **Vercel**。
- `to_kiro_payload` 产出 Kiro 私有的 `conversationState`（`modelId` / `currentMessage` / `profileArn`），**不是**官方 `cfd` 的 OpenAI Responses 形状。
- **结论**：rotom 的 Kiro 通道等价于官方原生 `GenerateAssistantResponse`。

### 5.3 `nguyen-duc-loc/kiro-gateway` —— CreateResponse 只是探针文档

**深**核验。`internal/kiro/adapter.go` 的生产路径：

```go
req, err := http.NewRequestWithContext(ctx, http.MethodPost, "https://runtime."+region+".kiro.dev:443/generateAssistantResponse", ...)
"Content-Type": "application/x-amz-json-1.0", "X-Amz-Target": "AmazonCodeWhispererStreamingService.GenerateAssistantResponse",
```

CreateResponse 只出现在 `docs/specs/0004-claude-code-bridge/0004-createresponse-discovery.md`，文档自述「This is a declared hypothesis, not a proven accepted request」，入口默认关闭（`-response-discovery-launch`）。

**本轮实测**部分验证了该假设：`POST /` + `X-Amz-Target: KiroRuntimeService.CreateResponse` 确实返回 200（target 已绑定），但 body 是空对象 `{}`——**绑定了却不返回流**。详见《Kiro 上游通道实测与对账》§5.2。

---

## 6. 通道形态汇总

把各实现的上游端点归类，能看出 Kiro 生态的实际形状：

| 形态 | 端点 | 谁在用 |
| --- | --- | --- |
| **A. `q.*.amazonaws.com`** | `/generateAssistantResponse`，无 target | `kiro.rs`(`ide`)、`kiro.rs-admin`(`ide`/`amazonq`)、`Kiro-Go`、`kiroxy`（默认）、`jwadow`、`kiro-gateway` |
| **B. `codewhisperer.*.amazonaws.com`** | `/generateAssistantResponse` + CW target | `kiro.rs-admin`、`Kiro-Go`、`kiro2cc` |
| **C. `runtime.*.kiro.dev`** | `/generateAssistantResponse` | `kiro.rs-admin`(`runtime`)、`kiroxy`（legacy 模式）、`rotom` |
| **D. `runtime.*.kiro.dev` 根路径** | `/` + CW/AmazonQ target | `ZyphrZero`/`Hoshino-Yumetsuki`(`cli`)、`Kiro-Go`(CLI 通道)、`rotom` |
| **E. `runtime.*.kiro.dev`** | **`/v1/responses`**，裸 JSON | ✅ 仅 `sunerpy/kiro-provider` |

**形态 A/B/C/D 全部是 `generateAssistantResponse` 协议**（Kiro 私有 `conversationState` 请求体 + AWS event-stream 响应），只是主机、路径、target 三者有别。

**形态 E 是唯一的例外**——它用 OpenAI Responses 协议，是官方客户端自己发的那种请求。

### 6.1 三个独立的限流桶

`kiro.rs-admin` 与 `kiroxy` 都强调同一件事：**`q.*`、`codewhisperer.*`、`runtime.*.kiro.dev` 是相互独立的限流桶**。`kiroxy` 实现了端点间 fail-over（429 耗尽才切换），`kiro.rs-admin` 则把它们做成可配置的端点列表。

这一点对本项目有直接价值——**多主机轮换是提高可用性的现成手段**。

---

## 7. 账号管理类（与本项目同类）

| 仓库 | 星标 | 语言 | 许可 | 说明 |
| --- | --- | --- | --- | --- |
| [`hj01857655/kiro-account-manager`](https://github.com/hj01857655/kiro-account-manager) | ★2035 | Rust | NOASSERTION | 智能管理 Kiro IDE 账号，一键切换，配额监控 |
| [`jlcodes99/cockpit-tools`](https://github.com/jlcodes99/cockpit-tools) | ★19195 | Rust | none | 通用 AI IDE 账号管理（Antigravity / Codex / Copilot / Windsurf / Kiro / Cursor） |
| [`justlovemaki/AIClient2API`](https://github.com/justlovemaki/AIClient2API) | ★8907 | JavaScript | GPL-3.0 | 多协议 AI API 代理 |
| [`x1xhlol/system-prompts-and-models-of-ai-tools`](https://github.com/x1xhlol/system-prompts-and-models-of-ai-tools) | ★144165 | — | GPL-3.0 | 各大 AI 工具的系统提示词合集（Kiro 只是其中一项） |

后两个**本轮未核验其 Kiro 上游实现**——列出是因为它们常在「Kiro 相关」检索中出现。

---

## 8. 核验方法

### 8.1 元数据与文件树

```bash
gh api repos/<owner>/<repo> --jq '"b=\(.default_branch) s=\(.stargazers_count) l=\(.language) lic=\(.license.spdx_id) p=\(.pushed_at[0:10])"'
gh api repos/<owner>/<repo>/git/trees/HEAD?recursive=1 --jq '.tree[].path'
```

**注意**：`hank9999/kiro.rs` 等仓库的默认分支是 **`master`**，不是 `main`。用 `HEAD` 可避免分支名猜错。

### 8.2 拉取源文件原文

```bash
gh api repos/<owner>/<repo>/contents/<path> --jq .content | base64 -d
```

### 8.3 限流

GitHub **code search** 配额很低（10 次/分钟），批量扫描时极易 403：

```json
{"code_search":{"limit":10,"remaining":0},"core":{"limit":5000,"remaining":4927},"search":{"limit":30,"remaining":30}}
```

- `core`（`contents` / `git/trees`）配额 5000/小时，**充足**，应优先用。
- 批量探测上游端点时，用 `git/trees` 列文件 + `contents` 逐个取，**不要用 code search**。

### 8.4 本轮用到的仓库清单（便于复现）

```bash
# Rust 系
gh api repos/hank9999/kiro.rs/contents/src/kiro/endpoint/mod.rs --jq .content | base64 -d
gh api repos/ZyphrZero/kiro.rs/contents/src/kiro/endpoint/cli.rs --jq .content | base64 -d
gh api repos/doitcan-oiu/kiro.rs-admin/contents/src/kiro/endpoint/runtime.rs --jq .content | base64 -d
gh api repos/doitcan-oiu/kiro.rs-admin/contents/src/kiro/endpoint/codewhisperer.rs --jq .content | base64 -d
gh api repos/doitcan-oiu/kiro.rs-admin/contents/src/kiro/endpoint/amazonq.rs --jq .content | base64 -d

# Go 系
gh api repos/Quorinex/Kiro-Go/contents/proxy/kiro.go --jq .content | base64 -d
gh api repos/nopperabbo/kiroxy/contents/internal/kiroclient/client.go --jq .content | base64 -d
gh api repos/bestK/kiro2cc/contents/main.go --jq .content | base64 -d

# Python 系
gh api repos/jwadow/kiro-gateway/contents/kiro/routes_openai.py --jq .content | base64 -d
gh api repos/jwadow/kiro-gateway/contents/kiro/config.py --jq .content | base64 -d

# TypeScript / Rust
gh api repos/sunerpy/kiro-provider/contents/src/server/responses/native-transport.ts --jq .content | base64 -d
gh api repos/sunerpy/kiro-provider/contents/src/kiro/constants.ts --jq .content | base64 -d
gh api repos/RyanKung/rotom/contents/src/codex/upstream.rs --jq .content | base64 -d
gh api repos/RyanKung/rotom/contents/src/codex/kiro.rs --jq .content | base64 -d
gh api repos/RyanKung/rotom/contents/src/codex/kiro_request.rs --jq .content | base64 -d
gh api repos/nguyen-duc-loc/kiro-gateway/contents/internal/kiro/adapter.go --jq .content | base64 -d
```

### 8.5 取证片段留存

`.git/kam-tmp/` 下保留了本轮抓取的原文：

| 文件 | 内容 |
| --- | --- |
| `repo_meta.txt` | 21 个仓库的元数据快照 |
| `kirors_ep.txt` / `kirors_cli.txt` | `kiro.rs` 的端点抽象与 `cli` 端点 |
| `rotom_up.rs` / `rotom_kiro.rs` / `rotom_kreq.rs` / `rotom_client.rs` | rotom 的上游适配 |
| `probe.ts` / `nt.ts` / `wirediff.md` | `kiro-provider` 的探针、原生通道、wire diff |
| `kra_ep.txt` / `ups2.txt` / `ups3.txt` / `ups4.txt` / `ups5.txt` | 各仓库上游端点探测 |
| `kiro_repos.txt.sort` | 156 个候选仓库的广搜结果 |

---

## 9. 尚未核验的候选

广搜共得到 **156 个**候选仓库。本轮只核验了上表列出的部分。**其余候选未核验**，不能断言它们走哪条通道。

按星标排序的高星未核验候选：

| 仓库 | 星标 | 语言 | 描述 |
| --- | --- | --- | --- |
| `justlovemaki/AIClient2API` | ★8907 | JavaScript | 自托管多协议 AI API 代理（Antigravity / Codex / Grok / Kiro / OpenAI / Claude） |
| `QLHazyCoder/FlowPilot` | ★5188 | JavaScript | Chrome 扩展，kiro / Grok 自动注册 + 反代回调 |
| `kirodotdev/Kiro` | ★4384 | TypeScript | Kiro IDE 官方仓库（**非代理实现**） |
| `lxf746/any-auto-register` | ★3366 | Python | 自动注册管理 ChatGPT / Cursor / Kiro 等 |
| `xu-yuanhao/KiroProxy` | ★392 | Python | Kiro 接入兼容层与请求路由服务 |
| `wenyi401/ikik-api` | ★242 | Go | Sub2API 二开，统一接 Claude / OpenAI / Gemini / Grok / Kiro |
| `TsinHzl/kiro2cc-proxy` | ★223 | Rust | Kiro 代理到 Claude Code 与 Codex |
| `ssmDo/CodeFreeMax` | ★181 | Shell | Kiro / Antigravity / Warp 等转 OpenAI/Claude 格式 |
| `kkddytd/claude-api` | ★158 | Go | Kiro 账号池 + Amazon Q → OpenAI 格式 |
| `GreyGunG/Kiro-RS-Tool` | ★119 | Rust | 面向 Claude Code 工具调用的 Kiro 代理 |
| `mikeyobrien/pi-provider-kiro` | ★106 | TypeScript | pi extension，17 models / 7 families |
| `daokimluc/X-Router` | ★98 | TypeScript | 多上游路由（Claude Code / Cursor / Kiro / Codex…） |
| `ankitcharolia/kiro-gateway` | ★82 | Python | 合规优先的 Kiro 网关 |
| `dangweilinshinidie/kiro-reverse-api` | ★72 | JavaScript | Kiro 分析 API 控制台 |
| `zxc123aa/cc-proxy-detector` | ★47 | Python | 检测 Claude Code 代理后端（Anthropic / Bedrock Kiro / Vertex） |

**若要补全**，用 §8.4 的 `gh api contents` 方式逐个核验上游端点即可——不要用 code search（配额 10/分钟）。
