use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::Local;
use futures_util::StreamExt;
use rand::{seq::SliceRandom, thread_rng};
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    time::Instant,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use url::Url;

use crate::{
    account::{Account, AccountStore},
    commands::common::{
        calc_expires_at, calc_status, get_usage_by_provider, refresh_token_by_provider,
        RefreshResult,
    },
    commands::machine_guid::get_machine_id,
    http_client::{
        build_kiro_custom_user_agent, resolve_kiro_upstream_region,
        should_send_codewhisperer_optout,
    },
};

use super::{
    append_gateway_request_log,
    converter::{
        build_kiro_payload, get_available_models, normalize_anthropic_request,
        normalize_responses_request,
    },
    models::{
        AnthropicContentBlock, AnthropicMessagesRequest, AnthropicMessagesResponse, AnthropicUsage,
        ModelsResponse, NormalizedMessage, NormalizedRequest, ToolCall, ToolCallFunction,
        WebSearchToolOptions,
    },
    stream::{self, aggregate_kiro_response, extract_json, parse_kiro_event_full, KiroEvent},
    thinking_parser::{SegmentType, ThinkingParser},
    GatewayConfig, GatewayRequestLogEntry, ResponseFormat, RouterState, DEFAULT_AGENT_MODE,
};

#[derive(Debug, Clone)]
struct UpstreamCredentials {
    access_token: String,
    profile_arn: Option<String>,
    region: String,
    source_label: String,
    user_agent: String,
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

const MAX_LOG_BODY_CHARS: usize = 20_000;
const STREAMING_RESPONSE_PLACEHOLDER: &str = "[streaming response omitted from request log]";

pub async fn models_handler() -> impl IntoResponse {
    Json(ModelsResponse {
        object: "list".to_string(),
        data: get_available_models(),
    })
}

pub async fn count_tokens_handler(payload: Value) -> impl IntoResponse {
    let mut chars = 0usize;
    if let Some(messages) = payload.get("messages").and_then(Value::as_array) {
        for message in messages {
            chars += extract_plain_text(message.get("content")).chars().count();
        }
    }
    if let Some(input) = payload.get("input") {
        chars += extract_plain_text(Some(input)).chars().count();
    }
    Json(json!({ "input_tokens": (chars / 4).max(1) }))
}

fn request_endpoint(format: ResponseFormat) -> &'static str {
    match format {
        ResponseFormat::Anthropic => "messages",
        ResponseFormat::Responses => "responses",
    }
}

