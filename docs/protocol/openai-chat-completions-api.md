# OpenAI Chat Completions API 完整协议规范

> 适用范围：OpenAI Chat Completions HTTP API。本文聚焦请求、响应、消息、多模态、工具调用和 SSE 流式传输。
>
> 规范来源（2026-10-23 对照官方 changelog 核对）：
> - [Chat Completions API Reference](https://platform.openai.com/docs/api-reference/chat)
> - [Function calling guide](https://platform.openai.com/docs/guides/function-calling)
> - [Vision guide](https://platform.openai.com/docs/guides/vision)
> - [Changelog](https://platform.openai.com/docs/changelog)
>
> OpenAI 会持续扩展模型和字段。实现时应以目标模型对应的官方 schema 为准，不应把本文示例中的可选字段视为所有模型都支持。

## 1. 协议概览

Chat Completions 是基于消息数组的对话协议。客户端把完整或部分对话放入 `messages`，服务端返回一个或多个 `choices`。模型不会直接执行客户端函数；工具调用只是模型输出的结构化请求，由客户端执行后再把结果作为下一轮消息传回。

基本流程：

```text
POST /v1/chat/completions
  -> assistant message 或 assistant.tool_calls
  -> 客户端执行工具（如有）
  -> 追加 tool message
  -> 再次 POST /v1/chat/completions
  -> 最终 assistant message
```

## 2. HTTP 请求

### 2.1 端点与认证

```http
POST https://api.openai.com/v1/chat/completions
Authorization: Bearer $OPENAI_API_KEY
Content-Type: application/json
Accept: application/json
```

建议：

- API key 只放在服务端或受控网关中。
- 不要把 key 写入前端代码、日志或错误响应。
- 网关应透传 HTTP 状态码、请求 ID 和限流信息，但应过滤敏感请求内容。

### 2.2 最小请求

```json
{
  "model": "目标模型 ID",
  "messages": [
    {
      "role": "user",
      "content": "用一句话介绍 HTTP。"
    }
  ]
}
```

### 2.3 常用请求字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `model` | string | 模型 ID，必填 |
| `messages` | array | 按顺序排列的消息，必填 |
| `stream` | boolean | 是否使用 SSE 流式响应 |
| `temperature` | number | 采样随机性；具体支持范围取决于模型 |
| `top_p` | number | nucleus sampling 参数，通常不与 `temperature` 同时调节 |
| `max_tokens` | integer | 旧字段；部分模型使用 `max_completion_tokens` |
| `max_completion_tokens` | integer | 限制生成 token 数，支持情况取决于模型 |
| `stop` | string/array/null | 停止序列；并非所有模型支持 |
| `n` | integer | 生成候选数量；大于 1 会增加成本 |
| `response_format` | object | 文本、JSON 或 JSON Schema 输出约束，按模型支持情况使用 |
| `tools` | array | 新版工具声明 |
| `tool_choice` | string/object | `none`、`auto`、`required` 或指定函数 |
| `parallel_tool_calls` | boolean | 是否允许一次响应返回多个工具调用 |
| `user` | string | 已废弃，由 `safety_identifier` 与 `prompt_cache_key` 取代 |
| `metadata` | object | 请求元数据，按接口版本使用 |
| `reasoning_effort` | string | 推理强度，取值随模型扩展（含 `max`） |
| `verbosity` | string | `low` / `medium` / `high`，约束回答详细程度 |
| `prompt_cache_key` | string | 缓存路由键，维持缓存命中 |
| `safety_identifier` | string | 终端用户稳定标识，用于滥用检测 |
| `service_tier` | string | `auto` / `default` / `flex`；`ultrafast` 仅特定模型 |
| `moderation` | object | `{ model }`；同一响应里返回输入与输出的审核分数 |
| `web_search_options` | object | `{ search_context_size, user_location }`，服务端联网检索配置 |
| `stream_options` | object | `{ include_usage }`，仅在 `stream: true` 时有效 |

不要同时提交互相冲突的旧版和新版参数。例如使用 `max_completion_tokens` 时，不要再依赖旧的 `max_tokens` 语义。

## 3. 消息协议

### 3.1 消息通用结构

消息至少包含 `role` 和 `content`；assistant 工具调用消息还可以包含 `tool_calls`。不同模型对 `name`、`audio`、`tool_call_id` 等字段的支持不同。

```json
{
  "role": "user",
  "content": "消息内容"
}
```

### 3.2 system 消息

用于定义模型行为、约束和上下文：

```json
{
  "role": "system",
  "content": "你是一个严谨的技术助手。"
}
```

部分新模型或兼容服务也允许 `developer` 角色。跨供应商适配时不能假设所有模型都接受该角色，应根据目标模型转换或合并。

### 3.3 user 消息

文本内容：

```json
{
  "role": "user",
  "content": "解释这张图片。"
}
```

多模态内容通常是数组：

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "描述图片中的内容。"},
    {
      "type": "image_url",
      "image_url": {
        "url": "https://example.com/image.jpg",
        "detail": "auto"
      }
    }
  ]
}
```

图片可使用受支持的公网 URL 或 data URL。文件、音频和视频能力取决于具体模型及 API 版本，不能仅因为 `content` 是数组就推断全部媒体类型可用。

### 3.4 assistant 消息

普通文本响应：

```json
{
  "role": "assistant",
  "content": "HTTP 是一种应用层协议。"
}
```

工具调用响应可能 `content` 为 `null`，并带有 `tool_calls`。

### 3.5 tool 消息

客户端执行工具后，必须使用对应的调用 ID 回传：

```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "{\"temperature\":28,\"condition\":\"晴\"}"
}
```

`content` 通常是字符串。若工具结果是对象，应先序列化为 JSON 字符串，并保证结果消息紧跟对应的 assistant 工具调用消息。

### 3.6 旧版 function 兼容格式

旧协议使用：

```json
{
  "role": "assistant",
  "content": null,
  "function_call": {
    "name": "get_weather",
    "arguments": "{\"city\":\"北京\"}"
  }
}
```

对应结果通常是 `role: "function"`。它与新版 `tools`、`tool_calls` 不是同一格式。新实现应优先使用新版字段，兼容层才解析旧字段。

## 4. 工具调用

### 4.1 工具声明

```json
{
  "type": "function",
  "function": {
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
}
```

请求中的 `tools` 是数组。`strict`、JSON Schema 支持范围和可接受的 schema 关键字取决于模型。

### 4.2 工具调用响应

```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"city\":\"北京\"}"
      }
    }
  ]
}
```

`arguments` 是 JSON 字符串，不是已经解析的对象。客户端必须解析、校验、鉴权，再调用本地函数或外部服务。

### 4.3 工具循环

```json
{
  "model": "目标模型 ID",
  "messages": [
    {"role": "user", "content": "北京天气如何？"},
    {
      "role": "assistant",
      "content": null,
      "tool_calls": [
        {
          "id": "call_abc123",
          "type": "function",
          "function": {
            "name": "get_weather",
            "arguments": "{\"city\":\"北京\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_abc123",
      "content": "{\"temperature\":28,\"condition\":\"晴\"}"
    }
  ],
  "tools": [/* 工具声明 */]
}
```

工具失败也应回传可控的工具结果，例如 `{"error":"天气服务暂时不可用"}`，而不是把内部堆栈直接暴露给模型或用户。

### 4.4 并行工具调用

当模型返回多个 `tool_calls` 时，客户端可以并行执行，但必须为每个调用回传一个 `tool` 消息，并使用准确的 `tool_call_id`。有副作用的操作不应仅因为协议允许并行就并行执行；应考虑锁、顺序和幂等性。

`tool_choice` 可用于允许模型选择、禁止工具或强制指定工具。`parallel_tool_calls: false` 可用于要求一次最多一个工具调用，具体支持情况以模型文档为准。

## 5. 非流式响应

典型响应：

```json
{
  "id": "chatcmpl_abc123",
  "object": "chat.completion",
  "created": 1710000000,
  "model": "目标模型 ID",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "HTTP 是一种应用层协议。",
        "refusal": null
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 12,
    "total_tokens": 32
  }
}
```

常见 `finish_reason` 包括 `stop`、`length`、`tool_calls`、`content_filter`。兼容实现应把未知值当作可扩展值处理，而不是枚举失败。

`choices` 可能包含多个候选。不要无条件读取 `choices[0]` 后假设不存在其他候选，也不要把 `message.content` 非空作为“没有工具调用”的唯一判断条件。

## 6. SSE 流式协议

请求：

```json
{
  "model": "目标模型 ID",
  "messages": [{"role":"user","content":"写三句话介绍 SSE。"}],
  "stream": true,
  "stream_options": {"include_usage": true}
}
```

响应的 HTTP `Content-Type` 通常为 `text/event-stream`。每个事件通常形如：

```text
data: {"id":"chatcmpl_abc123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant","content":"SSE"},"finish_reason":null}]}

