# Anthropic Messages API 完整协议规范

> 适用范围：Anthropic Messages HTTP API。本文覆盖请求、消息、文本与多模态内容、工具、非流式响应、SSE 流式事件、token 计数、缓存、错误和实现边界。
>
> 规范来源（2026-10-23 对照官方核对）：
> - [Messages API Reference](https://platform.claude.com/docs/en/api/messages)
> - [Migrating to API version 2026-10-21](https://platform.claude.com/docs/en/api/migration-2026-10-21)
> - [Tool search tool](https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-search-tool)
> - [Tool use](https://platform.claude.com/docs/en/agents-and-tools/tool-use/implement-tool-use)
> - [Streaming messages](https://platform.claude.com/docs/en/api/messages-streaming)
> - [Vision](https://platform.claude.com/docs/en/build-with-claude/vision)
>
> **API 版本**：`2026-10-21` 是 2023 年以来首个正式版本（`2023-06-01` 未废弃），含三条 breaking change：采样参数移除、assistant 消息禁止直接放文档块、user 消息禁止携带 citations。本文按 `2026-10-21` 描述；需要旧语义的客户端必须显式保留 `2023-06-01`。
>
> Anthropic 会按模型和 API 版本扩展字段。本文示例用于说明协议结构，具体模型能力和限制以目标账户、模型及官方 schema 为准。

## 1. 协议概览

Messages API 是基于消息数组和内容块的对话协议。请求中使用顶层 `system` 指令和 `messages` 历史，服务端返回一个 `message`。每条消息的 `content` 可以是字符串，也可以是有序内容块数组。工具调用和工具结果都属于内容块，而不是独立的消息角色字段。

基本流程：

```text
POST /v1/messages
  -> assistant message，可能含 tool_use content block
  -> 客户端执行工具
  -> 追加 user message，包含 tool_result block
  -> 再次 POST /v1/messages
  -> 最终 assistant message
```

Anthropic 的协议与 OpenAI 两种协议都使用 function/tool 的抽象概念，但数据模型不同：Anthropic 使用 `tool_use` 和 `tool_result` 内容块，并要求工具结果放在 `user` 消息中。

## 2. HTTP 请求

### 2.1 端点与认证

```http
POST https://api.anthropic.com/v1/messages
Authorization: Bearer $ANTHROPIC_API_KEY
anthropic-version: 2026-10-21
content-type: application/json
accept: application/json
```

`anthropic-version` 是 API 版本请求头，示例版本不代表所有账户或未来版本的唯一选择。部分环境还支持 beta 能力，需要额外的 beta 请求头，不能默认添加未启用的 beta 标识。

`2026-10-21` 起官方 SDK 默认把 API key 放在 `Authorization: Bearer`，服务端同时接受 `x-api-key`。读取或改写请求头的代理/网关必须两种都认。

Anthropic API key 只能放在服务端或受控网关中。网关应记录状态码、请求 ID 和用量，但不要记录 key、原始图片、敏感 prompt 或工具返回的凭据。

### 2.2 最小请求

```json
{
  "model": "目标 Claude 模型 ID",
  "max_tokens": 1024,
  "messages": [
    {
      "role": "user",
      "content": "用一句话介绍 HTTP。"
    }
  ]
}
```

`model`、`max_tokens` 和 `messages` 是核心请求字段。`max_tokens` 是 Anthropic Messages 的必填输出上限字段，不能直接替换成 OpenAI 的 `max_completion_tokens` 或 Responses 的 `max_output_tokens`。

### 2.3 常用请求字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `model` | string | Claude 模型 ID，必填 |
| `max_tokens` | integer | 最大生成 token 数，必填 |
| `messages` | array | 用户和 assistant 消息历史 |
| `system` | string/array | 顶层系统提示，不是 `messages` 中的 `system` role |
| ~~`temperature`~~ | — | **2026-10-21 移除**：请求携带该字段（即使等于默认值）返回 400；省略即按 1.0 采样 |
| ~~`top_p`~~ | — | **2026-10-21 移除**：同上，返回 400 |
| ~~`top_k`~~ | — | **2026-10-21 移除**：同上，返回 400 |
| `stop_sequences` | array | 自定义停止序列 |
| `stream` | boolean | 是否使用 SSE 流式响应 |
| `tools` | array | 工具声明 |
| `tool_choice` | object | `auto`、`any` 或指定工具等策略 |
| `metadata` | object | 用户 ID 等请求元数据 |
| `thinking` | object | 扩展思考能力配置，按模型支持情况使用；`display: "summarized"` 才返回思考文本，否则只有 thinking 块 |
| `output_config` | object | `{ effort, format }`；`effort` 取 `low` / `medium` / `high` / `xhigh` / `max` |

> `service_tier`、`speed`、`container` 不在当前 Messages 参考页的请求字段中；快速模式走 beta 请求头 `fast-mode-2026-02-01`，不是 `speed` 字段。代理层遇到这些字段按未知字段处理，不要假设其生效。

不应把所有模型都支持的字段写成硬性假设。代理层应根据模型能力过滤不兼容参数，尤其是 thinking、computer use、代码执行、缓存和 beta 字段。

## 3. 消息协议

### 3.1 角色

Messages API 的消息角色主要是 `user` 和 `assistant`：

```json
{
  "role": "user",
  "content": "你好"
}
```

```json
{
  "role": "assistant",
  "content": "你好，我可以帮助你。"
}
```

系统提示通过顶层 `system` 字段表达：

```json
{
  "system": "你是一个严谨的技术助手。",
  "messages": [{"role":"user","content":"解释 HTTP。"}]
}
```

不要把 OpenAI 的 `system` message 原样放入 Anthropic 的 `messages` 数组。转换器应把它提取到顶层 `system`，或者根据业务策略合并为系统内容块。

### 3.2 文本内容

`content` 可以是字符串：

```json
{
  "role": "user",
  "content": "解释这段代码。"
}
```

也可以是内容块数组：

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "解释下面的图片。"},
    {
      "type": "image",
      "source": {
        "type": "base64",
        "media_type": "image/jpeg",
        "data": "BASE64_DATA"
      }
    }
  ]
}
```

常见内容块类型包括：

- `text`：文本内容。`2026-10-21` 起 `user` 消息的 `text` 块带非空 `citations` 返回 400；`assistant` 消息的 citations 仍接受。
- `image`：图片输入，通常带 `source`。
- `document`：文档输入，只能放在 `user` 消息中。`2026-10-21` 起直接放在 `assistant` 消息返回 400；嵌套在其他块内的文档结果（如 web fetch 返回值）仍接受。
- `tool_use`：assistant 请求执行工具。
- `tool_result`：user 回传工具结果；content 里可含 `tool_reference` 块，供客户端自建工具检索（见 4.6）。
- `server_tool_use` / `tool_search_tool_result`：服务端工具（含 tool search）的调用与结果，见 4.6。
- thinking、redacted thinking 和引用相关块：按模型和 API 版本使用。

内容块是有序的。转换器不能随意改变图片、文本、工具调用和工具结果的顺序。

### 3.3 assistant 历史

多轮请求可以把上一轮 assistant 内容作为下一次请求的历史：

```json
{
  "role": "assistant",
  "content": [
    {"type":"text","text":"我需要查询天气。"},
    {
      "type":"tool_use",
      "id":"toolu_abc123",
      "name":"get_weather",
      "input":{"city":"北京"}
    }
  ]
}
```

工具循环中，assistant 的 `tool_use` 与下一条 user 的 `tool_result` 应保持对应关系。不要只保存文本而丢弃结构化块，否则下一轮模型无法正确理解工具调用历史。

### 3.4 预填充 assistant

某些使用方式允许以 assistant 消息结尾来预填充输出。该能力受模型、端点和账户策略影响。跨供应商适配时不要默认启用，也不要把 OpenAI 的 assistant 工具调用格式直接作为 Anthropic assistant 内容。

## 4. 工具协议

### 4.1 工具声明

```json
{
  "name": "get_weather",
  "description": "查询城市当前天气",
  "input_schema": {
    "type": "object",
    "properties": {
      "city": {
        "type": "string",
        "description": "城市名称"
      }
    },
    "required": ["city"]
  }
}
```

顶层请求：

```json
{
  "model": "目标 Claude 模型 ID",
  "max_tokens": 1024,
  "tools": [/* 工具声明 */],
  "messages": [
    {"role":"user","content":"北京天气如何？"}
  ]
}
```

Anthropic 使用 `input_schema`；OpenAI Chat Completions 使用 `parameters`，Responses 函数工具也使用 `parameters`。工具声明不能直接跨协议复制。

工具声明还可带 `defer_loading`，把定义从初始上下文挪到按需检索（见 4.6）：

```json
{
  "name": "get_weather",
  "description": "查询城市当前天气",
  "input_schema": {"type": "object", "properties": {"city": {"type": "string"}}},
  "defer_loading": true
}
```

延迟工具不能同时携带 `cache_control`（返回 400），缓存断点要放在非延迟工具上。

### 4.2 tool_use 内容块

模型请求调用工具时，assistant 响应的 `content` 中会出现：

```json
{
  "type": "tool_use",
  "id": "toolu_abc123",
  "name": "get_weather",
  "input": {
    "city": "北京"
  }
}
```

与 OpenAI 的字符串 `arguments` 不同，Anthropic 的 `input` 在非流式响应中通常已经是 JSON 对象。实现仍应对类型、字段、权限和业务范围做服务端校验，不能因为它是对象就当作可信输入。

### 4.3 tool_result 内容块

工具执行结果必须放在新的 `user` 消息中：

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_abc123",
      "content": "{\"temperature\":28,\"condition\":\"晴\"}"
    }
  ]
}
```

工具结果也可以用内容块数组表达：

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_abc123",
      "content": [
        {"type":"text","text":"温度 28 摄氏度，天气晴。"}
      ]
    }
  ]
}
```

失败结果：

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_abc123",
  "is_error": true,
  "content": "天气服务暂时不可用。"
}
```

