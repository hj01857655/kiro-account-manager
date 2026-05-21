use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::Local;
use futures_util::StreamExt;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use url::Url;

use crate::{
    clients::{
        http_client::{
            build_kiro_custom_user_agent, resolve_kiro_upstream_region,
            should_add_redirect_for_internal, should_send_codewhisperer_optout,
        },
        kiro_q_client::KiroQClient,
    },
    commands::common::{
        calc_expires_at, calc_status, get_usage_by_provider, refresh_token_by_provider,
        RefreshResult,
    },
    commands::machine_guid::get_machine_id,
    core::account::{Account, AccountStore},
};

const MAX_FAILURES_PER_ACCOUNT: u32 = 3;
const MAX_KIRO_PAYLOAD_SIZE: usize = 615 * 1024; // 615KB - Kiro API 的 HTTP 请求大小限制

// Token 限制的默认值（当无法从 API 获取时使用）
const SUMMARIZATION_THRESHOLD_PERCENT: f64 = 0.8; // 80% 触发总结（Kiro IDE 的阈值）
const COUNT_TOKENS_SAFETY_MULTIPLIER: f64 = 1.5;
const REQUEST_TOKENS_SAFETY_MULTIPLIER: f64 = 1.5;
const EMPTY_ANTHROPIC_RESPONSE_FALLBACK: &str = "[Gateway received an empty upstream response.]";

use super::{
    append_gateway_request_log,
    converter::{
        build_kiro_payload, get_available_models, normalize_anthropic_request,
        normalize_responses_request,
    },
    effective_client_api_keys,
    eventstream::decode_message,
    models::{
        AnthropicContentBlock, AnthropicMessagesRequest, AnthropicMessagesResponse, AnthropicUsage,
        ModelsResponse, NormalizedMessage, NormalizedRequest, OpenAIChatRequest, Tool, ToolCall,
        ToolCallFunction, WebSearchToolOptions,
    },
    stream::{self, aggregate_kiro_response, parse_kiro_event_full, KiroEvent},
    thinking_parser::{SegmentType, ThinkingParser},
    GatewayConfig, GatewayRequestLogEntry, ResponseFormat, ResponsesSessionEntry, RouterState,
    DEFAULT_AGENT_MODE,
};

#[derive(Debug, Clone)]
struct UpstreamCredentials {
    access_token: String,
    profile_arn: Option<String>,
    provider: Option<String>,
    region: String,
    source_label: String,
    user_agent: String,
    #[allow(dead_code)]
    auth_method: Option<String>,
    send_opt_out: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct ServerToolCall {
    id: String,
    name: String,
    input: Value,
    result_content: Value,
    tool_result_text: String,
}

#[derive(Debug, Clone)]
struct ProxyExecutionOutcome {
    aggregated: stream::AggregatedKiroResponse,
    server_tool_calls: Vec<ServerToolCall>,
}

async fn restore_responses_session_messages(
    state: &RouterState,
    request: &NormalizedRequest,
) -> Vec<NormalizedMessage> {
    let Some(mut current_response_id) = request.previous_response_id.clone() else {
        return request.messages.clone();
    };

    let sessions = state.responses_sessions.lock().await;
    let mut chain = Vec::new();
    while let Some(entry) = sessions.get(&current_response_id) {
        chain.push(entry.clone());
        let Some(previous) = entry.previous_response_id.clone() else {
            break;
        };
        current_response_id = previous;
    }
    drop(sessions);

    if chain.is_empty() {
        return request.messages.clone();
    }

    // 收集当前请求中的 tool_result_id，用于过滤最后一轮的 tool_calls
    let current_tool_result_ids: std::collections::HashSet<String> = request
        .messages
        .iter()
        .filter(|message| message.role == "tool")
        .filter_map(|message| message.tool_call_id.clone())
        .collect();

    chain.reverse();
    let mut merged = Vec::new();
    let chain_len = chain.len();
    for (index, entry) in chain.into_iter().enumerate() {
        let is_latest_entry = index + 1 == chain_len;

        // 对最后一轮的 tool_calls 进行过滤：只保留当前请求有对应 tool_result 的
        let effective_tool_calls = if is_latest_entry && !current_tool_result_ids.is_empty() {
            let filtered: Vec<_> = entry
                .tool_calls
                .iter()
                .filter(|(id, _, _)| current_tool_result_ids.contains(id))
                .cloned()
                .collect();
            // 如果过滤后为空（可能是 ID 不匹配），回退到全部
            if filtered.is_empty() {
                entry.tool_calls.clone()
            } else {
                filtered
            }
        } else {
            entry.tool_calls.clone()
        };

        merged.extend(entry.request_messages.clone());
        merged.push(NormalizedMessage {
            role: "assistant".to_string(),
            content: Some(Value::String(entry.response_text.clone())),
            tool_calls: if effective_tool_calls.is_empty() {
                None
            } else {
                Some(
                    effective_tool_calls
                        .iter()
                        .map(|(id, name, arguments)| ToolCall {
                            id: id.clone(),
                            call_type: "function".to_string(),
                            function: ToolCallFunction {
                                name: name.clone(),
                                arguments: arguments.clone(),
                            },
                        })
                        .collect(),
                )
            },
            tool_call_id: None,
            metadata: None,
        });
    }
    merged.extend(request.messages.clone());
    merged
}

/// 从历史 session 继承 tools 和 tool_choice（Responses API 有状态对话）
///
/// 当客户端使用 previous_response_id 但不重传 tools 时，
/// 需要从历史 session 中继承工具定义。
async fn restore_responses_session_request_options(
    state: &RouterState,
    request: &NormalizedRequest,
) -> (Option<Vec<Tool>>, Option<Value>) {
    let Some(mut current_response_id) = request.previous_response_id.clone() else {
        return (None, None);
    };

    let sessions = state.responses_sessions.lock().await;
    let mut inherited_tools = None;
    let mut inherited_tool_choice = None;

    while let Some(entry) = sessions.get(&current_response_id) {
        if inherited_tools.is_none() {
            inherited_tools = entry.request_tools.clone();
        }
        if inherited_tool_choice.is_none() {
            inherited_tool_choice = entry.request_tool_choice.clone();
        }
        if inherited_tools.is_some() && inherited_tool_choice.is_some() {
            break;
        }
        let Some(previous) = entry.previous_response_id.clone() else {
            break;
        };
        current_response_id = previous;
    }

    (inherited_tools, inherited_tool_choice)
}

async fn persist_responses_session_entry(
    state: &RouterState,
    response_id: &str,
    request_messages: Vec<NormalizedMessage>,
    request_tools: Option<Vec<Tool>>,
    request_tool_choice: Option<Value>,
    previous_response_id: Option<String>,
    aggregated: &stream::AggregatedKiroResponse,
) {
    let mut sessions = state.responses_sessions.lock().await;
    sessions.retain(|_, entry| entry.updated_at.elapsed() < Duration::from_secs(60 * 60));
    sessions.insert(
        response_id.to_string(),
        ResponsesSessionEntry {
            response_id: response_id.to_string(),
            previous_response_id,
            request_messages,
            request_tools,
            request_tool_choice,
            response_text: aggregated.text.clone(),
            tool_calls: aggregated.tool_calls.clone(),
            updated_at: Instant::now(),
        },
    );
}

#[derive(Debug, Clone, PartialEq)]
struct ResponsesOutputText {
    text: String,
    annotations: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WebSearchSource {
    title: String,
    url: String,
}

type UpstreamRequestError = (StatusCode, &'static str, String, Option<String>);

const STREAMING_RESPONSE_PLACEHOLDER: &str = "[streaming response omitted from request log]";
const MAX_SERVER_WEB_SEARCH_ITERATIONS: usize = 8;

#[derive(Debug, Clone)]
struct RequestLogContext<'a> {
    request_index: u64,
    endpoint: &'a str,
    client_addr: SocketAddr,
    request: Option<&'a NormalizedRequest>,
    upstream: Option<&'a UpstreamCredentials>,
    started_at: Instant,
    #[allow(dead_code)]
    request_body: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
struct GatewayErrorDetails<'a> {
    status: StatusCode,
    error_type: &'static str,
    message: &'a str,
    response_body: Option<&'a str>,
}

fn build_models_response() -> Value {
    serde_json::to_value(ModelsResponse {
        object: "list".to_string(),
        data: get_available_models(),
    })
    .unwrap_or_else(|_| json!({ "object": "list", "data": [] }))
}

fn build_count_tokens_response(payload: &Value) -> Value {
    json!({ "input_tokens": estimate_count_tokens_payload(payload).max(1) })
}

fn estimate_count_tokens_payload(payload: &Value) -> usize {
    let model_id = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tokenizer_type = TokenizerType::from_model_id(model_id);
    let serialized = serde_json::to_string(payload).unwrap_or_else(|_| payload.to_string());
    let raw_tokens = estimate_text_tokens(&serialized, tokenizer_type);
    ((raw_tokens as f64) * COUNT_TOKENS_SAFETY_MULTIPLIER).ceil() as usize
}

fn build_health_response() -> Value {
    json!({ "ok": true })
}

/// 获取账号可用模型列表
///
/// 调用 Kiro Q API 的 ListAvailableModels 接口获取账号权限内的模型
async fn get_available_models_for_upstream(
    upstream: &UpstreamCredentials,
) -> Result<Vec<String>, String> {
    let client = KiroQClient::new()?;

    let response = client
        .list_available_models(
            &upstream.access_token,
            &get_machine_id(),
            &upstream.region,
            upstream.profile_arn.as_deref(),
            None, // model_provider
            None, // next_token
        )
        .await?;

    // 解析返回的模型列表
    let models = response
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or("Invalid response: missing models array")?
        .iter()
        .filter_map(|m| {
            m.get("modelId")
                .and_then(|id| id.as_str())
                .map(String::from)
        })
        .collect();

    Ok(models)
}

fn check_payload_size(payload: &Value) -> usize {
    serde_json::to_string(payload).map(|s| s.len()).unwrap_or(0)
}

/// Token 估算器类型（根据模型选择不同的估算方法）
#[derive(Debug, Clone, Copy)]
enum TokenizerType {
    Claude,  // Anthropic Claude 模型
    OpenAI,  // OpenAI GPT 模型（使用 tiktoken）
    Llama,   // Meta Llama 模型
    Generic, // 通用估算（未知模型）
}

impl TokenizerType {
    /// 根据模型 ID 判断使用哪种估算方法
    fn from_model_id(model_id: &str) -> Self {
        let model_lower = model_id.to_lowercase();

        // Claude 系列：4.5, 4.6, 4.7 及所有变体
        if model_lower.contains("claude") {
            TokenizerType::Claude
        } else if model_lower.contains("gpt")
            || model_lower.contains("o1")
            || model_lower.contains("o3")
        {
            TokenizerType::OpenAI
        } else if model_lower.contains("llama") {
            TokenizerType::Llama
        } else {
            TokenizerType::Generic
        }
    }
}

/// 估算请求消息的 token 数量（支持多种模型）
///
/// 参考 Kiro IDE 源码：extension.js 行 310847-310873
/// - Claude: length / 4 + newlines * 0.5 + code_blocks * 2
/// - OpenAI: 使用 Generic 方法（tiktoken 需要额外依赖）
/// - Llama: length / 3.5
/// - Generic: length / 4 + newlines * 0.5 + code_blocks * 2
///
/// 注意：这是粗略估算，用于提前拒绝明显超长的请求
/// - Kiro API 的 max_input_tokens 是 200k
/// - Kiro IDE 在 80% (160k tokens) 时触发自动总结
/// - 网关在 160k tokens 时直接拒绝（无法实现 AI 总结）
fn estimate_request_tokens(messages: &[NormalizedMessage], model_id: &str) -> usize {
    let tokenizer_type = TokenizerType::from_model_id(model_id);

    messages
        .iter()
        .map(|msg| {
            let mut tokens = 0;

            // 估算 content 字段的 token 数
            if let Some(content) = &msg.content {
                let text = extract_plain_text(Some(content));
                tokens += estimate_text_tokens(&text, tokenizer_type);
            }

            // 估算 tool_calls 的 token 数
            if let Some(tool_calls) = &msg.tool_calls {
                for tool_call in tool_calls {
                    tokens += estimate_text_tokens(&tool_call.function.name, tokenizer_type);
                    tokens += estimate_text_tokens(&tool_call.function.arguments, tokenizer_type);
                }
            }

            tokens
        })
        .sum()
}

fn estimate_request_tokens_with_tools(
    messages: &[NormalizedMessage],
    tools: &Option<Vec<Tool>>,
    model_id: &str,
) -> usize {
    let raw_tokens = estimate_request_tokens(messages, model_id) + estimate_tools_tokens(tools, model_id);
    ((raw_tokens as f64) * REQUEST_TOKENS_SAFETY_MULTIPLIER).ceil() as usize
}

fn estimate_tools_tokens(tools: &Option<Vec<Tool>>, model_id: &str) -> usize {
    let Some(tools) = tools.as_ref().filter(|items| !items.is_empty()) else {
        return 0;
    };
    let tokenizer_type = TokenizerType::from_model_id(model_id);
    serde_json::to_string(tools)
        .map(|serialized| estimate_text_tokens(&serialized, tokenizer_type))
        .unwrap_or(0)
}

/// 估算单个文本的 token 数量（支持多种模型）
///
/// 参考 Kiro IDE 源码：extension.js 行 310878-310911
///
/// **Claude 估算**（行 310894-310895）：
/// ```javascript
/// estimateWithClaude(text) {
///   return Math.ceil(text.length / 4);
/// }
/// ```
///
/// **Llama 估算**（行 310888-310889）：
/// ```javascript
/// estimateWithLlama(text) {
///   return Math.ceil(text.length / 3.5);
/// }
/// ```
///
/// **Generic 估算**（行 310906-310911）：
/// ```javascript
/// estimateGeneric(text) {
///   const baseTokens = Math.ceil(text.length / 4);
///   const newlineTokens = Math.ceil(text.split('\n').length * 0.5);
///   const codeBlockTokens = (text.match(/```/g) || []).length * 2;
///   return baseTokens + newlineTokens + codeBlockTokens;
/// }
/// ```
fn estimate_text_tokens(text: &str, tokenizer_type: TokenizerType) -> usize {
    if text.is_empty() {
        return 0;
    }

    match tokenizer_type {
        TokenizerType::Claude => {
            // Claude: length / 4
            text.len().div_ceil(4)
        }
        TokenizerType::OpenAI => {
            // OpenAI: 使用 Generic 方法（tiktoken 需要额外依赖，这里简化处理）
            estimate_generic_tokens(text)
        }
        TokenizerType::Llama => {
            // Llama: length / 3.5 (向上取整)
            ((text.len() as f64 / 3.5).ceil() as usize).max(1)
        }
        TokenizerType::Generic => estimate_generic_tokens(text),
    }
}

/// 通用 token 估算方法（Kiro IDE 的 estimateGeneric）
///
/// 公式：
/// - base_tokens = ceil(length / 4)
/// - newline_tokens = ceil(lines * 0.5)
/// - code_block_tokens = code_blocks * 2
/// - total = base_tokens + newline_tokens + code_block_tokens
fn estimate_generic_tokens(text: &str) -> usize {
    // 基础估算：4 字符 = 1 token（向上取整）
    let base_tokens = text.len().div_ceil(4);

    // 换行符：每行 +0.5 token（向上取整）
    let lines = text.lines().count();
    let newline_tokens = lines.div_ceil(2);

    // 代码块：每个 ``` +2 tokens
    let code_blocks = text.matches("```").count();
    let code_block_tokens = code_blocks * 2;

    base_tokens + newline_tokens + code_block_tokens
}

/// 获取模型的最大输入 token 数
///
/// 根据模型 ID 返回对应的 maxInputTokens
///
/// 数据来源：
/// - Kiro 官方文档：https://kiro.dev/docs/models/
/// - Claude Opus 4.6/4.7：1M tokens
/// - Claude Sonnet 4.6：1M tokens
/// - 其他 Claude 4.x：200k tokens
async fn get_model_max_input_tokens(model_id: &str) -> usize {
    if let Some(override_tokens) = parse_model_context_window_override(model_id) {
        return override_tokens;
    }

    let model_lower = model_id.to_lowercase();

    // 根据模型 ID 返回对应的 token 限制
    if model_lower == "auto" {
        1_000_000 // auto 模型支持 1M tokens
    } else if model_lower.contains("opus-4.7") || model_lower.contains("opus-4-7") {
        1_000_000 // Claude Opus 4.7: 1M tokens
    } else if model_lower.contains("opus-4.6") || model_lower.contains("opus-4-6") {
        1_000_000 // Claude Opus 4.6: 1M tokens
    } else if model_lower.contains("sonnet-4.6") || model_lower.contains("sonnet-4-6") {
        1_000_000 // Claude Sonnet 4.6: 1M tokens
    } else if model_lower.contains("qwen") {
        256_000 // Qwen3 Coder Next: 256k tokens
    } else if model_lower.contains("llama") || model_lower.contains("deepseek") {
        128_000 // Llama/DeepSeek: 128k tokens
    } else {
        // Claude 4.5/4.0、OpenAI、MiniMax、GLM 等其他模型默认 200k tokens
        // 包括：
        // - claude-opus-4.5, claude-sonnet-4.5/4.0, claude-haiku-4.5/4.6/4.7
        // - gpt-4, gpt-4-turbo, o1, o3
        // - minimax-m2.5, minimax-m2.1
        // - glm-5
        200_000
    }
}

fn parse_model_context_window_override(model_id: &str) -> Option<usize> {
    let trimmed = model_id.trim();
    let without_close = trimmed.strip_suffix(']')?;
    let open_index = without_close.rfind('[')?;
    let value = without_close[open_index + 1..].trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let tokens = value.parse::<usize>().ok()?;
    (tokens >= 4096 && tokens <= 2_000_000).then_some(tokens)
}

/// 智能裁剪 Kiro payload 历史记录
///
/// 策略：
/// 1. 识别 tool call/result 配对（Assistant with tool_uses + User with tool_results）
/// 2. 从最旧的完整对话单元开始删除
/// 3. 保留最近的对话（至少保留最后 2 条消息）
/// 4. 避免破坏 tool_calls 和 tool_results 的配对关系
fn trim_kiro_payload_history(payload: &mut Value, max_bytes: usize) -> bool {
    let original_size = check_payload_size(payload);
    if original_size <= max_bytes {
        return false;
    }

    let original_len = payload
        .pointer("/conversationState/history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    if original_len == 0 {
        return false;
    }

    // 循环删除最旧的消息单元，直到满足大小要求
    let mut removed_count = 0;
    loop {
        // 检查当前大小
        let current_size = check_payload_size(payload);
        if current_size <= max_bytes {
            break;
        }

        // 获取当前历史记录
        let Some(history) = payload
            .pointer_mut("/conversationState/history")
            .and_then(|v| v.as_array_mut())
        else {
            break;
        };

        // 至少保留 2 条消息
        if history.len() <= 2 {
            break;
        }

        // 检查第一条消息是否是 Assistant 消息且包含 tool_uses
        let first_is_assistant_with_tools = history
            .first()
            .and_then(|msg| msg.get("assistant_response_message"))
            .and_then(|msg| msg.get("tool_uses"))
            .and_then(|tools| tools.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

        if first_is_assistant_with_tools && history.len() > 1 {
            // 检查第二条消息是否是 User 消息且包含 tool_results
            let second_has_tool_results = history
                .get(1)
                .and_then(|msg| msg.get("user_input_message"))
                .and_then(|msg| msg.get("user_input_message_context"))
                .and_then(|ctx| ctx.get("tool_results"))
                .and_then(|results| results.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);

            if second_has_tool_results {
                // 这是一个 tool call/result 配对，必须一起删除
                if history.len() > 3 {
                    // 确保删除后还剩至少 2 条消息
                    history.remove(0);
                    history.remove(0); // 删除第二条（现在变成第一条了）
                    removed_count += 2;
                    log::debug!(
                        "[Gateway] Removed tool call/result pair. Remaining: {}",
                        history.len()
                    );
                    continue;
                } else {
                    // 删除后会少于 2 条消息，停止裁剪
                    break;
                }
            }
        }

        // 单个消息可以安全删除
        history.remove(0);
        removed_count += 1;
        log::debug!(
            "[Gateway] Removed single message. Remaining: {}",
            history.len()
        );
    }

    let final_len = payload
        .pointer("/conversationState/history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    let trimmed = final_len < original_len;

    if trimmed {
        log::info!(
            "[Gateway] Trimmed history from {} to {} messages (removed {} messages)",
            original_len,
            final_len,
            removed_count
        );
    }

    trimmed
}

fn estimate_kiro_payload_tokens(payload: &Value, model_id: &str) -> usize {
    let tokenizer_type = TokenizerType::from_model_id(model_id);
    let serialized = serde_json::to_string(payload).unwrap_or_else(|_| payload.to_string());
    let raw_tokens = estimate_text_tokens(&serialized, tokenizer_type);
    ((raw_tokens as f64) * REQUEST_TOKENS_SAFETY_MULTIPLIER).ceil() as usize
}

fn trim_kiro_payload_history_to_token_limit(
    payload: &mut Value,
    model_id: &str,
    max_tokens: usize,
) -> bool {
    if estimate_kiro_payload_tokens(payload, model_id) <= max_tokens {
        return false;
    }

    let original_len = payload
        .pointer("/conversationState/history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    if original_len == 0 {
        return false;
    }

    let mut removed_count = 0;
    loop {
        if estimate_kiro_payload_tokens(payload, model_id) <= max_tokens {
            break;
        }

        let Some(history) = payload
            .pointer_mut("/conversationState/history")
            .and_then(|v| v.as_array_mut())
        else {
            break;
        };

        if history.len() <= 2 {
            break;
        }

        let first_is_assistant_with_tools = history
            .first()
            .and_then(|msg| msg.get("assistant_response_message"))
            .and_then(|msg| msg.get("tool_uses"))
            .and_then(|tools| tools.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

        if first_is_assistant_with_tools && history.len() > 1 {
            let second_has_tool_results = history
                .get(1)
                .and_then(|msg| msg.get("user_input_message"))
                .and_then(|msg| msg.get("user_input_message_context"))
                .and_then(|ctx| ctx.get("tool_results"))
                .and_then(|results| results.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);

            if second_has_tool_results {
                if history.len() > 3 {
                    history.remove(0);
                    history.remove(0);
                    removed_count += 2;
                    continue;
                }
                break;
            }
        }

        history.remove(0);
        removed_count += 1;
    }

    let final_len = payload
        .pointer("/conversationState/history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    let trimmed = final_len < original_len;

    if trimmed {
        log::info!(
            "[Gateway] Token trim removed {} history messages ({} -> {})",
            removed_count,
            original_len,
            final_len
        );
    }

    trimmed
}

async fn guarded_local_response(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    endpoint: &'static str,
    request_body: Option<&str>,
    response_body: Value,
) -> Response {
    let request_index = state
        .request_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let started_at = Instant::now();
    let log_context = RequestLogContext {
        request_index,
        endpoint,
        client_addr,
        request: None,
        upstream: None,
        started_at,
        request_body,
    };

    if state.config.local_only && !client_addr.ip().is_loopback() {
        let message = format!("已拒绝来自非本机地址的访问: {}", client_addr.ip());
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &log_context,
            GatewayErrorDetails {
                status: StatusCode::FORBIDDEN,
                error_type: "permission_error",
                message: &message,
                response_body: None,
            },
        )
        .await;
    }
    if !state.config.local_only
        && !state.config.allowed_ips.is_empty()
        && !ip_matches_allowlist(client_addr.ip(), &state.config.allowed_ips)
    {
        let message = format!("访问地址 {} 不在反代白名单中", client_addr.ip());
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &log_context,
            GatewayErrorDetails {
                status: StatusCode::FORBIDDEN,
                error_type: "permission_error",
                message: &message,
                response_body: None,
            },
        )
        .await;
    }
    if let Err(message) = verify_client_auth(&headers, &state.config) {
        let sanitized = sanitize_error(&message);
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &log_context,
            GatewayErrorDetails {
                status: StatusCode::UNAUTHORIZED,
                error_type: "authentication_error",
                message: &sanitized,
                response_body: None,
            },
        )
        .await;
    }

    let serialized = serialize_logged_value(&response_body);
    write_request_log(
        &log_context,
        StatusCode::OK,
        "success",
        None,
        Some(serialized.as_str()),
        None, // input_tokens
        None, // output_tokens
        None, // cache_read_input_tokens
        None, // cache_creation_input_tokens
    );
    Json(response_body).into_response()
}

pub async fn health_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
) -> Response {
    guarded_local_response(
        state,
        client_addr,
        headers,
        "health",
        None,
        build_health_response(),
    )
    .await
}

pub async fn models_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
) -> Response {
    guarded_local_response(
        state,
        client_addr,
        headers,
        "models",
        None,
        build_models_response(),
    )
    .await
}

pub async fn count_tokens_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    payload: Value,
) -> Response {
    let request_body = payload.to_string();
    guarded_local_response(
        state,
        client_addr,
        headers,
        "count_tokens",
        Some(request_body.as_str()),
        build_count_tokens_response(&payload),
    )
    .await
}