data: [DONE]
```

解析规则：

1. 按 SSE 的空行分隔事件读取，不要按 TCP 包边界解析。
2. 去掉 `data: ` 前缀后解析 JSON。
3. 按 `choices[].index` 维护候选状态。
4. 累加 `delta.content`，不能假设每个 chunk 都有文本。
5. 工具调用增量需要按 `tool_calls[].index` 合并 ID、函数名和 `function.arguments`。
6. 收到 `finish_reason` 后仍应继续读取到连接结束或 `[DONE]`。
7. 使用 `include_usage` 时，末尾可能有不含 choices 的 usage chunk。

工具调用增量示意：

```json
{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"get_weather","arguments":"{\"city\":"}}]}}]}
{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"北京\"}"}}]}}]}
```

只有在工具参数 JSON 完整、流结束且调用已确认后才执行工具。

## 7. 错误、重试与安全

错误响应通常包含 `error` 对象，例如：

```json
{
  "error": {
    "message": "请求参数无效",
    "type": "invalid_request_error",
    "param": "messages",
    "code": "invalid_value"
  }
}
```

实现应区分：

- `400` 类参数错误：修正请求，不要盲目重试。
- `401`/`403`：检查认证和权限。
- `404`：检查端点或模型 ID。
- `408`、`429`、`5xx`：在确认请求幂等后采用指数退避；记录服务端请求 ID。
- 流式连接中断：不能简单重复副作用工具调用，应使用调用 ID、幂等键或业务状态恢复。

工具参数必须做 JSON 解析、schema 校验、权限校验和资源范围校验。模型输出不是授权凭证。

## 8. 实现检查清单

- [ ] 使用 `messages` 数组保存准确的消息顺序。
- [ ] 新实现使用 `tools`/`tool_calls`，旧格式仅在兼容层解析。
- [ ] 工具结果按 `tool_call_id` 一一对应。
- [ ] 解析流式工具参数时按候选索引和工具索引聚合。
- [ ] 处理 `content: null`、拒答、截断和未知停止原因。
- [ ] 不把模型生成的工具名、参数或结果当作可信输入。
- [ ] 对重试、并行副作用和连接中断制定幂等策略。

## 9. 与其他协议的边界

Chat Completions 的工具调用是 assistant 消息中的 `tool_calls` 数组；Responses 使用独立的 `function_call` 输出项；Anthropic Messages 使用 `content` 中的 `tool_use`/`tool_result` 内容块。这三种结构不能直接互换，适配器必须重新构造消息或输出项。