`tool_use_id` 必须准确匹配 assistant 内容块中的 `id`。不要使用工具名代替 ID，也不要把结果放到 `role: "tool"` 消息中；那是 OpenAI Chat Completions 的格式。

### 4.4 多工具调用

一次 assistant 响应可以包含多个 `tool_use` 块。客户端可以并行执行互不依赖的只读工具，然后在同一条 `user` 消息中放入多个 `tool_result` 块：

```json
{
  "role": "user",
  "content": [
    {
      "type":"tool_result",
      "tool_use_id":"toolu_weather",
      "content":"天气：晴"
    },
    {
      "type":"tool_result",
      "tool_use_id":"toolu_time",
      "content":"当地时间：10:30"
    }
  ]
}
```

结果顺序应稳定并能追溯到原始调用。涉及写入、删除、付款或发送消息的工具，必须额外考虑用户确认、幂等键、锁和业务顺序。

### 4.5 tool_choice

工具选择策略通过 `tool_choice` 表达。常见语义包括允许模型自动选择、要求使用任意工具、指定某个工具或禁止工具；精确 schema 和与 thinking 的组合限制取决于 API 版本和模型。适配器应保留未知选择策略或在目标协议中明确降级，不能静默改成自动执行高风险工具。

### 4.6 工具检索（tool search）