fn serialize_logged_value(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn prepare_logged_body(body: &str) -> Option<String> {
    if body.trim().is_empty() {
        return None;
    }

    let sanitized = sanitize_error(body);
    let total_chars = sanitized.chars().count();
    if total_chars <= MAX_LOG_BODY_CHARS {
        return Some(sanitized);
    }

    let mut truncated: String = sanitized.chars().take(MAX_LOG_BODY_CHARS).collect();
    truncated.push_str(&format!(
        "\n...[truncated {} chars]",
        total_chars.saturating_sub(MAX_LOG_BODY_CHARS)
    ));
    Some(truncated)
}

fn write_request_log(
    request_index: u64,
    endpoint: &str,
    client_addr: SocketAddr,
    request: Option<&NormalizedRequest>,
    upstream: Option<&UpstreamCredentials>,
    status: StatusCode,
    outcome: &str,
    started_at: Instant,
    error: Option<&str>,
    request_body: Option<&str>,
    response_body: Option<&str>,
) {
    let duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let entry = GatewayRequestLogEntry {
        occurred_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        request_index,
        endpoint: endpoint.to_string(),
        client_ip: client_addr.ip().to_string(),
        model: request.map(|item| item.model.clone()),
        stream: request.map(|item| item.stream).unwrap_or(false),
        upstream_source: upstream.map(|item| item.source_label.clone()),
        region: upstream.map(|item| item.region.clone()),
        status_code: status.as_u16(),
        outcome: outcome.to_string(),
        duration_ms,
        error: error.map(str::to_string),
        request_body: request_body.and_then(prepare_logged_body),
        response_body: response_body.and_then(prepare_logged_body),
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
    }
}

async fn gateway_error_with_log(
    state: &RouterState,
    format: ResponseFormat,
    request_index: u64,
    endpoint: &str,
    client_addr: SocketAddr,
    request: Option<&NormalizedRequest>,
    upstream: Option<&UpstreamCredentials>,
    started_at: Instant,
    status: StatusCode,
    error_type: &'static str,
    message: &str,
    request_body: Option<&str>,
    response_body: Option<&str>,
) -> Response {
    *state.last_error.lock().await = Some(message.to_string());
    let logged_response_body = response_body.map(str::to_string).or_else(|| {
        Some(serialize_logged_value(&build_gateway_error_body(
            format, status, error_type, message,
        )))
    });
    write_request_log(
        request_index,
        endpoint,
        client_addr,
        request,
        upstream,
        status,
        "error",
        started_at,
        Some(message),
        request_body,
        logged_response_body.as_deref(),
    );
    gateway_error_response(format, status, error_type, message)
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

    if state.config.local_only && !client_addr.ip().is_loopback() {
        let message = format!("已拒绝来自非本机地址的访问: {}", client_addr.ip());
        return gateway_error_with_log(
            &state,
            format,
            request_index,
            endpoint,
            client_addr,
            None,
            None,
            started_at,
            StatusCode::FORBIDDEN,
            "permission_error",
            &message,
            Some(raw_request_body.as_str()),
            None,
        )
        .await;
    }
    if !state.config.local_only
        && !state.config.allowed_ips.is_empty()
        && !ip_matches_allowlist(client_addr.ip(), &state.config.allowed_ips)
    {
        let message = format!("访问地址 {} 不在网关白名单中", client_addr.ip());
        return gateway_error_with_log(
            &state,
            format,
            request_index,
            endpoint,
            client_addr,
            None,
            None,
            started_at,
            StatusCode::FORBIDDEN,
            "permission_error",
            &message,
            Some(raw_request_body.as_str()),
            None,
        )
        .await;
    }

    if let Err(message) = verify_client_auth(&headers, &state.config) {
        let sanitized = sanitize_error(&message);
        return gateway_error_with_log(
            &state,
            format,
            request_index,
            endpoint,
            client_addr,
            None,
            None,
            started_at,
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            &sanitized,
            Some(raw_request_body.as_str()),
            None,
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
                request_index,
                endpoint,
                client_addr,
                None,
                None,
                started_at,
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                &sanitized,
                Some(raw_request_body.as_str()),
                None,
            )
            .await;
        }
    };

    let upstream = match resolve_upstream_credentials(&state.config, request_index).await {
        Ok(creds) => creds,
        Err(message) => {
            let sanitized = sanitize_error(&message);
            return gateway_error_with_log(
                &state,
                format,
                request_index,
                endpoint,
                client_addr,
                Some(&request),
                None,
                started_at,
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                &sanitized,
                Some(raw_request_body.as_str()),
                None,
            )
            .await;
        }
    };

    if has_server_web_search_tool(&request) {
        let outcome = match execute_request_with_server_tools(&state, &upstream, &request).await {
            Ok(outcome) => outcome,
            Err((status, error_type, message)) => {
                return gateway_error_with_log(
                    &state,
                    format,
                    request_index,
                    endpoint,
                    client_addr,
                    Some(&request),
                    Some(&upstream),
                    started_at,
                    status,
                    error_type,
                    &message,
                    Some(raw_request_body.as_str()),
                    None,
                )
                .await;
            }
        };

        if request.stream {
            write_request_log(
                request_index,
                endpoint,
                client_addr,
                Some(&request),
                Some(&upstream),
                StatusCode::OK,
                "stream",
                started_at,
                None,
                Some(raw_request_body.as_str()),
                Some(STREAMING_RESPONSE_PLACEHOLDER),
            );
            return stream_completed_response(format, request.model, outcome);
        }

        let response = match format {
            ResponseFormat::Anthropic => build_anthropic_response(
                &request.model,
                &outcome.aggregated,
                &outcome.server_tool_calls,
            ),
            ResponseFormat::Responses => build_responses_response(
                &request.model,
                &outcome.aggregated,
                &outcome.server_tool_calls,
            ),
        };
        let response_body = serialize_logged_value(&response);
        write_request_log(
            request_index,
            endpoint,
            client_addr,
            Some(&request),
            Some(&upstream),
            StatusCode::OK,
            "success",
            started_at,
            None,
            Some(raw_request_body.as_str()),
            Some(response_body.as_str()),
        );
        return Json(response).into_response();
    }

    let upstream_payload =
        match build_kiro_payload(&state.http, &request, upstream.profile_arn.clone()).await {
            Ok(payload) => payload,
            Err(message) => {
                let sanitized = sanitize_error(&message);
                return gateway_error_with_log(
                    &state,
                    format,
                    request_index,
                    endpoint,
                    client_addr,
                    Some(&request),
                    Some(&upstream),
                    started_at,
                    StatusCode::BAD_REQUEST,
                    "invalid_request_error",
                    &sanitized,
                    Some(raw_request_body.as_str()),
                    None,
                )
                .await;
            }
        };
    let upstream_request_body = serde_json::to_string_pretty(&upstream_payload)
        .unwrap_or_else(|_| "[failed to serialize upstream payload]".to_string());

    let upstream_resp = match send_generate_request(&state.http, &upstream, &upstream_payload).await
    {
        Ok(resp) => resp,
        Err((status, error_type, message, upstream_response_body)) => {
            return gateway_error_with_log(
                &state,
                format,
                request_index,
                endpoint,
                client_addr,
                Some(&request),
                Some(&upstream),
                started_at,
                status,
                error_type,
                &message,
                Some(upstream_request_body.as_str()),
                upstream_response_body.as_deref(),
            )
            .await;
        }
    };

    if request.stream {
        write_request_log(
            request_index,
            endpoint,
            client_addr,
            Some(&request),
            Some(&upstream),
            upstream_resp.status(),
            "stream",
            started_at,
            None,
            Some(upstream_request_body.as_str()),
            Some(STREAMING_RESPONSE_PLACEHOLDER),
        );
        return stream_proxy_response(upstream_resp, format, request.model);
    }

    let body = match upstream_resp.text().await {
        Ok(body) => body,
        Err(error) => {
            let message = sanitize_error(&format!("读取上游响应失败: {error}"));
            return gateway_error_with_log(
                &state,
                format,
                request_index,
                endpoint,
                client_addr,
                Some(&request),
                Some(&upstream),
                started_at,
                StatusCode::BAD_GATEWAY,
                "api_error",
                &message,
                Some(upstream_request_body.as_str()),
                None,
            )
            .await;
        }
    };

    if let Some((status, error_type, message)) = detect_upstream_error_body(&body) {
        return gateway_error_with_log(
            &state,
            format,
            request_index,
            endpoint,
            client_addr,
            Some(&request),
            Some(&upstream),
            started_at,
            status,
            error_type,
            &message,
            Some(upstream_request_body.as_str()),
            Some(body.as_str()),
        )
        .await;
    }

    let aggregated = aggregate_kiro_response(&body);
    let response = match format {
        ResponseFormat::Anthropic => build_anthropic_response(&request.model, &aggregated, &[]),
        ResponseFormat::Responses => build_responses_response(&request.model, &aggregated, &[]),
    };
    write_request_log(
        request_index,
        endpoint,
        client_addr,
        Some(&request),
        Some(&upstream),
        StatusCode::OK,
        "success",
        started_at,
        None,
        Some(upstream_request_body.as_str()),
        Some(body.as_str()),
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

    if state.config.local_only && !client_addr.ip().is_loopback() {
        let message = format!("已拒绝来自非本机地址的 MCP 访问: {}", client_addr.ip());
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            request_index,
            "mcp",
            client_addr,
            None,
            None,
            started_at,
            StatusCode::FORBIDDEN,
            "permission_error",
            &message,
            Some(raw_request_body.as_str()),
            None,
        )
        .await;
    }
    if !state.config.local_only
        && !state.config.allowed_ips.is_empty()
        && !ip_matches_allowlist(client_addr.ip(), &state.config.allowed_ips)
    {
        let message = format!("MCP 访问地址 {} 不在网关白名单中", client_addr.ip());
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            request_index,
            "mcp",
            client_addr,
            None,
            None,
            started_at,
            StatusCode::FORBIDDEN,
            "permission_error",
            &message,
            Some(raw_request_body.as_str()),
            None,
        )
        .await;
    }
    if let Err(message) = verify_client_auth(&headers, &state.config) {
        let sanitized = sanitize_error(&message);
        return gateway_error_with_log(
            &state,
            ResponseFormat::Responses,
            request_index,
            "mcp",
            client_addr,
            None,
            None,
            started_at,
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            &sanitized,
            Some(raw_request_body.as_str()),
            None,
        )
        .await;
    }

    let upstream = match resolve_upstream_credentials(&state.config, request_index).await {
        Ok(creds) => creds,
        Err(message) => {
            let sanitized = sanitize_error(&message);
            return gateway_error_with_log(
                &state,
                ResponseFormat::Responses,
                request_index,
                "mcp",
                client_addr,
                None,
                None,
                started_at,
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                &sanitized,
                Some(raw_request_body.as_str()),
                None,
            )
            .await;
        }
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
                request_index,
                "mcp",
                client_addr,
                None,
                Some(&upstream),
                started_at,
                StatusCode::BAD_GATEWAY,
                "api_error",
                &message,
                Some(raw_request_body.as_str()),
                None,
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
                request_index,
                "mcp",
                client_addr,
                None,
                Some(&upstream),
                started_at,
                StatusCode::BAD_GATEWAY,
                "api_error",
                &message,
                Some(raw_request_body.as_str()),
                None,
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
            request_index,
            "mcp",
            client_addr,
            None,
            Some(&upstream),
            started_at,
            mapped_status,
            error_type,
            &message,
            Some(raw_request_body.as_str()),
            Some(body_text.as_str()),
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
        request_index,
        "mcp",
        client_addr,
        None,
        Some(&upstream),
        status,
        "success",
        started_at,
        None,
        Some(raw_request_body.as_str()),
        Some(logged_response_body.as_str()),
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
    let _max_uses = web_search_options
        .as_ref()
        .and_then(|options| options.max_uses)
        .map(|value| value.max(0) as usize)
        .unwrap_or(usize::MAX);
    let mut server_tool_calls = Vec::new();

    for _ in 0..8 {
        let upstream_payload =
            build_kiro_payload(&state.http, &working_request, upstream.profile_arn.clone())
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
        });
    }

    Err((
        StatusCode::BAD_GATEWAY,
        "api_error",
        "web_search 代理循环超过最大轮数".to_string(),
    ))
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

    let upstream_resp = with_kiro_upstream_headers(
        http.post(upstream_url),
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

    if !upstream_resp.status().is_success() {
        let status = upstream_resp.status();
        let body = upstream_resp.text().await.unwrap_or_default();
        let (mapped_status, error_type, message) = map_upstream_error(status, &body);
        return Err((mapped_status, error_type, message, Some(body)));
    }

    Ok(upstream_resp)
}

