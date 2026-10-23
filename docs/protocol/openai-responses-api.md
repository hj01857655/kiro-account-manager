# OpenAI Responses API 完整协议规范

> 适用范围：OpenAI Responses HTTP API。本文覆盖输入、输出项、多模态、工具、状态延续、SSE 流式事件、错误和实现边界。
>
> 规范来源（2026-10-23 对照官方 changelog 核对）：
> - [Responses API Reference](https://platform.openai.com/docs/api-reference/responses)
> - [Responses API guide](https://platform.openai.com/docs/guides/responses)
> - [Function calling guide](https://platform.openai.com/docs/guides/function-calling)
> - [Streaming guide](https://platform.openai.com/docs/guides/streaming-responses)
> - [Changelog](https://platform.openai.com/docs/changelog)
>
> OpenAI 会持续扩展响应项和内置工具。实现应以目标模型和当前 API schema 为准。

## 1. 协议概览

Responses API 是基于输入和输出 item 的统一响应协议。请求使用 `input`，服务端返回一个 `response` 对象，响应的 `output` 是由消息、工具调用、推理项和其他输出项组成的数组。工具调用不是 Chat Completions 中的 `message.tool_calls` 字段，而是独立的 `type: "function_call"` item。

基本流程：

```text
POST /v1/responses
  -> response.output 中的 message 或 function_call item
  -> 客户端执行 function_call（如有）
  -> 提交 function_call_output
  -> 再次 POST /v1/responses
  -> 最终 message/output_text
```

## 2. HTTP 请求

### 2.1 端点与认证

```http
POST https://api.openai.com/v1/responses
Authorization: Bearer $OPENAI_API_KEY
Content-Type: application/json
Accept: application/json
```

API key 应只在服务端或受控网关中使用。生产网关应记录请求 ID、状态码和用量，但不得记录完整密钥、敏感输入或未脱敏工具结果。

### 2.2 最小请求

```json
{
  "model": "目标模型 ID",
  "input": "用一句话介绍 HTTP。"
}
```

### 2.3 常用请求字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `model` | string | 模型 ID，必填 |
| `input` | string/array | 输入文本或输入 item 数组 |
| `instructions` | string | 高层行为指令；与输入分开表达 |
| `tools` | array | 函数工具、内置工具或 MCP 工具声明 |
| `tool_choice` | string/object | 工具选择策略 |
| `parallel_tool_calls` | boolean | 是否允许并行工具调用 |
| `stream` | boolean | 是否返回 SSE 流 |
| `temperature` | number | 采样参数，按模型支持情况使用 |
| `top_p` | number | nucleus sampling 参数 |
| `max_output_tokens` | integer | 输出 token 上限，Responses 使用该命名 |
| `truncation` | string | 上下文超长时的截断策略，按接口版本使用 |
| `previous_response_id` | string | 延续先前 response 的会话状态 |
| `conversation` | string | 使用服务端会话容器时的会话标识，按版本使用 |
| `include` | array | 请求额外返回的字段或输出内容 |
| `metadata` | object | 业务元数据 |
| `store` | boolean | 是否保存响应，按组织和接口策略使用 |
| `background` | boolean | 后台响应能力，按目标模型和账户权限使用 |
| `reasoning` | object | 推理模型相关配置，按模型支持情况使用；`effort` 支持到 `max`（GPT-5.6+） |
| `text` | object | 文本格式、结构化输出等配置 |
| `service_tier` | string | 处理层级；`auto` / `default` / `flex`，`ultrafast` 仅特定模型（GPT-6 Astra、GPT-6.1 Sol） |
| `moderation` | object | `{ model }`；在同一响应里返回输入与输出的审核分数 |
| `prompt_cache_key` | string | 缓存路由键；与 `safety_identifier` 一起取代旧 `user` |
| `safety_identifier` | string | 终端用户稳定标识，用于滥用检测 |
| `context_management` | array | `[{ type: "compaction", compact_threshold }]`；服务端上下文压缩 |
| `prompt` | object | 引用控制台模板（`id` + 可选 `version`）与动态 `variables`；Chat Completions 不支持 |

不要把 Chat Completions 的 `messages`、`max_completion_tokens` 或旧版 `functions` 直接当作 Responses 请求字段。适配器必须重建 Responses 输入。

## 3. 输入协议

### 3.1 字符串输入

最简单的 `input` 是字符串：

```json
{
  "model": "目标模型 ID",
  "input": "解释这段代码。"
}
```

### 3.2 输入 item 数组

```json
{
  "model": "目标模型 ID",
  "instructions": "你是一个严谨的技术助手。",
  "input": [
    {
      "role": "user",
      "content": [
        {"type": "input_text", "text": "描述这张图片。"},
        {
          "type": "input_image",
          "image_url": "https://example.com/image.jpg",
          "detail": "auto"
        }
      ]
    }
  ]
}
```

不同版本的输入 image、file、audio 字段以及可接受的 URL、file ID 和 data URL 形式可能不同。不要将 Chat Completions 的 `image_url` content part 原样塞进 Responses，而应使用 Responses 的输入 item 类型。

### 3.3 角色和内容

Responses 输入消息常使用 `role` 与 `content`。内容块使用显式类型：

- `input_text`：文本输入。
- `input_image`：图片输入。
- `input_file`：文件输入。

示例文件输入：

```json
{
  "type": "input_file",
  "file_id": "file_abc123"
}
```

输入还可以包含从前一轮响应中保留的 item，例如工具调用历史。若使用 `previous_response_id` 延续状态，客户端仍必须正确提交本轮工具输出或新增输入。

### 3.4 instructions 与输入的边界

`instructions` 适合本轮高层指令；`input` 表示用户内容和对话 item。服务端或代理层不应把不可信的用户文本拼接成系统级指令。多租户系统应明确区分开发者策略、用户输入和工具返回数据。

## 4. 工具协议

### 4.1 函数工具声明

Responses 的函数工具通常是扁平结构：

```json
{
  "type": "function",
  "name": "get_weather",
  "description": "查询城市当前天气",
  "parameters": {
    "type": "object",
    "properties": {
      "city": {"type": "string", "description": "城市名称"}
    },
    "required": ["city"],
    "additionalProperties": false
  },
  "strict": true
}
```

这与 Chat Completions 的 `{ "type": "function", "function": { ... } }` 嵌套声明不同。自定义工具、web search、file search、computer use、MCP 等内置或托管工具具有各自 schema，不能统一假设成函数工具。

### 4.2 function_call 输出项

模型请求调用工具时，响应中会出现独立 item：

```json
{
  "type": "function_call",
  "id": "fc_abc123",
  "call_id": "call_abc123",
  "name": "get_weather",
  "arguments": "{\"city\":\"北京\"}",
  "status": "completed"
}
```

重要字段：

- `type`：固定表示该 item 是函数调用。
- `call_id`：客户端回传结果时使用的关联 ID。
- `name`：工具名。
- `arguments`：JSON 字符串，需要解析和校验。
- `id`：该输出 item 的 ID；它与 `call_id` 的用途不同。
- `status`：部分版本或模型会提供调用状态。

Responses 没有把这一项包装成 `assistant.tool_calls`。不要把 `function_call` 误认为 Chat Completions 旧版的 `message.function_call`，二者虽然名称相似，但属于不同协议层次。

### 4.3 function_call_output 输入项

执行工具后，将结果作为下一次 Responses 输入：

```json
{
  "type": "function_call_output",
  "call_id": "call_abc123",
  "output": "{\"temperature\":28,\"condition\":\"晴\"}"
}
```

完整请求可以使用 response ID 延续：

```json
{
  "model": "目标模型 ID",
  "previous_response_id": "resp_abc123",
  "input": [
    {
      "type": "function_call_output",
      "call_id": "call_abc123",
      "output": "{\"temperature\":28,\"condition\":\"晴\"}"
    }
  ]
}
```

也可以在不使用服务端延续状态时，把必要的历史 item 与本轮输出一并提交。两种方式不能混淆：`previous_response_id` 只负责关联前一轮 response，不代表客户端可以省略尚未提交的工具结果。

### 4.4 多个工具调用

`response.output` 可能包含多个 `function_call` item。客户端可以并行执行无依赖的只读工具，然后为每个 `call_id` 提交一项 `function_call_output`。有副作用的工具必须使用权限、确认、幂等和顺序控制；协议支持并行不代表业务操作可以安全并行。

### 4.5 工具错误

工具执行失败时仍应回传可序列化的结果，例如：

```json
{
  "type": "function_call_output",
  "call_id": "call_abc123",
  "output": "{\"error\":\"天气服务暂时不可用\"}"
}
```

不要把内部堆栈、访问令牌或数据库连接信息直接放入 output。

## 5. 非流式响应

典型 response：

```json
{
  "id": "resp_abc123",
  "object": "response",
  "created_at": 1710000000,
  "status": "completed",
  "model": "目标模型 ID",
  "output": [
    {
      "type": "message",
      "id": "msg_abc123",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "HTTP 是一种应用层协议。",
          "annotations": []
        }
      ]
    }
  ],
  "usage": {
    "input_tokens": 20,
    "output_tokens": 12,
    "total_tokens": 32
  }
}
```

### 5.1 output item

`output` 是有顺序的 item 数组，可能包含：

- `message`：模型面向用户的内容。
- `function_call`：请求客户端执行函数。
- 推理相关 item：具体可见性和字段取决于模型、权限和 `include`。
- 内置工具调用及其结果相关 item。
- MCP、web search、file search 等工具项。

`output_text` 是 SDK 或服务层提供的便捷聚合文本，不应替代对 `output` item 的完整处理。仅检查 `output_text` 可能漏掉工具调用、拒答或其他结构化输出。

工具检索产生的两类 item：

```json
{
  "type": "tool_search_call",
  "id": "tsc_abc123",
  "call_id": "call_abc123",
  "arguments": {"query": "weather"},
  "execution": "client",
  "status": "completed"
}
```

```json
{
  "type": "tool_search_output",
  "call_id": "call_abc123",
  "tools": [
    {"type": "function", "name": "get_weather", "parameters": {}, "strict": true, "defer_loading": true}
  ],
  "execution": "client",
  "status": "completed"
}
```

`execution` 取 `server`（服务端执行）或 `client`（客户端执行）；`tools` 数组直接携带被加载工具的完整定义，不只是一个名字。请求侧用 `tools: [{ "type": "tool_search", "execution": "client", "parameters": {...} }]` 声明 BYOT 检索工具，服务端托管的检索则由工具定义上的 `defer_loading: true` 触发。

### 5.2 状态和用量

常见响应状态包括 `completed`、`in_progress`、`queued`、`failed`、`incomplete`、`cancelled` 等，具体集合随能力变化。`incomplete` 可能由 token 限制或内容过滤导致；客户端应读取原因字段而不是把它当成正常完成。

用量字段可能包含输入、输出、推理、缓存等细分 token。网关转发时应保留未知用量字段，避免因为版本扩展而丢失信息。

## 6. SSE 流式协议

请求：

```json
{
  "model": "目标模型 ID",
  "input": "写三句话介绍 SSE。",
  "stream": true
}
```

响应使用 `text/event-stream`。每个事件通常是 SSE 的 `event` 与 `data`，数据 JSON 中有 `type` 字段。不要把 Responses 流当成 Chat Completions chunk 解析。

常见事件类别包括：

| 事件 | 用途 |
|---|---|
| `response.created` | 响应创建 |
| `response.in_progress` | 响应开始处理 |
| `response.output_item.added` | 新 output item 出现 |
| `response.output_item.done` | output item 完成 |
| `response.content_part.added` | 新内容 part 出现 |
| `response.content_part.done` | 内容 part 完成 |
| `response.output_text.delta` | 文本增量 |
| `response.output_text.done` | 文本输出完成 |
| `response.function_call_arguments.delta` | 函数参数增量 |
| `response.function_call_arguments.done` | 函数参数完整 |
| `response.completed` | 响应完成 |
| `response.failed` | 响应失败 |
| `error` | 流级错误 |

事件集合会随 Responses 能力扩展；解析器应按 `type` 分派，并对未知事件保留或忽略，不应因新增事件导致整个流失败。

函数调用流式处理建议：

1. 读取并保存 output item 的上下文和 item ID。
2. 按 item 关联 `response.function_call_arguments.delta`。
3. 累加 `arguments` 字符串，直到 `response.function_call_arguments.done`。
4. 解析并校验完整 JSON 后再执行工具。
5. 使用事件中的 `call_id` 提交 `function_call_output`。
6. 等待 `response.completed` 或明确失败事件，正确释放连接。

文本增量示意：

```text
event: response.output_text.delta
data: {"type":"response.output_text.delta","delta":"Responses"}

event: response.output_text.done
data: {"type":"response.output_text.done","text":"Responses ..."}
```

不要按 `choices[].delta.content`、`[DONE]` 或 `finish_reason` 解析 Responses 流，那是 Chat Completions 的模型。

## 7. 状态延续与会话

### 7.1 previous_response_id

使用 `previous_response_id` 可以把本轮输入关联到前一个 response：

```json
{
  "model": "目标模型 ID",
  "previous_response_id": "resp_abc123",
  "input": "继续解释第二点。"
}
```

使用该机制时，客户端应明确响应保存策略、过期策略、权限边界和敏感数据生命周期。

### 7.2 客户端维护历史

也可以由客户端维护输入 item，并在每轮提交完整历史。这更容易做审计、重放和跨供应商转换，但请求体会变大。两种策略不要在同一业务会话中无意间重复提交历史，避免重复上下文和成本上涨。

## 8. 结构化输出和多模态

Responses 的文本配置可用于请求 JSON Schema 或其他结构化文本格式，具体字段以当前 API schema 为准。结构化输出约束的是模型文本输出，不会替代工具参数校验。

图片、文件、音频、工具结果和模型文本属于不同 item/content 类型。适配器必须按目标协议转换类型，不能仅复制 `content` 数组。

## 9. 错误、重试与安全

错误响应通常包含错误类型、消息、参数和代码。实现应区分参数错误、认证错误、权限错误、限流、服务端错误和流连接中断：

- `4xx` 参数或权限错误通常需要修正请求，不应盲目重试。
- `429`、超时和临时 `5xx` 可在请求幂等时指数退避。
- 工具执行成功但提交 output 失败时，不得无条件重复有副作用工具；应使用 call ID、幂等键或业务状态恢复。
- 对 `response.failed`、`response.incomplete` 和流断开分别记录原因。

模型生成的工具名和参数都必须经过服务端校验和授权。任何删除、付款、发送消息、改密钥等操作都应设置额外的确认或策略门槛。

## 10. Chat Completions 迁移对照

| Chat Completions | Responses |
|---|---|
| `messages` | `input` 或输入 item 数组 |
| `system` message | `instructions` 或对应输入消息 |
| `choices[].message` | `output[]` 中的 `message` item |
| `message.content` | `message.content[]` 中的 `output_text` |
| `message.tool_calls[]` | `output[]` 中的 `function_call` item |
| `role: "tool"` | `function_call_output` item |
| `tool_call_id` | `call_id` |
| `stream` chunk / `delta` | 按 `type` 分派的响应事件 |
| `max_completion_tokens` | `max_output_tokens` |

两套协议的名称相似处不能掩盖结构差异。转换时必须重新生成输入消息、输出 item 和工具结果 item。

## 11. 实现检查清单

- [ ] 用 `input`/`instructions` 构造请求，不直接套用 `messages`。
- [ ] 识别 `output` 中所有 item，而不仅仅读取 `output_text`。
- [ ] 用 `call_id` 关联 `function_call_output`。
- [ ] 流式按事件 `type` 和 item ID 聚合。
- [ ] 处理 queued、in_progress、completed、failed、incomplete 等状态。
- [ ] 保留未知扩展字段，避免版本升级时数据丢失。
- [ ] 对工具执行、重试和状态延续实现幂等与权限控制。