工具数量到几十上百时，全量下发定义既撑爆上下文也拉低选择准确率。规范做法是保留一个检索工具，其余工具标记延迟加载：

```json
{
  "tools": [
    {"type": "tool_search_tool_regex_20251119", "name": "tool_search_tool_regex"},
    {"name": "get_weather", "description": "查询城市当前天气", "input_schema": {"type": "object"}, "defer_loading": true}
  ]
}
```

两种变体：`tool_search_tool_regex_20251119`（正则检索，模式最长 200 字符）与 `tool_search_tool_bm25_20251119`（自然语言检索，查询最长 500 字符）。检索工具自身不能延迟加载，且至少要有一个非延迟工具，否则返回 400。

模型检索时，assistant 内容里出现成对的 `server_tool_use` 与 `tool_search_tool_result`：

```json
[
  {
    "type": "server_tool_use",
    "id": "srvtoolu_01ABC123",
    "name": "tool_search_tool_regex",
    "input": {"pattern": "weather", "limit": 10}
  },
  {
    "type": "tool_search_tool_result",
    "tool_use_id": "srvtoolu_01ABC123",
    "content": {
      "type": "tool_search_tool_search_result",
      "tool_references": [{"type": "tool_reference", "tool_name": "get_weather"}]
    }
  }
]
```

要点：

- `limit` 取 1–10000，默认 5；无匹配时 `tool_references` 为空数组，不是错误。
- `tool_search_tool_result` 必须原样留在历史里；不要为 `srvtoolu_...` 回 `tool_result`，那会被拒绝。
- 检索失败以 200 返回，错误在 `content.type: "tool_search_tool_result_error"` 的 `error_code`：`invalid_tool_input` / `unavailable` / `too_many_requests` / `execution_time_exceeded`。
- 服务端检索由 API 自己把 `tool_reference` 展开成完整定义；客户端自建检索则返回标准 `tool_result`，content 里放 `tool_reference` 块。
- 单请求最多 10000 个延迟工具；`tool_reference.tool_name` 长度 1–256，字符集 `[a-zA-Z0-9_-]`。