fn is_external_idp_auth_method(auth_method: Option<&str>) -> bool {
    auth_method.is_some_and(|value| value.trim().eq_ignore_ascii_case("external_idp"))
}

fn with_kiro_upstream_headers(
    builder: reqwest::RequestBuilder,
    upstream: &UpstreamCredentials,
    accept: &str,
    include_opt_out: bool,
    include_agent_mode: bool,
    include_profile_arn_header: bool,
) -> reqwest::RequestBuilder {
    let mut builder = builder
        .header("Authorization", format!("Bearer {}", upstream.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header(header::USER_AGENT, upstream.user_agent.clone())
        .header("x-amz-user-agent", upstream.user_agent.clone());

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
    if is_external_idp_auth_method(upstream.auth_method.as_deref()) {
        builder = builder.header("TokenType", "EXTERNAL_IDP");
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
    }
}

fn verify_client_auth(headers: &HeaderMap, config: &GatewayConfig) -> Result<(), String> {
    let Some(expected) = config
        .access_token
        .as_ref()
        .filter(|token| !token.trim().is_empty())
    else {
        return Err("客户端 API Key 未配置".to_string());
    };

    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.strip_prefix("Bearer ").or(Some(value)));
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());

    if authorization == Some(expected.as_str()) || api_key == Some(expected.as_str()) {
        Ok(())
    } else {
        Err("客户端 API Key 无效".to_string())
    }
}

