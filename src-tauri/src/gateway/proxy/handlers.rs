//! 轻量路由与本地响应：health / models / count_tokens / tokens / OpenAI Chat 入口。

use super::*;

pub fn build_models_response() -> Value {
    serde_json::to_value(ModelsResponse {
        object: "list".to_string(),
        data: get_available_models(),
    })
    .unwrap_or_else(|_| json!({ "object": "list", "data": [] }))
}

pub fn build_health_response() -> Value {
    json!({ "ok": true })
}

/// 获取账号可用模型列表（含上游声明的 AMRF schema）
///
/// 调用 Kiro Management API 的 ListAvailableModels 接口获取账号权限内的模型。
/// 返回完整的模型条目数组而非仅 modelId 列表——`additionalModelRequestFieldsSchema`
/// 里带着该模型支持的 effort 档位枚举与 effort 字段位置（output_config / reasoning），
/// 是档位判定的权威来源。
pub async fn get_available_models_for_upstream(
    upstream: &UpstreamCredentials,
) -> Result<Vec<Value>, String> {
    let client = KiroClient::from_client(upstream.http.clone());
    let (machine_id, profile_arn) = get_available_models_call_context(upstream);

    let response = client
        .list_available_models(
            &upstream.access_token,
            machine_id,
            &upstream.region,
            profile_arn,
        )
        .await?;

    // 解析返回的模型列表
    let models = response
        .get("models")
        .or_else(|| response.get("availableModels"))
        .and_then(|v| v.as_array())
        .ok_or("Invalid response: missing models array")?
        .clone();

    Ok(models)
}

pub fn get_available_models_call_context(upstream: &UpstreamCredentials) -> (&str, Option<&str>) {
    (
        upstream.machine_id.as_str(),
        upstream.available_models_profile_arn.as_deref(),
    )
}

pub async fn generate_local_response(
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
        upstream_source_hint: None,
        region_hint: None,
        started_at,
        request_body,
        request_body_hint: None,
        model_hint: None,
        is_stream: None,
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
        let message = format!("访问地址 {} 不在2API白名单中", client_addr.ip());
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
        None, // error_type
        Some(serialized.as_str()),
        None, // input_tokens
        None, // output_tokens
        None, // cache_read_input_tokens
        None, // cache_creation_input_tokens
        &state,
    );
    Json(response_body).into_response()
}

pub async fn health_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
) -> Response {
    generate_local_response(
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
    generate_local_response(
        state,
        client_addr,
        headers,
        "models",
        None,
        build_models_response(),
    )
    .await
}

pub async fn anthropic_count_tokens_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    payload: Value,
) -> Response {
    let request_body = payload.to_string();
    generate_local_response(
        state,
        client_addr,
        headers,
        "count_tokens",
        Some(request_body.as_str()),
        build_count_tokens_response(&payload),
    )
    .await
}

pub async fn openai_tokens_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    payload: Value,
) -> Response {
    let request_body = payload.to_string();
    generate_local_response(
        state,
        client_addr,
        headers,
        "tokens",
        Some(request_body.as_str()),
        build_openai_tokens_response(&payload),
    )
    .await
}

pub async fn openai_chat_handler(
    state: RouterState,
    client_addr: SocketAddr,
    headers: HeaderMap,
    payload: Value,
) -> Response {
    // Convert OpenAI Chat format to Anthropic Messages format
    let converted_payload = match normalize_openai_chat_payload(&payload) {
        Ok(p) => p,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("content-type", "application/json")
                .body(
                    json!({
                        "error": {
                            "message": format!("Invalid OpenAI Chat request: {}", e),
                            "type": "invalid_request_error"
                        }
                    })
                    .to_string()
                    .into(),
                )
                .unwrap();
        }
    };

    // Convert NormalizedRequest back to Value for proxy_handler
    let payload_value = match serde_json::to_value(&converted_payload) {
        Ok(v) => v,
        Err(e) => {
            log::error!("[网关] 序列化转换后的请求失败: {}", e);
            let error_body = json!({
                "error": {
                    "message": format!("Failed to serialize converted request: {}", e),
                    "type": "internal_error"
                }
            })
            .to_string();
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("content-type", "application/json")
                .body(Body::from(error_body))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Internal Server Error"))
                        .unwrap()
                });
        }
    };

    // Call proxy_handler with the converted payload
    proxy_handler(
        state,
        client_addr,
        headers,
        payload_value,
        ResponseFormat::OpenAI,
    )
    .await
}

pub fn normalize_request(format: ResponseFormat, payload: &Value) -> Result<NormalizedRequest, String> {
    match format {
        ResponseFormat::Anthropic => {
            let request: AnthropicMessagesRequest = serde_json::from_value(payload.clone())
                .map_err(|error| format!("Anthropic 请求解析失败: {error}"))?;
            Ok(normalize_anthropic_request(&request))
        }
        ResponseFormat::Responses => normalize_openai_responses_request(payload),
        ResponseFormat::OpenAI => {
            let request: OpenAIChatRequest = serde_json::from_value(payload.clone())
                .map_err(|error| format!("OpenAI 请求解析失败: {error}"))?;
            crate::gateway::converter::normalize_openai_chat_request(&request)
        }
    }
}