## 5. 非流式响应

典型响应：

```json
{
  "id": "msg_abc123",
  "type": "message",
  "role": "assistant",
  "model": "目标 Claude 模型 ID",
  "content": [
    {
      "type": "text",
      "text": "HTTP 是一种应用层协议。"
    }
  ],
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 20,
    "output_tokens": 12
  }
}
```

### 5.1 响应字段

| 字段 | 说明 |
|---|---|
| `id` | 消息响应 ID |
| `type` | 通常为 `message` |
| `role` | 通常为 `assistant` |
| `model` | 实际使用的模型 |
| `content` | 有序内容块数组 |
| `stop_reason` | 结束原因 |
| `stop_sequence` | 命中的停止序列（如有） |
| `usage` | 输入和输出 token 用量 |

`content` 可能同时包含文本、工具调用、thinking 或其他内容块。不要假设 `content[0]` 永远是文本，也不要仅拼接文本就丢失工具调用。

### 5.2 stop_reason

常见停止原因包括：

- `end_turn`：正常结束。
- `max_tokens`：达到输出上限。
- `stop_sequence`：命中自定义停止序列。
- `tool_use`：模型请求执行工具。
- 其他模型或能力相关停止原因。

实现应把停止原因作为可扩展字符串处理。出现 `tool_use` 时，必须扫描全部内容块并处理每个 `tool_use`，不能直接把响应当成最终文本。

### 5.3 拒答与扩展内容

拒答、thinking、引用、文档结果等能力可能以特定内容块或字段出现。代理层应保留未知内容块，至少不能因为只支持 `text` 而破坏整个响应。向不支持这些块的下游转换时，应制定明确的降级策略并记录信息损失。

## 6. SSE 流式协议

请求：

```json
{
  "model": "目标 Claude 模型 ID",
  "max_tokens": 1024,
  "stream": true,
  "messages": [
    {"role":"user","content":"写三句话介绍 SSE。"}
  ]
}
```

响应为 `text/event-stream`。Anthropic 流不是 OpenAI Chat 的 `choices[].delta`，而是有明确事件类型和消息/内容块生命周期。

### 6.1 典型事件顺序

```text
event: message_start
data: {"type":"message_start","message":{...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"SSE"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{...}}

event: message_stop
data: {"type":"message_stop"}
```

常见事件：

| 事件 | 用途 |
|---|---|
| `message_start` | 初始化消息和初始 usage |
| `content_block_start` | 开始一个内容块 |
| `content_block_delta` | 内容块增量 |
| `content_block_stop` | 当前内容块完成 |
| `message_delta` | 消息级增量，如 stop reason 和最终 usage |
| `message_stop` | 整条消息结束 |
| `ping` | 保活事件 |
| `error` | 流错误 |

### 6.2 文本增量

文本增量通常位于：

```json
{
  "type": "content_block_delta",
  "index": 0,
  "delta": {
    "type": "text_delta",
    "text": "增量文本"
  }
}
```

客户端按 `index` 维护内容块，并累加同一块的文本。不要假设事件一次只对应一个完整汉字、单词或 JSON token。

### 6.3 tool_use 增量

工具调用流会先启动一个 `tool_use` 内容块，再通过 delta 逐步传输输入 JSON。典型结构：

```json
{
  "type": "content_block_start",
  "index": 0,
  "content_block": {
    "type": "tool_use",
    "id": "toolu_abc123",
    "name": "get_weather",
    "input": {}
  }
}
```

随后可能收到 JSON 增量事件，例如：

```json
{
  "type": "content_block_delta",
  "index": 0,
  "delta": {
    "type": "input_json_delta",
    "partial_json": "{\"city\":\"北京\"}"
  }
}
```

处理要求：

1. 按内容块 `index` 保存 `id`、工具名和部分 JSON。
2. 累加 `partial_json`，不要按单个事件直接解析。
3. 收到 `content_block_stop` 后再解析完整 JSON。
4. 校验工具名、schema、权限和业务范围。
5. 将结果以 `tool_result` 放入下一条 `user` 消息。

thinking 或其他特殊内容块也可能使用专用 delta 类型。解析器应按 `delta.type` 分派，并对未知类型保持向前兼容。

工具检索在流里也是普通内容块：先 `content_block_start`（`server_tool_use`）加 `input_json_delta` 传检索参数，检索执行期间流暂停，随后一个 `content_block_start` 推送 `tool_search_tool_result`，模型再继续输出。

