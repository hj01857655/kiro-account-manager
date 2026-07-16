use crate::{
    db,
    error::{AppError, AppResult},
    kiro,
    models::{AccountRecord, AccountSecrets},
    state::AppState,
};
use async_stream::stream;
use axum::{
    body::Body,
    extract::State,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use chrono::Utc;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{convert::Infallible, time::Instant};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum Protocol {
    Anthropic,
    OpenAi,
    Responses,
}

pub async fn list_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    authorize(&state, &headers)?;
    let (record, secrets) = select_account(&state, preferred_account(&headers), None).await?;
    let upstream = kiro::request_models(&record, &secrets)
        .await
        .map_err(gateway_error)?;
    let mut ids = Vec::new();
    collect_model_ids(&upstream, &mut ids);
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        ids.extend(["auto", "claude-sonnet-4.5", "claude-sonnet-4.5-thinking"].map(str::to_string));
    }
    Ok(Json(json!({
        "object": "list",
        "data": ids.into_iter().map(|id| json!({ "id": id, "object": "model", "created": 0, "owned_by": "kiro" })).collect::<Vec<_>>()
    })))
}

pub async fn messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> AppResult<Response> {
    proxy(state, headers, body, Protocol::Anthropic, "/v1/messages").await
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> AppResult<Response> {
    proxy(
        state,
        headers,
        body,
        Protocol::OpenAi,
        "/v1/chat/completions",
    )
    .await
}

pub async fn responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> AppResult<Response> {
    proxy(state, headers, body, Protocol::Responses, "/v1/responses").await
}

async fn proxy(
    state: AppState,
    headers: HeaderMap,
    request: Value,
    protocol: Protocol,
    endpoint: &'static str,
) -> AppResult<Response> {
    authorize(&state, &headers)?;
    let started = Instant::now();
    let model = request
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("auto")
        .to_string();
    let stream_requested = request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let prompt = extract_prompt(&request, protocol)?;
    if prompt.len() > 400 * 1024 {
        return Err(AppError::bad_request("gateway prompt exceeds 400 KiB"));
    }
    let preferred = preferred_account(&headers);
    let (mut record, mut secrets) = select_account(&state, preferred.clone(), None).await?;
    let mut upstream = send_upstream(&record, &secrets, &model, &prompt).await?;
    if state.config.gateway_auto_switch && should_switch(upstream.status()) {
        let failed_id = record.id.clone();
        record_failure(&state, &failed_id, upstream.status().as_u16()).await;
        let next = select_account(&state, preferred, Some(&failed_id)).await;
        if let Ok(selected) = next {
            (record, secrets) = selected;
            upstream = send_upstream(&record, &secrets, &model, &prompt).await?;
        }
    }
    let status = upstream.status();
    if !status.is_success() {
        record_metric(
            &state,
            endpoint,
            &model,
            Some(&record.id),
            status.as_u16(),
            started.elapsed().as_millis() as i64,
        )
        .await;
        return Err(AppError::new(
            StatusCode::BAD_GATEWAY,
            "gateway_upstream_error",
            format!("Kiro upstream returned HTTP {}", status.as_u16()),
        ));
    }
    record_success(&state, &record.id).await;
    if stream_requested {
        Ok(stream_response(
            state, upstream, protocol, endpoint, model, record.id, started,
        ))
    } else {
        let bytes = upstream
            .bytes()
            .await
            .map_err(|_| gateway_error("failed to read Kiro response"))?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(gateway_error("Kiro response exceeded 16 MiB"));
        }
        let (text, input_tokens, output_tokens) = aggregate_eventstream(&bytes);
        record_metric(
            &state,
            endpoint,
            &model,
            Some(&record.id),
            200,
            started.elapsed().as_millis() as i64,
        )
        .await;
        Ok(non_stream_response(
            protocol,
            &model,
            text,
            input_tokens,
            output_tokens,
        ))
    }
}

