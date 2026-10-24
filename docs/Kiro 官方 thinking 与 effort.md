# Kiro 官方 thinking 与 effort

本文只记录 **Kiro 自身的事实**（官方客户端源码 + 官方文档原文），不做推测。
每条事实都附取证位置；源码引用为 `extension.js` 原文（未改写）。

**相关文档**：上游通道的活体实测与开源实现对账（各实现支持哪些通道、社区结论复核）不在本文范围，见 **《Kiro 上游通道实测与对账》**（`docs/Kiro 上游通道实测与对账.md`）；各仓库的完整索引与核验深度见 **《Kiro 开源项目索引》**（`docs/Kiro 开源项目索引.md`）。§8.3 是指向它们的指针。

## 1. 取证来源

| 来源 | 路径 | 性质 |
| --- | --- | --- |
| 官方 Kiro 客户端扩展 | `C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js` | 官方事实 |
| 官方文档（随客户端分发） | `docs/kiro-re/`（抓包与端点取证）、`.git/kam-tmp/kiro_docs.txt`（官方 docs 文本） | 官方事实 |

源码经 `dist/extension.js` 混淆，函数名被压缩为 `Ibt`/`A4e`/`Cbt` 等短名。本文沿用这些原名，便于逐字复核。

## 2. 两条端点链路互不等价

Kiro 客户端同时存在两条会把 reasoning 配置发往上游的路径，**字段形状不同**。

### 2.0 先说清楚方向：这里有**两个同名但方向相反**的 `/v1/responses`

这是最容易踩的坑，务必先分清：

| | 官方的 `/v1/responses` | 项目的 `/v1/responses` |
| --- | --- | --- |
| 方向 | **出站**——Kiro 客户端 → Kiro 后端 | **入站**——第三方客户端 → 本项目 |
| 代码位置 | 官方扩展 `extension.js`（`pfd`/`cfd`） | `src-tauri/src/gateway/mod.rs:1148`（`openai_responses_handler`） |
| 作用 | 官方客户端在服务端开启 flag 后，改用 OpenAI Responses 协议与后端对话 | 本项目对外暴露的 OpenAI Responses 兼容层，把只会说该协议的客户端接进来 |
| 对端 | Kiro 自有后端 | 本项目自身 |

两条路合起来看：

```
第三方客户端(Codex 等) ──OpenAI Responses──▶ 本项目 /v1/responses
                                                │ 内部转换
                                                ▼
                                        Kiro 上游 generateAssistantResponse
                                        （即下文的原生路径）

Kiro 客户端 ──OpenAI Responses──▶ Kiro 后端 /v1/responses   ← 官方那条
```

**同名，但一个在入口、一个在出口。** 本文档 §2.1 / §2.2 讨论的是**官方出站**的两条链路；项目自己的入站路由不在本文取证范围内。

### 2.1 原生路径：`GenerateAssistantResponse`（KRS）

扩展通过 `buildRequestCommand` 构造命令：

```js
buildRequestCommand(t,e,r,n,s){
  let a=new ypd({conversationState:e,profileArn:t.profileArn,additionalModelRequestFields:r,agentMode:this.agentMode,systemPrompt:n});
  ...
}
```

事实：

- `additionalModelRequestFields`（AMRF）与 `conversationState` **同级**，位于请求体**顶层**——不在 `conversationState` 内部。
- 同一份常量表里 `Gsd="additionalModelRequestFields"` 与 `ead="conversationState"` 并列出现，佐证两者是同一层的兄弟字段。
- AMRF 的内容由 `Cbt` 构造（见 §7）。
- 该链路的响应侧事件含 `ReasoningContentEvent`（原生带签名的 reasoning）。

### 2.2 `/v1/responses` 路径

```js
function cfd(t){let{conversationState:e,systemPrompt:r,modelId:n,origin:s,profileArn:a,agentMode:l,additionalModelRequestFields:u}=t,...;return{model:n,input:sfd(e),...r!==void 0&&{instructions:r},...d&&{tools:d},stream:!0,origin:s,...a!==void 0&&{profileArn:a},...Object.keys(p).length>0&&{metadata:p},...lfd(u)}}
```

```js
function lfd(t){let e=Iui(t),r=Cui(t),n=Tui(t);return e===void 0&&r===void 0&&n===void 0?{}:{reasoning:{...e!==void 0&&{type:e},...r!==void 0&&{effort:r},...n!==void 0&&{summary:n}}}}
```

事实：

- `/v1/responses` **不透传** AMRF 的原始形状，而是把它**摊平**成顶层 `reasoning: {type, effort, summary}`。
- `type` 来自 `thinking.type`（`Iui`），`effort` 来自 `output_config.effort` 或 `reasoning.effort`（`Cui`），`summary` 来自 `thinking.display`（`Tui`）。
- 三者全空 → 不产生 `reasoning` 字段。
- **结论：`/v1/responses` 的 `reasoning` 形状不能反推 `GenerateAssistantResponse` 的 AMRF 形状。** 两者是不同端点的不同协议。

#### 2.2.1 出站请求的实际形态

`cfd` 产出的请求体是**标准 OpenAI Responses 请求**，Kiro 私有信息放在额外字段与 header 里：

| 字段 | 来源 | 说明 |
| --- | --- | --- |
| `model` | `modelId` | 模型 id |
| `input` | `sfd(e)` | OpenAI 扁平消息数组（Kiro 的 `history` + `currentMessage` 摊平） |
| `instructions` | `systemPrompt` | 系统提示词（可选） |
| `tools` | `afd(e)` | 转成 OpenAI function 形状 |
| `stream` | 固定 `true` | 恒为流式 |
| `origin` / `profileArn` | 调用方 | Kiro 私有字段 |
| `metadata` | `p` | Kiro 私有元数据 |
| `reasoning` | `lfd(u)` | 从 AMRF 摊平而来（§8） |

`tools` 的转换（`afd`）——Kiro 的 `toolSpecification` 转成 OpenAI function 形状：

```js
function afd(t){let e=t.currentMessage?.userInputMessage?.userInputMessageContext?.tools;if(!e||e.length===0)return;
  let r=[];for(let n of e){let s=n.toolSpecification;!s||!s.name||r.push({type:"function",name:s.name,description:s.description,parameters:s.inputSchema.json})}
  return r.length>0?r:void 0}
```

`metadata` 里的 Kiro 私有键（`cfd` 内构造的 `p`）：

```js
p["kiro:agent_mode"], p["kiro:agent_task_type"], p["kiro:conversation_id"],
p["kiro:root_conversation_id"], p["kiro:agent_continuation_id"]
```

请求头（`pfd`）：

```js
{"Content-Type":"application/json", Authorization:`Bearer ${token}`,
 "x-amzn-kiro-origin":origin, "x-amzn-codewhisperer-optout":contentCollectionEnabled?"false":"true",
 ...profileArn?{"x-amzn-kiro-profile":profileArn}:{}, ...tokenType?{TokenType:tokenType}:{}, ...userAgent?{"User-Agent":userAgent}:{}}
```

- 端点由 `new URL("/v1/responses", endpoint)` 从 KRS endpoint 派生——**与原生链路同一个 `krsEndpoint`**，只是路径不同。
- `pfd` 会检测 `http:` 协议并告警 bearer token 明文传输（`[create-response-raw] HTTP endpoint detected`）。

#### 2.2.2 响应侧事件名

`/v1/responses` 返回的是 **OpenAI Responses 的 SSE 事件名**（官方扩展全文精确检索，带引号形式）：

| 事件 | 用途 |
| --- | --- |
| `response.output_text.delta` | 正文增量 |
| `response.reasoning_text.delta` | 推理文本增量 |
| `response.reasoning.delta` | 推理增量（另一形态） |
| `response.function_call_arguments.delta` / `.done` | 工具调用参数流式与结束 |
| `response.output_item.added` / `.done` | 输出项增删 |
| `response.refusal.delta` / `.done` | 拒答内容 |
| `response.completed` / `.failed` / `.incomplete` | 终态 |

（以上 12 个事件名在官方扩展中**均以带引号字面量**形式出现，各命中 1 次。）

**易误报的两个相似串**——它们不是 SSE 事件名：

| 相似串 | 实际是什么 |
| --- | --- |
| `response.input.invalid` | 实为 `subagent.response.input.invalid`，是**子代理输入校验失败**的 debug 日志键（`C.debug("subagent.response.input.invalid", {...})`） |
| `response.diagnostics.reason` | 实为 `a.response.diagnostics.reason`，是 **Cedar 权限策略**求值结果对象上的字段（`interpretCedarDecision`），与 Responses 协议无关 |

对照原生链路：原生用的是 KRS 私有事件名（`assistantResponseEvent`、`reasoningContentEvent` 等，见 §8.2），**与上面这套 OpenAI 事件名互不相同**。

推论：官方这条链路的响应侧**没有** `ReasoningContentEvent`（那是原生链路的形状），因此 §8.2 记录的 `text`/`redactedContent`/`signature` 三成员只适用于原生路径。

## 3. 模型元数据来源：`ListAvailableModels`