async fn resolve_upstream_credentials(
    config: &GatewayConfig,
    request_index: u64,
) -> Result<UpstreamCredentials, String> {
    match config.account_mode.as_str() {
        "single" | "group" => resolve_managed_account_credentials(config, request_index).await,
        "local" => Err("网关不再支持 local 模式，请改用 single/group 账号池模式".to_string()),
        _ => Err("accountMode 必须是 single/group".to_string()),
    }
}

async fn resolve_managed_account_credentials(
    config: &GatewayConfig,
    request_index: u64,
) -> Result<UpstreamCredentials, String> {
    let mut store = AccountStore::new();
    store.reload();
    let mut accounts = match config.account_mode.as_str() {
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
        _ => Vec::new(),
    };

    if accounts.is_empty() {
        return Err("未找到符合网关配置的可用账号".to_string());
    }

    order_accounts(&mut accounts, &config.strategy, request_index);
    let mut last_error = "没有可用的账号可供网关使用".to_string();

    for (index, account) in accounts.iter().enumerate() {
        match refresh_token_by_provider(account).await {
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

                persist_account_refresh(
                    account,
                    &refresh,
                    usage_data.clone(),
                    is_banned,
                    is_auth_error,
                );

                if (is_banned || is_auth_error) && index + 1 < accounts.len() {
                    last_error = format!("账号 {} 已不可用，尝试下一个账号", account.label);
                    continue;
                }

                if let Some(usage_data) = usage_data {
                    if usage_exceeds_threshold(&usage_data, config.threshold)
                        && index + 1 < accounts.len()
                    {
                        last_error = format!(
                            "账号 {} 已达到阈值 {}%，尝试下一个账号",
                            account.label, config.threshold
                        );
                        continue;
                    }
                }

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

                return Ok(UpstreamCredentials {
                    access_token: refresh.access_token,
                    profile_arn,
                    region,
                    source_label: format_managed_upstream_source(config, account),
                    user_agent: build_kiro_custom_user_agent(&machine_id),
                    auth_method: account.auth_method.clone(),
                    send_opt_out: should_send_codewhisperer_optout(),
                });
            }
            Err(error) => {
                last_error = format!(
                    "刷新账号 {} 失败: {}",
                    account.label,
                    sanitize_error(&error)
                );
            }
        }
    }

    Err(last_error)
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
        _ => account_label,
    }
}