async fn send_upstream(
    record: &AccountRecord,
    secrets: &AccountSecrets,
    requested_model: &str,
    prompt: &str,
) -> AppResult<reqwest::Response> {
    let access_token = secrets
        .access_token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::bad_request("selected account has no access token"))?;
    let region = record
        .region
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("us-east-1");
    let model = map_model(requested_model);
    let conversation_id = Uuid::new_v4().to_string();
    let mut payload = json!({
        "conversationState": {
            "chatTriggerType": "MANUAL",
            "conversationId": conversation_id,
            "agentContinuationId": Uuid::new_v4().to_string(),
            "agentTaskType": "vibe",
            "currentMessage": { "userInputMessage": {
                "content": prompt,
                "modelId": model,
                "origin": "AI_EDITOR"
            }}
        }
    });
    if let Some(profile_arn) = record
        .profile_arn
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        payload["profileArn"] = Value::String(profile_arn.clone());
    }
    let mut request = kiro::client_for(record, secrets)
        .map_err(gateway_error)?
        .post(format!(
            "https://runtime.{region}.kiro.dev/generateAssistantResponse"
        ))
        .bearer_auth(access_token)
        .header("content-type", "application/json")
        .header("accept", "application/vnd.amazon.eventstream")
        .header(
            "user-agent",
            format!("KiroIDE-0.6.18-{}", record.machine_id),
        )
        .header(
            "x-amz-user-agent",
            format!("aws-sdk-rust/1.0 md/kiro#{}", record.machine_id),
        )
        .header("amz-sdk-invocation-id", Uuid::new_v4().to_string())
        .header("amz-sdk-request", "attempt=1; max=3")
        .header("x-amzn-kiro-agent-mode", "vibe");
    if let Some(profile_arn) = record
        .profile_arn
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        request = request.header("x-amzn-kiro-profile-arn", profile_arn);
    }
    request
        .json(&payload)
        .send()
        .await
        .map_err(|_| gateway_error("failed to reach Kiro runtime"))
}

fn stream_response(
    state: AppState,
    response: reqwest::Response,
    protocol: Protocol,
    endpoint: &'static str,
    model: String,
    account_id: String,
    started: Instant,
) -> Response {
    let mut source = response.bytes_stream();
    let id = Uuid::new_v4().simple().to_string();
    let model_for_stream = model.clone();
    let output = stream! {
        if matches!(protocol, Protocol::Anthropic) {
            yield Ok::<Bytes, Infallible>(Bytes::from(format!("event: message_start\ndata: {}\n\n", json!({"type":"message_start","message":{"id":format!("msg_{id}"),"type":"message","role":"assistant","model":model_for_stream,"content":[],"stop_reason":null,"usage":{"input_tokens":0,"output_tokens":0}}}))));
            yield Ok(Bytes::from("event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n"));
        }
        let mut buffer = Vec::new();
        while let Some(chunk) = source.next().await {
            let Ok(chunk) = chunk else { break; };
            buffer.extend_from_slice(&chunk);
            for payload in take_payloads(&mut buffer) {
                if let Some(text) = extract_event_text(&payload) {
                    let event = stream_event(protocol, &id, &model_for_stream, &text);
                    yield Ok(Bytes::from(event));
                }
            }
            if buffer.len() > 16 * 1024 * 1024 { break; }
        }
        yield Ok(Bytes::from(stream_end(protocol, &id, &model_for_stream)));
        record_metric(&state, endpoint, &model_for_stream, Some(&account_id), 200, started.elapsed().as_millis() as i64).await;
    };
    let mut response = Body::from_stream(output).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream; charset=utf-8"),
    );
    response.headers_mut().insert(
        "cache-control",
        HeaderValue::from_static("no-cache, no-transform"),
    );
    response
        .headers_mut()
        .insert("x-accel-buffering", HeaderValue::from_static("no"));
    response
}