扩展分页拉取上游模型清单，映射为运行时模型表（`bd()` → `ASe.getAvailableModels()`）：

```js
if(w.models?.length&&m.push(...w.models.filter(v=>!!v.modelId).map(v=>{
  let S=Xyu(v.additionalModelRequestFieldsSchema)?v.additionalModelRequestFieldsSchema:void 0,
      _=eAu(S), E=tAu(S);
  return{
    modelId:v.modelId, modelName:v.modelName, description:v.description, provider:v.modelProvider,
    rateMultiplier:v.rateMultiplier, rateUnit:v.rateUnit,
    maxInputTokens:v.tokenLimits?.maxInputTokens,
    refusalFallbackModels:v.refusalFallbackModels?.map(I=>I.modelId).filter(I=>!!I),
    effortLevels:_?.levels, effortSchemaPath:_?.schemaPath, defaultEffortLevel:_?.defaultLevel,
    thinkingToggleable:E?.toggleable, defaultThinkingEnabled:E?.defaultEnabled
  }
})))
```

事实：

- 上游每个模型可携带 `additionalModelRequestFieldsSchema`（AMRF schema），`tokenLimits.maxInputTokens`，以及 `refusalFallbackModels`。
- 运行时模型表的字段：`effortLevels` / `effortSchemaPath` / `defaultEffortLevel` / `thinkingToggleable` / `defaultThinkingEnabled`。
- **AMRF schema 是上游声明的能力来源**；schema 缺失时，官方对 effort 与 thinking 的处理见 §5、§6。

### 3.1 `bd()` 读到的到底是哪份模型表

`bd()` 是全部 effort / thinking 判定的唯一数据入口：

```js
function bd(){return ASe.getAvailableModels()}
```

`ASe` 是一个可被替换的全局变量（`y4e(t){ASe=t}`），默认值为空表：

```js
nAu={models:[],defaultModel:{id:"",name:""}},ASe={getAvailableModels:c(()=>nAu,"getAvailableModels"), ...}
```

运行时它被替换为 model registry provider：

```js
this.modelConfigProvider=new gUp({...}), y4e(this.modelConfigProvider)
```

`gUp` 类的完整数据链路（本轮逐层取证）：

| 层 | 位置 | 做什么 |
| --- | --- | --- |
| 1. 网络 | `Jyu` | 分页拉 `ListAvailableModels`，**逐模型**映射成中间形状 |
| 2. 归一化 | `yUp` → `AUp` | 把中间形状转成运行时模型表 |
| 3. 缓存 | `this.cache=r` | 存入 provider |
| 4. 读取 | `getAvailableModels(){return this.cache}` | `bd()` 取到这份 |

第 1 层（`Jyu` 内）的逐模型映射，**没有**任何兜底：

```js
{modelId:v.modelId, modelName:v.modelName, description:v.description, provider:v.modelProvider,
 rateMultiplier:v.rateMultiplier, rateUnit:v.rateUnit,
 maxInputTokens:v.tokenLimits?.maxInputTokens,
 refusalFallbackModels:v.refusalFallbackModels?.map(I=>I.modelId).filter(I=>!!I),
 effortLevels:_?.levels, effortSchemaPath:_?.schemaPath, defaultEffortLevel:_?.defaultLevel,
 thinkingToggleable:E?.toggleable, defaultThinkingEnabled:E?.defaultEnabled}
```

第 2 层（`AUp`）**有**兜底：

```js
function yUp(t){let e=t.models.map(AUp), ...}
function AUp(t){
  let e={id:t.modelId,name:t.modelName??t.modelId};
  ...
  t.effortLevels&&t.effortLevels.length>0&&(
    e.effortLevels=t.effortLevels,
    e.effortSchemaPath=t.effortSchemaPath,
    e.defaultEffortLevel=t.defaultEffortLevel??t.effortLevels[0]),   // ← 兜底
  t.thinkingToggleable&&(e.thinkingToggleable=!0, ...),
  e
}
```

**这一层兜底是本轮最重要的发现**，它直接改变了 §6.5 的结论：

- 在 `bd()` 链路上，**只要模型有 `effortLevels`，`defaultEffortLevel` 必然非空**（缺失时被补成 `effortLevels[0]`）。
- 因此 `Ibt` 的第三道门 `!s.defaultEffortLevel` **在主路径上永不触发**；`oAu` 的第二分支（"无默认档但有档位"）在 `bd()` 链路上**不可达**。
- 结论：**官方主路径实际只有一套档位选择逻辑**（`Ibt`），§6.5 里"两套互斥逻辑"的表述只对"绕过 `AUp` 的其它数据源"成立。
- 推论：官方"显式档位不可用时回落到 `effortLevels[0]`"这个行为，是 `AUp` 兜底 + `Ibt` 取 `defaultEffortLevel` 两条规则**叠加**的结果，而不是 `Ibt` 内部写死的。
- 注意 `AUp` 的兜底**只覆盖 `defaultEffortLevel`**：`effortSchemaPath` 是无条件透传，可能为 `undefined`（但 `eAu` 保证：有 `levels` 就一定有 `schemaPath`，故二者同生共死）。
- 另有一处**不经过 `AUp`** 的模型构造（`_meta.kiro` 用的 `defaultEffortLevel:n.defaultEffortLevel`，无 `??` 兜底），但它不喂 `bd()`，不影响 §6.5。

### 3.2 `tokenLimits.maxOutputTokens` 官方客户端从不读取

上游 `TokenLimits` 形状确实含两个成员：

```js
Fhu="maxInputTokens", jhu="maxOutputTokens", Bgu=[3,Wn,Mfu,0,[Fhu,jhu],[1,1]]
```

但本轮全扩展检索的结果是：

| 检索项 | 命中 |
| --- | --- |
| `maxOutputTokens` 字面量 | 2 处，**全部**在常量名表（`jhu="maxOutputTokens"` 与 `Bgu` 形状定义） |
| `.maxOutputTokens` 运行时属性访问 | **0 处** |
| `tokenLimits?.maxInputTokens` 读取点 | 有（`Jyu` 映射内） |
| `tokenLimits` 出现 | 3 处（形状定义 ×2 + `Jyu` 映射 ×1） |

事实：

- 官方客户端**只读 `tokenLimits.maxInputTokens`**，从不读 `maxOutputTokens`。
- 因此官方**没有**任何"按输出窗口 clamp `max_tokens`"的行为。
- 项目当前按 `tokenLimits.maxOutputTokens` clamp `max_tokens`（见 `converter/effort.rs`）是**项目自己的决定**，不是官方行为；上游确实下发该字段，客户端只是不用它。

## 4. thinking 能力判定（`tAu`）

```js
function tAu(t){
  if(!t)return;
  let e=t.properties?.thinking?.properties?.type, r=e?.enum;
  if(!Array.isArray(r)||r.length===0||!r.includes("disabled")||!r.includes("adaptive"))return;
  let n=e?.default;
  return{toggleable:!0, ...typeof n=="string"?{defaultEnabled:n!==gSe}:{}}
}
```

事实：

- 判据是 `properties.thinking.properties.type.enum` **同时包含 `disabled` 与 `adaptive`**。
- 不满足 → **整个函数返回 undefined**，该模型既没有 `thinkingToggleable` 也没有 `defaultThinkingEnabled`。
- `defaultEnabled = (type.default !== "disabled")`；schema 未声明 `default` 时该字段**不存在**（不是 `true`）。

`gSe="disabled"`，`g4e="adaptive"`。

## 5. thinking type 决策（`A4e`）

```js
function A4e(t,e){
  if(!t||t==="auto")return{thinkingType:void 0};
  let r=bd().models.find(n=>n.id===t);
  return r?.thinkingToggleable!==!0
    ? {thinkingType:void 0}
    : e==="disabled"||e==="adaptive"
      ? {thinkingType:e}
      : {thinkingType: r.defaultThinkingEnabled??!0 ? g4e : gSe}
}
```

事实（逐条）：

1. `modelId` 为空或为 `auto` → `thinkingType` 为 undefined（不发 thinking）。
2. 模型 `thinkingToggleable !== true` → `thinkingType` 为 undefined（**完全不发** thinking 字段）。
3. 客户端显式值是 `"disabled"` 或 `"adaptive"` → 直接采用。
4. 其他值（含 Anthropic 风格的 `"enabled"`）→ 走默认分支，等价于"未指定"。
5. 默认分支：`defaultThinkingEnabled ?? true` → `adaptive`，否则 `disabled`。`defaultThinkingEnabled` 来自 schema 的 `thinking.properties.type.default`（§4）。
6. 官方客户端**只认** `disabled` / `adaptive` 两个值。`"enabled"` 不被识别，等同未指定。

调用侧（`GMr`）决定"客户端是否显式"：

```js
function GMr(t,e){let{thinkingType:r}=A4e(t,e.thinking===void 0?void 0:e.thinking?g4e:gSe);return{modelId:t,choices:e,maxInputTokens:w4e(t),thinkingType:r,...oAu(t,e.effortLevel,r)}}
```

- 客户端 `thinking` 字段**缺失** → 传 `undefined` 给 `A4e` → 走默认分支。
- 客户端 `thinking` 为真 → 传 `g4e`（`adaptive`）。
- 客户端 `thinking` 为假 → 传 `gSe`（`disabled`）。

