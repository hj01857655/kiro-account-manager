//! 请求日志：日志上下文、结构化日志写入、网关错误响应与日志落盘。

use super::*;

/// 上游请求错误：原为 4 元组，结构化后可携带类型化扩展字段
/// （如 429 的服务端限流窗口），不再靠 message 字符串夹带。
#[derive(Debug, Clone)]
pub struct UpstreamRequestError {
    pub status: StatusCode,
    pub error_type: &'static str,
    pub message: String,
    pub response_body: Option<String>,
    /// 429 时 `x-amzn-kiro-ratelimit-retry-after` 指定的重试窗口（毫秒）
    pub rate_limit_retry_after_ms: Option<u64>,
}

impl UpstreamRequestError {
    pub fn new(
        status: StatusCode,
        error_type: &'static str,
        message: String,
        response_body: Option<String>,
    ) -> Self {
        Self {
            status,
            error_type,
            message,
            response_body,
            rate_limit_retry_after_ms: None,
        }
    }
}

#[allow(dead_code)]
pub const STREAMING_RESPONSE_PLACEHOLDER: &str = "[streaming response omitted from request log]";

#[derive(Debug, Clone)]
pub struct RequestLogContext<'a> {
    pub(super) request_index: u64,
    pub(super) endpoint: &'a str,
    pub(super) client_addr: SocketAddr,
    pub(super) request: Option<&'a NormalizedRequest>,
    pub(super) upstream: Option<&'a UpstreamCredentials>,
    pub(super) upstream_source_hint: Option<String>,
    pub(super) region_hint: Option<String>,
    pub(super) started_at: Instant,
    #[allow(dead_code)]
    pub(super) request_body: Option<&'a str>,
    pub(super) request_body_hint: Option<String>,
    /// 从原始请求体提取的 model（用于错误日志）
    pub(super) model_hint: Option<String>,
    /// 是否流式请求（避免 request 为 None 时丢失信息）
    pub(super) is_stream: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct GatewayErrorDetails<'a> {
    pub(super) status: StatusCode,
    pub(super) error_type: &'a str,
    pub(super) message: &'a str,
    pub(super) response_body: Option<&'a str>,
}

pub fn get_request_endpoint(format: ResponseFormat) -> &'static str {
    match format {
        ResponseFormat::Anthropic => "v1/messages",
        ResponseFormat::Responses => "v1/responses",
        ResponseFormat::OpenAI => "v1/chat/completions",
    }
}

pub fn get_client_log_prefix(format: ResponseFormat) -> &'static str {
    match format {
        ResponseFormat::Anthropic => "anthropic-messages",
        ResponseFormat::Responses => "openai-responses",
        ResponseFormat::OpenAI => "openai-chat",
    }
}

pub fn get_client_log_prefix_for_endpoint(endpoint: &str) -> &'static str {
    match endpoint {
        "v1/messages" => "anthropic-messages",
        "v1/responses" => "openai-responses",
        "v1/chat/completions" => "openai-chat",
        _ => "client",
    }
}

pub fn get_client_sse_log_file(event: Option<&str>, payload: &str) -> &'static str {
    if event.is_some() {
        return "anthropic-messages-response-sse.log";
    }

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        if value
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|item| item.starts_with("response."))
        {
            return "openai-responses-response-sse.log";
        }

        if value
            .get("object")
            .and_then(Value::as_str)
            .is_some_and(|item| item.starts_with("chat.completion"))
        {
            return "openai-chat-response-sse.log";
        }
    }

    "client-response-sse.log"
}