fn request_endpoint(format: ResponseFormat) -> &'static str {
    match format {
        ResponseFormat::Anthropic => "messages",
        ResponseFormat::Responses => "responses",
        ResponseFormat::OpenAI => "chat_completions",
    }
}

fn serialize_logged_value(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn write_request_log(
    context: &RequestLogContext<'_>,
    status: StatusCode,
    outcome: &str,
    error: Option<&str>,
    _response_body: Option<&str>,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    cache_read_input_tokens: Option<i32>,
    cache_creation_input_tokens: Option<i32>,
) {
    // 调试日志：记录 tokens 信息
    if input_tokens.is_none() && output_tokens.is_none() {
        eprintln!(
            "⚠️  [write_request_log] No tokens info for request #{}",
            context.request_index
        );
    } else {
        eprintln!("📊 [write_request_log] Request #{}: input={:?}, output={:?}, cache_read={:?}, cache_creation={:?}",
            context.request_index, input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens);
    }

    let duration_ms = context
        .started_at
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let entry = GatewayRequestLogEntry {
        occurred_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        request_index: context.request_index,
        endpoint: context.endpoint.to_string(),
        client_ip: context.client_addr.ip().to_string(),
        model: context.request.map(|item| item.model.clone()),
        stream: context.request.map(|item| item.stream).unwrap_or(false),
        upstream_source: context.upstream.map(|item| item.source_label.clone()),
        region: context.upstream.map(|item| item.region.clone()),
        status_code: status.as_u16(),
        outcome: outcome.to_string(),
        duration_ms,
        error: error.map(str::to_string),
        request_body: None,
        response_body: None,
        input_tokens,
        output_tokens,
        cache_read_input_tokens,
        cache_creation_input_tokens,
    };
    let _ = append_gateway_request_log(&entry);
}

fn build_gateway_error_body(
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

async fn gateway_error_with_log(
    state: &RouterState,
    format: ResponseFormat,
    context: &RequestLogContext<'_>,
    error: GatewayErrorDetails<'_>,
) -> Response {
    *state.last_error.lock().await = Some(error.message.to_string());
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
        Some(error.message),
        logged_response_body.as_deref(),
        None, // input_tokens
        None, // output_tokens
        None, // cache_read_input_tokens
        None, // cache_creation_input_tokens
    );
    gateway_error_response(format, error.status, error.error_type, error.message)
}

pub async fn proxy_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    payload: Value,
    format: ResponseFormat,
) -> Response {
    let request_index = state
        .request_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let endpoint = request_endpoint(format);
    let started_at = Instant::now();
    let raw_request_body = payload.to_string();
    let base_log_context = RequestLogContext {
        request_index,
        endpoint,
        client_addr,
        request: None,
        upstream: None,
        started_at,
        request_body: Some(raw_request_body.as_str()),
    };

    if state.config.local_only && !client_addr.ip().is_loopback() {
        let message = format!("已拒绝来自非本机地址的访问: {}", client_addr.ip());
        return gateway_error_with_log(
            &state,
            format,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::FORBIDDEN,
                error_type: "permission_error",
                message: &message,
                response_body: None,
            },
        )
        .await;
    }
    if !state.config.local_only
        && !state.config.allowed_ips.is_empty()
        && !ip_matches_allowlist(client_addr.ip(), &state.config.allowed_ips)
    {
        let message = format!("访问地址 {} 不在反代白名单中", client_addr.ip());
        return gateway_error_with_log(
            &state,
            format,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::FORBIDDEN,
                error_type: "permission_error",
                message: &message,
                response_body: None,
            },
        )
        .await;
    }

    if let Err(message) = verify_client_auth(&headers, &state.config) {
        let sanitized = sanitize_error(&message);
        return gateway_error_with_log(
            &state,
            format,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::UNAUTHORIZED,
                error_type: "authentication_error",
                message: &sanitized,
                response_body: None,
            },
        )
        .await;
    }

    let request = match normalize_request(format, &payload) {
        Ok(request) => request,
        Err(message) => {
            let sanitized = sanitize_error(&message);
            return gateway_error_with_log(
                &state,
                format,
                &base_log_context,
                GatewayErrorDetails {
                    status: StatusCode::BAD_REQUEST,
                    error_type: "invalid_request_error",
                    message: &sanitized,
                    response_body: None,
                },
            )
            .await;
        }
    };
    let request = if matches!(format, ResponseFormat::Responses) {
        let mut resumed = request.clone();
        resumed.messages = restore_responses_session_messages(&state, &request).await;
        // 如果当前请求没有 tools/tool_choice，从历史 session 继承
        if resumed.tools.is_none() || resumed.tool_choice.is_none() {
            let (inherited_tools, inherited_tool_choice) =
                restore_responses_session_request_options(&state, &request).await;
            if resumed.tools.is_none() {
                resumed.tools = inherited_tools;
            }
            if resumed.tool_choice.is_none() {
                resumed.tool_choice = inherited_tool_choice;
            }
        }
        resumed
    } else {
        request
    };

    let request_log_context = RequestLogContext {
        request: Some(&request),
        ..base_log_context.clone()
    };

    let upstream = match resolve_upstream_credentials(&state.config, &state).await {
        Ok(creds) => creds,
        Err(message) => {
            let sanitized = sanitize_error(&message);
            return gateway_error_with_log(
                &state,
                format,
                &request_log_context,
                GatewayErrorDetails {
                    status: StatusCode::UNAUTHORIZED,
                    error_type: "authentication_error",
                    message: &sanitized,
                    response_body: None,
                },
            )
            .await;
        }
    };
    let response_id = format!("resp_{}", short_uuid());
    let message_id = format!("msg_{}", short_uuid());
    let created_at = chrono::Utc::now().timestamp();

    let upstream_log_context = RequestLogContext {
        upstream: Some(&upstream),
        ..request_log_context.clone()
    };

    if has_server_web_search_tool(&request) {
        let outcome = match execute_request_with_server_tools(&state, &upstream, &request).await {
            Ok(outcome) => outcome,
            Err((status, error_type, message)) => {
                return gateway_error_with_log(
                    &state,
                    format,
                    &upstream_log_context,
                    GatewayErrorDetails {
                        status,
                        error_type,
                        message: &message,
                        response_body: None,
                    },
                )
                .await;
            }
        };

        if request.stream && matches!(format, ResponseFormat::Responses) {
            let response = build_stream_responses_completed_event(
                &request.model,
                &outcome.aggregated,
                &outcome.server_tool_calls,
                &response_id,
                &message_id,
                created_at,
                request.previous_response_id.as_deref(),
            );
            let response_body = serialize_logged_value(&response);
            write_request_log(
                &upstream_log_context,
                StatusCode::OK,
                "success",
                None,
                Some(response_body.as_str()),
                Some(outcome.aggregated.input_tokens),
                Some(outcome.aggregated.output_tokens),
                outcome.aggregated.cache_read_input_tokens,
                outcome.aggregated.cache_creation_input_tokens,
            );
            let body = format!("data: {}\n\ndata: [DONE]\n\n", response);
            return Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/event-stream"),
                )
                .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
                .header(header::CONNECTION, HeaderValue::from_static("keep-alive"))
                .body(Body::from(body))
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }

        let response = match format {
            ResponseFormat::Anthropic => build_anthropic_response(
                &request.model,
                &outcome.aggregated,
                &outcome.server_tool_calls,
            ),
            ResponseFormat::Responses => build_responses_response_with_ids(
                &request.model,
                &outcome.aggregated,
                &outcome.server_tool_calls,
                &response_id,
                &message_id,
                created_at,
                request.previous_response_id.as_deref(),
            ),
            ResponseFormat::OpenAI => serde_json::to_value(stream::build_openai_response(
                &request.model,
                &outcome.aggregated,
            ))
            .unwrap_or_else(|_| json!({})),
        };
        let response_body = serialize_logged_value(&response);
        if matches!(format, ResponseFormat::Responses) {
            persist_responses_session_entry(
                &state,
                &response_id,
                request.messages.clone(),
                request.tools.clone(),
                request.tool_choice.clone(),
                request.previous_response_id.clone(),
                &outcome.aggregated,
            )
            .await;
        }
        write_request_log(
            &upstream_log_context,
            StatusCode::OK,
            "success",
            None,
            Some(response_body.as_str()),
            Some(outcome.aggregated.input_tokens),
            Some(outcome.aggregated.output_tokens),
            outcome.aggregated.cache_read_input_tokens,
            outcome.aggregated.cache_creation_input_tokens,
        );
        return Json(response).into_response();
    }

    // 【第一层防护】Token 预估（硬限制，带缓存）
    // 基于 Kiro IDE 的 80% 总结阈值
    // Kiro IDE 在 80% 时触发 Truncation Summarization，网关无法实现，所以直接拒绝

    // 生成缓存 key：messages 的 JSON 序列化 + model_id
    let cache_key = format!(
        "{}:{}:{}",
        serde_json::to_string(&request.messages).unwrap_or_default(),
        serde_json::to_string(&request.tools).unwrap_or_default(),
        request.model
    );

    // 尝试从缓存获取
    let estimated_tokens = {
        let mut cache = state.token_cache.lock().await;
        cache.get(&cache_key)
    };

    // 如果缓存未命中，计算并存入缓存
    let estimated_tokens = if let Some(tokens) = estimated_tokens {
        tokens
    } else {
        let tokens =
            estimate_request_tokens_with_tools(&request.messages, &request.tools, &request.model);
        let mut cache = state.token_cache.lock().await;
        cache.insert(cache_key, tokens);
        tokens
    };

    // 从可用模型列表中获取该模型的 maxInputTokens
    let max_input_tokens = get_model_max_input_tokens(&request.model).await;
    let threshold_tokens = (max_input_tokens as f64 * SUMMARIZATION_THRESHOLD_PERCENT) as usize;

    if estimated_tokens > threshold_tokens {
        log::warn!(
            "[Gateway] Estimated {} tokens exceeds threshold {} tokens (80% of {}). Will trim Kiro history before upstream request.",
            estimated_tokens,
            threshold_tokens,
            max_input_tokens
        );
    }

    // 获取账号可用模型列表（用于模型降级）
    let available_models = match get_available_models_for_upstream(&upstream).await {
        Ok(models) => {
            log::debug!(
                "[Gateway] 账号 {} 可用模型: {:?}",
                upstream.source_label,
                models
            );
            Some(models)
        }
        Err(e) => {
            log::warn!(
                "[Gateway] 无法获取账号 {} 的可用模型列表: {}，将不进行模型降级",
                upstream.source_label,
                e
            );
            None
        }
    };

    let upstream_payload = match build_kiro_payload(
        &state.http,
        &request,
        upstream.profile_arn.clone(),
        available_models.as_deref(),
    )
    .await
    {
        Ok(payload) => payload,
        Err(message) => {
            let sanitized = sanitize_error(&message);
            return gateway_error_with_log(
                &state,
                format,
                &upstream_log_context,
                GatewayErrorDetails {
                    status: StatusCode::BAD_REQUEST,
                    error_type: "invalid_request_error",
                    message: &sanitized,
                    response_body: None,
                },
            )
            .await;
        }
    };

    // 【第二层防护】Payload 大小裁剪（硬限制 - 615KB）
    // 如果 payload 超过 Kiro API 的 HTTP 请求大小限制，自动裁剪历史记录
    let mut payload_value = serde_json::to_value(&upstream_payload).unwrap_or_else(|_| json!({}));

    let original_tokens = estimate_kiro_payload_tokens(&payload_value, &request.model);
    if original_tokens > threshold_tokens {
        log::info!(
            "[Gateway] Kiro payload estimate {} tokens exceeds threshold {} tokens. Trimming history...",
            original_tokens,
            threshold_tokens
        );
        let trimmed = trim_kiro_payload_history_to_token_limit(
            &mut payload_value,
            &request.model,
            threshold_tokens,
        );
        if trimmed {
            let final_tokens = estimate_kiro_payload_tokens(&payload_value, &request.model);
            log::info!(
                "[Gateway] Kiro payload token estimate trimmed from {} to {} tokens",
                original_tokens,
                final_tokens
            );
        }
    }

    let original_size = check_payload_size(&payload_value);
    if original_size > MAX_KIRO_PAYLOAD_SIZE {
        log::info!(
            "[Gateway] Payload size {} bytes exceeds limit {} bytes. Trimming history...",
            original_size,
            MAX_KIRO_PAYLOAD_SIZE
        );
        let trimmed = trim_kiro_payload_history(&mut payload_value, MAX_KIRO_PAYLOAD_SIZE);
        if trimmed {
            let final_size = check_payload_size(&payload_value);
            log::info!(
                "[Gateway] Payload trimmed from {} bytes to {} bytes",
                original_size,
                final_size
            );
        }
    }

    let upstream_request_body = serde_json::to_string_pretty(&payload_value)
        .unwrap_or_else(|_| "[failed to serialize upstream payload]".to_string());
    let upstream_payload_log_context = RequestLogContext {
        request_body: Some(upstream_request_body.as_str()),
        ..upstream_log_context.clone()
    };

    // 账号重试循环：遇到 429 错误时切换账号
    const MAX_ACCOUNT_RETRIES: u32 = 3;
    let mut account_attempt = 0;
    let mut tried_account_ids: HashSet<String> = HashSet::new();

    let upstream_resp = loop {
        account_attempt += 1;

        if account_attempt > MAX_ACCOUNT_RETRIES {
            // 所有账号都尝试过了，返回最后一个错误
            return gateway_error_with_log(
                &state,
                format,
                &upstream_payload_log_context,
                GatewayErrorDetails {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    error_type: "rate_limit_error",
                    message: "所有账号均达到速率限制，请稍后再试",
                    response_body: None,
                },
            )
            .await;
        }

        // 如果不是第一次尝试，需要重新选择账号
        let current_upstream = if account_attempt > 1 {
            match resolve_upstream_credentials(&state.config, &state).await {
                Ok(creds) => {
                    // 检查是否已经尝试过这个账号
                    let account_id = extract_account_id_from_upstream(&creds);
                    if tried_account_ids.contains(&account_id) {
                        log::warn!(
                            "[Gateway] 账号 {} 已尝试过，继续尝试下一个 (尝试: {}/{})",
                            creds.source_label,
                            account_attempt,
                            MAX_ACCOUNT_RETRIES
                        );
                        continue;
                    }
                    tried_account_ids.insert(account_id);
                    creds
                }
                Err(message) => {
                    let sanitized = sanitize_error(&message);
                    log::warn!(
                        "[Gateway] 重新选择账号失败 (尝试: {}/{}): {}",
                        account_attempt,
                        MAX_ACCOUNT_RETRIES,
                        sanitized
                    );
                    // 如果是账号不可用，继续尝试
                    if message.contains("未找到符合反代配置的可用账号") {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    // 其他错误直接返回
                    return gateway_error_with_log(
                        &state,
                        format,
                        &upstream_payload_log_context,
                        GatewayErrorDetails {
                            status: StatusCode::UNAUTHORIZED,
                            error_type: "authentication_error",
                            message: &sanitized,
                            response_body: None,
                        },
                    )
                    .await;
                }
            }
        } else {
            // 第一次尝试，使用已选择的账号
            let account_id = extract_account_id_from_upstream(&upstream);
            tried_account_ids.insert(account_id);
            upstream.clone()
        };

        // 发送请求
        match send_generate_request(&state.http, &current_upstream, &payload_value).await {
            Ok(resp) => break resp,
            Err((status, error_type, message, upstream_response_body)) => {
                // 检查是否是 429 错误
                if status == StatusCode::TOO_MANY_REQUESTS {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 标记账号为速率限制
                    state.load_balancer.mark_rate_limited(&account_id).await;
                    state.load_balancer.record_failure(&account_id).await;

                    log::warn!(
                        "[Gateway] 账号 {} 返回 429 错误，标记为速率限制并切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        MAX_ACCOUNT_RETRIES
                    );

                    // 继续尝试下一个账号
                    continue;
                }

                // 其他错误直接返回
                return gateway_error_with_log(
                    &state,
                    format,
                    &upstream_payload_log_context,
                    GatewayErrorDetails {
                        status,
                        error_type,
                        message: &message,
                        response_body: upstream_response_body.as_deref(),
                    },
                )
                .await;
            }
        }
    };

    if request.stream {
        // 流式开始时不记录日志，等流式结束后再记录完整的 tokens
        // 将 log_context 转换为 'static 生命周期
        let static_log_context = RequestLogContext {
            request_index: upstream_payload_log_context.request_index,
            endpoint: Box::leak(
                upstream_payload_log_context
                    .endpoint
                    .to_string()
                    .into_boxed_str(),
            ),
            client_addr: upstream_payload_log_context.client_addr,
            request: None,  // 不持有引用
            upstream: None, // 不持有引用
            started_at: upstream_payload_log_context.started_at,
            request_body: None,
        };

        return stream_proxy_response(
            state.clone(),
            upstream_resp,
            format,
            request.model.clone(),
            request.messages.clone(),
            request.tools.clone(),
            request.tool_choice.clone(),
            request.previous_response_id.clone(),
            Vec::new(),
            static_log_context,
        );
    }

    let body = match upstream_resp.text().await {
        Ok(body) => body,
        Err(error) => {
            let message = sanitize_error(&format!("读取上游响应失败: {error}"));
            return gateway_error_with_log(
                &state,
                format,
                &upstream_payload_log_context,
                GatewayErrorDetails {
                    status: StatusCode::BAD_GATEWAY,
                    error_type: "api_error",
                    message: &message,
                    response_body: None,
                },
            )
            .await;
        }
    };

    if let Some((status, error_type, message)) = detect_upstream_error_body(&body) {
        return gateway_error_with_log(
            &state,
            format,
            &upstream_payload_log_context,
            GatewayErrorDetails {
                status,
                error_type,
                message: &message,
                response_body: Some(body.as_str()),
            },
        )
        .await;
    }

    let aggregated = aggregate_kiro_response(&body);
    let response = match format {
        ResponseFormat::Anthropic => build_anthropic_response(&request.model, &aggregated, &[]),
        ResponseFormat::Responses => build_responses_response_with_ids(
            &request.model,
            &aggregated,
            &[],
            &response_id,
            &message_id,
            created_at,
            request.previous_response_id.as_deref(),
        ),
        ResponseFormat::OpenAI => {
            serde_json::to_value(stream::build_openai_response(&request.model, &aggregated))
                .unwrap_or_else(|_| json!({}))
        }
    };
    if matches!(format, ResponseFormat::Responses) {
        persist_responses_session_entry(
            &state,
            &response_id,
            request.messages.clone(),
            request.tools.clone(),
            request.tool_choice.clone(),
            request.previous_response_id.clone(),
            &aggregated,
        )
        .await;
    }
    write_request_log(
        &upstream_payload_log_context,
        StatusCode::OK,
        "success",
        None,
        Some(body.as_str()),
        Some(aggregated.input_tokens),
        Some(aggregated.output_tokens),
        aggregated.cache_read_input_tokens,
        aggregated.cache_creation_input_tokens,
    );
    Json(response).into_response()
}