## 6. effort 枚举、字段路径与档位选择

### 6.1 字段路径探测顺序（`Zyu`）

```js
Zyu=[{path:"output_config",extract:c(t=>t.properties?.output_config?.properties?.effort,"extract")},
     {path:"reasoning",   extract:c(t=>t.properties?.reasoning?.properties?.effort,"extract")}]
```

事实：**先 `output_config`，后 `reasoning`**，按顺序取第一个有非空 `enum` 的。

### 6.2 枚举解析（`eAu`）

```js
function eAu(t){if(t)for(let{path:e,extract:r}of Zyu){let n=r(t);if(n?.enum&&n.enum.length>0)return{levels:n.enum,schemaPath:e,defaultLevel:n.default}}}
```

事实：

- `levels` = effort 属性的 `enum` 数组。
- `schemaPath` = 该 effort 所在的字段路径（`"output_config"` 或 `"reasoning"`）。
- `defaultLevel` = **该 effort 属性自身的 `default` 字段**（如 `properties.output_config.properties.effort.default`）。schema 未声明时为 `undefined`。

### 6.3 thinking 关闭时的档位过滤（`xui` / `m4e`）

```js
m4e=new Set(["xhigh","max"]);
function xui(t,e){return e==="disabled"?t.filter(r=>!m4e.has(r)):t}
```

事实：`thinkingType === "disabled"` 时，从档位集合里剔除 `xhigh` 与 `max`；其余情况原样返回。

**`xui` 与 `ySe` 的职责划分**（两者都基于同一个 `m4e=new Set(["xhigh","max"])`，但作用对象与时机不同）：

| | `xui(levels, thinkingType)` | `ySe(level, levels)` |
| --- | --- | --- |
| 输入 | 档位**集合** | 单个**档位** + 档位集合 |
| 作用 | 从集合中**剔除** `xhigh`/`max` | 把一个 `xhigh`/`max` **替换**为集合中的其他档位 |
| 触发条件 | `thinkingType === "disabled"` | `thinkingType === "disabled"`（调用方判断）且档位本身 ∈ `m4e` |
| 返回 | 新集合（**不修改**原数组） | 单个档位或 `undefined` |
| 调用点 | `Ibt` 内、`oAu` 第二分支内（共 2 处） | `Cbt`、`VMr`、`xHr`、`U4t`（共 4 处） |

- `xui` 是**集合级**的可用性过滤：它决定"哪些档位此刻可用"，被 `Ibt`/`oAu` 用来做"显式档位是否在可用集合内"的判断与默认值选取。
- `ySe` 是**单值级**的替换：它把一个被禁用的高档位映射到另一个具体档位，本身**不判断**档位是否可用。
- 关键：`ySe` 的输入集合是**未过滤**的原始 `effortLevels`（`Mue`/`VMr`/`Cbt` 传的都不是 `xui` 的输出）。`ySe` 内部自己 filter 掉 `m4e`，所以它的返回值**必然不是** `xhigh`/`max`（或是某个非 `xhigh`/`max` 档，或是 `undefined`）。
- 因此**两套机制互不替代**：`xui` 管"可用集合"，`ySe` 管"具体降级取值"，二者都只在 `thinkingType === "disabled"` 时生效。

### 6.4 降级（`ySe`）

```js
function ySe(t,e){if(!t||!m4e.has(t))return t;let r=e.filter(n=>!m4e.has(n));return r.length>0?r[r.length-1]:void 0}
```

事实：

- 只有当档位**本身就是** `xhigh`/`max` 时才降级（首行 `!m4e.has(t)` 守卫）。
- 降级目标 = 集合中**非** `xhigh`/`max` 的**最后一个**（`r[r.length-1]`）。
- 集合里没有非 `xhigh`/`max` 的档位 → 返回 `undefined`（该档位不可用）。
- 注意 `ySe` 接收的是**未过滤**的原始档位集合。

**这里没有排序，取的是"最后一个"而不是"最高档"**——这是官方实现的事实，文档此前把它解读成了"最高档"，属于误读：

- `ySe` 内部**只有一次 `filter`**，没有任何 `sort`。`r[r.length-1]` 取的是 filter 之后的**末尾元素**，即"过滤后集合中声明顺序上的最后一个"。
- 官方代码**从不对 `effortLevels` 排序**：全扩展检索 `sort` 无任何一处作用于 effort 档位集合；`effortLevels` 的唯一来源是上游 schema 的 `enum` 数组（§6.2），即**按上游声明的顺序原样保留**。
- 因此"最后一个 = 最高档"只在**上游恰好按升序声明 enum** 时成立。这是上游数据的性质，不是客户端代码的保证。
- 推论：若上游把 enum 声明为降序或乱序，`ySe` 会降级到**声明顺序上最后一个非 `xhigh`/`max` 档**，可能低于"最高档"。实现时**不得**假设 `enum` 已排序，也**不得**自行排序（自行排序会偏离官方行为）。

### 6.5 档位选择（`Ibt` / `oAu`）

```js
function Ibt(t,e,r){
  let n={effortLevel:void 0,effortSchemaPath:void 0,effortLevels:void 0};
  if(!t||t==="auto")return n;
  let s=bd().models.find(u=>u.id===t);
  if(!s?.effortSchemaPath||!s.effortLevels?.length||!s.defaultEffortLevel)return n;
  let a=xui(s.effortLevels,r);
  if(a.length===0)return n;
  let l=a.includes(s.defaultEffortLevel)?s.defaultEffortLevel:a[a.length-1];
  return{effortLevel:e&&a.includes(e)?e:l, effortSchemaPath:s.effortSchemaPath, effortLevels:s.effortLevels}
}
```

```js
function oAu(t,e,r){
  let n=bd().models.find(a=>a.id===t);
  if(n?.defaultEffortLevel||!n?.effortLevels?.length)return Ibt(t,e,r);
  let s=xui(n.effortLevels,r);
  return s.length===0
    ? {effortLevel:void 0,effortSchemaPath:void 0,effortLevels:void 0}
    : {effortLevel: e!==void 0&&s.includes(e)?e:s[0], effortSchemaPath:n.effortSchemaPath, effortLevels:n.effortLevels}
}
```

事实——档位选择的完整判定表（`a` = `xui` 过滤后的集合，`e` = 客户端显式档位）：

**先看 `oAu` 的分派**（`oAu` 只有两个出口，二者互斥）：

| `oAu` 的 `if` 条件 | 走向 |
| --- | --- |
| `n?.defaultEffortLevel` **为真**（模型声明了 effort 默认档） | → `Ibt(t,e,r)` |
| `!n?.effortLevels?.length`（模型**没有**档位） | → `Ibt(t,e,r)` |
| 既无默认档、又有档位 | → `oAu` 第二分支（内联，不走 `Ibt`） |

**再走 `Ibt` 内部**（`Ibt` 有**三道独立的提前返回**门）：

| `Ibt` 内的门 | 结果 |
| --- | --- |
| `!t \|\| t==="auto"` | 不发 effort（返回全 undefined） |
| `!s?.effortSchemaPath` | 不发 effort |
| `!s.effortLevels?.length` | 不发 effort |
| `!s.defaultEffortLevel` | 不发 effort |
| `a = xui(...)` 过滤后为空 | 不发 effort |

**关键点：`Ibt` 的第三道门 `!s.defaultEffortLevel` 是硬性提前返回。** 就 `Ibt` 函数本身而言：

- `Ibt` **只在模型声明了 `effort.default` 时才可能产出档位**。`defaultEffortLevel` 缺失 → 直接返回全 undefined，**不会**退而使用 `effortLevels[0]`。
- 因此"无默认档时用过滤后集合的最低档"这条规则**只存在于 `oAu` 的第二分支**（即模型无默认档但有档位时），**`Ibt` 路径下不可能发生**。
- `Ibt` 与 `oAu` 第二分支是**两套互斥逻辑**：默认档来源不同，且 `Ibt` 额外要求 `effortSchemaPath` 存在（缺失即不发 effort），`oAu` 第二分支不检查它。

### 6.5.1 但在 `bd()` 主链路上，第二分支不可达

**本轮修正。** 上一版把上表的两条分支当成官方运行时都会走到的路径，这是错的。§3.1 已取证：模型表经 `AUp` 归一化时，`defaultEffortLevel` 被兜底为 `t.defaultEffortLevel??t.effortLevels[0]`。

因此在 `bd()` 链路上：

| 条件 | `AUp` 之后的结果 |
| --- | --- |
| 模型有 `effortLevels` | `defaultEffortLevel` **必然非空**（缺失时补成 `effortLevels[0]`） |
| 模型无 `effortLevels` | `effortLevels` / `effortSchemaPath` / `defaultEffortLevel` **三者全缺** |

于是：