pub fn serialize_logged_value(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// 从原始请求体 JSON 中提取 model 字段（用于错误日志）
pub fn extract_model_from_payload(payload_str: &str) -> Option<String> {
    serde_json::from_str::<Value>(payload_str)
        .ok()?
        .get("model")?
        .as_str()
        .map(String::from)
}

pub fn write_request_log(
    context: &RequestLogContext<'_>,
    status: StatusCode,
    outcome: &str,
    error: Option<&str>,
    error_type: Option<&str>,
    _response_body: Option<&str>,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    cache_read_input_tokens: Option<i32>,
    cache_creation_input_tokens: Option<i32>,
    state: &RouterState,
) {
    let duration_ms = context
        .started_at
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    // 只在出错时记录日志
    if !status.is_success() {
        log::error!(
            "请求失败 #{} | {} | {} | {}ms | {}",
            context.request_index,
            context.endpoint,
            status.as_u16(),
            duration_ms,
            error.unwrap_or("未知错误")
        );
    }

    // 生成请求摘要
    let request_summary = context.request.map(|req| {
        use crate::gateway::RequestSummary;
        RequestSummary {
            message_count: req.messages.len(),
            tool_count: req.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            total_content_length: req
                .messages
                .iter()
                .filter_map(|m| m.content.as_ref())
                .map(|c| c.to_string().len())
                .sum(),
            has_images: req.messages.iter().any(|m| {
                m.content
                    .as_ref()
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("image"))
                    })
                    .unwrap_or(false)
            }),
        }
    });

    // 生成响应摘要
    let response_summary = _response_body.and_then(|body| {
        use crate::gateway::ResponseSummary;
        serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .map(|v| ResponseSummary {
                content_length: body.len(),
                tool_calls_count: v
                    .get("content")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter(|item| {
                                item.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                            })
                            .count()
                    })
                    .unwrap_or(0),
                stop_reason: v
                    .get("stop_reason")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
            })
    });

    // 流式响应信息
    let stream_info = if context.is_stream.unwrap_or(false) {
        Some(crate::gateway::StreamInfo {
            chunk_count: 0,    // 需要在流式处理中累计
            first_chunk_ms: 0, // 需要在流式处理中记录
        })
    } else {
        None
    };

    let upstream_source = context
        .upstream
        .map(|item| item.source_label.clone())
        .or_else(|| context.upstream_source_hint.clone());
    let region = context
        .upstream
        .map(|item| item.region.clone())
        .or_else(|| context.region_hint.clone());

    let entry = GatewayRequestLogEntry {
        occurred_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        request_id: uuid::Uuid::new_v4().to_string(),
        request_index: context.request_index,
        endpoint: context.endpoint.to_string(),
        client_ip: context.client_addr.ip().to_string(),
        model: context
            .request
            .map(|item| item.model.clone())
            .or_else(|| context.model_hint.clone()),
        stream: context
            .is_stream
            .or_else(|| context.request.map(|item| item.stream))
            .unwrap_or(false),
        upstream_source,
        region,
        status_code: status.as_u16(),
        outcome: outcome.to_string(),
        duration_ms,
        error: error.map(str::to_string),
        request_body: context
            .request_body
            .map(str::to_string)
            .or_else(|| context.request_body_hint.clone()),
        response_body: _response_body.map(str::to_string),
        input_tokens,
        output_tokens,
        cache_read_input_tokens,
        cache_creation_input_tokens,
        error_type: error_type.map(str::to_string),
        request_summary,
        response_summary,
        stream_info,
    };

    // 如果关闭了日志记录，跳过
    if !state.config.log_requests {
        return;
    }

    // 写入文件日志
    let _ = append_gateway_request_log(&entry);

    // 保存到内存日志存储（异步）
    let log_store = state.log_store.clone();
    let entry_clone = entry.clone();
    tokio::spawn(async move {
        log_store.add(entry_clone).await;
    });
}

pub fn build_gateway_error_body(
    format: ResponseFormat,
    status: StatusCode,
    error_type: &str,
    message: &str,
) -> Value {
    match format {
        ResponseFormat::Anthropic => json!({
            "type": "error",
            "error": {
                "type": error_type,
                "message": message
            }
        }),
        ResponseFormat::Responses => json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": status.as_u16()
            }
        }),
        ResponseFormat::OpenAI => json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": status.as_u16()
            }
        }),
    }
}

pub async fn gateway_error_with_log(
    state: &RouterState,
    format: ResponseFormat,
    context: &RequestLogContext<'_>,
    error: GatewayErrorDetails<'_>,
) -> Response {
    // 如果有 response_body，尝试从中提取 message 用于 last_error
    let error_message = if error.message.is_empty() {
        error
            .response_body
            .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
            .and_then(|json| {
                json.pointer("/message")
                    .or_else(|| json.pointer("/error/message"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "上游错误".to_string())
    } else {
        error.message.to_string()
    };

    *state.last_error.lock().await = Some(error_message.clone());

    // 尝试从错误响应体中提取token信息
    let (input_tokens, output_tokens, cache_read, cache_creation) = error
        .response_body
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .and_then(|json| {
            let usage = json.get("usage")?;
            Some((
                usage
                    .get("input_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                usage
                    .get("output_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                usage
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
            ))
        })
        .unwrap_or((None, None, None, None));

    // 日志中记录的响应体：优先使用原始响应，否则构造
    let logged_response_body = error.response_body.map(str::to_string).or_else(|| {
        Some(serialize_logged_value(&build_gateway_error_body(
            format,
            error.status,
            error.error_type,
            error.message,
        )))
    });

    write_request_log(
        context,
        error.status,
        "error",
        if error.message.is_empty() {
            None
        } else {
            Some(error.message)
        },
        Some(error.error_type),
        logged_response_body.as_deref(),
        input_tokens,
        output_tokens,
        cache_read,
        cache_creation,
        state,
    );
    gateway_error_response(
        format,
        error.status,
        error.error_type,
        error.message,
        error.response_body,
    )
}