pub async fn mcp_proxy_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    payload: Value,
) -> Response {
    let request_index = state
        .request_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let started_at = Instant::now();
    let raw_request_body = payload.to_string();
    let base_log_context = RequestLogContext {
        request_index,
        endpoint: "mcp",
        client_addr,
        request: None,
        upstream: None,
        started_at,
        request_body: Some(raw_request_body.as_str()),
    };

    if state.config.local_only && !client_addr.ip().is_loopback() {
        let message = format!("已拒绝来自非本机地址的 MCP 访问: {}", client_addr.ip());
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::FORBIDDEN,
                error_type: "permission_error",
                message: &message,
                response_body: None,
            },
        )
        .await;
    }
    if !state.config.local_only
        && !state.config.allowed_ips.is_empty()
        && !ip_matches_allowlist(client_addr.ip(), &state.config.allowed_ips)
    {
        let message = format!("MCP 访问地址 {} 不在反代白名单中", client_addr.ip());
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::FORBIDDEN,
                error_type: "permission_error",
                message: &message,
                response_body: None,
            },
        )
        .await;
    }
    if let Err(message) = verify_client_auth(&headers, &state.config) {
        let sanitized = sanitize_error(&message);
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::UNAUTHORIZED,
                error_type: "authentication_error",
                message: &sanitized,
                response_body: None,
            },
        )
        .await;
    }

    let upstream = match resolve_upstream_credentials(&state.config, &state).await {
        Ok(creds) => creds,
        Err(message) => {
            let sanitized = sanitize_error(&message);
            return gateway_error_with_log(
                &state,
                ResponseFormat::Responses,
                &base_log_context,
                GatewayErrorDetails {
                    status: StatusCode::UNAUTHORIZED,
                    error_type: "authentication_error",
                    message: &sanitized,
                    response_body: None,
                },
            )
            .await;
        }
    };
    let upstream_log_context = RequestLogContext {
        upstream: Some(&upstream),
        ..base_log_context.clone()
    };

    let upstream_url = format!("https://q.{}.amazonaws.com/mcp", upstream.region);
    let upstream_resp = match with_kiro_upstream_headers(
        state.http.post(upstream_url),
        &upstream,
        "application/json",
        false,
        false,
        true,
    )
    .json(&payload)
    .send()
    .await
    {
        Ok(resp) => resp,
        Err(error) => {
            let message = sanitize_error(&format!("MCP 上游请求失败: {error}"));
            return gateway_error_with_log(
                &state,
                ResponseFormat::Responses,
                &upstream_log_context,
                GatewayErrorDetails {
                    status: StatusCode::BAD_GATEWAY,
                    error_type: "api_error",
                    message: &message,
                    response_body: None,
                },
            )
            .await;
        }
    };

    let status = upstream_resp.status();
    let content_type = upstream_resp.headers().get(header::CONTENT_TYPE).cloned();
    let body = match upstream_resp.bytes().await {
        Ok(body) => body,
        Err(error) => {
            let message = sanitize_error(&format!("读取 MCP 上游响应失败: {error}"));
            return gateway_error_with_log(
                &state,
                ResponseFormat::Responses,
                &upstream_log_context,
                GatewayErrorDetails {
                    status: StatusCode::BAD_GATEWAY,
                    error_type: "api_error",
                    message: &message,
                    response_body: None,
                },
            )
            .await;
        }
    };

    if !status.is_success() {
        let body_text = String::from_utf8_lossy(&body).to_string();
        let (mapped_status, error_type, message) = map_upstream_error(status, &body_text);
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            &upstream_log_context,
            GatewayErrorDetails {
                status: mapped_status,
                error_type,
                message: &message,
                response_body: Some(body_text.as_str()),
            },
        )
        .await;
    }

    let mut builder = Response::builder().status(status);
    if let Some(value) = content_type {
        builder = builder.header(header::CONTENT_TYPE, value);
    } else {
        builder = builder.header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }

    let logged_response_body = String::from_utf8_lossy(&body).to_string();
    write_request_log(
        &upstream_log_context,
        status,
        "success",
        None,
        Some(logged_response_body.as_str()),
        None, // input_tokens - MCP 代理无 tokens
        None, // output_tokens
        None, // cache_read_input_tokens
        None, // cache_creation_input_tokens
    );
    builder.body(Body::from(body)).unwrap_or_else(|error| {
        gateway_error_response(
            ResponseFormat::Responses,
            StatusCode::INTERNAL_SERVER_ERROR,
            "api_error",
            &format!("构建 MCP 响应失败: {error}"),
        )
    })
}

async fn execute_request_with_server_tools(
    state: &RouterState,
    upstream: &UpstreamCredentials,
    request: &NormalizedRequest,
) -> Result<ProxyExecutionOutcome, (StatusCode, &'static str, String)> {
    let mut working_request = request.clone();
    let web_search_options = request
        .tools
        .as_ref()
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.tool_type.starts_with("web_search_"))
        })
        .and_then(|tool| tool.web_search.clone());
    let _max_uses = server_web_search_iteration_limit(
        web_search_options
            .as_ref()
            .and_then(|options| options.max_uses),
    );
    let mut server_tool_calls = Vec::new();

    // 获取账号可用模型列表（用于模型降级）
    let available_models = match get_available_models_for_upstream(upstream).await {
        Ok(models) => {
            log::debug!(
                "[Gateway] 账号 {} 可用模型: {:?}",
                upstream.source_label,
                models
            );
            Some(models)
        }
        Err(e) => {
            log::warn!(
                "[Gateway] 无法获取账号 {} 的可用模型列表: {}，将不进行模型降级",
                upstream.source_label,
                e
            );
            None
        }
    };

    for _ in 0.._max_uses {
        let upstream_payload = build_kiro_payload(
            &state.http,
            &working_request,
            upstream.profile_arn.clone(),
            available_models.as_deref(),
        )
        .await
        .map_err(|message| {
            (
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                sanitize_error(&message),
            )
        })?;
        let upstream_resp = send_generate_request(&state.http, upstream, &upstream_payload)
            .await
            .map_err(|(status, error_type, message, _)| (status, error_type, message))?;
        let body = upstream_resp.text().await.map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                "api_error",
                sanitize_error(&format!("读取上游响应失败: {error}")),
            )
        })?;
        let aggregated = aggregate_kiro_response(&body);
        let web_search_calls: Vec<(String, String, String)> = aggregated
            .tool_calls
            .iter()
            .filter(|(_, name, _)| name == "web_search")
            .cloned()
            .collect();

        if web_search_calls.is_empty() {
            return Ok(ProxyExecutionOutcome {
                aggregated,
                server_tool_calls,
            });
        }

        working_request
            .messages
            .push(normalized_assistant_message_from_aggregated(&aggregated));

        let mut tool_result_blocks = Vec::new();
        for (id, name, arguments) in web_search_calls {
            let input =
                serde_json::from_str(&arguments).unwrap_or_else(|_| json!({ "query": arguments }));
            let mcp_arguments = build_web_search_mcp_arguments(&input);
            let mcp_result = call_mcp_tool(&state.http, upstream, &name, mcp_arguments).await?;
            let (result_content, tool_result_text) =
                parse_web_search_mcp_result(&mcp_result, web_search_options.as_ref());

            server_tool_calls.push(ServerToolCall {
                id: id.clone(),
                name,
                input: input.clone(),
                result_content: result_content.clone(),
                tool_result_text,
            });
            tool_result_blocks.push(json!({
                "type": "web_search_tool_result",
                "tool_use_id": id,
                "content": result_content
            }));
        }

        working_request.messages.push(NormalizedMessage {
            role: "user".to_string(),
            content: Some(Value::Array(tool_result_blocks)),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        });
    }

    Err((
        StatusCode::BAD_GATEWAY,
        "api_error",
        "web_search 代理循环超过最大轮数".to_string(),
    ))
}

fn server_web_search_iteration_limit(max_uses: Option<i32>) -> usize {
    max_uses
        .unwrap_or(MAX_SERVER_WEB_SEARCH_ITERATIONS as i32)
        .max(0)
        .min(MAX_SERVER_WEB_SEARCH_ITERATIONS as i32) as usize
}

async fn send_generate_request<T: serde::Serialize + ?Sized>(
    http: &Client,
    upstream: &UpstreamCredentials,
    upstream_payload: &T,
) -> Result<reqwest::Response, UpstreamRequestError> {
    let upstream_url = format!(
        "https://q.{}.amazonaws.com/generateAssistantResponse",
        upstream.region
    );

    const MAX_RETRIES: u32 = 3;
    let mut attempt = 0;

    loop {
        attempt += 1;

        let upstream_resp = with_kiro_upstream_headers(
            http.post(&upstream_url),
            upstream,
            "application/vnd.amazon.eventstream",
            true,
            true,
            false,
        )
        .json(upstream_payload)
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                "api_error",
                sanitize_error(&format!("上游请求失败: {error}")),
                None,
            )
        })?;

        let status = upstream_resp.status();

        if status.is_success() {
            return Ok(upstream_resp);
        }

        let body = upstream_resp.text().await.unwrap_or_default();

        // 429 错误不重试，直接返回（让外层切换账号）
        if status == StatusCode::TOO_MANY_REQUESTS {
            let (mapped_status, error_type, message) = map_upstream_error(status, &body);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 其他错误（403、5xx）才重试
        let should_retry =
            attempt < MAX_RETRIES && (status == StatusCode::FORBIDDEN || status.is_server_error());

        if should_retry {
            let backoff_ms = 1000 * 2u64.pow(attempt - 1);
            log::warn!(
                "上游请求失败 (状态: {}, 尝试: {}/{}), {}ms 后重试",
                status,
                attempt,
                MAX_RETRIES,
                backoff_ms
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            continue;
        }

        let (mapped_status, error_type, message) = map_upstream_error(status, &body);
        return Err((mapped_status, error_type, message, Some(body)));
    }
}

fn with_kiro_upstream_headers(
    builder: reqwest::RequestBuilder,
    upstream: &UpstreamCredentials,
    accept: &str,
    include_opt_out: bool,
    include_agent_mode: bool,
    include_profile_arn_header: bool,
) -> reqwest::RequestBuilder {
    let invocation_id = uuid::Uuid::new_v4().to_string();

    let mut builder = builder
        .header("Authorization", format!("Bearer {}", upstream.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("host", format!("q.{}.amazonaws.com", upstream.region))
        .header(header::USER_AGENT, upstream.user_agent.clone())
        .header("x-amz-user-agent", upstream.user_agent.clone())
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", "attempt=1; max=3");

    if include_opt_out && upstream.send_opt_out {
        builder = builder.header("x-amzn-codewhisperer-optout", "true");
    }
    if include_agent_mode {
        builder = builder.header("x-amzn-kiro-agent-mode", DEFAULT_AGENT_MODE);
    }
    if include_profile_arn_header {
        if let Some(profile_arn) = upstream
            .profile_arn
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            builder = builder.header("x-amzn-kiro-profile-arn", profile_arn);
        }
    }
    if should_add_redirect_for_internal(upstream.provider.as_deref()) {
        builder = builder.header("redirect-for-internal", "true");
    }

    builder
}

async fn call_mcp_tool(
    http: &Client,
    upstream: &UpstreamCredentials,
    tool_name: &str,
    arguments: Value,
) -> Result<Value, (StatusCode, &'static str, String)> {
    let upstream_url = format!("https://q.{}.amazonaws.com/mcp", upstream.region);
    let payload = json!({
        "jsonrpc": "2.0",
        "id": short_uuid(),
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });

    let response = with_kiro_upstream_headers(
        http.post(upstream_url),
        upstream,
        "application/json",
        false,
        false,
        true,
    )
    .json(&payload)
    .send()
    .await
    .map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            "api_error",
            sanitize_error(&format!("MCP 上游请求失败: {error}")),
        )
    })?;

    let status = response.status();
    let body = response.text().await.map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            "api_error",
            sanitize_error(&format!("读取 MCP 上游响应失败: {error}")),
        )
    })?;
    if !status.is_success() {
        let (mapped_status, error_type, message) = map_upstream_error(status, &body);
        return Err((mapped_status, error_type, message));
    }

    let value: Value = serde_json::from_str(&body)
        .unwrap_or_else(|_| json!({ "result": { "content": [{ "type": "text", "text": body }] } }));
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("MCP 工具调用失败")
            .to_string();
        return Err((
            StatusCode::BAD_GATEWAY,
            "api_error",
            sanitize_error(&message),
        ));
    }

    Ok(value.get("result").cloned().unwrap_or(value))
}