- `oAu` 的 `if` 条件 `n?.defaultEffortLevel || !n?.effortLevels?.length` 在 `bd()` 链路上**恒为真** → **永远走 `Ibt`**。
- `oAu` 的第二分支（"无默认档但有档位"）**在主路径上不可达**。
- `Ibt` 的第三道门 `!s.defaultEffortLevel` 在主路径上**永不触发**（有档位时它必非空；无档位时 `effortLevels` 也空，第二道门先拦住）。
- 净效果：官方主路径**只有一套**档位选择逻辑，即 `Ibt`。上文"两套互斥逻辑"的表述仅对**绕过 `AUp` 的其它数据源**成立。

**各分支的最终取值**（仅在理论上的 `oAu` 第二分支可达时才有意义）：

| 分支 | 默认档 | 显式档位 `e` ∈ `a` | 显式档位 `e` ∉ `a` |
| --- | --- | --- | --- |
| `Ibt`（有 `defaultEffortLevel`） | `l` = `defaultEffortLevel` ∈ `a` ? `defaultEffortLevel` : `a[a.length-1]` | 用 `e` | 用 `l` |
| `oAu` 第二分支（无默认档、有档位） | `s[0]`（过滤后集合的**第一个**） | 用 `e` | 用 `s[0]` |

要点：

- 官方**没有**"向上取最近可用档"这回事。显式档位不可用时直接回落到该模型的默认档。
- 在 `bd()` 主链路上，"默认档"实际等于 `defaultEffortLevel ?? effortLevels[0]`，**上游 schema 没声明 `effort.default` 时就是 `effortLevels[0]`**——这是 `AUp` 兜底 + `Ibt` 取默认档**叠加**的结果，不是 `Ibt` 内部写死的。
- 但这个回落值**不保证在 `xui` 过滤后的集合内**：`Ibt` 里 `a.includes(s.defaultEffortLevel)?s.defaultEffortLevel:a[a.length-1]`——若默认档本身是 `xhigh`/`max` 且 thinking 已关闭，它会**再退**到过滤后集合的声明顺序最后一个。
- `Ibt` 与 `oAu` 第二分支返回的 `effortLevels` 字段都是**未过滤的原始档位集合**（`s.effortLevels` / `n.effortLevels`），过滤只用于选档。

### 6.6 客户端侧的预降级（`VMr`）

```js
function VMr(t,e){
  let{thinkingType:r}=A4e(t,e.thinking===void 0?void 0:e.thinking?g4e:gSe),
      n=bd().models.find(s=>s.id===t)?.effortLevels??[];
  return GMr(t,{effortLevel: e.effortLevel!==void 0&&n.includes(e.effortLevel)&&r==="disabled" ? ySe(e.effortLevel,n) : e.effortLevel, thinking:e.thinking})
}
```

事实：当 thinking 为 `disabled`、且客户端显式档位在**原始**档位集合内时，档位先经 `ySe` 降级（`xhigh`/`max` → 次高档），再进入 `GMr`/`oAu` 的选择流程。

### 6.7 Session 层与 model routing 层（`U4t` / `xHr` / `OMs` / `Jke` / `bln` / `Mue` / `x1s`）

官方还有**一整套与 §5/§6 并行的 session 层判定**，它不经过 `A4e`/`Ibt`/`oAu`，而是直接读模型表。文档此前完全没记这一层。

```js
function Jke(t){if(!t||t==="auto")return!1;let{models:e}=bd();return e.find(r=>r.id===t)?.thinkingToggleable===!0}
function OMs(t){if(!t||t==="auto")return!0;let{models:e}=bd();return e.find(r=>r.id===t)?.defaultThinkingEnabled??!0}
function sqt(t,e){return e??OMs(t)}
function NMs(t,e){if(Jke(t))return sqt(t,e)?g4e:gSe}
function Mue(t){if(!t||t==="auto")return[];let{models:e}=bd();return e.find(r=>r.id===t)?.effortLevels??[]}
function bln(t){if(!t||t==="auto")return;let{models:e}=bd();return e.find(r=>r.id===t)?.effortSchemaPath}
function U4t(t,e,r,n){
  if(!Jke(e)||sqt(e,r))return t;
  let s=ySe(t,Mue(e));
  return s!==t&&C.debug("session.effort.capped_for_thinking_off",{...n,modelId:e,from:t,to:s}),s
}
function xHr(t,e){return{...t,thinkingType:e,effortLevel:e==="disabled"?ySe(t.effortLevel,t.effortLevels??[]):t.effortLevel}}
```

事实：

- `Jke` = "模型是否支持 thinking 切换"（等价于 §4 的 `thinkingToggleable === true`）。
- `OMs` = "模型默认是否开 thinking"（`defaultThinkingEnabled ?? true`）。
- `sqt` = session 显式值优先，否则用模型默认；`NMs` 在模型**支持**切换时返回 `adaptive`/`disabled`，不支持时返回 `undefined`（与 `A4e` 的差异见下表）。
- `U4t` 是 session 层的**降级闸门**：仅当模型支持切换**且** session 的 thinking 为 `disabled` 时才降级档位（`ySe`），降级打 debug 日志 `session.effort.capped_for_thinking_off`（注意与 §7 的 `llm.request.effort.capped_for_thinking_off` 是**两处不同的日志**）。
- `xHr` 在 model 对象上覆写 `thinkingType`，并在 `thinkingType === "disabled"` 时降级 `effortLevel`。
- `Mue`/`bln` 是**只读**取档位集合 / 字段路径，不做过滤；`xui` 的过滤结果不参与这两个函数。

**model routing 层（`x1s`）**：

```js
function x1s(t,e){
  let r=Mue(t),n;
  r.length===0?n=void 0
    :e.effortLevel!==void 0&&r.includes(e.effortLevel)?n=e.effortLevel
    :n=bd().models.find(a=>a.id===t)?.defaultEffortLevel??r[0];
  let s={provider:Ic,modelId:t,maxInputTokens:w4e(t),effortLevel:n,effortSchemaPath:bln(t),effortLevels:r};
  return e.thinkingType!==void 0
    ? xHr(s,e.thinkingType)
    : {...s,effortLevel:U4t(n,t,e.thinkingEnabled,{sessionId:e.sessionId,entryPath:e.entryPath}),thinkingType:NMs(t,e.thinkingEnabled)}
}
```

事实：

- `x1s` 的默认档选取规则**与 §6.5 不同**：显式档位 ∉ `r` 时回落 `defaultEffortLevel ?? r[0]`——**没有 `xui` 过滤**，也没有 `effortSchemaPath` 存在性检查。
- `x1s` 有两条互斥出口：`thinkingType !== undefined` → `xHr`（只改 `effortLevel`）；否则 → `U4t` + `NMs`（session 层闸门）。

**`A4e` 与 `NMs` 的差异**（两者都产出 `thinkingType`，但语义不同）：

| | `A4e`（§5） | `NMs`（session 层） |
| --- | --- | --- |
| 参数 | `(modelId, 客户端 thinking 布尔)` | `(modelId, session thinkingEnabled)` |
| 客户端显式值 | `thinking === undefined` ? 未指定 : (真→`adaptive`, 假→`disabled`) | `sqt` = `thinkingEnabled ?? OMs(modelId)` |
| 模型不支持切换时 | 返回 `{thinkingType: undefined}` | 返回 `undefined`（**同样不发**） |
| 客户端显式 `disabled` | 原样采用 `disabled` | `sqt` 为假 → `disabled`（同） |
| 客户端显式 `adaptive` | 原样采用 `adaptive` | `sqt` 为真 → `adaptive`（同） |
| 差异点 | 显式值**只认** `disabled`/`adaptive`，其它值（如 `enabled`）走默认分支 | 值是**布尔**，不存在"非法值"分支 |

**第三条 `Ibt` 直调路径（子代理）**——文档此前漏记：

```js
Z=A4e(ue,f.model.thinkingType),
G=ne||q?{...f.model,modelId:ue,maxInputTokens:w4e(ue),choices:{...f.model.choices,effortLevel:q},...Z,...Ibt(ue,q,Z.thinkingType)}:f.model
```

事实：

- 子代理启动时**直接调用 `Ibt`**，绕过 `GMr`/`oAu`。
- 传参是 `Ibt(ue, q, Z.thinkingType)`：`ue` = 实际模型 id，`q` = 客户端显式 effortLevel，`Z.thinkingType` = `A4e` 的结果。
- 因为 `Ibt` 有 `!s.defaultEffortLevel` 这道硬门，**模型未声明 `effort.default` 时，子代理路径同样不发 effort**。
- `Ibt` 的第三参 `r` 是 `thinkingType`，会传给 `xui` 做过滤——即子代理路径也遵守 "thinking 关闭则 `xhigh`/`max` 不可用"。

**`Ibt` 的三个调用点小结**：

| 调用点 | 场景 | 显式档位来源 | 备注 |
| --- | --- | --- | --- |
| `oAu` 内（§6.5） | 主模型请求 | `e.effortLevel` | 有 `defaultEffortLevel` 时走此路径 |
| 子代理直调（上方） | 子代理换模型 | `q`（`f.model.effortLevel`） | 绕过 `oAu` |
| （`oAu` 第二分支内联） | 无 `defaultEffortLevel` 但有档位 | `e.effortLevel` | 不走 `Ibt`，逻辑内联 |