fn order_accounts(accounts: &mut [Account], strategy: &str, request_index: u64) {
    match strategy {
        "most_quota" => accounts.sort_by(|left, right| {
            remaining_quota(right)
                .partial_cmp(&remaining_quota(left))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "random" => accounts.shuffle(&mut thread_rng()),
        _ => {
            if !accounts.is_empty() {
                accounts.rotate_left((request_index as usize) % accounts.len());
            }
        }
    }
}

fn persist_account_refresh(
    account: &Account,
    refresh: &RefreshResult,
    usage_data: Option<Value>,
    is_banned: bool,
    is_auth_error: bool,
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
        let _ = store.save_to_file();
    }
}

fn remaining_quota(account: &Account) -> f64 {
    let Some(usage_data) = account.usage_data.as_ref() else {
        return 0.0;
    };
    let Some((current, limit)) = extract_usage_totals(usage_data) else {
        return 0.0;
    };
    if limit <= 0 {
        return 0.0;
    }
    (limit - current).max(0) as f64
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

fn build_responses_response(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
) -> Value {
    build_responses_response_with_ids(
        model,
        aggregated,
        server_tool_calls,
        &format!("resp_{}", short_uuid()),
        &format!("msg_{}", short_uuid()),
        chrono::Utc::now().timestamp(),
    )
}

fn build_responses_response_with_ids(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    server_tool_calls: &[ServerToolCall],
    response_id: &str,
    message_id: &str,
    created_at: i64,
) -> Value {
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
    response_id: &str,
    message_id: &str,
    created_at: i64,
) -> Value {
    json!({
        "type": "response.completed",
        "response": build_responses_response_with_ids(
            model,
            aggregated,
            &[],
            response_id,
            message_id,
            created_at,
        )
    })
}

fn stream_completed_response(
    format: ResponseFormat,
    model: String,
    outcome: ProxyExecutionOutcome,
) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(32);
    tokio::spawn(async move {
        let created_at = chrono::Utc::now().timestamp();
        match format {
            ResponseFormat::Anthropic => {
                let message_id = format!("msg_{}", short_uuid());
                let blocks =
                    build_anthropic_content_blocks(&outcome.aggregated, &outcome.server_tool_calls);
                let start = json!({
                    "type": "message_start",
                    "message": {
                        "id": message_id,
                        "type": "message",
                        "role": "assistant",
                        "content": [],
                        "model": model,
                        "stop_reason": Value::Null,
                        "stop_sequence": Value::Null,
                        "usage": {
                            "input_tokens": outcome.aggregated.input_tokens,
                            "output_tokens": outcome.aggregated.output_tokens
                        }
                    }
                });
                send_event(&tx, Some("message_start"), &start.to_string()).await;
                for (index, block) in blocks.iter().enumerate() {
                    let start_block = match block.block_type.as_str() {
                        "text" => json!({
                            "type": "content_block_start",
                            "index": index,
                            "content_block": { "type": "text", "text": "" }
                        }),
                        "thinking" => json!({
                            "type": "content_block_start",
                            "index": index,
                            "content_block": { "type": "thinking", "thinking": "" }
                        }),
                        _ => json!({
                            "type": "content_block_start",
                            "index": index,
                            "content_block": block
                        }),
                    };
                    send_event(&tx, Some("content_block_start"), &start_block.to_string()).await;
                    if let Some(text) = &block.text {
                        let delta = json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": { "type": "text_delta", "text": text }
                        });
                        send_event(&tx, Some("content_block_delta"), &delta.to_string()).await;
                    } else if let Some(thinking) = &block.thinking {
                        let delta = json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": { "type": "thinking_delta", "thinking": thinking }
                        });
                        send_event(&tx, Some("content_block_delta"), &delta.to_string()).await;
                    }
                    let stop = json!({ "type": "content_block_stop", "index": index });
                    send_event(&tx, Some("content_block_stop"), &stop.to_string()).await;
                }
                let finish = json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": if outcome.aggregated.tool_calls.is_empty() { "end_turn" } else { "tool_use" },
                        "stop_sequence": Value::Null
                    },
                    "usage": { "output_tokens": outcome.aggregated.output_tokens }
                });
                send_event(&tx, Some("message_delta"), &finish.to_string()).await;
                send_event(&tx, Some("message_stop"), "{\"type\":\"message_stop\"}").await;
            }
            ResponseFormat::Responses => {
                let response_id = format!("resp_{}", short_uuid());
                let message_id = format!("msg_{}", short_uuid());
                let response = build_responses_response_with_ids(
                    &model,
                    &outcome.aggregated,
                    &outcome.server_tool_calls,
                    &response_id,
                    &message_id,
                    created_at,
                );
                if let Some(output_text) = response
                    .get("output_text")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                {
                    let delta = json!({
                        "type": "response.output_text.delta",
                        "response_id": response_id,
                        "delta": output_text
                    });
                    send_data(&tx, &delta.to_string()).await;
                }
                let completed = json!({
                    "type": "response.completed",
                    "response": response
                });
                send_data(&tx, &completed.to_string()).await;
                send_data(&tx, "[DONE]").await;
            }
        }
    });

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