fn has_server_web_search_tool(request: &NormalizedRequest) -> bool {
    request
        .tools
        .as_ref()
        .map(|tools| {
            tools
                .iter()
                .any(|tool| tool.tool_type.starts_with("web_search_"))
        })
        .unwrap_or(false)
}

fn normalized_assistant_message_from_aggregated(
    aggregated: &stream::AggregatedKiroResponse,
) -> NormalizedMessage {
    NormalizedMessage {
        role: "assistant".to_string(),
        content: if aggregated.text.is_empty() {
            None
        } else {
            Some(Value::String(aggregated.text.clone()))
        },
        tool_calls: if aggregated.tool_calls.is_empty() {
            None
        } else {
            Some(
                aggregated
                    .tool_calls
                    .iter()
                    .map(|(id, name, arguments)| ToolCall {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        function: ToolCallFunction {
                            name: name.clone(),
                            arguments: arguments.clone(),
                        },
                    })
                    .collect(),
            )
        },
        tool_call_id: None,
        metadata: if aggregated.thinking.is_empty() {
            None
        } else {
            Some(json!({
                "reasoningContent": {
                    "reasoningText": {
                        "text": aggregated.thinking
                    }
                }
            }))
        },
    }
}

fn build_web_search_mcp_arguments(input: &Value) -> Value {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .or_else(|| input.get("search_query").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    json!({ "query": query })
}

fn parse_web_search_mcp_result(
    result: &Value,
    options: Option<&WebSearchToolOptions>,
) -> (Value, String) {
    let text = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                if item.get("type").and_then(Value::as_str) == Some("text") {
                    item.get("text").and_then(Value::as_str).map(str::to_string)
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| result.to_string());

    let filtered_results = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| value.get("results").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|item| domain_matches_filters(item, options))
        .map(normalize_anthropic_web_search_result)
        .collect::<Vec<_>>();

    let tool_result_text = if filtered_results.is_empty() {
        text
    } else {
        json!({ "results": filtered_results.clone() }).to_string()
    };

    (Value::Array(filtered_results), tool_result_text)
}

fn domain_matches_filters(item: &Value, options: Option<&WebSearchToolOptions>) -> bool {
    let Some(options) = options else {
        return true;
    };
    let domain = item
        .get("url")
        .and_then(Value::as_str)
        .and_then(extract_domain_from_url);
    let Some(domain) = domain else {
        return true;
    };

    if let Some(allowed) = options
        .allowed_domains
        .as_ref()
        .filter(|items| !items.is_empty())
    {
        if !allowed
            .iter()
            .any(|entry| domain_matches_rule(&domain, entry))
        {
            return false;
        }
    }
    if let Some(blocked) = options.blocked_domains.as_ref() {
        if blocked
            .iter()
            .any(|entry| domain_matches_rule(&domain, entry))
        {
            return false;
        }
    }
    true
}

fn extract_domain_from_url(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()?
        .host_str()
        .map(|host| host.trim_start_matches("www.").to_ascii_lowercase())
}

fn domain_matches_rule(domain: &str, rule: &str) -> bool {
    let normalized = rule.trim().trim_start_matches("www.").to_ascii_lowercase();
    domain == normalized || domain.ends_with(&format!(".{normalized}"))
}

fn normalize_anthropic_web_search_result(item: Value) -> Value {
    match item {
        Value::Object(mut map) => {
            map.insert(
                "type".to_string(),
                Value::String("web_search_result".to_string()),
            );
            Value::Object(map)
        }
        other => other,
    }
}

fn ip_matches_allowlist(ip: IpAddr, allowlist: &[String]) -> bool {
    allowlist.iter().any(|entry| {
        let entry = entry.trim();
        entry
            .parse::<IpAddr>()
            .map(|allowed| allowed == ip)
            .unwrap_or(false)
            || entry
                .parse::<ipnet::IpNet>()
                .map(|network| network.contains(&ip))
                .unwrap_or(false)
    })
}

fn normalize_request(format: ResponseFormat, payload: &Value) -> Result<NormalizedRequest, String> {
    match format {
        ResponseFormat::Anthropic => {
            let request: AnthropicMessagesRequest = serde_json::from_value(payload.clone())
                .map_err(|error| format!("Anthropic 请求解析失败: {error}"))?;
            Ok(normalize_anthropic_request(&request))
        }
        ResponseFormat::Responses => normalize_responses_request(payload),
        ResponseFormat::OpenAI => {
            let request: OpenAIChatRequest = serde_json::from_value(payload.clone())
                .map_err(|error| format!("OpenAI 请求解析失败: {error}"))?;
            Ok(crate::gateway::converter::normalize_openai_chat_request(
                &request,
            ))
        }
    }
}

fn verify_client_auth(headers: &HeaderMap, config: &GatewayConfig) -> Result<(), String> {
    let expected_keys = effective_client_api_keys(config);
    if expected_keys.is_empty() {
        return Err("客户端 API Key 未配置".to_string());
    }

    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());

    if expected_keys.iter().any(|expected| {
        authorization == Some(expected.as_str()) || api_key == Some(expected.as_str())
    }) {
        Ok(())
    } else {
        Err("客户端 API Key 无效".to_string())
    }
}

async fn resolve_upstream_credentials(
    config: &GatewayConfig,
    state: &RouterState,
) -> Result<UpstreamCredentials, String> {
    match config.account_mode.as_str() {
        "single" | "group" | "pool" => resolve_managed_account_credentials(config, state).await,
        "local" => Err("反代不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string()),
        _ => Err("accountMode 必须是 single/group/pool".to_string()),
    }
}

async fn resolve_managed_account_credentials(
    config: &GatewayConfig,
    state: &RouterState,
) -> Result<UpstreamCredentials, String> {
    let mut store = AccountStore::new();
    store.reload();

    // 自愈机制：检查是否所有账号都因 "TooManyFailures" 被禁用
    let all_disabled_by_failures = match config.account_mode.as_str() {
        "single" => store
            .accounts
            .iter()
            .filter(|account| config.account_id.as_deref() == Some(account.id.as_str()))
            .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures")),
        "group" => {
            let group_accounts: Vec<_> = store
                .accounts
                .iter()
                .filter(|account| config.group_id.as_deref() == account.group_id.as_deref())
                .collect();

            !group_accounts.is_empty()
                && group_accounts
                    .iter()
                    .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures"))
        }
        "pool" => {
            let pool_accounts: Vec<_> = store
                .accounts
                .iter()
                .filter(|account| account.is_available())
                .collect();

            !pool_accounts.is_empty()
                && pool_accounts
                    .iter()
                    .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures"))
        }
        _ => false,
    };

    if all_disabled_by_failures {
        eprintln!("[Gateway] 所有账号均已被自动禁用，执行自愈机制重置失败计数");
        for account in store.accounts.iter_mut() {
            if account.disabled_reason.as_deref() == Some("TooManyFailures") {
                account.failure_count = 0;
                account.status = "active".to_string();
                account.disabled_reason = None;
            }
        }
        let _ = store.save_to_file();
    }

    let accounts = match config.account_mode.as_str() {
        "single" => store
            .accounts
            .iter()
            .filter(|account| config.account_id.as_deref() == Some(account.id.as_str()))
            .cloned()
            .collect::<Vec<_>>(),
        "group" => store
            .accounts
            .iter()
            .filter(|account| {
                config.group_id.as_deref() == account.group_id.as_deref() && account.is_available()
            })
            .cloned()
            .collect::<Vec<_>>(),
        "pool" => store
            .accounts
            .iter()
            .filter(|account| account.is_available())
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    if accounts.is_empty() {
        return Err("未找到符合反代配置的可用账号".to_string());
    }

    // 使用 LoadBalancer 选择账号
    let selected_account = state.load_balancer.select_account(&accounts).await;

    let Some(account) = selected_account else {
        return Err("LoadBalancer 未能选择可用账号".to_string());
    };

    // 增加连接计数
    state.load_balancer.increment_connections(&account.id).await;
    let request_start = Instant::now();

    match refresh_token_by_provider(&account).await {
        Ok(refresh) => {
            let provider = account.provider.as_deref().unwrap_or("Google").to_string();
            let usage_result = get_usage_by_provider(&provider, &refresh.access_token).await;
            let mut usage_data = None;
            let mut is_banned = false;
            let mut is_auth_error = false;

            if let Ok(usage) = usage_result {
                usage_data = Some(usage.usage_data);
                is_banned = usage.is_banned;
                is_auth_error = usage.is_auth_error;
            }

            // 失败追踪：如果账号被封禁或认证失败，累加失败计数
            let should_increment_failure = is_banned || is_auth_error;

            persist_account_refresh(
                &account,
                &refresh,
                usage_data.clone(),
                is_banned,
                is_auth_error,
                should_increment_failure,
            );

            // 减少连接计数
            state.load_balancer.decrement_connections(&account.id).await;

            if is_banned || is_auth_error {
                // 记录失败
                state.load_balancer.record_failure(&account.id).await;
                return Err(format!("账号 {} 已不可用", account.label));
            }

            if let Some(usage_data) = &usage_data {
                if usage_exceeds_threshold(usage_data, config.threshold) {
                    return Err(format!(
                        "账号 {} 已达到阈值 {}%",
                        account.label, config.threshold
                    ));
                }
            }

            // 记录成功
            let response_time_ms = request_start.elapsed().as_millis() as u64;
            state
                .load_balancer
                .record_success(&account.id, response_time_ms)
                .await;

            let machine_id = account
                .machine_id
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(get_machine_id);
            let profile_arn = refresh.profile_arn.or_else(|| account.profile_arn.clone());
            let region = resolve_kiro_upstream_region(
                profile_arn.as_deref(),
                account.region.as_deref(),
                &config.region,
            );

            Ok(UpstreamCredentials {
                access_token: refresh.access_token,
                profile_arn,
                provider: account.provider.clone(),
                region,
                source_label: format_managed_upstream_source(config, &account),
                user_agent: build_kiro_custom_user_agent(&machine_id),
                auth_method: account.auth_method.clone(),
                send_opt_out: should_send_codewhisperer_optout(),
            })
        }
        Err(error) => {
            // 减少连接计数
            state.load_balancer.decrement_connections(&account.id).await;
            // 记录失败
            state.load_balancer.record_failure(&account.id).await;

            Err(format!(
                "刷新账号 {} 失败: {}",
                account.label,
                sanitize_error(&error)
            ))
        }
    }
}

fn format_managed_upstream_source(config: &GatewayConfig, account: &Account) -> String {
    let account_label = if !account.label.trim().is_empty() {
        account.label.trim().to_string()
    } else if let Some(email) = account
        .email
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        email.trim().to_string()
    } else if let Some(user_id) = account
        .user_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        user_id.trim().to_string()
    } else {
        account.id.clone()
    };

    match config.account_mode.as_str() {
        "single" => format!("single:{account_label}"),
        "group" => format!(
            "group:{}:{account_label}",
            config.group_id.as_deref().unwrap_or("unknown")
        ),
        "pool" => format!("pool:{account_label}"),
        _ => account_label,
    }
}

fn persist_account_refresh(
    account: &Account,
    refresh: &RefreshResult,
    usage_data: Option<Value>,
    is_banned: bool,
    is_auth_error: bool,
    should_increment_failure: bool,
) {
    let mut store = AccountStore::new();
    if let Some(target) = store
        .accounts
        .iter_mut()
        .find(|candidate| candidate.id == account.id)
    {
        target.access_token = Some(refresh.access_token.clone());
        target.refresh_token = refresh.refresh_token.clone();
        target.expires_at = Some(calc_expires_at(refresh.expires_in));
        if let Some(profile_arn) = refresh.profile_arn.clone() {
            target.profile_arn = Some(profile_arn);
        }
        target.id_token = refresh.id_token.clone();
        target.sso_session_id = refresh.sso_session_id.clone();
        if let Some(data) = usage_data {
            target.usage_data = Some(data);
        }
        target.status = calc_status(is_banned, is_auth_error);

        // 失败追踪逻辑
        if should_increment_failure {
            target.failure_count += 1;
            target.last_failure_at = Some(Local::now().to_rfc3339());

            // 如果失败次数达到阈值，自动禁用账号
            if target.failure_count >= MAX_FAILURES_PER_ACCOUNT {
                target.status = "disabled".to_string();
                target.disabled_reason = Some("TooManyFailures".to_string());
                eprintln!(
                    "[Gateway] 账号 {} 失败次数达到 {}，自动禁用",
                    target.label, MAX_FAILURES_PER_ACCOUNT
                );
            }
        } else {
            // 请求成功，重置失败计数并累加成功计数
            target.failure_count = 0;
            target.success_count += 1;
            target.last_failure_at = None;

            // 如果之前因为失败过多被禁用，现在恢复
            if target.disabled_reason.as_deref() == Some("TooManyFailures") {
                target.disabled_reason = None;
                if target.status == "disabled" {
                    target.status = "active".to_string();
                }
            }
        }

        let _ = store.save_to_file();
    }
}

fn usage_exceeds_threshold(usage_data: &Value, threshold: i32) -> bool {
    let Some((current, limit)) = extract_usage_totals(usage_data) else {
        return false;
    };
    if limit <= 0 {
        return false;
    }
    (current as f64 / limit as f64) * 100.0 >= f64::from(threshold)
}

fn extract_usage_totals(usage_data: &Value) -> Option<(i64, i64)> {
    let breakdown = usage_data
        .get("usageBreakdownList")
        .and_then(Value::as_array)?
        .first()?;
    let current = breakdown
        .get("currentUsage")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let limit = breakdown
        .get("usageLimit")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Some((current, limit))
}

fn extract_plain_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        item.get("content")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::Object(map)) => map
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| map.get("content").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn slice_text_by_char_range(text: &str, start: usize, end: usize) -> Option<String> {
    if end < start {
        return None;
    }

    let chars: Vec<char> = text.chars().collect();
    if start > chars.len() || end > chars.len() {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

fn infer_citation_text(citation: &stream::AggregatedCitation, message_text: &str) -> String {
    if let Some(text) = citation.text.as_ref() {
        return text.clone();
    }

    citation
        .target
        .get("range")
        .and_then(|range| {
            let start = range.get("start").and_then(Value::as_u64)? as usize;
            let end = range.get("end").and_then(Value::as_u64)? as usize;
            slice_text_by_char_range(message_text, start, end)
        })
        .unwrap_or_default()
}

fn extract_anthropic_citation_bounds(
    citation: &stream::AggregatedCitation,
    message_text: &str,
) -> Option<(usize, usize)> {
    if let Some(range) = citation.target.get("range") {
        let start = range.get("start").and_then(Value::as_u64)? as usize;
        let end = range.get("end").and_then(Value::as_u64)? as usize;
        if end < start {
            return None;
        }
        return Some((start, end));
    }

    let start = citation.target.get("location").and_then(Value::as_u64)? as usize;
    let cited_text = infer_citation_text(citation, message_text);
    Some((start, start + cited_text.chars().count()))
}

fn build_anthropic_text_citation(
    citation: &stream::AggregatedCitation,
    message_text: &str,
) -> Option<Value> {
    let (start_char_index, end_char_index) =
        extract_anthropic_citation_bounds(citation, message_text)?;
    let cited_text = infer_citation_text(citation, message_text);

    Some(json!({
        "type": "char_location",
        "cited_text": cited_text,
        "document_index": 0,
        "document_title": citation.link,
        "start_char_index": start_char_index,
        "end_char_index": end_char_index,
        "file_id": Value::Null
    }))
}

fn build_anthropic_text_citations(
    citations: &[stream::AggregatedCitation],
    message_text: &str,
) -> Option<Value> {
    let mapped: Vec<Value> = citations
        .iter()
        .filter_map(|citation| build_anthropic_text_citation(citation, message_text))
        .collect();

    if mapped.is_empty() {
        None
    } else {
        Some(Value::Array(mapped))
    }
}

fn build_anthropic_citation_delta_event(
    index: usize,
    citation: &stream::AggregatedCitation,
    message_text: &str,
) -> Option<Value> {
    Some(json!({
        "type": "content_block_delta",
        "index": index,
        "delta": {
            "type": "citations_delta",
            "citation": build_anthropic_text_citation(citation, message_text)?
        }
    }))
}

fn build_anthropic_content_blocks(
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
) -> Vec<AnthropicContentBlock> {
    let mut content = Vec::new();
    if !aggregated.thinking.is_empty() {
        content.push(AnthropicContentBlock {
            block_type: "thinking".to_string(),
            text: None,
            thinking: Some(aggregated.thinking.clone()),
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
            citations: None,
        });
    }
    for call in server_tool_calls {
        content.push(AnthropicContentBlock {
            block_type: "server_tool_use".to_string(),
            text: None,
            thinking: None,
            id: Some(call.id.clone()),
            name: Some(call.name.clone()),
            input: Some(call.input.clone()),
            tool_use_id: None,
            content: None,
            citations: None,
        });
        content.push(AnthropicContentBlock {
            block_type: "web_search_tool_result".to_string(),
            text: None,
            thinking: None,
            id: None,
            name: None,
            input: None,
            tool_use_id: Some(call.id.clone()),
            content: Some(call.result_content.clone()),
            citations: None,
        });
    }
    if !aggregated.text.is_empty() {
        content.push(AnthropicContentBlock {
            block_type: "text".to_string(),
            text: Some(aggregated.text.clone()),
            thinking: None,
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
            citations: build_anthropic_text_citations(&aggregated.citations, &aggregated.text),
        });
    }
    for (id, name, arguments) in &aggregated.tool_calls {
        content.push(AnthropicContentBlock {
            block_type: "tool_use".to_string(),
            text: None,
            thinking: None,
            id: Some(id.clone()),
            name: Some(name.clone()),
            input: Some(serde_json::from_str(arguments).unwrap_or_else(|_| json!({}))),
            tool_use_id: None,
            content: None,
            citations: None,
        });
    }
    if content.is_empty() {
        content.push(AnthropicContentBlock {
            block_type: "text".to_string(),
            text: Some(EMPTY_ANTHROPIC_RESPONSE_FALLBACK.to_string()),
            thinking: None,
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
            citations: None,
        });
    }
    content
}

fn build_anthropic_response(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
) -> Value {
    let content = build_anthropic_content_blocks(aggregated, server_tool_calls);
    serde_json::to_value(AnthropicMessagesResponse {
        id: format!("msg_{}", short_uuid()),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content,
        model: model.to_string(),
        stop_reason: Some(if aggregated.tool_calls.is_empty() {
            "end_turn".to_string()
        } else {
            "tool_use".to_string()
        }),
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens: aggregated.input_tokens,
            output_tokens: aggregated.output_tokens,
        },
    })
    .unwrap_or_else(|_| json!({}))
}

fn build_responses_citation_annotations(citations: &[stream::AggregatedCitation]) -> Vec<Value> {
    citations
        .iter()
        .map(|citation| {
            let mut value = json!({
                "type": "url_citation",
                "url": citation.link,
                "target": citation.target,
                "citationLink": citation.link
            });
            if let Some(range) = citation.target.get("range") {
                if let Some(start_index) = range.get("start").and_then(Value::as_u64) {
                    value["start_index"] = Value::from(start_index);
                }
                if let Some(end_index) = range.get("end").and_then(Value::as_u64) {
                    value["end_index"] = Value::from(end_index);
                }
            }
            if let Some(text) = citation.text.as_ref() {
                value["citationText"] = Value::String(text.clone());
            }
            value
        })
        .collect()
}

fn build_responses_annotation_added_event(
    response_id: &str,
    message_id: &str,
    annotation: Value,
    annotation_index: usize,
    sequence_number: usize,
) -> Value {
    json!({
        "type": "response.output_text.annotation.added",
        "response_id": response_id,
        "item_id": message_id,
        "output_index": 0,
        "content_index": 0,
        "annotation_index": annotation_index,
        "annotation": annotation,
        "sequence_number": sequence_number
    })
}

fn extract_web_search_sources(server_tool_calls: &[ServerToolCall]) -> Vec<WebSearchSource> {
    let mut seen = HashSet::new();
    let mut sources = Vec::new();

    for call in server_tool_calls
        .iter()
        .filter(|call| call.name == "web_search")
    {
        let Some(results) = call.result_content.as_array() else {
            continue;
        };

        for item in results {
            let Some(url) = item
                .get("url")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            if !seen.insert(url.to_string()) {
                continue;
            }

            let title = item
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(url)
                .to_string();

            sources.push(WebSearchSource {
                title,
                url: url.to_string(),
            });
        }
    }

    sources
}

fn build_responses_output_text(
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
) -> ResponsesOutputText {
    let mut text = aggregated.text.clone();
    let mut annotations = build_responses_citation_annotations(&aggregated.citations);
    let sources = extract_web_search_sources(server_tool_calls);

    if !sources.is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str("Sources:\n");

        for (index, source) in sources.iter().enumerate() {
            let prefix = format!("[{}] ", index + 1);
            let start_index = text.chars().count() + prefix.chars().count();
            text.push_str(&prefix);
            text.push_str(&source.title);
            let end_index = start_index + source.title.chars().count();
            annotations.push(json!({
                "type": "url_citation",
                "start_index": start_index,
                "end_index": end_index,
                "url": source.url,
                "title": source.title
            }));
            text.push('\n');
        }

        text.pop();
    }

    ResponsesOutputText { text, annotations }
}