## 7. AMRF 组装（`Cbt` / `bui` / `rAu`）——原生路径

```js
function Cbt(t){
  let e=t.thinkingType==="disabled"?ySe(t.effortLevel,t.effortLevels??[]):t.effortLevel;
  e!==t.effortLevel&&C.debug("llm.request.effort.capped_for_thinking_off",{requested:t.effortLevel,applied:e,effortSchemaPath:t.effortSchemaPath});
  let r={};
  return e&&t.effortSchemaPath&&Object.assign(r,bui(e,t.effortSchemaPath)),
         t.thinkingType&&Object.assign(r,rAu(t.thinkingType)),
         Object.keys(r).length>0?r:void 0
}
```

```js
function bui(t,e){switch(e){case"output_config":return{output_config:{effort:t}};case"reasoning":return{reasoning:{effort:t}}}}
function rAu(t){return{thinking:{type:t}}}
```

事实：

- effort 与 thinking 是**两次独立的 `Object.assign`**，互不依赖：effort 走 `bui`（按 `effortSchemaPath` 决定信封），thinking 走 `rAu`。
- `bui` 只有两个分支：`output_config` → `{output_config:{effort}}`；`reasoning` → `{reasoning:{effort}}`。
- **`rAu` 产出的 thinking 信封只有 `{type}`，没有 `display`。** 官方原生路径**不发送** `thinking.display`。
- thinking 信封的挂载条件只是 `t.thinkingType` 非空——即 §5 的 `A4e` 判定结果。thinking 与 effort 字段路径**无关**：`output_config` 路径的模型照样会带 `thinking` 信封，反之亦然。
- `Cbt` 内还有一次独立的 `ySe` 降级：`thinkingType === "disabled"` 时把 `xhigh`/`max` 降级（用的是**未过滤**的 `effortLevels`）。降级会打 debug 日志 `llm.request.effort.capped_for_thinking_off`。
- 三者全空（无档位 / 无 schema path / 无 thinkingType）→ **`Cbt` 返回 `undefined`**。注意这是函数返回值层面的事实；这个 `undefined` 是否会变成"报文里没有该字段"，要经过 `buildRequestCommand` 与 Smithy 序列化两层，见 §7.1。

### 7.1 "属性一直在"与"字段不出现"是两件事

`buildRequestCommand` **无条件**把 `additionalModelRequestFields` 放进命令对象：

```js
buildRequestCommand(t,e,r,n,s){
  let a=new ypd({conversationState:e,profileArn:t.profileArn,additionalModelRequestFields:r,agentMode:this.agentMode,systemPrompt:n});
  ...
}
```

因此**属性确实一直在**（own property 恒存在），值为 `undefined` 时属性值就是 `undefined`。但"属性在 JS 对象上"≠"键在 HTTP 请求体里"——两者由 Smithy 序列化层区分开：

**请求体 schema**（`eld` 为 `GenerateAssistantResponseRequest` 的结构体定义）：

```js
eld=[3,un,Uid,0,[ead,hOt,Hsd,Gsd,ecd],[[()=>Qcd,0],0,[0,{[Yae]:Pcd}],15,[()=>cp,0]],1]
```

- 成员顺序：`ead`(conversationState) → `hOt`(profileArn) → `Hsd`(agentMode) → `Gsd`(additionalModelRequestFields) → `ecd`(systemPrompt)。
- 第 5 槽（`e[4]`）是**成员名数组**，第 6 槽（`e[5]`）是**与之一一对应的描述符数组**。描述符可以是**裸数字**（只有 schema 类型码、无 traits），也可以是 `[schema, traits]` 二元组。
- `structIterator` 会把描述符统一包成 `[描述符, 0]` 交给 `Nn` 构造：`for(; eNr(s);) n.push(s[1]), s=s[0]`（`eNr` = 长度为 2 的数组），逐层剥出 traits 与 schema。
- `Gsd` 的描述符是**裸数字 `15`** → schema 类型码 `15`，而 `isDocumentSchema(){return this.getSchema()===15}`——即 AMRF 是 Smithy **document 类型**（任意 JSON）。
- `Gsd` 的 traits 为 `0`，经 `Iie(0)`（`for(let n of["httpLabel","idempotent","idempotencyToken","sensitive","httpPayload","httpResponseCode","httpQueryParams"])(t>>r++&1)===1`）解码为**空对象**——**无任何 trait**，特别地**没有** `idempotencyToken`。
- 对照 `Hsd`(agentMode) 的描述符是 `[0,{[Yae]:Pcd}]`（`Yae="httpHeader"`），`ead`(conversationState) 是 `[()=>Qcd,0]`，`hOt`(profileArn) 是裸数字 `0`——可见 traits 槽确实独立于类型码存在。

**序列化时 `undefined` 成员被跳过**（`@smithy/core` 的 JSON 结构体序列化器）：

```js
writeStruct(e,r){this.ensure(2),this.json[this.i++]=sci;let n=!1,s=typeof r.__type=="string",a;s&&(a=new Set);for(let[l,u]of e.structIterator()){let d=r[l];if(d==null&&!u.isIdempotencyToken())continue; ... }}
```

事实：

- `d==null` 同时覆盖 `undefined` 与 `null`；由于 `Gsd` 的 traits 不含 `idempotencyToken`，`undefined` 成员**直接 `continue` 跳过**，不会写出键。
- 有值时走 `writeValue` → `s.isDocumentSchema()` → `writeMap(s,r,!0)`，按 JSON document 写出。
- 所以：**属性一直在 JS 对象上，但值为 `undefined` 时线上请求体中不出现 `additionalModelRequestFields` 键**。
- 官方其它路径也显式做同样判断，例如 `invoke` 里 `...r.additionalModelRequestFields!==void 0&&{additionalModelRequestFields:r.additionalModelRequestFields}`；模型切换时则显式置 `undefined`：`additionalModelRequestFields:n===s?f.additionalModelRequestFields:void 0`。
- `/v1/responses` 路径同样安全：`cfd` 里 `...lfd(u)`，`lfd(undefined)` 时 `Iui(undefined)` 因 `typeof undefined!=="object"` 返回 `undefined`，最终 `lfd` 返回 `{}`，不产生 `reasoning` 字段。

## 8. `/v1/responses` 的字段读取（`Iui` / `Cui` / `Tui`）

```js
function Iui(t){if(typeof t!="object"||t===null)return;let e=t.thinking;if(typeof e!="object"||e===null)return;let r=e.type;return typeof r=="string"&&r.length>0?r:void 0}
function Cui(t){if(!(typeof t!="object"||t===null))for(let e of Object.keys({output_config:!0,reasoning:!0})){let r=t[e];if(typeof r=="object"&&r!==null){let n=r.effort;if(typeof n=="string"&&n.length>0)return n}}}
function Tui(t){if(typeof t!="object"||t===null)return;let e=t.thinking;if(typeof e!="object"||e===null)return;let r=e.display;return typeof r=="string"&&r.length>0?r:void 0}
```

事实：

- `Cui` 同样**先 `output_config` 后 `reasoning`**。
- `Tui` 读 `thinking.display`——**仅**在 `/v1/responses` 路径被消费，映射为 `reasoning.summary`。
- 官方扩展全文精确检索 `thinking.display` 命中 0 处：**没有任何一处把 `display` 写进发往 `GenerateAssistantResponse` 的 AMRF**。

### 8.1 `/v1/responses` 链路是真实可达的，由 feature flag 切换

文档此前只记录了 `lfd`/`cfd`/`pfd` 的存在，没有证明这条链路在运行时会被走到。本轮取证补上：

**开关**（`Bzi` / `xud` / `Co`）：

```js
function Bzi(t){return t?.get(Co.AB_CREATE_RESPONSE_API)===xud}   // xud="create_response"
```

**消费点**（`usesCreateResponsePath`）：

```js
usesCreateResponsePath(){return Bzi(this.featureConfigRegistry)}
```

**分派处**（`_streamResponseChunksOnce`，`f=this.usesCreateResponsePath()`）：

```js
let f=this.usesCreateResponsePath();
C.info("q.converse.dispatch",{api:f?"CreateResponse":"GenerateAssistantResponse", modelId:this.modelType||"<empty>", ...});
```

事实：

- 两条链路**由同一个 feature flag 二选一**：`create_response_api === "create_response"` 时走 `/v1/responses`，否则走 `GenerateAssistantResponse`。
- 遥测事件名也随之切换（`q.converse.dispatch` 的 `api` 字段）。
- 所以 `/v1/responses` **不是"仅存代码"**，而是一条受服务端 feature flag 控制的真实链路。

**两条链路的 AMRF 是同一个对象**（`cfd` 直接展开会话态的 `additionalModelRequestFields`）：

```js
function cfd(t){let{conversationState:e,systemPrompt:r,modelId:n,origin:s,profileArn:a,agentMode:l,additionalModelRequestFields:u}=t, ...,
  ...lfd(u)}}
```