## 7. Token 计数、缓存和长上下文

Messages API 的响应包含 `usage`，通常至少区分 `input_tokens` 和 `output_tokens`。缓存、批处理、thinking 或其他能力可能增加更多用量字段。代理层应保留未知字段，不能只定义两个固定字段后丢弃扩展信息。

若需要在发送前估算输入 token，可使用 Anthropic 的 token 计数接口；计数结果是估算或预处理结果，最终账单应以实际 Messages 响应 usage 为准。

提示缓存能力会涉及 cache control、缓存命中和计费字段，必须按照目标 API 版本和模型支持情况使用。缓存内容不得跨用户或权限边界错误复用。

## 8. 多模态与文件

图片输入一般使用 `image` 内容块和 `source`：

```json
{
  "type": "image",
  "source": {
    "type": "base64",
    "media_type": "image/png",
    "data": "BASE64_DATA"
  }
}
```

支持的媒体类型、尺寸、大小和文档形式取决于模型与 API 版本。客户端应在发送前做 MIME、大小和来源校验。不要把 OpenAI 的 `image_url` 对象原样传给 Anthropic；需要把 URL 下载、转为目标支持的 source，或使用目标协议支持的 URL 形式。

## 9. 错误、重试与安全

错误响应通常包含：

```json
{
  "type": "error",
  "error": {
    "type": "invalid_request_error",
    "message": "请求参数无效"
  }
}
```

实现应区分：

- 参数、schema 或内容错误：修正请求，不要自动重试。
- 认证、权限或模型访问错误：检查 key、组织权限和模型 ID。
- 限流、超时和临时服务端错误：在请求幂等时指数退避。
- SSE 中的 `error`：标记当前响应不完整，不能把已收到的片段当成完整成功结果。
- 工具已执行但网络失败：使用 `tool_use_id`、业务幂等键或执行记录，防止重复副作用。

工具输入来自模型，必须经过 JSON Schema、类型、权限、租户范围和资源状态校验。`tool_result` 不应包含 API key、内部路径、堆栈、数据库凭据或未脱敏个人数据。

## 10. OpenAI 协议迁移对照

| Anthropic Messages | OpenAI Chat Completions | OpenAI Responses |
|---|---|---|
| 顶层 `system` | `messages` 中的 system message | `instructions` 或输入消息 |
| `messages` | `messages` | `input`/输入 item |
| `content` 内容块数组 | `message.content` 内容或 content parts | item/content parts |
| assistant `tool_use` | assistant `tool_calls[]` | `function_call` output item |
| user `tool_result` | `role: "tool"` | `function_call_output` item |
| `tool_use.id` | `tool_calls[].id` | `call_id` |
| `tool.input` 对象 | `function.arguments` JSON 字符串 | `arguments` JSON 字符串 |
| `input_schema` | `function.parameters` | `parameters` |
| `content_block_delta` | chat chunk `delta` | response event `type` |
| `max_tokens` | `max_completion_tokens` 等 | `max_output_tokens` |
| `server_tool_use` + `tool_search_tool_result` | 无对应物 | `tool_search_call` + `tool_search_output` |

这张表只是语义对照，不是逐字段复制规则。转换器必须重新构造消息、内容块、工具声明和流式事件。

## 11. 实现检查清单

- [ ] 使用顶层 `system`，不要把 system role 直接当作 Messages 历史消息。
- [ ] 保留 `content` 的块顺序和未知块。
- [ ] 识别全部 `tool_use`，使用 `tool_use_id` 回传结果。
- [ ] 工具结果放在 `user` 消息的 `tool_result` 块中。
- [ ] 流式按 `message`、`content_block` 和 `delta` 生命周期解析。
- [ ] 工具 JSON 增量在内容块结束后再执行。
- [ ] 处理 `stop_reason`、usage、拒答、thinking 和扩展内容。
- [ ] 不发送 `temperature` / `top_p` / `top_k`：`2026-10-21` 起携带即 400。
- [ ] 延迟工具不进初始上下文；检索工具自身不延迟，且至少保留一个非延迟工具。
- [ ] `tool_search_tool_result` 原样回传，不为 `srvtoolu_...` 生成 `tool_result`。
- [ ] 对限流重试、流断开和副作用工具实现幂等策略。
- [ ] 不把 Anthropic 内容块直接当作 OpenAI 消息或 Responses item。