fn build_responses_web_search_call(call: &ServerToolCall) -> Value {
    let mut action = json!({
        "type": "search"
    });
    if let Some(query) = call
        .input
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        action["query"] = Value::String(query.to_string());
    }

    let sources = extract_web_search_sources(std::slice::from_ref(call));
    if !sources.is_empty() {
        action["sources"] = Value::Array(
            sources
                .into_iter()
                .map(|source| {
                    json!({
                        "type": "source",
                        "url": source.url,
                        "title": source.title
                    })
                })
                .collect(),
        );
    }

    json!({
        "id": call.id,
        "type": "web_search_call",
        "status": "completed",
        "action": action
    })
}

fn build_responses_message_content(
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
) -> Vec<Value> {
    let output_text = build_responses_output_text(aggregated, server_tool_calls);
    let mut content = Vec::new();
    if !output_text.text.is_empty() {
        content.push(json!({
            "type": "output_text",
            "text": output_text.text,
            "annotations": output_text.annotations
        }));
    }
    if !aggregated.thinking.is_empty() {
        content.push(json!({
            "type": "reasoning",
            "summary": aggregated.thinking
        }));
    }
    for (id, name, arguments) in &aggregated.tool_calls {
        content.push(json!({
            "type": "function_call",
            "call_id": id,
            "name": name,
            "arguments": arguments
        }));
    }
    content
}

#[allow(dead_code)]
fn build_responses_response(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
    previous_response_id: Option<&str>,
) -> Value {
    build_responses_response_with_ids(
        model,
        aggregated,
        server_tool_calls,
        &format!("resp_{}", short_uuid()),
        &format!("msg_{}", short_uuid()),
        chrono::Utc::now().timestamp(),
        previous_response_id,
    )
}

fn build_responses_response_with_ids(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
    response_id: &str,
    message_id: &str,
    created_at: i64,
    previous_response_id: Option<&str>,
) -> Value {
    let output_text = build_responses_output_text(aggregated, server_tool_calls);
    let content = build_responses_message_content(aggregated, server_tool_calls);

    let mut output: Vec<Value> = server_tool_calls
        .iter()
        .filter(|call| call.name == "web_search")
        .map(build_responses_web_search_call)
        .collect();
    output.push(json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "content": content
    }));

    json!({
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "status": "completed",
        "model": model,
        "previous_response_id": previous_response_id,
        "output": output,
        "output_text": output_text.text,
        "usage": {
            "input_tokens": aggregated.input_tokens,
            "output_tokens": aggregated.output_tokens,
            "total_tokens": aggregated.input_tokens + aggregated.output_tokens
        }
    })
}

fn build_stream_responses_completed_event(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
    response_id: &str,
    message_id: &str,
    created_at: i64,
    previous_response_id: Option<&str>,
) -> Value {
    json!({
        "type": "response.completed",
        "response": build_responses_response_with_ids(
            model,
            aggregated,
            server_tool_calls,
            response_id,
            message_id,
            created_at,
            previous_response_id,
        )
    })
}

fn build_stream_responses_function_call_arguments_done_event(
    response_id: &str,
    call_id: &str,
    arguments: &str,
) -> Value {
    json!({
        "type": "response.function_call_arguments.done",
        "response_id": response_id,
        "call_id": call_id,
        "arguments": arguments
    })
}

fn build_stream_responses_output_text_done_event(response_id: &str, text: &str) -> Value {
    json!({
        "type": "response.output_text.done",
        "response_id": response_id,
        "text": text
    })
}

fn build_stream_responses_reasoning_done_event(response_id: &str, text: &str) -> Value {
    json!({
        "type": "response.reasoning.done",
        "response_id": response_id,
        "text": text
    })
}

fn gateway_error_response(
    format: ResponseFormat,
    status: StatusCode,
    error_type: &str,
    message: &str,
) -> Response {
    let body = build_gateway_error_body(format, status, error_type, message);
    (status, Json(body)).into_response()
}

fn map_upstream_error(status: StatusCode, body: &str) -> (StatusCode, &'static str, String) {
    let sanitized = sanitize_error(&extract_error_message(body));
    let explicit_error_type = extract_error_type(body);
    let text = body.to_lowercase();
    let mapped_status = if status == StatusCode::BAD_GATEWAY || status == StatusCode::OK {
        if explicit_error_type == Some("authentication_error") {
            StatusCode::UNAUTHORIZED
        } else if explicit_error_type == Some("permission_error") {
            StatusCode::FORBIDDEN
        } else if explicit_error_type == Some("rate_limit_error") {
            StatusCode::TOO_MANY_REQUESTS
        } else if explicit_error_type == Some("invalid_request_error") {
            StatusCode::BAD_REQUEST
        } else if text.contains("throttlingexception")
            || text.contains("servicequotaexceededexception")
        {
            StatusCode::TOO_MANY_REQUESTS
        } else if text.contains("accessdeniedexception") {
            StatusCode::FORBIDDEN
        } else if text.contains("validationexception") {
            StatusCode::BAD_REQUEST
        } else if text.contains("serviceunavailableexception") {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::BAD_GATEWAY
        }
    } else {
        status
    };

    let error_type = explicit_error_type.unwrap_or(match mapped_status {
        StatusCode::UNAUTHORIZED => "authentication_error",
        StatusCode::FORBIDDEN => "permission_error",
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND | StatusCode::CONFLICT => {
            "invalid_request_error"
        }
        _ => "api_error",
    });

    (mapped_status, error_type, sanitized)
}

fn extract_error_type(body: &str) -> Option<&'static str> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let raw = value
        .pointer("/error/type")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/type").and_then(Value::as_str))?;

    match raw {
        "authentication_error" => Some("authentication_error"),
        "permission_error" => Some("permission_error"),
        "rate_limit_error" => Some("rate_limit_error"),
        "invalid_request_error" => Some("invalid_request_error"),
        "api_error" => Some("api_error"),
        _ => None,
    }
}

fn detect_upstream_error_body(body: &str) -> Option<(StatusCode, &'static str, String)> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    let value = serde_json::from_str::<Value>(trimmed).ok()?;
    let object = value.as_object()?;
    let has_error_container = object.get("error").is_some();
    let has_error_metadata = object.get("__type").and_then(Value::as_str).is_some()
        || object.get("errorCode").and_then(Value::as_str).is_some()
        || object.get("Message").and_then(Value::as_str).is_some();
    let has_message_only_error = object.get("message").and_then(Value::as_str).is_some()
        && object.get("content").is_none()
        && object.get("output").is_none()
        && object.get("choices").is_none()
        && object.get("results").is_none();

    if has_error_container || has_error_metadata || has_message_only_error {
        Some(map_upstream_error(StatusCode::OK, trimmed))
    } else {
        None
    }
}

fn extract_error_message(body: &str) -> String {
    if body.trim().is_empty() {
        return "上游返回空错误响应".to_string();
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        for pointer in [
            "/message",
            "/Message",
            "/error/message",
            "/reason",
            "/__type",
            "/errorCode",
        ] {
            if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
                return text.to_string();
            }
        }
    }
    body.to_string()
}

fn sanitize_error(message: &str) -> String {
    let mut sanitized = message.to_string();
    for pattern in [
        r"Bearer\s+[A-Za-z0-9._\-]+",
        r#""accessToken"\s*:\s*"[^"]+""#,
        r#""refreshToken"\s*:\s*"[^"]+""#,
        r#""clientSecret"\s*:\s*"[^"]+""#,
        r#"sk-[A-Za-z0-9]+"#,
    ] {
        if let Ok(regex) = Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, "[REDACTED]").to_string();
        }
    }
    sanitized
}

fn short_uuid() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

fn extract_account_id_from_upstream(upstream: &UpstreamCredentials) -> String {
    // 从 source_label 提取账号标识
    // 格式：single:email@example.com 或 group:group_name:email@example.com
    upstream
        .source_label
        .split(':')
        .last()
        .unwrap_or(&upstream.source_label)
        .to_string()
}