- `cfd` 收到的 `u` 就是会话态那个 AMRF 对象（原生链路经 `Cbt` 组装，见 §7）。
- 但**同一个对象被两条链路按不同规则解读**：
  - 原生：`Cbt` → `bui`（按 `effortSchemaPath` 产出 `{output_config:{effort}}` 或 `{reasoning:{effort}}`）+ `rAu`（`{thinking:{type}}`）。
  - `/v1/responses`：`lfd` → `Iui`（读 `thinking.type`）+ `Cui`（读 `output_config.effort` 或 `reasoning.effort`）+ `Tui`（读 `thinking.display`）→ 统一产出 `{reasoning:{type,effort,summary}}`。
- 推论：**`/v1/responses` 链路会消费 `thinking.display`，原生链路不会**（§7、§8）。

#### 8.1.1 开关的完整判定链

判定式只有一行（`Bzi`）：

```js
function Bzi(t){return t?.get(Co.AB_CREATE_RESPONSE_API)===xud}   // xud="create_response"
```

即 **flag 的值必须精确等于字符串 `"create_response"`**——不是 `true`，不是 `"enabled"`。

**flag 的键名与默认值**（`Co` 常量表 + `RR` 默认表）：

```js
AB_CREATE_RESPONSE_API:"create_response_api",   // Co
create_response_api:{default:"disabled"},        // RR
```

事实：

- 默认值是字符串 `"disabled"`，**默认走原生 `GenerateAssistantResponse`**。
- 全文只有两个与该 flag 相关的字符串字面量：`"create_response_api"`（键名，1 次）与 `"create_response"`（开启值，1 次）。
- 另两个同形态的字符串型 flag 是 `memory_internal_enabled` 与 `mom_config`，默认值同样是 `"disabled"`（`RR` 里共 3 个 `default:"disabled"`）。

**取值来源：多层 provider 依次求值**（`LQr` 的 `resolve`）：

```js
resolve(t){
  for(let e of this.providers){
    let r;
    try{ r=e.get(t) }catch(n){ C.warn("[FeatureConfigRegistry] provider threw resolving feature; treating as no value",{...}); continue }
    if(r!==ij) return{value:r, source:e.source}
  }
  return{value:RR[t].default, source:"default"}
}
```

- `ij` 是"无值"哨兵（`ij=Symbol("feature-config/no-value")`），provider 返回它表示"我不管这个 key"。
- provider 按 `Qqi=["governance","env","client","session","experiment"]` 的顺序排序，**第一个返回非哨兵的值胜出**。
- 官方扩展中**实际实现**的 provider 只有两个：`source="env"`（`aHe`）与 `source="experiment"`（`UQr`）；`governance`/`client`/`session` 三个来源**在扩展里没有实现**。

**因此能真正打开这个开关的只有 experiment provider**（服务端下发）：

```js
UQr=class{ source="experiment";
  constructor(t){this.service=t}
  get(t){ let e=this.service.getConfiguration(),
           r=(0,Kqi.createHash)("sha256").update(fud+t).digest("hex");
           return Object.prototype.hasOwnProperty.call(e,r) ? e[r] : ij }
}
```

- 实验配置来自 KRS 端点 `GetFeatureConfiguration`（`at("getFeatureConfiguration","network",...)` → `uud`），带 `x-kiro-eval-treatments` / `x-kiro-eval-strategy` header（`B0t` + `wsi`）。
- 配置以**哈希键**存储：`sha256("kiro-feature-config-key-salt-3e9b1d7a" + "create_response_api")`。
- 该 flag 的哈希键 = `8242f5d7e306898aef7ec21ccbe97fd8e9150efbe8f093b12c8dc8b7d46fb604`。
- 服务端把该键的值设为 `"create_response"` 时开关打开；设为其它任何值（含 `"disabled"`）都走原生链路。
- 缓存 TTL `ttlMs??9e5`（15 分钟），身份变化（token/profileArn 变化）会丢弃缓存重新拉取。

**env provider 无法打开这个开关**（两道限制）：

```js
aHe=class{ source="env";
  get(t){ let e=Eud[t];
           if(e===void 0) return ij;                       // ← 限制1
           let r=this.env[e]?.trim();
           return r===void 0||r==="" ? ij
                : (r!=="true"&&r!=="false" ? (C.warn("featureConfig.env.unparsable",{...}), ij)   // ← 限制2
                                             : r==="true") } }
```

- **限制 1**：`Eud={..._ud, ...Sud()}` 是「flag 键 → 环境变量名」的映射表，而 `_ud` 里**没有** `AB_CREATE_RESPONSE_API`（`_ud` 只映射 16 个 flag，`Sud()` 只补 `unifiedAgent` / `vibeToDefaultV2Routing` 两个）。键不在表里 → 直接返回 `ij`（无值）。
- **限制 2**：即便在表里，env provider 只接受 `"true"`/`"false"`，其它值一律告警 `featureConfig.env.unparsable` 并返回 `ij`。而本 flag 要的是字符串 `"create_response"`——即使加了映射也解析不出来。
- 全扩展的 `KIRO_FEATURE_*` 环境变量共 18 个，**没有** `KIRO_FEATURE_CREATE_RESPONSE_API`。

**结论**：官方这条 `/v1/responses` 链路**只能由服务端通过实验配置下发打开**，本地无法用环境变量强制启用。默认状态下 Kiro 走原生 `GenerateAssistantResponse`。

**对项目的意义**：项目未实现这条链路（`src-tauri/src` 全文检索 `create_response` / `CreateResponse` 命中 0），统一走原生上游。这也意味着项目的行为等价于官方**默认档**（原生路径），无需考虑 flag 切换带来的差异。

### 8.2 `ReasoningContentEvent` 的线上形状

原生链路的推理内容以 `reasoningContentEvent` 事件下发。其 Smithy 形状定义：

```js
Cxs="redactedContent", Dxs="signature", iU="text",
cRp=[3,xr,xbp,8,[iU,Cxs,Dxs],[[()=>vi,0],21,[()=>vi,0]]]     // xbp="ReasoningContentEvent"
```

事实：

- 三个成员：`text`（string）、`redactedContent`（blob，类型码 `21`）、`signature`（string）。
- `redactedContent` 的类型码 `21` 由 `isBlobSchema(){let e=this.getSchema();return e===21||e===42}` 判定为 blob。
- 官方扩展把该事件转成内部消息条目：`withReasoningContent(e,r,n,s)` 写入 `{type:"reasoningContent", text, signature, modelId, redactedContent}`，序列化时导出为 `reasoningContent` / `reasoningSignature` / `reasoningModelId` / `reasoningRedactedContent`。
- 项目侧对应：`gateway/stream.rs` 的 `parse_reasoning_text` / `parse_reasoning_signature` 读 `reasoningContentEvent.text` / `.signature`；`converter/content.rs` 处理 `redactedContent`。**项目已覆盖这三个成员**。
- 另有一条 KRS 侧同名形状 `csd="ReasoningContentEvent"`（`lsd="ReasoningContent"`），与上面的事件形状是同一概念的两处声明。

### 8.3 外部实现对账与实跑复核

本文只记录 Kiro 官方的事实。**上游通道的活体实测**与**开源实现的适配对账**（各实现支持哪些通道、社区报告哪些结论、本机 Python 实跑结果）已独立成文：

> **《Kiro 上游通道实测与对账》** —— `docs/Kiro 上游通道实测与对账.md`
>
> 含：本机实跑结论（官方出站 CreateResponse 链路可用、Free 账号的字段拒绝矩阵、两套错误协议）、各开源实现实际适配的上游通道清单（`kiro.rs` 只有 `ide` 一条等）、与社区 `kiro-provider` 的逐字对账、社区报告与本机实跑的差异对照。

该文档的结论**不是官方行为**，只代表当次观测（账号为 Free、region `us-east-1`、模型 `claude-sonnet-4.5` 等），引用时必须带账号前提。

## 9. 官方文档原文（能力表）

### 9.1 Claude 系

| Model | thinking.type | thinking.display | output_config.effort | max_tokens |
| --- | --- | --- | --- | --- |
| Claude Opus 5 | adaptive, disabled | summarized, omitted | low, medium, high, xhigh, max | 1024–128000 |
| Claude Opus 4.8 | adaptive, disabled | summarized, omitted | low, medium, high, xhigh, max | 1024–128000 |
| Claude Opus 4.7 | adaptive, disabled | summarized, omitted | low, medium, high, xhigh, max | 1024–64000 |
| Claude Opus 4.6 | adaptive, disabled | summarized, omitted | low, medium, high, max | 1024–64000 |
| Claude Sonnet 5.5 | adaptive, **between_tools** | summarized, omitted | low, medium, high, xhigh, max | 1024–64000 |
| Claude Sonnet 5 | adaptive, disabled | summarized, omitted | low, medium, high, xhigh, max | 1024–128000 |
| Claude Sonnet 4.6 | adaptive, disabled | summarized, omitted | low, medium, high, max | 1024–64000 |

### 9.2 GPT-5.6 系

| Model | reasoning.effort | max_tokens |
| --- | --- | --- |
| GPT-5.6 Terra | none, low, medium, high, xhigh, max | 1024–128000 |
| GPT-5.6 Sol | none, low, medium, high, xhigh, max | 1024–128000 |
| GPT-5.6 Luna | none, low, medium, high, xhigh, max | 1024–128000 |