fn stream_event(protocol: Protocol, id: &str, model: &str, text: &str) -> String {
    match protocol {
        Protocol::OpenAi => format!(
            "data: {}\n\n",
            json!({"id":format!("chatcmpl-{id}"),"object":"chat.completion.chunk","created":Utc::now().timestamp(),"model":model,"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]})
        ),
        Protocol::Anthropic => format!(
            "event: content_block_delta\ndata: {}\n\n",
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":text}})
        ),
        Protocol::Responses => format!(
            "event: response.output_text.delta\ndata: {}\n\n",
            json!({"type":"response.output_text.delta","item_id":format!("msg_{id}"),"output_index":0,"content_index":0,"delta":text})
        ),
    }
}

fn stream_end(protocol: Protocol, id: &str, model: &str) -> String {
    match protocol {
        Protocol::OpenAi => format!("data: {}\n\ndata: [DONE]\n\n", json!({"id":format!("chatcmpl-{id}"),"object":"chat.completion.chunk","created":Utc::now().timestamp(),"model":model,"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})),
        Protocol::Anthropic => "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":0}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n".to_string(),
        Protocol::Responses => format!("event: response.completed\ndata: {}\n\n", json!({"type":"response.completed","response":{"id":format!("resp_{id}"),"object":"response","status":"completed","model":model}})),
    }
}

fn non_stream_response(
    protocol: Protocol,
    model: &str,
    text: String,
    input_tokens: i64,
    output_tokens: i64,
) -> Response {
    let id = Uuid::new_v4().simple().to_string();
    let value = match protocol {
        Protocol::OpenAi => {
            json!({"id":format!("chatcmpl-{id}"),"object":"chat.completion","created":Utc::now().timestamp(),"model":model,"choices":[{"index":0,"message":{"role":"assistant","content":text},"finish_reason":"stop"}],"usage":{"prompt_tokens":input_tokens,"completion_tokens":output_tokens,"total_tokens":input_tokens+output_tokens}})
        }
        Protocol::Anthropic => {
            json!({"id":format!("msg_{id}"),"type":"message","role":"assistant","model":model,"content":[{"type":"text","text":text}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":input_tokens,"output_tokens":output_tokens}})
        }
        Protocol::Responses => {
            json!({"id":format!("resp_{id}"),"object":"response","created_at":Utc::now().timestamp(),"status":"completed","model":model,"output":[{"id":format!("msg_{id}"),"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":text,"annotations":[]}]}],"usage":{"input_tokens":input_tokens,"output_tokens":output_tokens,"total_tokens":input_tokens+output_tokens}})
        }
    };
    Json(value).into_response()
}

async fn select_account(
    state: &AppState,
    preferred: Option<String>,
    excluded: Option<&str>,
) -> AppResult<(AccountRecord, AccountSecrets)> {
    let selected_id = preferred
        .or_else(|| state.config.gateway_default_account.clone())
        .filter(|id| excluded != Some(id.as_str()));
    let record = if let Some(id) = selected_id {
        let account = db::get_account_record(&state.pool, &id).await?;
        if account.enabled == 0
            || account.status != "active"
            || excluded == Some(account.id.as_str())
        {
            return Err(AppError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "gateway_account_unavailable",
                "configured gateway account is unavailable",
            ));
        }
        account
    } else {
        sqlx::query_as::<_, AccountRecord>("SELECT * FROM accounts WHERE enabled=1 AND status='active' AND (? IS NULL OR id<>?) ORDER BY failure_count ASC, success_count ASC, updated_at ASC LIMIT 1")
            .bind(excluded).bind(excluded).fetch_optional(&state.pool).await?
            .ok_or_else(|| AppError::new(StatusCode::SERVICE_UNAVAILABLE, "no_gateway_account", "no available gateway account"))?
    };
    let secrets = db::decrypt_secrets(&state.crypto, &record)?;
    Ok((record, secrets))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    if !state.config.gateway_enabled {
        return Err(AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "gateway_disabled",
            "gateway is disabled",
        ));
    }
    let expected = state
        .config
        .gateway_api_key
        .as_deref()
        .ok_or_else(AppError::internal)?;
    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .trim();
    if supplied.len() != expected.len()
        || !bool::from(supplied.as_bytes().ct_eq(expected.as_bytes()))
    {
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_gateway_key",
            "invalid gateway API key",
        ));
    }
    Ok(())
}

fn preferred_account(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-account-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_prompt(request: &Value, protocol: Protocol) -> AppResult<String> {
    let mut parts = Vec::new();
    match protocol {
        Protocol::OpenAi | Protocol::Anthropic => {
            if let Some(system) = request.get("system") {
                append_content(&mut parts, "System", system);
            }
            for item in request
                .get("messages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let role = item.get("role").and_then(Value::as_str).unwrap_or("user");
                if let Some(content) = item.get("content") {
                    append_content(&mut parts, role, content);
                }
            }
        }
        Protocol::Responses => {
            if let Some(input) = request.get("input") {
                append_content(&mut parts, "user", input);
            }
        }
    }
    if let Some(instructions) = request.get("instructions").and_then(Value::as_str) {
        parts.insert(0, format!("System: {instructions}"));
    }
    if let Some(tools) = request
        .get("tools")
        .and_then(Value::as_array)
        .filter(|tools| !tools.is_empty())
    {
        parts.push(format!(
            "Available tools (return a concise JSON tool call when needed): {}",
            Value::Array(tools.clone())
        ));
    }
    if parts.is_empty() {
        return Err(AppError::bad_request("request contains no message content"));
    }
    Ok(parts.join("\n\n"))
}

fn append_content(parts: &mut Vec<String>, role: &str, content: &Value) {
    match content {
        Value::String(text) if !text.trim().is_empty() => parts.push(format!("{role}: {text}")),
        Value::Array(items) => {
            for item in items {
                if let Some(text) = item
                    .get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
                {
                    if !text.trim().is_empty() {
                        parts.push(format!("{role}: {text}"));
                    }
                }
            }
        }
        _ => {}
    }
}

fn map_model(model: &str) -> String {
    match model.trim().to_ascii_lowercase().as_str() {
        "gpt-4" | "gpt-4o" | "gpt-4-turbo" | "gpt-4o-mini" | "claude-3-5-sonnet-latest" => {
            "claude-sonnet-4.5".to_string()
        }
        "claude-sonnet-4-5" => "claude-sonnet-4.5".to_string(),
        "claude-haiku-4-5" => "claude-haiku-4.5".to_string(),
        "" => "auto".to_string(),
        value => value.replace("-thinking", ""),
    }
}

fn should_switch(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 401 | 402 | 403 | 423 | 429) || status.is_server_error()
}

fn take_payloads(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut output = Vec::new();
    let mut consumed = 0;
    while buffer.len().saturating_sub(consumed) >= 12 {
        let frame = &buffer[consumed..];
        let total = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        let headers = u32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]) as usize;
        if !(16..=16 * 1024 * 1024).contains(&total) || 12 + headers > total - 4 {
            consumed += 1;
            continue;
        }
        if frame.len() < total {
            break;
        }
        output.push(frame[12 + headers..total - 4].to_vec());
        consumed += total;
    }
    if consumed > 0 {
        buffer.drain(..consumed);
    }
    output
}