fn stream_proxy_response(
    upstream_resp: reqwest::Response,
    format: ResponseFormat,
    model: String,
) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(64);
    tokio::spawn(async move {
        let mut upstream_stream = upstream_resp.bytes_stream();
        let mut buffer = String::new();
        let mut parser = ThinkingParser::new();
        let mut aggregated = stream::AggregatedKiroResponse::default();
        let mut tool_accumulators: HashMap<String, (String, String)> = HashMap::new();
        let mut message_started = false;
        let mut next_block_index = 0usize;
        let mut text_block_index: Option<usize> = None;
        let mut thinking_block_index: Option<usize> = None;
        let mut tool_block_indexes: HashMap<String, usize> = HashMap::new();
        let mut saw_tool_calls = false;
        let mut input_tokens = 0i32;
        let mut output_tokens = 0i32;
        let anthropic_id = format!("msg_{}", short_uuid());
        let response_id = format!("resp_{}", short_uuid());
        let message_id = format!("msg_{}", short_uuid());
        let created_at = chrono::Utc::now().timestamp();
        let mut responses_sequence_number = 0usize;

        while let Some(chunk_result) = upstream_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    while let Some(start) = buffer.find('{') {
                        let remaining = &buffer[start..];
                        let Some(json_str) = extract_json(remaining) else {
                            break;
                        };
                        let json_len = json_str.len();
                        if let Some(event) = parse_kiro_event_full(&json_str) {
                            match event {
                                KiroEvent::Usage {
                                    input_tokens: input,
                                    output_tokens: output,
                                } => {
                                    input_tokens = input;
                                    output_tokens = output;
                                    aggregated.input_tokens = input;
                                    aggregated.output_tokens = output;
                                }
                                KiroEvent::ContextUsage { percentage } => {
                                    aggregated.context_usage_percentage = Some(percentage);
                                    if matches!(format, ResponseFormat::Anthropic) {
                                        let data =
                                            json!({"type":"context_usage","percentage":percentage});
                                        send_event(&tx, Some("context_usage"), &data.to_string())
                                            .await;
                                    }
                                }
                                KiroEvent::Thinking(text) => {
                                    aggregated.thinking.push_str(&text);
                                    handle_stream_text(
                                        &tx,
                                        format,
                                        &model,
                                        &anthropic_id,
                                        &response_id,
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
                                    tool_accumulators
                                        .entry(id.clone())
                                        .or_insert((name.clone(), String::new()));
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
                                            close_content_block(&tx, &mut text_block_index).await;
                                            close_content_block(&tx, &mut thinking_block_index)
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
                                            let data = json!({
                                                "type": "response.output_item.added",
                                                "response_id": response_id,
                                                "item": {
                                                    "type": "function_call",
                                                    "call_id": id,
                                                    "name": name,
                                                    "arguments": ""
                                                }
                                            });
                                            send_data(&tx, &data.to_string()).await;
                                        }
                                    }
                                }
                                KiroEvent::ToolUseInputDelta { id, input_delta } => {
                                    if let Some((_, current_input)) = tool_accumulators.get_mut(&id)
                                    {
                                        current_input.push_str(&input_delta);
                                    } else {
                                        tool_accumulators.insert(
                                            id.clone(),
                                            (String::new(), input_delta.clone()),
                                        );
                                    }
                                    match format {
                                        ResponseFormat::Anthropic => {
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
                                        if let Some((name, input)) = tool_accumulators.remove(&id) {
                                            aggregated.tool_calls.push((id.clone(), name, input));
                                        }
                                        if let Some(index) = tool_block_indexes.remove(&id) {
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
                                        if let Some((name, input)) = tool_accumulators.remove(&id) {
                                            aggregated.tool_calls.push((id.clone(), name, input));
                                        }
                                        let data = json!({
                                            "type": "response.output_item.done",
                                            "response_id": response_id,
                                            "call_id": id
                                        });
                                        send_data(&tx, &data.to_string()).await;
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
                                            close_content_block(&tx, &mut thinking_block_index)
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
                                                let data = build_responses_annotation_added_event(
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
                            }
                        }
                        buffer = buffer[start + json_len..].to_string();
                    }
                }
                Err(error) => {
                    let data = json!({"type":"error","message":sanitize_error(&format!("流式读取失败: {error}"))});
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
        aggregated.tool_calls = stream::deduplicate_tool_calls(aggregated.tool_calls);

        match format {
            ResponseFormat::Anthropic => {
                close_content_block(&tx, &mut text_block_index).await;
                close_content_block(&tx, &mut thinking_block_index).await;
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
                let completed = build_stream_responses_completed_event(
                    &model,
                    &aggregated,
                    &response_id,
                    &message_id,
                    created_at,
                );
                send_data(&tx, &completed.to_string()).await;
                send_data(&tx, "[DONE]").await;
            }
        }
    });

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
    }
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
) {
    let chunk = if let Some(event) = event {
        format!("event: {event}\ndata: {payload}\n\n")
    } else {
        format!("data: {payload}\n\n")
    };
    let _ = tx.send(Ok(Bytes::from(chunk))).await;
}

async fn send_data(tx: &mpsc::Sender<Result<Bytes, Infallible>>, payload: &str) {
    send_event(tx, None, payload).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_request_only_accepts_responses_payloads() {
        let responses_payload = json!({
            "model": "claude-3-7-sonnet-20250219",
            "stream": true,
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
        let chat_error = normalize_request(ResponseFormat::Responses, &chat_payload)
            .expect_err("chat payload should be rejected by responses-only normalization");

        assert_eq!(responses_request.model, "claude-3-7-sonnet-20250219");
        assert!(responses_request.stream);
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
        assert!(
            chat_error.contains("/v1/responses"),
            "unexpected error: {chat_error}"
        );
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
        );

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
        };

        let response = build_responses_response_with_ids(
            "gpt-4.1",
            &aggregated,
            &[],
            "resp_test",
            "msg_test",
            123,
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
        };

        let response = build_responses_response_with_ids(
            "gpt-4.1",
            &aggregated,
            &[],
            "resp_test",
            "msg_test",
            123,
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
        };

        let event = build_stream_responses_completed_event(
            "gpt-4.1",
            &aggregated,
            "resp_test",
            "msg_test",
            123,
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
    fn build_responses_annotation_added_event_uses_sdk_shape() {
        let event = build_responses_annotation_added_event(
            "resp_test",
            "msg_test",
            json!({
                "type": "url_citation",
                "url": "https://example.com/rust",
                "citationText": "Rust"
            }),
            2,
            7,
        );

        assert_eq!(event["type"], "response.output_text.annotation.added");
        assert_eq!(event["response_id"], "resp_test");
        assert_eq!(event["item_id"], "msg_test");
        assert_eq!(event["output_index"], 0);
        assert_eq!(event["content_index"], 0);
        assert_eq!(event["annotation_index"], 2);
        assert_eq!(event["sequence_number"], 7);
        assert_eq!(event["annotation"]["type"], "url_citation");
    }

    #[test]
    fn with_kiro_upstream_headers_adds_generate_request_headers() {
        let upstream = UpstreamCredentials {
            access_token: "token-1".to_string(),
            profile_arn: None,
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
        assert_eq!(
            request
                .headers()
                .get("TokenType")
                .and_then(|value| value.to_str().ok()),
            Some("EXTERNAL_IDP")
        );
        assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
    }

    #[test]
    fn with_kiro_upstream_headers_keeps_runtime_requests_minimal() {
        let upstream = UpstreamCredentials {
            access_token: "token-2".to_string(),
            profile_arn: None,
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
    }

    #[test]
    fn with_kiro_upstream_headers_adds_mcp_profile_arn_header() {
        let upstream = UpstreamCredentials {
            access_token: "token-3".to_string(),
            profile_arn: Some(
                "arn:aws:codewhisperer:us-east-1:123456789012:profile/test".to_string(),
            ),
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
    }
}