### 9.3 字段语义（官方原文）

- `thinking.type`
  - `adaptive`：模型自行判断是否需要扩展思考。
  - `disabled`：在**支持该值**的模型上完全关闭。
  - Claude Sonnet 5.5 用 `between_tools` 代替 `disabled`——它在工具调用之间应用最低思考档，支持 `low`/`medium`/`high`。
- `thinking.display`
  - `summarized`：展示精简版推理。
  - `omitted`：从输出中隐藏。
  - **仅在 `type` 为 `adaptive` 时生效。**
- `max_tokens`：各模型下限均为 1024，上限见上表。

### 9.4 `chat.modelDefaults` 是用户配置，不是请求字段

官方文档给出的配置示例形如：

```json
{
  "chat.modelDefaults": {
    "claude-opus-4.8": {
      "output_config": { "effort": "max" },
      "thinking": { "type": "adaptive", "display": "summarized" },
      "max_tokens": 128000
    }
  }
}
```

事实：

- `chat.modelDefaults` 是 **Kiro 用户侧设置**（`modelDefaults` 键名在扩展全文与 Kiro 主程序 bundle 中**均未出现**，说明由设置层/用户配置持有，不经客户端代码回写请求）。
- 文档里 `thinking.display` 与 `max_tokens` 都出现在这个用户配置结构里；而在**发往上游的原生请求**中，`display` 并不出现（§7、§11.2），`max_tokens` 也不由 `Cbt` 组装（§11.1）。
- 官方文档同一节明确写：`output_config.effort` 列举可用档位，`thinking.type` 与 `thinking.display` 控制推理行为与可见性，`max_tokens` 设置输出长度上限——即**三者都是模型级配置项**，而只有 `effort` 与 `thinking.type` 出现在原生 AMRF 里。
- 补充：`display` 的唯一消费点是 `/v1/responses` 链路的 `Tui`（§8.1），原生链路完全不发。

## 10. 由上述事实可直接推出的结论

1. **原生 AMRF 的信封形状只有两种可能**，由上游 schema 决定：`{output_config:{effort}}` 或 `{reasoning:{effort}}`（`bui`），且 `output_config` 优先。
2. **原生 AMRF 的 thinking 信封恒为 `{thinking:{type}}`**，无 `display`、无 `budget_tokens`、无 `summary`。
3. **默认档位必须来自上游 schema**，不能有固定值：走 `Ibt`，用 `defaultEffortLevel`（被 thinking 过滤后回落到过滤后集合**声明顺序的最后一个**）。
   - **本轮修正**：上一版说"没声明 `effort.default` 则 `Ibt` 直接不发 effort"——这只对 `Ibt` 函数本身成立；在 `bd()` 主链路上，`AUp` 已把 `defaultEffortLevel` 兜底为 `effortLevels[0]`，所以**主路径永远走 `Ibt`**，不存在"无默认档"的情况（§3.1、§6.5.1）。
4. **客户端显式档位不可用时回落到模型默认档**，官方没有"夹到最近档"的行为。主链路上该默认档 = `defaultEffortLevel ?? effortLevels[0]`。
5. **thinking 关闭时 `xhigh`/`max` 不可用**：`xui` 从可用集合中剔除它们，`ySe` 把一个被选中的 `xhigh`/`max` 降级为集合中**声明顺序最后一个**的非 `xhigh`/`max` 档（官方不排序，§6.4）。
6. **模型不支持 thinking 切换时完全不发 thinking 字段**（判据：enum 同含 `disabled` 与 `adaptive`）。
7. **客户端的 `thinking.type: "enabled"` 在官方语义下等价于"未指定"**，由 schema 的 `thinking.type.default` 决定 `adaptive` 还是 `disabled`。
8. **官方只认 `disabled` / `adaptive`**；`between_tools`（Sonnet 5.5）不在官方客户端的识别范围内，且该模型 enum 不含 `disabled` → 按 §4 不发 thinking。
9. **`auto` 路由模型不发 effort 也不发 thinking**（`Ibt`/`A4e` 均对 `auto` 提前返回）。
10. **`max_tokens` 不在原生 AMRF 的官方组装路径内**（`Cbt` 只组装 effort + thinking）。
11. **AMRF 在请求体顶层**，与 `conversationState` 同级；属性在 JS 命令对象上恒存在，但值为 `undefined` 时序列化层跳过该键，线上请求体不出现该字段（§7.1）。
12. **`/v1/responses` 的 `reasoning` 形状不可用于推断原生 AMRF**——那是另一个端点的转换结果。
13. **官方存在三套并行的档位/思考判定**，不能混为一谈：
    - `A4e`/`Ibt`/`oAu`/`GMr`/`VMr`（§5、§6）——客户端模型表路径；
    - `Jke`/`OMs`/`sqt`/`NMs`/`U4t`/`xHr`（§6.7）——session 层；
    - `x1s`（§6.7）——model routing 层，**默认档规则与前两者都不同**（`defaultEffortLevel ?? r[0]`，不过滤、不检查 `effortSchemaPath`）。
    实现若要"对齐官方"，必须先确定自己模拟的是哪一条路径。
14. **降级日志有两处**：`llm.request.effort.capped_for_thinking_off`（§7 `Cbt`）与 `session.effort.capped_for_thinking_off`（§6.7 `U4t`），二者触发条件不同（前者看 `thinkingType`，后者看模型是否支持切换 + session 的 thinking 状态）。
15. **官方的 `/v1/responses` 与项目的同名路由方向相反**（§2.0）：官方那条是**出站**（客户端 → Kiro 后端，OpenAI Responses 协议），项目那条是**入站**（第三方客户端 → 本项目，OpenAI Responses 兼容层）。二者共用同一个 `krsEndpoint`/上游，但**请求体与响应事件名互不相同**——官方出站用 OpenAI 事件名，原生用 KRS 私有事件名（§2.2.2）。
16. **官方两条链路的响应侧事件形状不同**：原生路径有 `ReasoningContentEvent`（带 `text`/`redactedContent`/`signature`，§8.2）；`/v1/responses` 路径没有该事件，改用 `response.reasoning_text.delta` 等 OpenAI 事件名。因此 §8.2 的三成员结论**只适用于原生路径**。
17. **官方出站的 CreateResponse 走裸 HTTP，不走 Smithy RPC**：`KiroRuntimeService` 在官方扩展里只有 4 个 operation（`GenerateAssistantResponse` / `GetFeatureConfiguration` / `InvokeMCP` / `InvokeMCPStream`），**没有 `CreateResponse`**，因此该链路不带 `x-amz-target`，content-type 是 `application/json`。
18. **官方出站的 host 是 `https://runtime.<region>.kiro.dev`**（与原生链路同源不同路径：`/v1/responses` vs `/generateAssistantResponse`）。

> 第 19 条起的结论（外部对账、活体实测、各开源实现的通道清单）属于**非官方事实**，见 **《Kiro 上游通道实测与对账》**。

## 11. 尚未取到实证的项

本轮把原清单三条全部查清，结论如下。**剩余未取证项为零**（限于本扩展源码可观测范围）。

### 11.1 `max_tokens` 在原生链路是否有其它下发路径 —— 已查清：没有

- 扩展内 `max_tokens` 字面量共 15 处命中，逐处核对后确认**全部**属于 stop reason（`end_turn`/`max_tokens` stop reason 常量）、OpenTelemetry 遥测属性（`gen_ai.request.max_tokens`）与迭代恢复逻辑（`max_tokens reasoning recovery`），**没有一处**是发往上游的请求字段。
- `Cbt`（原生 AMRF 组装）只产出 effort + thinking 两种信封。
- **新增事实**：上游 `TokenLimits` 形状确实含 `maxOutputTokens`，但官方客户端**从不读取**它（§3.2：运行时属性访问 0 处）——即官方**没有**"按输出窗口 clamp `max_tokens`"的行为。
- 结论：项目按 `tokenLimits.maxOutputTokens` clamp 是**项目自己的决定**，不是官方行为。

### 11.2 `thinking.display` 在原生链路是否被主程序消费 —— 已查清：不被消费

检索范围与结果：

| 检索目标 | 范围 | 命中 |
| --- | --- | --- |
| `thinking.display` | 扩展全文 | 0 |
| `modelDefaults` | 扩展全文 | 0 |
| `summarized` | 扩展全文 | 9，**全部**是无关文本（会话摘要提示词、上下文压缩标记等） |
| `thinking.display` / `modelDefaults` / `summarized` / `reasoning.summary` | Kiro 主程序 `workbench.desktop.main.js`（19 MB） | **全部 0** |
| `thinking.display` / `modelDefaults` | 整个 `resources/app` 目录（含 `out/`、`node_modules/`） | **全部 0** |

同一 bundle 的阳性对照（证明检索本身有效）：`workbench` 447、`kiro` 36、`thinking` 32、`reasoning` 24、`disabled` 126。

事实：