fn aggregate_eventstream(bytes: &[u8]) -> (String, i64, i64) {
    let mut buffer = bytes.to_vec();
    let mut text = String::new();
    let mut input = 0;
    let mut output = 0;
    for payload in take_payloads(&mut buffer) {
        if let Some(delta) = extract_event_text(&payload) {
            text.push_str(&delta);
        }
        if let Ok(value) = serde_json::from_slice::<Value>(&payload) {
            input = find_i64(&value, &["inputTokens", "input_tokens"]).unwrap_or(input);
            output = find_i64(&value, &["outputTokens", "output_tokens"]).unwrap_or(output);
        }
    }
    if text.is_empty() {
        if let Ok(value) = serde_json::from_slice::<Value>(bytes) {
            text = extract_text_value(&value).unwrap_or_default();
        }
    }
    (text, input, output)
}

fn extract_event_text(payload: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(payload)
        .ok()
        .and_then(|value| extract_text_value(&value))
}

fn extract_text_value(value: &Value) -> Option<String> {
    value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("content"))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("contentBlockDelta")
                .and_then(|item| item.get("delta"))
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("delta")
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str)
        })
        .or_else(|| value.get("content").and_then(Value::as_str))
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

fn find_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    if let Value::Object(object) = value {
        for key in keys {
            if let Some(number) = object.get(*key).and_then(Value::as_i64) {
                return Some(number);
            }
        }
        for child in object.values() {
            if let Some(number) = find_i64(child, keys) {
                return Some(number);
            }
        }
    }
    None
}

fn collect_model_ids(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for key in ["modelId", "model_id"] {
                if let Some(id) = object.get(key).and_then(Value::as_str) {
                    ids.push(id.to_string());
                }
            }
            for child in object.values() {
                collect_model_ids(child, ids);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_model_ids(child, ids);
            }
        }
        _ => {}
    }
}

async fn record_metric(
    state: &AppState,
    endpoint: &str,
    model: &str,
    account_id: Option<&str>,
    status: u16,
    duration_ms: i64,
) {
    let _ = sqlx::query("INSERT INTO gateway_metrics (endpoint,model,account_id,status_code,duration_ms,created_at) VALUES (?,?,?,?,?,?)")
        .bind(endpoint).bind(model).bind(account_id).bind(i64::from(status)).bind(duration_ms).bind(db::now()).execute(&state.pool).await;
}

async fn record_failure(state: &AppState, account_id: &str, status: u16) {
    let _ = sqlx::query("UPDATE accounts SET failure_count=failure_count+1,last_failure_at=?,disabled_reason=?,updated_at=? WHERE id=?")
        .bind(db::now()).bind(format!("gateway upstream HTTP {status}")).bind(db::now()).bind(account_id).execute(&state.pool).await;
}

async fn record_success(state: &AppState, account_id: &str) {
    let _ = sqlx::query("UPDATE accounts SET success_count=success_count+1,disabled_reason=NULL,updated_at=? WHERE id=?").bind(db::now()).bind(account_id).execute(&state.pool).await;
}

fn gateway_error(message: impl Into<String>) -> AppError {
    AppError::new(StatusCode::BAD_GATEWAY, "gateway_error", message)
}

#[cfg(test)]
mod tests {
    use super::{map_model, take_payloads};

    #[test]
    fn maps_common_openai_alias() {
        assert_eq!(map_model("gpt-4o"), "claude-sonnet-4.5");
    }

    #[test]
    fn incomplete_eventstream_frame_is_retained() {
        let mut buffer = vec![0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(take_payloads(&mut buffer).is_empty());
        assert_eq!(buffer.len(), 12);
    }
}