fn stream_proxy_response(
    state: RouterState,
    upstream_resp: reqwest::Response,
    format: ResponseFormat,
    model: String,
    request_messages: Vec<NormalizedMessage>,
    request_tools: Option<Vec<Tool>>,
    request_tool_choice: Option<Value>,
    previous_response_id: Option<String>,
    server_tool_calls: Vec<ServerToolCall>,
    log_context: RequestLogContext<'static>,
) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(2048);
    let estimated_input_tokens =
        estimate_request_tokens_with_tools(&request_messages, &request_tools, &model)
            .min(i32::MAX as usize) as i32;
    tokio::spawn(async move {
        let mut upstream_stream = upstream_resp.bytes_stream();
        let mut raw_buffer = Vec::new();
        let mut parser = ThinkingParser::new();
        let mut aggregated = stream::AggregatedKiroResponse::default();
        aggregated.input_tokens = estimated_input_tokens;
        let mut tool_accumulators: HashMap<String, (String, String)> = HashMap::new();
        let mut message_started = false;
        let mut next_block_index = 0usize;
        let mut text_block_index: Option<usize> = None;
        let mut thinking_block_index: Option<usize> = None;
        let mut tool_block_indexes: HashMap<String, usize> = HashMap::new();
        let mut saw_tool_calls = false;
        let mut input_tokens = estimated_input_tokens;
        let mut output_tokens = 0i32;
        let anthropic_id = format!("msg_{}", short_uuid());
        let response_id = format!("resp_{}", short_uuid());
        let message_id = format!("msg_{}", short_uuid());
        let created_at = chrono::Utc::now().timestamp();
        let completion_id = format!("chatcmpl-{}", short_uuid());
        let mut responses_sequence_number = 0usize;
        let mut responses_next_output_index = 1usize;
        let mut responses_tool_output_indexes: HashMap<String, usize> = HashMap::new();

        if matches!(format, ResponseFormat::Responses) {
            let created = json!({
                "type": "response.created",
                "response": {
                    "id": response_id,
                    "object": "response",
                    "created_at": created_at,
                    "status": "in_progress",
                    "model": model,
                    "output": []
                }
            });
            if !send_data(&tx, &created.to_string()).await {
                return;
            }

            let output_item_added = json!({
                "type": "response.output_item.added",
                "response_id": response_id,
                "output_index": 0,
                "item": {
                    "id": message_id,
                    "type": "message",
                    "status": "in_progress",
                    "role": "assistant",
                    "content": []
                }
            });
            if !send_data(&tx, &output_item_added.to_string()).await {
                return;
            }
        } else if matches!(format, ResponseFormat::OpenAI) {
            let completion_id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
            let created = chrono::Utc::now().timestamp();
            let delta = crate::gateway::models::OpenAIChatDelta {
                role: Some("assistant".to_string()),
                content: Some("".to_string()),
                tool_calls: None,
            };
            let chunk =
                stream::build_openai_chunk(&completion_id, created, &model, delta, None, None);
            if let Ok(chunk_json) = serde_json::to_string(&chunk) {
                if !send_data(&tx, &chunk_json).await {
                    return;
                }
            }
        }

        const STALLED_STREAM_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(300);

        loop {
            let chunk_result =
                match tokio::time::timeout(STALLED_STREAM_TIMEOUT, upstream_stream.next()).await {
                    Ok(Some(result)) => result,
                    Ok(None) => break,
                    Err(_) => {
                        log::error!("流式响应超时: 5分钟内未收到数据");
                        let data = json!({
                            "type": "error",
                            "message": "流式响应超时: 5分钟内未收到数据"
                        });
                        send_data(&tx, &data.to_string()).await;
                        break;
                    }
                };

            match chunk_result {
                Ok(bytes) => {
                    // 累积二进制数据
                    raw_buffer.extend_from_slice(&bytes);
                    // 逐个解码 EventStream 消息
                    loop {
                        match decode_message(&raw_buffer) {
                            Ok(Some((msg, consumed_bytes))) => {
                                // 成功解码一个消息
                                let message_type =
                                    msg.headers.get(":message-type").map(String::as_str);
                                let event_type = msg.headers.get(":event-type").map(String::as_str);

                                if matches!(message_type, Some("error") | Some("exception")) {
                                    let error_text = String::from_utf8_lossy(&msg.payload);
                                    log::error!(
                                        "EventStream 上游错误: message_type={:?}, event_type={:?}, payload={}",
                                        message_type,
                                        event_type,
                                        error_text
                                    );
                                    let data = json!({
                                        "type": "error",
                                        "message": sanitize_error(error_text.as_ref())
                                    });
                                    send_data(&tx, &data.to_string()).await;
                                    raw_buffer.drain(..consumed_bytes);
                                    break;
                                }

                                if !matches!(message_type, Some("event")) {
                                    raw_buffer.drain(..consumed_bytes);
                                    continue;
                                }

                                // 将 payload 转换为文本
                                let json_text = String::from_utf8_lossy(&msg.payload);

                                // 解析 JSON 事件
                                if let Some(event) = parse_kiro_event_full(&json_text) {
                                    match event {
                                        KiroEvent::Usage {
                                            input_tokens: input,
                                            output_tokens: output,
                                            cache_read_input_tokens,
                                            cache_creation_input_tokens,
                                        } => {
                                            input_tokens = input;
                                            output_tokens = output;
                                            aggregated.input_tokens = input;
                                            aggregated.output_tokens = output;
                                            aggregated.cache_read_input_tokens =
                                                cache_read_input_tokens;
                                            aggregated.cache_creation_input_tokens =
                                                cache_creation_input_tokens;
                                        }
                                        KiroEvent::ContextUsage { percentage } => {
                                            aggregated.context_usage_percentage = Some(percentage);
                                        }
                                        KiroEvent::Thinking(text) => {
                                            aggregated.thinking.push_str(&text);
                                            handle_stream_text(
                                                &tx,
                                                format,
                                                &model,
                                                &anthropic_id,
                                                &response_id,
                                                &completion_id,
                                                created_at,
                                                &text,
                                                true,
                                                &mut message_started,
                                                &mut next_block_index,
                                                &mut text_block_index,
                                                &mut thinking_block_index,
                                                input_tokens,
                                                output_tokens,
                                            )
                                            .await;
                                        }
                                        KiroEvent::Text(text) => {
                                            aggregated.text.push_str(&text);
                                            for segment in parser.push_and_parse(&text) {
                                                handle_stream_text(
                                                    &tx,
                                                    format,
                                                    &model,
                                                    &anthropic_id,
                                                    &response_id,
                                                    &completion_id,
                                                    created_at,
                                                    &segment.content,
                                                    segment.segment_type == SegmentType::Thinking,
                                                    &mut message_started,
                                                    &mut next_block_index,
                                                    &mut text_block_index,
                                                    &mut thinking_block_index,
                                                    input_tokens,
                                                    output_tokens,
                                                )
                                                .await;
                                            }
                                        }
                                        KiroEvent::ToolUseStart { id, name } => {
                                            saw_tool_calls = true;
                                            let entry = tool_accumulators
                                                .entry(id.clone())
                                                .or_insert((String::new(), String::new()));
                                            if entry.0.is_empty() {
                                                entry.0 = name.clone();
                                            }
                                            match format {
                                                ResponseFormat::Anthropic => {
                                                    if tool_block_indexes.contains_key(&id) {
                                                        raw_buffer.drain(..consumed_bytes);
                                                        continue;
                                                    }
                                                    ensure_anthropic_message_start(
                                                        &tx,
                                                        &mut message_started,
                                                        &anthropic_id,
                                                        &model,
                                                        input_tokens,
                                                        output_tokens,
                                                    )
                                                    .await;
                                                    close_content_block(&tx, &mut text_block_index)
                                                        .await;
                                                    close_content_block(
                                                        &tx,
                                                        &mut thinking_block_index,
                                                    )
                                                    .await;
                                                    let index = next_block_index;
                                                    next_block_index += 1;
                                                    tool_block_indexes.insert(id.clone(), index);
                                                    let data = json!({
                                                        "type": "content_block_start",
                                                        "index": index,
                                                        "content_block": {
                                                            "type": "tool_use",
                                                            "id": id,
                                                            "name": name,
                                                            "input": {}
                                                        }
                                                    });
                                                    send_event(
                                                        &tx,
                                                        Some("content_block_start"),
                                                        &data.to_string(),
                                                    )
                                                    .await;
                                                }
                                                ResponseFormat::Responses => {
                                                    if responses_tool_output_indexes
                                                        .contains_key(&id)
                                                    {
                                                        raw_buffer.drain(..consumed_bytes);
                                                        continue;
                                                    }
                                                    let output_index = responses_next_output_index;
                                                    responses_next_output_index += 1;
                                                    responses_tool_output_indexes
                                                        .insert(id.clone(), output_index);
                                                    let data = json!({
                                                        "type": "response.output_item.added",
                                                        "response_id": response_id,
                                                        "output_index": output_index,
                                                        "item": {
                                                            "id": id,
                                                            "type": "function_call",
                                                            "status": "in_progress",
                                                            "call_id": id,
                                                            "name": name,
                                                            "arguments": ""
                                                        }
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                                ResponseFormat::OpenAI => {
                                                    if responses_tool_output_indexes
                                                        .contains_key(&id)
                                                    {
                                                        raw_buffer.drain(..consumed_bytes);
                                                        continue;
                                                    }
                                                    let output_index = responses_next_output_index;
                                                    responses_next_output_index += 1;
                                                    responses_tool_output_indexes
                                                        .insert(id.clone(), output_index);
                                                    let data = json!({
                                                        "type": "response.output_item.added",
                                                        "response_id": response_id,
                                                        "output_index": output_index,
                                                        "item": {
                                                            "id": id,
                                                            "type": "function_call",
                                                            "status": "in_progress",
                                                            "call_id": id,
                                                            "name": name,
                                                            "arguments": ""
                                                        }
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                            }
                                        }
                                        KiroEvent::ToolUseInputDelta {
                                            id,
                                            name,
                                            input_delta,
                                        } => {
                                            let mut started_from_delta = false;
                                            if let Some((existing_name, current_input)) =
                                                tool_accumulators.get_mut(&id)
                                            {
                                                if existing_name.is_empty() {
                                                    if let Some(name) = name.as_ref() {
                                                        *existing_name = name.clone();
                                                    }
                                                }
                                                current_input.push_str(&input_delta);
                                            } else {
                                                tool_accumulators.insert(
                                                    id.clone(),
                                                    (
                                                        name.clone().unwrap_or_default(),
                                                        input_delta.clone(),
                                                    ),
                                                );
                                                started_from_delta = true;
                                            }
                                            match format {
                                                ResponseFormat::Anthropic => {
                                                    if !tool_block_indexes.contains_key(&id) {
                                                        if let Some(name) = name.as_ref() {
                                                            saw_tool_calls = true;
                                                            ensure_anthropic_message_start(
                                                                &tx,
                                                                &mut message_started,
                                                                &anthropic_id,
                                                                &model,
                                                                input_tokens,
                                                                output_tokens,
                                                            )
                                                            .await;
                                                            close_content_block(
                                                                &tx,
                                                                &mut text_block_index,
                                                            )
                                                            .await;
                                                            close_content_block(
                                                                &tx,
                                                                &mut thinking_block_index,
                                                            )
                                                            .await;
                                                            let index = next_block_index;
                                                            next_block_index += 1;
                                                            tool_block_indexes
                                                                .insert(id.clone(), index);
                                                            let data = json!({
                                                                "type": "content_block_start",
                                                                "index": index,
                                                                "content_block": {
                                                                    "type": "tool_use",
                                                                    "id": id,
                                                                    "name": name,
                                                                    "input": {}
                                                                }
                                                            });
                                                            send_event(
                                                                &tx,
                                                                Some("content_block_start"),
                                                                &data.to_string(),
                                                            )
                                                            .await;
                                                        }
                                                    }
                                                    if let Some(index) =
                                                        tool_block_indexes.get(&id).copied()
                                                    {
                                                        let data = json!({
                                                            "type": "content_block_delta",
                                                            "index": index,
                                                            "delta": {
                                                                "type": "input_json_delta",
                                                                "partial_json": input_delta
                                                            }
                                                        });
                                                        send_event(
                                                            &tx,
                                                            Some("content_block_delta"),
                                                            &data.to_string(),
                                                        )
                                                        .await;
                                                    }
                                                }
                                                ResponseFormat::Responses => {
                                                    if started_from_delta {
                                                        if let Some(name) = name.as_ref() {
                                                            let output_index =
                                                                responses_next_output_index;
                                                            responses_next_output_index += 1;
                                                            responses_tool_output_indexes
                                                                .insert(id.clone(), output_index);
                                                            let data = json!({
                                                                "type": "response.output_item.added",
                                                                "response_id": response_id,
                                                                "output_index": output_index,
                                                                "item": {
                                                                    "id": id,
                                                                    "type": "function_call",
                                                                    "status": "in_progress",
                                                                    "call_id": id,
                                                                    "name": name,
                                                                    "arguments": ""
                                                                }
                                                            });
                                                            send_data(&tx, &data.to_string()).await;
                                                        }
                                                    }
                                                    let data = json!({
                                                        "type": "response.function_call_arguments.delta",
                                                        "response_id": response_id,
                                                        "call_id": id,
                                                        "delta": input_delta
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                                ResponseFormat::OpenAI => {
                                                    if started_from_delta {
                                                        if let Some(name) = name.as_ref() {
                                                            let output_index =
                                                                responses_next_output_index;
                                                            responses_next_output_index += 1;
                                                            responses_tool_output_indexes
                                                                .insert(id.clone(), output_index);
                                                            let data = json!({
                                                                "type": "response.output_item.added",
                                                                "response_id": response_id,
                                                                "output_index": output_index,
                                                                "item": {
                                                                    "id": id,
                                                                    "type": "function_call",
                                                                    "status": "in_progress",
                                                                    "call_id": id,
                                                                    "name": name,
                                                                    "arguments": ""
                                                                }
                                                            });
                                                            send_data(&tx, &data.to_string()).await;
                                                        }
                                                    }
                                                    let data = json!({
                                                        "type": "response.function_call_arguments.delta",
                                                        "response_id": response_id,
                                                        "call_id": id,
                                                        "delta": input_delta
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                            }
                                        }
                                        KiroEvent::ToolUseStop { id } => match format {
                                            ResponseFormat::Anthropic => {
                                                if let Some((name, input)) =
                                                    tool_accumulators.remove(&id)
                                                {
                                                    aggregated.tool_calls.push((
                                                        id.clone(),
                                                        name,
                                                        input,
                                                    ));
                                                }
                                                if let Some(index) = tool_block_indexes.remove(&id)
                                                {
                                                    let data = json!({
                                                        "type": "content_block_stop",
                                                        "index": index
                                                    });
                                                    send_event(
                                                        &tx,
                                                        Some("content_block_stop"),
                                                        &data.to_string(),
                                                    )
                                                    .await;
                                                }
                                            }
                                            ResponseFormat::Responses => {
                                                if let Some((name, input)) =
                                                    tool_accumulators.remove(&id)
                                                {
                                                    aggregated.tool_calls.push((
                                                        id.clone(),
                                                        name.clone(),
                                                        input.clone(),
                                                    ));
                                                    let done = build_stream_responses_function_call_arguments_done_event(
                                                        &response_id,
                                                        &id,
                                                        &input,
                                                    );
                                                    send_data(&tx, &done.to_string()).await;
                                                    let output_index =
                                                        responses_tool_output_indexes
                                                            .remove(&id)
                                                            .unwrap_or_else(|| {
                                                                let idx =
                                                                    responses_next_output_index;
                                                                responses_next_output_index += 1;
                                                                idx
                                                            });
                                                    let data = json!({
                                                        "type": "response.output_item.done",
                                                        "response_id": response_id,
                                                        "output_index": output_index,
                                                        "item": {
                                                            "id": id,
                                                            "type": "function_call",
                                                            "status": "completed",
                                                            "call_id": id,
                                                            "name": name,
                                                            "arguments": input
                                                        }
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                            }
                                            ResponseFormat::OpenAI => {
                                                if let Some((name, input)) =
                                                    tool_accumulators.remove(&id)
                                                {
                                                    aggregated.tool_calls.push((
                                                        id.clone(),
                                                        name.clone(),
                                                        input.clone(),
                                                    ));
                                                    let done = build_stream_responses_function_call_arguments_done_event(
                                                        &response_id,
                                                        &id,
                                                        &input,
                                                    );
                                                    send_data(&tx, &done.to_string()).await;
                                                    let output_index =
                                                        responses_tool_output_indexes
                                                            .remove(&id)
                                                            .unwrap_or_else(|| {
                                                                let idx =
                                                                    responses_next_output_index;
                                                                responses_next_output_index += 1;
                                                                idx
                                                            });
                                                    let data = json!({
                                                        "type": "response.output_item.done",
                                                        "response_id": response_id,
                                                        "output_index": output_index,
                                                        "item": {
                                                            "id": id,
                                                            "type": "function_call",
                                                            "status": "completed",
                                                            "call_id": id,
                                                            "name": name,
                                                            "arguments": input
                                                        }
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                            }
                                        },
                                        KiroEvent::Citation { text, link, target } => {
                                            let citation =
                                                stream::AggregatedCitation { text, link, target };
                                            aggregated.citations.push(citation.clone());

                                            match format {
                                                ResponseFormat::Anthropic => {
                                                    ensure_anthropic_message_start(
                                                        &tx,
                                                        &mut message_started,
                                                        &anthropic_id,
                                                        &model,
                                                        input_tokens,
                                                        output_tokens,
                                                    )
                                                    .await;
                                                    close_content_block(
                                                        &tx,
                                                        &mut thinking_block_index,
                                                    )
                                                    .await;
                                                    if text_block_index.is_none() {
                                                        let index = next_block_index;
                                                        next_block_index += 1;
                                                        text_block_index = Some(index);
                                                        let data = json!({
                                                            "type": "content_block_start",
                                                            "index": index,
                                                            "content_block": {
                                                                "type": "text",
                                                                "text": ""
                                                            }
                                                        });
                                                        send_event(
                                                            &tx,
                                                            Some("content_block_start"),
                                                            &data.to_string(),
                                                        )
                                                        .await;
                                                    }
                                                    if let Some(index) = text_block_index {
                                                        if let Some(data) =
                                                            build_anthropic_citation_delta_event(
                                                                index,
                                                                &citation,
                                                                &aggregated.text,
                                                            )
                                                        {
                                                            send_event(
                                                                &tx,
                                                                Some("content_block_delta"),
                                                                &data.to_string(),
                                                            )
                                                            .await;
                                                        }
                                                    }
                                                }
                                                ResponseFormat::Responses => {
                                                    if let Some(annotation) =
                                                        build_responses_citation_annotations(
                                                            std::slice::from_ref(&citation),
                                                        )
                                                        .into_iter()
                                                        .next()
                                                    {
                                                        let data =
                                                            build_responses_annotation_added_event(
                                                                &response_id,
                                                                &message_id,
                                                                annotation,
                                                                aggregated.citations.len() - 1,
                                                                responses_sequence_number,
                                                            );
                                                        responses_sequence_number += 1;
                                                        send_data(&tx, &data.to_string()).await;
                                                    }
                                                }
                                                ResponseFormat::OpenAI => {
                                                    // OpenAI format - similar to Responses
                                                    if let Some(annotation) =
                                                        build_responses_citation_annotations(
                                                            std::slice::from_ref(&citation),
                                                        )
                                                        .into_iter()
                                                        .next()
                                                    {
                                                        let data =
                                                            build_responses_annotation_added_event(
                                                                &response_id,
                                                                &message_id,
                                                                annotation,
                                                                aggregated.citations.len() - 1,
                                                                responses_sequence_number,
                                                            );
                                                        responses_sequence_number += 1;
                                                        send_data(&tx, &data.to_string()).await;
                                                    }
                                                }
                                            }
                                        }
                                        KiroEvent::Metering { usage, .. } => {
                                            aggregated.metering_usage = Some(usage);
                                        }
                                    }
                                }

                                // 清理已处理的字节
                                raw_buffer.drain(..consumed_bytes);
                            }
                            Ok(None) => {
                                // 缓冲区数据不足，等待更多数据
                                break;
                            }
                            Err(error) => {
                                // 解码失败，记录错误并清空缓冲区
                                log::error!("EventStream 解码失败: {}", error);
                                raw_buffer.clear();
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    log::error!("流式读取错误: {:?}", error);
                    let error_msg = format!("流式读取失败: {error}");
                    log::error!("错误详情: {}", error_msg);
                    let data = json!({"type":"error","message":sanitize_error(&error_msg)});
                    send_data(&tx, &data.to_string()).await;
                    break;
                }
            }
        }

        for segment in parser.flush() {
            handle_stream_text(
                &tx,
                format,
                &model,
                &anthropic_id,
                &response_id,
                &completion_id,
                created_at,
                &segment.content,
                segment.segment_type == SegmentType::Thinking,
                &mut message_started,
                &mut next_block_index,
                &mut text_block_index,
                &mut thinking_block_index,
                input_tokens,
                output_tokens,
            )
            .await;
        }
        for (id, (name, input)) in tool_accumulators.drain() {
            if !name.is_empty() {
                aggregated.tool_calls.push((id, name, input));
            }
        }
        aggregated.tool_calls = stream::deduplicate_tool_calls(aggregated.tool_calls);

        match format {
            ResponseFormat::Anthropic => {
                if next_block_index == 0 {
                    emit_anthropic_empty_response_fallback(
                        &tx,
                        &mut message_started,
                        &anthropic_id,
                        &model,
                        input_tokens,
                        output_tokens,
                        &mut next_block_index,
                    )
                    .await;
                }
                close_content_block(&tx, &mut text_block_index).await;
                close_content_block(&tx, &mut thinking_block_index).await;
                for (_, index) in tool_block_indexes.drain() {
                    let data = json!({
                        "type": "content_block_stop",
                        "index": index
                    });
                    send_event(&tx, Some("content_block_stop"), &data.to_string()).await;
                }
                let finish = json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": if saw_tool_calls { "tool_use" } else { "end_turn" },
                        "stop_sequence": Value::Null
                    },
                    "usage": {
                        "output_tokens": output_tokens
                    }
                });
                send_event(&tx, Some("message_delta"), &finish.to_string()).await;
                send_event(&tx, Some("message_stop"), "{\"type\":\"message_stop\"}").await;
            }
            ResponseFormat::Responses => {
                let output_text = build_responses_output_text(&aggregated, &server_tool_calls);
                if !output_text.text.is_empty() {
                    let text_done = build_stream_responses_output_text_done_event(
                        &response_id,
                        &output_text.text,
                    );
                    send_data(&tx, &text_done.to_string()).await;
                }
                if !aggregated.thinking.is_empty() {
                    let reasoning_done = build_stream_responses_reasoning_done_event(
                        &response_id,
                        &aggregated.thinking,
                    );
                    send_data(&tx, &reasoning_done.to_string()).await;
                }
                let content = build_responses_message_content(&aggregated, &server_tool_calls);
                let output_item_done = json!({
                    "type": "response.output_item.done",
                    "response_id": response_id,
                    "output_index": 0,
                    "item": {
                        "id": message_id,
                        "type": "message",
                        "status": "completed",
                        "role": "assistant",
                        "content": content
                    }
                });
                send_data(&tx, &output_item_done.to_string()).await;

                let completed = build_stream_responses_completed_event(
                    &model,
                    &aggregated,
                    &server_tool_calls,
                    &response_id,
                    &message_id,
                    created_at,
                    previous_response_id.as_deref(),
                );
                send_data(&tx, &completed.to_string()).await;
                persist_responses_session_entry(
                    &state,
                    &response_id,
                    request_messages.clone(),
                    request_tools.clone(),
                    request_tool_choice.clone(),
                    previous_response_id.clone(),
                    &aggregated,
                )
                .await;
                send_data(&tx, "[DONE]").await;
            }
            ResponseFormat::OpenAI => {
                // 发送 tool_calls（如果有）
                if !aggregated.tool_calls.is_empty() {
                    let tool_calls_delta: Vec<_> = aggregated
                        .tool_calls
                        .iter()
                        .enumerate()
                        .map(|(index, (id, name, arguments))| {
                            json!({
                                "index": index,
                                "id": id,
                                "type": "function",
                                "function": {
                                    "name": name,
                                    "arguments": arguments
                                }
                            })
                        })
                        .collect();

                    let chunk = stream::build_openai_chunk(
                        &completion_id,
                        created_at,
                        &model,
                        crate::gateway::models::OpenAIChatDelta {
                            role: None,
                            content: None,
                            tool_calls: Some(
                                tool_calls_delta
                                    .iter()
                                    .map(|tc| crate::gateway::models::OpenAIDeltaToolCall {
                                        index: tc["index"].as_i64().unwrap() as i32,
                                        id: tc["id"].as_str().unwrap().to_string(),
                                        call_type: "function".to_string(),
                                        function: crate::gateway::models::OpenAIToolCallFunction {
                                            name: tc["function"]["name"]
                                                .as_str()
                                                .unwrap()
                                                .to_string(),
                                            arguments: tc["function"]["arguments"]
                                                .as_str()
                                                .unwrap()
                                                .to_string(),
                                        },
                                    })
                                    .collect(),
                            ),
                        },
                        None,
                        None,
                    );
                    let chunk_json = serde_json::to_string(&chunk).unwrap_or_default();
                    send_data(&tx, &chunk_json).await;
                }

                // 发送最终 chunk（带 finish_reason 和 usage）
                let finish_reason = if saw_tool_calls { "tool_calls" } else { "stop" };
                let final_chunk = stream::build_openai_chunk(
                    &completion_id,
                    created_at,
                    &model,
                    crate::gateway::models::OpenAIChatDelta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    },
                    Some(finish_reason.to_string()),
                    Some(crate::gateway::models::OpenAIChatUsage {
                        prompt_tokens: input_tokens,
                        completion_tokens: output_tokens,
                        total_tokens: input_tokens + output_tokens,
                    }),
                );
                let final_json = serde_json::to_string(&final_chunk).unwrap_or_default();
                send_data(&tx, &final_json).await;
                send_data(&tx, "[DONE]").await;
            }
        }

        // 流式结束后记录完整的 tokens
        write_request_log(
            &log_context,
            StatusCode::OK,
            "stream",
            None,
            Some(STREAMING_RESPONSE_PLACEHOLDER),
            Some(aggregated.input_tokens),
            Some(aggregated.output_tokens),
            aggregated.cache_read_input_tokens,
            aggregated.cache_creation_input_tokens,
        );
    }); // tokio::spawn 闭合

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        )
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .header(header::CONNECTION, HeaderValue::from_static("keep-alive"))
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[allow(clippy::too_many_arguments)]
async fn handle_stream_text(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    format: ResponseFormat,
    model: &str,
    anthropic_id: &str,
    response_id: &str,
    completion_id: &str,
    created: i64,
    text: &str,
    is_thinking: bool,
    message_started: &mut bool,
    next_block_index: &mut usize,
    text_block_index: &mut Option<usize>,
    thinking_block_index: &mut Option<usize>,
    input_tokens: i32,
    output_tokens: i32,
) {
    if text.is_empty() {
        return;
    }

    match format {
        ResponseFormat::Anthropic => {
            ensure_anthropic_message_start(
                tx,
                message_started,
                anthropic_id,
                model,
                input_tokens,
                output_tokens,
            )
            .await;

            if is_thinking {
                close_content_block(tx, text_block_index).await;
                if thinking_block_index.is_none() {
                    let index = *next_block_index;
                    *next_block_index += 1;
                    *thinking_block_index = Some(index);
                    let data = json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": {
                            "type": "thinking",
                            "thinking": ""
                        }
                    });
                    send_event(tx, Some("content_block_start"), &data.to_string()).await;
                }
                let data = json!({
                    "type": "content_block_delta",
                    "index": thinking_block_index.unwrap_or_default(),
                    "delta": {
                        "type": "thinking_delta",
                        "thinking": text
                    }
                });
                send_event(tx, Some("content_block_delta"), &data.to_string()).await;
            } else {
                close_content_block(tx, thinking_block_index).await;
                if text_block_index.is_none() {
                    let index = *next_block_index;
                    *next_block_index += 1;
                    *text_block_index = Some(index);
                    let data = json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": {
                            "type": "text",
                            "text": ""
                        }
                    });
                    send_event(tx, Some("content_block_start"), &data.to_string()).await;
                }
                let data = json!({
                    "type": "content_block_delta",
                    "index": text_block_index.unwrap_or_default(),
                    "delta": {
                        "type": "text_delta",
                        "text": text
                    }
                });
                send_event(tx, Some("content_block_delta"), &data.to_string()).await;
            }
        }
        ResponseFormat::Responses => {
            let data = json!({
                "type": if is_thinking { "response.reasoning.delta" } else { "response.output_text.delta" },
                "response_id": response_id,
                "delta": text
            });
            send_data(tx, &data.to_string()).await;
        }
        ResponseFormat::OpenAI => {
            if is_thinking {
                return;
            }
            let delta = crate::gateway::models::OpenAIChatDelta {
                role: if !*message_started {
                    *message_started = true;
                    Some("assistant".to_string())
                } else {
                    None
                },
                content: Some(text.to_string()),
                tool_calls: None,
            };
            let chunk = crate::gateway::stream::build_openai_chunk(
                completion_id,
                created,
                model,
                delta,
                None,
                None,
            );
            if let Ok(chunk_json) = serde_json::to_string(&chunk) {
                send_data(tx, &chunk_json).await;
            }
        }
    }
}

async fn emit_anthropic_empty_response_fallback(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    message_started: &mut bool,
    anthropic_id: &str,
    model: &str,
    input_tokens: i32,
    output_tokens: i32,
    next_block_index: &mut usize,
) {
    ensure_anthropic_message_start(
        tx,
        message_started,
        anthropic_id,
        model,
        input_tokens,
        output_tokens,
    )
    .await;

    let index = *next_block_index;
    *next_block_index += 1;

    let start = json!({
        "type": "content_block_start",
        "index": index,
        "content_block": {
            "type": "text",
            "text": ""
        }
    });
    send_event(tx, Some("content_block_start"), &start.to_string()).await;

    let delta = json!({
        "type": "content_block_delta",
        "index": index,
        "delta": {
            "type": "text_delta",
            "text": EMPTY_ANTHROPIC_RESPONSE_FALLBACK
        }
    });
    send_event(tx, Some("content_block_delta"), &delta.to_string()).await;

    let stop = json!({
        "type": "content_block_stop",
        "index": index
    });
    send_event(tx, Some("content_block_stop"), &stop.to_string()).await;
}

async fn ensure_anthropic_message_start(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    message_started: &mut bool,
    anthropic_id: &str,
    model: &str,
    input_tokens: i32,
    output_tokens: i32,
) {
    if *message_started {
        return;
    }
    let data = json!({
        "type": "message_start",
        "message": {
            "id": anthropic_id,
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": model,
            "stop_reason": Value::Null,
            "stop_sequence": Value::Null,
            "usage": {
                "input_tokens": input_tokens,
                "output_tokens": output_tokens
            }
        }
    });
    send_event(tx, Some("message_start"), &data.to_string()).await;
    *message_started = true;
}

async fn close_content_block(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    index: &mut Option<usize>,
) {
    if let Some(current) = index.take() {
        let data = json!({
            "type": "content_block_stop",
            "index": current
        });
        send_event(tx, Some("content_block_stop"), &data.to_string()).await;
    }
}

async fn send_event(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    event: Option<&str>,
    payload: &str,
) -> bool {
    let chunk = if let Some(event) = event {
        format!("event: {event}\ndata: {payload}\n\n")
    } else {
        format!("data: {payload}\n\n")
    };
    tx.send(Ok(Bytes::from(chunk))).await.is_ok()
}

async fn send_data(tx: &mpsc::Sender<Result<Bytes, Infallible>>, payload: &str) -> bool {
    send_event(tx, None, payload).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::token_cache::TokenCache;
    use serde_json::json;
    use std::sync::{atomic::AtomicU64, Arc};
    use tokio::sync::Mutex as AsyncMutex;

    fn proxy_test_state() -> RouterState {
        RouterState {
            config: GatewayConfig {
                access_token: Some("sk-test".to_string()),
                account_mode: "single".to_string(),
                account_id: Some("test-account".to_string()),
                ..GatewayConfig::default()
            },
            request_count: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(AsyncMutex::new(None)),
            http: Client::new(),
            responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
            load_balancer: Arc::new(crate::gateway::load_balancer::LoadBalancer::new(
                crate::gateway::load_balancer::LoadBalancerStrategy::RoundRobin,
            )),
        }
    }

    #[test]
    fn normalize_request_accepts_openai_chat_payloads() {
        let responses_payload = json!({
            "model": "claude-3-7-sonnet-20250219",
            "stream": true,
            "previous_response_id": "resp_prev_123",
            "tool_choice": { "type": "function", "name": "search_docs" },
            "tools": [
                {
                    "type": "function",
                    "name": "search_docs",
                    "description": "搜索文档",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "q": { "type": "string" }
                        },
                        "required": ["q"]
                    }
                }
            ],
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        { "type": "input_text", "text": "先检索 gateway" }
                    ]
                },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "search_docs",
                    "arguments": "{\"q\":\"gateway\"}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call_1",
                    "output": "命中结果"
                }
            ]
        });

        let chat_payload = json!({
            "model": "claude-3-7-sonnet-20250219",
            "stream": true,
            "tool_choice": { "type": "function", "name": "search_docs" },
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "search_docs",
                        "description": "搜索文档",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "q": { "type": "string" }
                            },
                            "required": ["q"]
                        }
                    }
                }
            ],
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "type": "input_text", "text": "先检索 gateway" }
                    ]
                },
                {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "search_docs",
                                "arguments": "{\"q\":\"gateway\"}"
                            }
                        }
                    ]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "命中结果"
                }
            ]
        });

        let responses_request = normalize_request(ResponseFormat::Responses, &responses_payload)
            .expect("responses payload should normalize");
        let chat_request = normalize_request(ResponseFormat::Responses, &chat_payload)
            .expect("chat payload should normalize through the OpenAI protocol adapter");

        assert_eq!(responses_request.model, "claude-3-7-sonnet-20250219");
        assert!(responses_request.stream);
        assert_eq!(
            responses_request.previous_response_id.as_deref(),
            Some("resp_prev_123")
        );
        assert_eq!(
            responses_request.tool_choice,
            Some(json!({ "type": "function", "name": "search_docs" }))
        );
        assert_eq!(responses_request.tools.as_ref().map(Vec::len), Some(1));
        assert_eq!(responses_request.tools.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            responses_request
                .tools
                .as_ref()
                .and_then(|items| items.first())
                .map(|tool| tool.function.name.as_str()),
            Some("search_docs")
        );
        assert_eq!(responses_request.messages.len(), 3);
        assert_eq!(
            responses_request.messages[1]
                .tool_calls
                .as_ref()
                .and_then(|items| items.first())
                .map(|call| &call.function.arguments),
            Some(&"{\"q\":\"gateway\"}".to_string())
        );
        assert_eq!(
            responses_request.messages[2].content,
            Some(json!("命中结果"))
        );
        assert_eq!(chat_request.model, responses_request.model);
        assert_eq!(chat_request.stream, responses_request.stream);
        assert_eq!(chat_request.tool_choice, responses_request.tool_choice);
        assert_eq!(chat_request.tools.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            chat_request.messages.len(),
            responses_request.messages.len()
        );
        assert_eq!(
            chat_request.messages[1]
                .tool_calls
                .as_ref()
                .and_then(|items| items.first())
                .map(|call| &call.function.arguments),
            Some(&"{\"q\":\"gateway\"}".to_string())
        );
        assert_eq!(chat_request.messages[2].content, Some(json!("命中结果")));
    }

    #[test]
    fn test_tokenizer_type_from_model_id() {
        assert!(matches!(
            TokenizerType::from_model_id("claude-3-7-sonnet-20250219"),
            TokenizerType::Claude
        ));
        assert!(matches!(
            TokenizerType::from_model_id("gpt-4"),
            TokenizerType::OpenAI
        ));
        assert!(matches!(
            TokenizerType::from_model_id("o1-preview"),
            TokenizerType::OpenAI
        ));
        assert!(matches!(
            TokenizerType::from_model_id("llama-3-70b"),
            TokenizerType::Llama
        ));
        assert!(matches!(
            TokenizerType::from_model_id("unknown-model"),
            TokenizerType::Generic
        ));
    }

    #[test]
    fn test_estimate_text_tokens_claude() {
        let text = "Hello, world!";
        let tokens = estimate_text_tokens(text, TokenizerType::Claude);
        assert_eq!(tokens, (text.len() + 3) / 4);
    }

    #[test]
    fn test_estimate_text_tokens_llama() {
        let text = "Hello, world!";
        let tokens = estimate_text_tokens(text, TokenizerType::Llama);
        assert_eq!(tokens, ((text.len() as f64 / 3.5).ceil() as usize).max(1));
    }

    #[test]
    fn test_estimate_text_tokens_generic() {
        let text = "Hello\nWorld\n```rust\nfn main() {}\n```";
        let tokens = estimate_text_tokens(text, TokenizerType::Generic);

        let base_tokens = (text.len() + 3) / 4;
        let lines = text.lines().count();
        let newline_tokens = (lines + 1) / 2;
        let code_blocks = text.matches("```").count();
        let code_block_tokens = code_blocks * 2;
        let expected = base_tokens + newline_tokens + code_block_tokens;

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_estimate_request_tokens() {
        let messages = vec![
            NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!("Hello, how are you?")),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
            NormalizedMessage {
                role: "assistant".to_string(),
                content: Some(json!("I'm doing well, thank you!")),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
        ];

        let tokens = estimate_request_tokens(&messages, "claude-3-7-sonnet-20250219");
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_request_tokens_with_tools_includes_tool_schema() {
        let messages = vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("Use the tool")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }];
        let tools = Some(vec![Tool {
            tool_type: "function".to_string(),
            function: crate::gateway::models::ToolFunction {
                name: "big_tool".to_string(),
                description: Some("A".repeat(4096)),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "B".repeat(4096)
                        }
                    }
                })),
            },
            web_search: None,
        }]);

        assert!(
            estimate_request_tokens_with_tools(&messages, &tools, "claude-opus-4.7")
                > estimate_request_tokens(&messages, "claude-opus-4.7")
        );
    }

    #[test]
    fn test_estimate_request_tokens_with_tools_applies_safety_multiplier() {
        let messages = vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("A".repeat(4000))),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }];

        let raw_tokens = estimate_request_tokens(&messages, "claude-opus-4.7[131072]");
        let safe_tokens = estimate_request_tokens_with_tools(
            &messages,
            &None,
            "claude-opus-4.7[131072]",
        );

        assert!(safe_tokens > raw_tokens);
        assert!(safe_tokens >= ((raw_tokens as f64) * REQUEST_TOKENS_SAFETY_MULTIPLIER) as usize);
    }

    #[test]
    fn test_count_tokens_response_counts_system_tools_and_tool_results() {
        let small = build_count_tokens_response(&json!({
            "model": "claude-opus-4.7",
            "messages": [{ "role": "user", "content": "hello" }]
        }));
        let large = build_count_tokens_response(&json!({
            "model": "claude-opus-4.7",
            "system": "S".repeat(4096),
            "tools": [{
                "name": "analyze_function",
                "description": "D".repeat(4096),
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "addr": { "type": "string", "description": "A".repeat(2048) }
                    }
                }
            }],
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "tool_1",
                    "content": "R".repeat(8192)
                }]
            }]
        }));

        let small_tokens = small.get("input_tokens").and_then(Value::as_u64).unwrap();
        let large_tokens = large.get("input_tokens").and_then(Value::as_u64).unwrap();
        assert!(large_tokens > small_tokens + 4000);
    }

    #[test]
    fn test_check_payload_size() {
        let payload = json!({
            "model": "claude-3-7-sonnet-20250219",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let size = check_payload_size(&payload);
        assert!(size > 0);
    }

    #[test]
    fn test_trim_kiro_payload_history_removes_oldest_messages() {
        let mut payload = json!({
            "conversationState": {
                "history": [
                    {
                        "user_input_message": {
                            "user_input_message_context": {
                                "text": "First message"
                            }
                        }
                    },
                    {
                        "assistant_response_message": {
                            "text": "First response"
                        }
                    },
                    {
                        "user_input_message": {
                            "user_input_message_context": {
                                "text": "Second message"
                            }
                        }
                    },
                    {
                        "assistant_response_message": {
                            "text": "Second response"
                        }
                    }
                ]
            }
        });

        let max_bytes = 100;
        let trimmed = trim_kiro_payload_history(&mut payload, max_bytes);

        assert!(trimmed);
        let history = payload
            .pointer("/conversationState/history")
            .and_then(|v| v.as_array())
            .unwrap();
        assert!(history.len() < 4);
        assert!(history.len() >= 2);
    }

    #[test]
    fn test_trim_kiro_payload_history_preserves_tool_call_pairs() {
        let mut payload = json!({
            "conversationState": {
                "history": [
                    {
                        "assistant_response_message": {
                            "text": "Let me search for that",
                            "tool_uses": [
                                {
                                    "id": "call_1",
                                    "name": "search",
                                    "input": {"q": "test"}
                                }
                            ]
                        }
                    },
                    {
                        "user_input_message": {
                            "user_input_message_context": {
                                "tool_results": [
                                    {
                                        "call_id": "call_1",
                                        "output": "Found results"
                                    }
                                ]
                            }
                        }
                    },
                    {
                        "user_input_message": {
                            "user_input_message_context": {
                                "text": "Recent message"
                            }
                        }
                    }
                ]
            }
        });

        let max_bytes = 200;
        let trimmed = trim_kiro_payload_history(&mut payload, max_bytes);

        if trimmed {
            let history = payload
                .pointer("/conversationState/history")
                .and_then(|v| v.as_array())
                .unwrap();

            if history.len() == 1 {
                assert!(history[0].get("user_input_message").is_some());
            }
        }
    }

    #[test]
    fn test_trim_kiro_payload_history_to_token_limit_removes_old_history() {
        let mut payload = json!({
            "conversationState": {
                "history": [
                    {
                        "user_input_message": {
                            "user_input_message_context": { "text": "old ".repeat(4096) }
                        }
                    },
                    {
                        "assistant_response_message": { "text": "old response ".repeat(4096) }
                    },
                    {
                        "user_input_message": {
                            "user_input_message_context": { "text": "recent question" }
                        }
                    }
                ],
                "currentMessage": {
                    "userInputMessage": {
                        "content": "current question",
                        "modelId": "claude-opus-4.7"
                    }
                }
            }
        });

        let before = estimate_kiro_payload_tokens(&payload, "claude-opus-4.7[8192]");
        let trimmed =
            trim_kiro_payload_history_to_token_limit(&mut payload, "claude-opus-4.7[8192]", 8192);
        let after = estimate_kiro_payload_tokens(&payload, "claude-opus-4.7[8192]");

        assert!(trimmed);
        assert!(after < before);
        let history_len = payload
            .pointer("/conversationState/history")
            .and_then(|v| v.as_array())
            .map(Vec::len)
            .unwrap_or_default();
        assert!(history_len < 3);
    }

    #[tokio::test]
    async fn test_get_model_max_input_tokens() {
        assert_eq!(get_model_max_input_tokens("auto").await, 1_000_000);
        assert_eq!(
            get_model_max_input_tokens("claude-opus-4.7[131072]").await,
            131_072
        );
        assert_eq!(
            get_model_max_input_tokens("gpt-5.5[200000]").await,
            200_000
        );
        assert_eq!(
            get_model_max_input_tokens("claude-3-7-sonnet-20250219").await,
            200_000
        );
        assert_eq!(get_model_max_input_tokens("gpt-4").await, 200_000);
        assert_eq!(get_model_max_input_tokens("deepseek-chat").await, 128_000);
        assert_eq!(get_model_max_input_tokens("llama-3-70b").await, 128_000);
        assert_eq!(get_model_max_input_tokens("unknown-model").await, 200_000);
    }

    #[tokio::test]
    async fn restore_responses_session_messages_replays_previous_assistant_turn() {
        let state = proxy_test_state();
        {
            let mut sessions = state.responses_sessions.lock().await;
            sessions.insert(
                "resp_prev_123".to_string(),
                ResponsesSessionEntry {
                    response_id: "resp_prev_123".to_string(),
                    previous_response_id: None,
                    request_messages: vec![NormalizedMessage {
                        role: "user".to_string(),
                        content: Some(json!("第一问")),
                        tool_calls: None,
                        tool_call_id: None,
                        metadata: None,
                    }],
                    response_text: "第一答".to_string(),
                    tool_calls: vec![(
                        "call_1".to_string(),
                        "search_docs".to_string(),
                        "{\"q\":\"gateway\"}".to_string(),
                    )],
                    request_tools: None,
                    request_tool_choice: None,
                    updated_at: Instant::now(),
                },
            );
        }

        let request = NormalizedRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!("第二问")),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            }],
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            tools: None,
            tool_choice: None,
            previous_response_id: Some("resp_prev_123".to_string()),
            thinking: None,
        };

        let merged = restore_responses_session_messages(&state, &request).await;

        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].role, "user");
        assert_eq!(merged[1].role, "assistant");
        assert_eq!(merged[2].role, "user");
        assert_eq!(merged[1].content, Some(json!("第一答")));
        assert_eq!(
            merged[1]
                .tool_calls
                .as_ref()
                .and_then(|items| items.first())
                .map(|call| call.function.name.as_str()),
            Some("search_docs")
        );
    }

    #[test]
    fn verify_client_auth_accepts_any_configured_client_api_key() {
        let config = GatewayConfig {
            access_token: Some("sk-primary".to_string()),
            client_api_keys: vec!["sk-primary".to_string(), "sk-secondary".to_string()],
            ..GatewayConfig::default()
        };

        let mut bearer_headers = HeaderMap::new();
        bearer_headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer sk-secondary"),
        );
        assert!(verify_client_auth(&bearer_headers, &config).is_ok());

        let mut x_api_key_headers = HeaderMap::new();
        x_api_key_headers.insert("x-api-key", HeaderValue::from_static("sk-primary"));
        assert!(verify_client_auth(&x_api_key_headers, &config).is_ok());

        let mut invalid_headers = HeaderMap::new();
        invalid_headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer sk-unknown"),
        );
        assert!(verify_client_auth(&invalid_headers, &config).is_err());
    }

    #[test]
    fn parse_web_search_mcp_result_preserves_results_and_filters_domains() {
        let result = json!({
            "content": [{
                "type": "text",
                "text": "{\"results\":[{\"title\":\"Rust Blog\",\"url\":\"https://blog.rust-lang.org/inside-rust\"},{\"title\":\"Other\",\"url\":\"https://example.com/post\"}]}"
            }],
            "isError": false
        });
        let options = WebSearchToolOptions {
            max_uses: Some(3),
            allowed_domains: Some(vec!["blog.rust-lang.org".to_string()]),
            blocked_domains: Some(vec!["example.com".to_string()]),
            user_location: None,
        };

        let (content, tool_result_text) = parse_web_search_mcp_result(&result, Some(&options));

        assert_eq!(
            content,
            json!([{
                "type": "web_search_result",
                "title": "Rust Blog",
                "url": "https://blog.rust-lang.org/inside-rust"
            }])
        );
        assert!(tool_result_text.contains("blog.rust-lang.org"));
        assert!(!tool_result_text.contains("example.com"));
    }

    #[test]
    fn server_web_search_iteration_limit_uses_max_uses() {
        assert_eq!(server_web_search_iteration_limit(None), 8);
        assert_eq!(server_web_search_iteration_limit(Some(1)), 1);
        assert_eq!(server_web_search_iteration_limit(Some(3)), 3);
        assert_eq!(server_web_search_iteration_limit(Some(99)), 8);
        assert_eq!(server_web_search_iteration_limit(Some(0)), 0);
        assert_eq!(server_web_search_iteration_limit(Some(-5)), 0);
    }

    #[test]
    fn detect_upstream_error_body_maps_success_status_error_payloads() {
        let error = detect_upstream_error_body(
            r#"{"error":{"message":"Invalid model. Please select a different model to continue.","type":"invalid_request_error"}}"#,
        )
        .expect("error payload should be detected");

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert_eq!(error.1, "invalid_request_error");
        assert!(error.2.contains("Invalid model"));
    }

    #[test]
    fn build_anthropic_response_emits_server_tool_blocks() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "我查到了结果".to_string(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            input_tokens: 10,
            output_tokens: 20,
            context_usage_percentage: None,
            citations: Vec::new(),
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };
        let response = build_anthropic_response(
            "claude-sonnet-4-5",
            &aggregated,
            &[ServerToolCall {
                id: "srv_1".to_string(),
                name: "web_search".to_string(),
                input: json!({ "query": "Rust release" }),
                result_content: json!([{
                    "type": "web_search_result",
                    "title": "Rust Blog",
                    "url": "https://blog.rust-lang.org"
                }]),
                tool_result_text: "{\"results\":[]}".to_string(),
            }],
        );

        assert_eq!(response["content"][0]["type"], "server_tool_use");
        assert_eq!(response["content"][1]["type"], "web_search_tool_result");
        assert_eq!(response["content"][2]["type"], "text");
        assert_eq!(response["content"][1]["tool_use_id"], "srv_1");
    }

    #[test]
    fn build_responses_response_emits_web_search_call_and_url_citations() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "我查到了 Rust 发布记录。".to_string(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            input_tokens: 12,
            output_tokens: 24,
            context_usage_percentage: None,
            citations: Vec::new(),
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };
        let response = build_responses_response_with_ids(
            "gpt-4.1",
            &aggregated,
            &[ServerToolCall {
                id: "srv_1".to_string(),
                name: "web_search".to_string(),
                input: json!({ "query": "Rust release" }),
                result_content: json!([
                    {
                        "type": "web_search_result",
                        "title": "Rust Blog",
                        "url": "https://blog.rust-lang.org"
                    },
                    {
                        "type": "web_search_result",
                        "title": "Inside Rust",
                        "url": "https://blog.rust-lang.org/inside-rust"
                    }
                ]),
                tool_result_text: "{\"results\":[]}".to_string(),
            }],
            "resp_test",
            "msg_test",
            123,
            Some("resp_prev_123"),
        );

        assert_eq!(response["previous_response_id"], "resp_prev_123");
        assert_eq!(response["output"][0]["type"], "web_search_call");
        assert_eq!(response["output"][0]["action"]["query"], "Rust release");
        assert_eq!(response["output"][1]["type"], "message");
        assert_eq!(response["output"][1]["content"][0]["type"], "output_text");
        assert_eq!(
            response["output"][1]["content"][0]["annotations"][0]["type"],
            "url_citation"
        );
        assert_eq!(
            response["output"][1]["content"][0]["annotations"][0]["url"],
            "https://blog.rust-lang.org"
        );
        assert_eq!(
            response["output"][1]["content"][0]["annotations"][0]["title"],
            "Rust Blog"
        );
        assert!(response["output_text"]
            .as_str()
            .expect("output_text should be present")
            .contains("Sources:\n[1] Rust Blog"));
    }

    #[test]
    fn build_responses_response_emits_kiro_citation_annotations() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "Hello Rust".to_string(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            input_tokens: 3,
            output_tokens: 5,
            context_usage_percentage: None,
            citations: vec![stream::AggregatedCitation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: json!({ "range": { "start": 6, "end": 10 } }),
            }],
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };

        let response = build_responses_response_with_ids(
            "gpt-5.4",
            &aggregated,
            &[],
            "resp_test",
            "msg_test",
            123,
            None,
        );

        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["type"],
            "url_citation"
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["start_index"],
            6
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["end_index"],
            10
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["url"],
            "https://example.com/rust"
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["citationText"],
            "Rust"
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["citationLink"],
            "https://example.com/rust"
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["target"]["range"]["start"],
            6
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["target"]["range"]["end"],
            10
        );
        assert!(response["output"][0]["content"][0]["annotations"][0]["title"].is_null());
    }

    #[test]
    fn build_responses_response_omits_guessed_range_for_location_citations() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "Hello Rust".to_string(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            input_tokens: 3,
            output_tokens: 5,
            context_usage_percentage: None,
            citations: vec![stream::AggregatedCitation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: json!({ "location": 6 }),
            }],
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };

        let response = build_responses_response_with_ids(
            "gpt-4.1",
            &aggregated,
            &[],
            "resp_test",
            "msg_test",
            123,
            None,
        );

        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["type"],
            "url_citation"
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["citationText"],
            "Rust"
        );
        assert_eq!(
            response["output"][0]["content"][0]["annotations"][0]["target"]["location"],
            6
        );
        assert!(response["output"][0]["content"][0]["annotations"][0]["start_index"].is_null());
        assert!(response["output"][0]["content"][0]["annotations"][0]["end_index"].is_null());
        assert!(response["output"][0]["content"][0]["annotations"][0]["title"].is_null());
    }

    #[test]
    fn build_anthropic_response_maps_kiro_citations_into_sdk_shape() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "Hello Rust".to_string(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            input_tokens: 3,
            output_tokens: 5,
            context_usage_percentage: None,
            citations: vec![stream::AggregatedCitation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: json!({ "range": { "start": 6, "end": 10 } }),
            }],
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };

        let response = build_anthropic_response("claude-sonnet-4-5", &aggregated, &[]);

        assert_eq!(response["content"][0]["type"], "text");
        assert_eq!(
            response["content"][0]["citations"][0]["type"],
            "char_location"
        );
        assert_eq!(
            response["content"][0]["citations"][0]["start_char_index"],
            6
        );
        assert_eq!(response["content"][0]["citations"][0]["end_char_index"], 10);
        assert_eq!(response["content"][0]["citations"][0]["cited_text"], "Rust");
        assert_eq!(
            response["content"][0]["citations"][0]["document_title"],
            "https://example.com/rust"
        );
        assert!(response["content"][0]["citations"][0]["file_id"].is_null());
    }

    #[test]
    fn build_anthropic_response_emits_text_block_for_empty_upstream_response() {
        let response = build_anthropic_response(
            "claude-sonnet-4-5",
            &stream::AggregatedKiroResponse::default(),
            &[],
        );

        assert_eq!(response["stop_reason"], "end_turn");
        assert_eq!(response["content"].as_array().unwrap().len(), 1);
        assert_eq!(response["content"][0]["type"], "text");
        assert_eq!(
            response["content"][0]["text"],
            EMPTY_ANTHROPIC_RESPONSE_FALLBACK
        );
    }

    #[tokio::test]
    async fn emit_anthropic_empty_response_fallback_starts_and_stops_text_block() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut message_started = false;
        let mut next_block_index = 0usize;

        emit_anthropic_empty_response_fallback(
            &tx,
            &mut message_started,
            "msg_test",
            "claude-sonnet-4-5",
            123,
            0,
            &mut next_block_index,
        )
        .await;
        drop(tx);

        let mut output = String::new();
        while let Some(chunk) = rx.recv().await {
            let bytes = chunk.expect("stream chunk should be infallible");
            output.push_str(std::str::from_utf8(&bytes).expect("SSE chunk should be UTF-8"));
        }

        assert!(message_started);
        assert_eq!(next_block_index, 1);
        assert!(output.contains("event: message_start"));
        assert!(output.contains("event: content_block_start"));
        assert!(output.contains("\"type\":\"text_delta\""));
        assert!(output.contains(EMPTY_ANTHROPIC_RESPONSE_FALLBACK));
        assert!(output.contains("event: content_block_stop"));
    }

    #[test]
    fn build_stream_responses_completed_event_keeps_citations_and_tool_calls() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "Hello Rust".to_string(),
            thinking: String::new(),
            tool_calls: vec![(
                "call_1".to_string(),
                "search_docs".to_string(),
                "{\"q\":\"rust\"}".to_string(),
            )],
            input_tokens: 3,
            output_tokens: 5,
            context_usage_percentage: None,
            citations: vec![stream::AggregatedCitation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: json!({ "range": { "start": 6, "end": 10 } }),
            }],
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };

        let event = build_stream_responses_completed_event(
            "gpt-4.1",
            &aggregated,
            &[],
            "resp_test",
            "msg_test",
            123,
            None,
        );

        assert_eq!(event["type"], "response.completed");
        assert_eq!(event["response"]["output_text"], "Hello Rust");
        assert_eq!(
            event["response"]["output"][0]["content"][0]["annotations"][0]["citationText"],
            "Rust"
        );
        assert!(event["response"]["output"][0]["content"][0]["annotations"][0]["title"].is_null());
        assert_eq!(
            event["response"]["output"][0]["content"][1]["type"],
            "function_call"
        );
        assert_eq!(
            event["response"]["output"][0]["content"][1]["call_id"],
            "call_1"
        );
    }

    #[test]
    fn build_stream_responses_completed_event_keeps_server_web_search_output() {
        let aggregated = stream::AggregatedKiroResponse {
            text: "Hello Rust".to_string(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            input_tokens: 3,
            output_tokens: 5,
            context_usage_percentage: None,
            citations: Vec::new(),
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            metering_usage: None,
        };

        let event = build_stream_responses_completed_event(
            "gpt-4.1",
            &aggregated,
            &[ServerToolCall {
                id: "srv_1".to_string(),
                name: "web_search".to_string(),
                input: json!({ "query": "Rust release" }),
                result_content: json!([{
                    "type": "web_search_result",
                    "title": "Rust Blog",
                    "url": "https://blog.rust-lang.org"
                }]),
                tool_result_text: "{\"results\":[]}".to_string(),
            }],
            "resp_test",
            "msg_test",
            123,
            None,
        );

        assert_eq!(event["response"]["output"][0]["type"], "web_search_call");
        assert_eq!(
            event["response"]["output"][0]["action"]["query"],
            "Rust release"
        );
        assert_eq!(event["response"]["output"][1]["type"], "message");
    }

    #[test]
    fn build_stream_responses_done_events_use_expected_shape() {
        let function_done = build_stream_responses_function_call_arguments_done_event(
            "resp_test",
            "call_1",
            "{\"q\":\"rust\"}",
        );
        let text_done = build_stream_responses_output_text_done_event("resp_test", "Hello Rust");
        let reasoning_done = build_stream_responses_reasoning_done_event("resp_test", "Think");

        assert_eq!(
            function_done["type"],
            "response.function_call_arguments.done"
        );
        assert_eq!(function_done["response_id"], "resp_test");
        assert_eq!(function_done["call_id"], "call_1");
        assert_eq!(function_done["arguments"], "{\"q\":\"rust\"}");

        assert_eq!(text_done["type"], "response.output_text.done");
        assert_eq!(text_done["response_id"], "resp_test");
        assert_eq!(text_done["text"], "Hello Rust");

        assert_eq!(reasoning_done["type"], "response.reasoning.done");
        assert_eq!(reasoning_done["response_id"], "resp_test");
        assert_eq!(reasoning_done["text"], "Think");
    }

    #[test]
    fn with_kiro_upstream_headers_adds_generate_request_headers() {
        let upstream = UpstreamCredentials {
            access_token: "token-1".to_string(),
            profile_arn: None,
            provider: None,
            region: "us-east-1".to_string(),
            source_label: "single:test".to_string(),
            user_agent: "KiroIDE 0.11.34 machine-123".to_string(),
            auth_method: Some("external_idp".to_string()),
            send_opt_out: true,
        };

        let request = with_kiro_upstream_headers(
            reqwest::Client::new()
                .post("https://q.us-east-1.amazonaws.com/generateAssistantResponse"),
            &upstream,
            "application/vnd.amazon.eventstream",
            true,
            true,
            false,
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer token-1")
        );
        assert_eq!(
            request
                .headers()
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok()),
            Some("KiroIDE 0.11.34 machine-123")
        );
        assert_eq!(
            request
                .headers()
                .get("x-amz-user-agent")
                .and_then(|value| value.to_str().ok()),
            Some("KiroIDE 0.11.34 machine-123")
        );
        assert_eq!(
            request
                .headers()
                .get("x-amzn-codewhisperer-optout")
                .and_then(|value| value.to_str().ok()),
            Some("true")
        );
        assert_eq!(
            request
                .headers()
                .get("x-amzn-kiro-agent-mode")
                .and_then(|value| value.to_str().ok()),
            Some(DEFAULT_AGENT_MODE)
        );
        // TokenType header 已移除（会导致某些接口 403 错误）
        assert!(request.headers().get("TokenType").is_none());
        assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
        assert!(request.headers().get("redirect-for-internal").is_none());
    }

    #[test]
    fn with_kiro_upstream_headers_keeps_runtime_requests_minimal() {
        let upstream = UpstreamCredentials {
            access_token: "token-2".to_string(),
            profile_arn: None,
            provider: None,
            region: "us-east-1".to_string(),
            source_label: "single:test".to_string(),
            user_agent: "KiroIDE 0.11.34 machine-456".to_string(),
            auth_method: Some("social".to_string()),
            send_opt_out: true,
        };

        let request = with_kiro_upstream_headers(
            reqwest::Client::new()
                .get("https://q.us-east-1.amazonaws.com/ListAvailableModels?origin=AI_EDITOR"),
            &upstream,
            "application/json",
            false,
            false,
            false,
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get("x-amz-user-agent")
                .and_then(|value| value.to_str().ok()),
            Some("KiroIDE 0.11.34 machine-456")
        );
        assert!(request
            .headers()
            .get("x-amzn-codewhisperer-optout")
            .is_none());
        assert!(request.headers().get("x-amzn-kiro-agent-mode").is_none());
        assert!(request.headers().get("TokenType").is_none());
        assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
        assert!(request.headers().get("redirect-for-internal").is_none());
    }

    #[test]
    fn with_kiro_upstream_headers_adds_mcp_profile_arn_header() {
        let upstream = UpstreamCredentials {
            access_token: "token-3".to_string(),
            profile_arn: Some(
                "arn:aws:codewhisperer:us-east-1:123456789012:profile/test".to_string(),
            ),
            provider: None,
            region: "us-east-1".to_string(),
            source_label: "single:test".to_string(),
            user_agent: "KiroIDE 0.11.34 machine-789".to_string(),
            auth_method: Some("social".to_string()),
            send_opt_out: true,
        };

        let request = with_kiro_upstream_headers(
            reqwest::Client::new().post("https://q.us-east-1.amazonaws.com/mcp"),
            &upstream,
            "application/json",
            false,
            false,
            true,
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get("x-amzn-kiro-profile-arn")
                .and_then(|value| value.to_str().ok()),
            Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/test")
        );
        assert!(request.headers().get("redirect-for-internal").is_none());
    }

    #[test]
    fn with_kiro_upstream_headers_adds_redirect_for_internal_only_for_internal_provider() {
        let upstream = UpstreamCredentials {
            access_token: "token-4".to_string(),
            profile_arn: None,
            provider: Some("Internal".to_string()),
            region: "us-east-1".to_string(),
            source_label: "single:test".to_string(),
            user_agent: "KiroIDE 0.11.34 machine-999".to_string(),
            auth_method: Some("IdC".to_string()),
            send_opt_out: true,
        };

        let request = with_kiro_upstream_headers(
            reqwest::Client::new()
                .post("https://q.us-east-1.amazonaws.com/generateAssistantResponse"),
            &upstream,
            "application/vnd.amazon.eventstream",
            true,
            true,
            false,
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get("redirect-for-internal")
                .and_then(|value| value.to_str().ok()),
            Some("true")
        );
    }

    #[test]
    fn with_kiro_upstream_headers_does_not_add_redirect_for_enterprise_or_builderid() {
        for provider in ["Enterprise", "BuilderId"] {
            let upstream = UpstreamCredentials {
                access_token: "token-5".to_string(),
                profile_arn: None,
                provider: Some(provider.to_string()),
                region: "us-east-1".to_string(),
                source_label: "single:test".to_string(),
                user_agent: "KiroIDE 0.11.34 machine-1000".to_string(),
                auth_method: Some("IdC".to_string()),
                send_opt_out: true,
            };

            let request = with_kiro_upstream_headers(
                reqwest::Client::new()
                    .post("https://q.us-east-1.amazonaws.com/generateAssistantResponse"),
                &upstream,
                "application/vnd.amazon.eventstream",
                true,
                true,
                false,
            )
            .build()
            .expect("request should build");

            assert!(
                request.headers().get("redirect-for-internal").is_none(),
                "provider {provider} should not add redirect-for-internal"
            );
        }
    }
}