- `chat.modelDefaults` 这个键名在**扩展与主程序中都不存在**——它由设置层/用户配置持有，运行时**不经过** `thinking.display` 写回请求。
- `display` 只在 `/v1/responses` 路径被读取（`Tui` → `reasoning.summary`），见 §8、§8.1。
- 原生链路（`GenerateAssistantResponse`）**不发送** `thinking.display`。

### 11.3 `additionalModelRequestFieldsSchema` 在两端点是否有形状差异 —— 已查清：同一份，解读规则不同

- schema **只有一个来源**：`ListAvailableModels` 响应里的 `additionalModelRequestFieldsSchema`（§3）。
- 两端点**共用同一个 AMRF 对象**（`cfd` 直接展开会话态的 `additionalModelRequestFields`，§8.1）。
- 差异在**解读规则**，不在 schema 本身：
  - 原生：按 `effortSchemaPath` 产出 `{output_config:{effort}}` 或 `{reasoning:{effort}}`（`bui`）。
  - `/v1/responses`：读 `output_config.effort` 或 `reasoning.effort`（`Cui`），产出 `{reasoning:{effort}}`。
- 注意 `Cui` 的读取顺序是 `output_config` → `reasoning`，与 `Zyu` 的探测顺序一致，但 `Cui` **不看** `effortSchemaPath`，只是按固定顺序取第一个命中的。

### 11.4 `refusalFallbackModels` 的实际用法 —— 已查清

两套来源，需区分：

| 来源 | 位置 | 内容 |
| --- | --- | --- |
| 模型表内 | `ListAvailableModels` → `v.refusalFallbackModels?.map(I=>I.modelId)` | 该模型自己的拒答回退目标 |
| 链配置 | `getModelFallbacks` 端点（`fetchFallbackChains`） | 按 model id 组织的回退链 |

消费点 `HMr`：

```js
function HMr(t,e,r,n){
  let s=bd().models,
      a=n==="refusal"?s.find(f=>f.id===e)?.refusalFallbackModels:void 0,
      l=t[e]??a;                       // 显式链优先于模型表
  if(!l||l.length===0)return{kind:"declined",reason:"noChainConfigured"};
  let u=new Set(s.map(f=>f.id)),
      d=l.filter(f=>f!==e&&u.has(f));  // 排除自身，且必须是被服务的模型
  if(d.length===0)return{kind:"declined",reason:"noServedTarget"};
  ...
}
```

事实：

- `HMr` 的用途是**模型拒答时的回退链选择**，与 effort/thinking 无关。
- 链的选择优先级：显式配置的链（`t[e]`）**优先于**模型表里的 `refusalFallbackModels`。
- 候选必须**排除自身**且**在上游已服务的模型集合内**，否则返回 `declined`（`noChainConfigured` / `noServedTarget`）。
- 这解释了为什么 `refusalFallbackModels` 在 `AUp` 里是**原样透传**（不做有效性过滤）——过滤发生在 `HMr` 使用时。

### 11.5 仍无法在本环境取证的项（属环境限制，非源码问题）

- 需 Pro 账号或 4.6+ `additionalModelRequestFieldsSchema` 真实样本，才能实拉上游 schema 校验 §9 的能力表（当前能力表来自官方文档原文）。

### 11.6 `create_response_api` 的完整语义 —— 已查清（默认值 / 判定值 / 下发通道 / 覆盖能力）

该 flag 的**取值语义**已完整取证，此前列为"未观测到实际值"，现拆分如下：

| 项 | 值 | 取证位置 |
| --- | --- | --- |
| 枚举键 | `AB_CREATE_RESPONSE_API` | 枚举表 |
| 明文名 | `"create_response_api"` | 同上 |
| **默认值** | `"disabled"` | `RR` 默认值表 |
| **命中值** | `"create_response"`（严格 `===`） | `xud="create_response"` |
| 判定函数 | `Bzi(t){return t?.get(Co.AB_CREATE_RESPONSE_API)===xud}` | — |

**下发通道**：`GetFeatureConfiguration`（`KiroRuntimeService` 的 4 个 operation 之一）。`ExperimentConfigService`（`BQr`）持有纯内存 `configuration`，**不写盘**，TTL 默认 15 分钟（`ttlMs = 9e5`），失败退避 1 分钟（`Hqi = 6e4`）。

**配置键是哈希化的**：

```js
fud = "kiro-feature-config-key-salt-3e9b1d7a"
UQr.get(t) {
  let r = sha256(fud + t).hex
  return hasOwnProperty(configuration, r) ? configuration[r] : ij
}
```

即服务端下发的字典里，key 是 `sha256("kiro-feature-config-key-salt-3e9b1d7a" + "create_response_api")`。

**env var 覆盖对此 flag 无效**（已实测）：

```js
mud(t) { let e = t.toUpperCase(), r = `KIRO_FEATURE_${e}`; return e.endsWith("_ENABLED") ? r : `${r}_ENABLED` }
```

按命名规则推导出 `KIRO_FEATURE_CREATE_RESPONSE_API_ENABLED`，但 env 覆盖仅在 `_ud` 映射表登记的 flag 上生效，而 `_ud` 里**没有** `Co.AB_CREATE_RESPONSE_API`。另外 `yud()` 会校验 `typeof RR[t].default == "boolean"`，非布尔直接返回 false——该 flag 是字符串枚举型，天然不在覆盖范围内。

**仍然无法取证的部分**（真正的环境限制）：本账号当前是否被服务端分发为 `"create_response"`。判断方法不是读源码，而是看运行日志：

```js
C.info("q.converse.dispatch", { api: f ? "CreateResponse" : "GenerateAssistantResponse", ... })
```

`api` 字段直接给出每次请求实际走的路径。


## 12. 复核方法

```bash
EXT="C:/Users/12925/AppData/Local/Programs/Kiro/resources/app/extensions/kiro.kiro-agent/dist/extension.js"
grep -b -o "function Cbt(" "$EXT"          # 按字节偏移 dd 取原文
grep -o "Zyu=\[[^]]\{0,260\}\]" "$EXT"     # 字段路径顺序
grep -b -o "m4e=new Set" "$EXT"            # xhigh/max 集合
grep -b -o "ySe(" "$EXT"                   # ySe 的 4 个调用点
grep -b -o "xui(" "$EXT"                   # xui 的 2 个调用点
```

本轮新增的复核点：

```bash
# §3.1 bd() 的模型表链路与 AUp 兜底（决定 §6.5 的结论）
grep -o "function AUp(.\{0,700\}" "$EXT"                # 含 ?? effortLevels[0] 兜底
grep -o "defaultEffortLevel=_?.defaultLevel" "$EXT"    # Jyu 层无兜底
grep -o "function yUp(.\{0,200\}" "$EXT"                # .map(AUp)
grep -o "getAvailableModels(){return this.cache}" "$EXT"
grep -o "function bd(){return ASe.getAvailableModels()}" "$EXT"

# §3.2 maxOutputTokens 从不被读取
grep -c "maxOutputTokens" "$EXT"        # 2（仅常量名表）
grep -c "\.maxOutputTokens" "$EXT"      # 0（运行时属性访问）

# §8.1 /v1/responses 链路的 feature flag
grep -o "function Bzi(.\{0,200\}" "$EXT"                # create_response_api === xud
grep -b -o "usesCreateResponsePath" "$EXT"              # 3 个使用点

# §11.2 主程序侧确认（阳性对照同样有效）
MAIN="C:/Users/12925/AppData/Local/Programs/Kiro/resources/app/out/vs/workbench/workbench.desktop.main.js"
grep -c "thinking\.display" "$MAIN"     # 0
grep -c "modelDefaults" "$MAIN"        # 0
grep -c "workbench" "$MAIN"             # 447（阳性对照）

# §8.3 外部对账的官方侧复核点
grep -o '\.s("KiroRuntimeService","[A-Za-z]*"' "$EXT"   # 4 个 operation，无 CreateResponse
grep -o 'thd="kiro-runtime:CreateResponse"' "$EXT"      # 路由/遥测键
grep -o 'Vfd={"kiro-ide":"AI_EDITOR".*}' "$EXT"         # origin 取值全集

# §11.6 create_response_api 完整语义
grep -o 'AB_CREATE_RESPONSE_API:"create_response_api"' "$EXT"
grep -o 'xud="create_response"' "$EXT"
grep -o 'create_response_api:{default:"[^"]*"}' "$EXT"
grep -o 'fud="kiro-feature-config-key-salt-[a-z0-9]*"' "$EXT"
grep -o 'function Bzi(.\{0,120\}' "$EXT"
grep -c '_ud={\[Co.AB_CREATE_RESPONSE_API\]' "$EXT"      # 0：不在 env 覆盖表
```

上游通道的活体实测、社区实现对账、以及社区侧的源码复核命令，见 **《Kiro 上游通道实测与对账》** §13（复核方法）；其结论汇总见该文档 §12，图像与 web 搜索的实测见该文档 §10。

`.git/kam-tmp/` 下保留了历轮官方 `extension.js` 的逐字取证片段（`fn_*.txt`、`gmr.txt`、`schema.txt`、`cache.txt`、`ase*.txt`、`at*.txt`、`batch1.txt`、`flag.txt`、`urcp.txt`、`shapecode.txt` 等），可直接对照。
