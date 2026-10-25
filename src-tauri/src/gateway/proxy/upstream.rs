//! 主代理流程：proxy_handler 及其上游调用、上游请求头注入。

use super::*;

/// 对请求 payload 做隐私遮蔽：逐字段递归处理所有字符串值
/// （messages / system / tools / input 全部覆盖）。
///
/// 返回 (遮蔽后的 payload, 是否触发 Block)。Block 由调用方负责
/// fail-closed（拒绝请求），本函数只如实报告。
pub fn mask_payload_for_upstream(
    payload: &Value,
    engine: &mut crate::gateway::masking::MaskEngine,
) -> (Value, bool) {
    let mut blocked = false;
    let masked = mask_value(payload, engine, &mut blocked);
    (masked, blocked)
}

/// 递归遮蔽 JSON 值里的所有字符串。
fn mask_value(
    value: &Value,
    engine: &mut crate::gateway::masking::MaskEngine,
    blocked: &mut bool,
) -> Value {
    match value {
        Value::String(s) => {
            let outcome = engine.mask_text(s);
            if outcome.blocked {
                *blocked = true;
            }
            Value::String(outcome.text)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|item| mask_value(item, engine, blocked))
                .collect(),
        ),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k.clone(), mask_value(v, engine, blocked));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// 收集 payload 遮蔽时的审计条目（只含元数据，不含真值）。
///
/// 与 `mask_payload_for_upstream` 分开跑：遮蔽本身已把 payload 换成占位符版，
/// 这里对**原始 payload** 再扫一遍只为拿条目（遮蔽幂等——同一值第二次
/// 命中的占位符与第一次一致，重扫不会污染映射表）。
fn collect_mask_items(
    payload: &Value,
    engine: &mut crate::gateway::masking::MaskEngine,
) -> Vec<crate::gateway::masking::MaskRecordItem> {
    let mut items = Vec::new();
    collect_items_recursive(payload, engine, &mut items);
    items
}

fn collect_items_recursive(
    value: &Value,
    engine: &mut crate::gateway::masking::MaskEngine,
    items: &mut Vec<crate::gateway::masking::MaskRecordItem>,
) {
    match value {
        Value::String(s) => {
            let outcome = engine.mask_text(s);
            items.extend(outcome.items);
        }
        Value::Array(arr) => {
            for item in arr {
                collect_items_recursive(item, engine, items);
            }
        }
        Value::Object(map) => {
            for (_, v) in map {
                collect_items_recursive(v, engine, items);
            }
        }
        _ => {}
    }
}

/// 按协议跑字段消费判定，命中 `Rejected` 就返回 (字段名, 客户端可见原因)。
///
/// 顶层字段与嵌套路径都查：Responses 的 `text.format` 是两层结构，
/// 只在顶层扫会漏掉它。
pub fn reject_unsupported_fields(
    format: ResponseFormat,
    payload: &Value,
) -> Option<(String, String)> {
    let protocol = match format {
        ResponseFormat::Anthropic => crate::gateway::field_support::Protocol::AnthropicMessages,
        ResponseFormat::OpenAI => {
            crate::gateway::field_support::Protocol::OpenAIChatCompletions
        }
        ResponseFormat::Responses => {
            crate::gateway::field_support::Protocol::OpenAIResponses
        }
    };

    if let Some(hit) = crate::gateway::field_support::find_rejected_field(protocol, payload) {
        return Some(hit);
    }

    // 嵌套路径：目前只有 Responses 的 text.format
    if protocol == crate::gateway::field_support::Protocol::OpenAIResponses {
        if let Some(hit) = crate::gateway::field_support::find_rejected_path(
            protocol,
            payload,
            &["text", "format"],
        ) {
            return Some(hit);
        }
    }

    None
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
    // 一次请求一个 id：JSONL 与各 *.log dump 共用，排障时能把它们对上
    let request_id = uuid::Uuid::new_v4().to_string();

    // 低频收敛日志体积：每 256 个请求扫一次日志目录，避免每次请求都做磁盘扫描。
    // index 为 0 时也会执行，可顺带处理启动前就已超限的历史文件（如已涨到 221MB 的 kiro-request.log）。
    if request_index % 256 == 0 {
        crate::gateway::enforce_raw_log_caps();
    }

    let endpoint = get_request_endpoint(format);
    let get_client_log_prefix = get_client_log_prefix(format);
    let started_at = Instant::now();
    let raw_request_body = payload.to_string();

    // 写入客户端请求到日志文件
    {
        let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let body_end = safe_truncate(&raw_request_body, 50000);
        let entry = format!(
            "[{}] kind=client_request idx={} request_id={} endpoint={} bytes={} truncated={} body={}\n",
            chrono::Local::now().format("%H:%M:%S"),
            request_index,
            request_id,
            endpoint,
            raw_request_body.len(),
            body_end < raw_request_body.len(),
            &raw_request_body[..body_end]
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(format!("{get_client_log_prefix}-request.log")))
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    }

    // 认证提前到日志上下文构造之前：拒绝时无需写入 Key 归因信息。
    let model_hint = extract_model_from_payload(&raw_request_body);
    if let Err(message) = verify_client_auth(&headers, &state.config) {
        let sanitized = sanitize_error(&message);
        return gateway_error_with_log(
            &state,
            format,
            &RequestLogContext {
                request_index,
                endpoint,
                client_addr,
                request: None,
                upstream: None,
                upstream_source_hint: None,
                region_hint: None,
                started_at,
                request_body: Some(raw_request_body.as_str()),
                request_body_hint: None,
                model_hint: model_hint.clone(),
                is_stream: None,
                client_user_agent: None,
                request_id: request_id.clone(),
            },
            GatewayErrorDetails {
                status: StatusCode::UNAUTHORIZED,
                error_type: "authentication_error",
                message: &sanitized,
                response_body: None,
            },
        )
        .await;
    }

    // 隐私遮蔽：对发给上游的 payload 做值替换（本地 *.log 已保留原文）。
    // Block 命中即 fail-closed：拒绝请求，绝不带敏感值发出去。
    let payload = if state.config.privacy_mask_enabled {
        let (masked_payload, blocked, items) = {
            let mut engine = state.mask_engine.lock().await;
            let (masked_payload, blocked) = mask_payload_for_upstream(&payload, &mut engine);
            // 重新扫一遍拿审计条目：mask_payload_for_upstream 只回文本与
            // blocked 标志，条目在遮蔽过程中产生。这里用 items 收集版重跑
            // 一遍（遮蔽幂等：第二次跑时值已在映射表里，占位符一致）。
            let audit_items = collect_mask_items(&payload, &mut engine);
            (masked_payload, blocked, audit_items)
        };

        // 有命中（Mask 或 Warn）就记审计：记录只含元数据，不含真值
        if !items.is_empty() {
            use crate::gateway::masking::MaskAuditEntry;
            let preview = safe_truncate(&masked_payload.to_string(), 2048).to_string();
            let audit = state.mask_audit.clone();
            let entry = MaskAuditEntry {
                occurred_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                request_id: request_id.clone(),
                endpoint: endpoint.to_string(),
                client_user_agent: extract_client_user_agent(&headers),
                items,
                masked_preview: preview,
            };
            tokio::spawn(async move {
                audit.record(entry);
            });
        }

        if blocked {
            let sanitized = "请求包含被策略禁止外发的敏感内容，已拒绝".to_string();
            return gateway_error_with_log(
                &state,
                format,
                &RequestLogContext {
                    request_index,
                    endpoint,
                    client_addr,
                    request: None,
                    upstream: None,
                    upstream_source_hint: None,
                    region_hint: None,
                    started_at,
                    request_body: Some(raw_request_body.as_str()),
                    request_body_hint: None,
                    model_hint: model_hint.clone(),
                    is_stream: None,
                    client_user_agent: None,
                    request_id: request_id.clone(),
                },
                GatewayErrorDetails {
                    status: StatusCode::FORBIDDEN,
                    error_type: "permission_error",
                    message: &sanitized,
                    response_body: None,
                },
            )
            .await;
        }
        masked_payload
    } else {
        payload
    };

    let client_user_agent = extract_client_user_agent(&headers);

    let base_log_context = RequestLogContext {
        request_index,
        endpoint,
        client_addr,
        request: None,
        upstream: None,
        upstream_source_hint: None,
        region_hint: None,
        started_at,
        request_body: Some(raw_request_body.as_str()),
        request_body_hint: None,
        model_hint,
        is_stream: None,
        client_user_agent,
        request_id: request_id.clone(),
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
        let message = format!("访问地址 {} 不在 kiro2api 白名单中", client_addr.ip());
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

    // 字段消费判定：明确不支持的字段返回 400，而不是静默丢弃。
    // 放在 normalize 之前——被拒的字段根本不该进入后续流程。
    // reason 文案已自带字段名（判定表逐条写死），此处只把字段名落进日志，
    // 便于事后统计「客户端最常踩哪个字段」。
    if let Some((field, reason)) = reject_unsupported_fields(format, &payload) {
        log::info!("[网关] 拒绝不支持的字段: {} ({})", field, reason);
        return gateway_error_with_log(
            &state,
            format,
            &base_log_context,
            GatewayErrorDetails {
                status: StatusCode::BAD_REQUEST,
                error_type: "invalid_request_error",
                message: &reason,
                response_body: None,
            },
        )
        .await;
    }

    // 可选：通过 x-account-id 指定本次请求使用的账号（须在当前路由可用集合内）
    let preferred_account_id = extract_preferred_account_id(&headers);
    if let Some(ref account_id) = preferred_account_id {
        log::info!("[网关] 请求指定账号: {}", account_id);
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

    // 模型名不在这里改写：客户端要么点名 Kiro 原生模型（含 GPT-5.6 系列），
    // 要么由 converter::get_internal_model_id 归一化并在不认识时兜底。
    // 历史上有过一层用户可配的「模型映射」规则，因源模型清单是编造的
    // （gpt-5.5/5.4/5.3 等上游并不存在）且与 Anthropic 协议路径不一致，已移除。

    // 添加详细的请求日志（参考 Kiro-account-manager 的日志设计）
    let messages_count = request.messages.len();
    let tools_count = request.tools.as_ref().map(|t| t.len()).unwrap_or(0);
    let has_tool_choice = request.tool_choice.is_some();
    let content_length: usize = request
        .messages
        .iter()
        .filter_map(|m| m.content.as_ref())
        .map(|c| c.to_string().len())
        .sum();

    log::info!(
        "[请求详情] 请求 #{} | 模型={} | 流式={} | 消息数={} | 工具数={} | 工具选择={} | 内容长度={}",
        request_index,
        request.model,
        request.stream,
        messages_count,
        tools_count,
        has_tool_choice,
        content_length
    );
    let mut request = if matches!(format, ResponseFormat::Responses) {
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

    // Token 估算和裁剪（在创建 log context 之前）
    // 应用系统提示过滤
    let has_filters = state.config.filter_claude_code
        || state.config.filter_strip_boundaries
        || state.config.filter_env_noise
        || !state.config.prompt_filter_rules.is_empty();
    if has_filters {
        for msg in &mut request.messages {
            if msg.role == "system" {
                if let Some(serde_json::Value::String(text)) = &msg.content {
                    let filtered = crate::gateway::prompt_filter::apply_prompt_filters(&state.config, text);
                    msg.content = Some(serde_json::Value::String(filtered));
                }
            }
        }
    }

    // ===== 响应缓存：查找 =====
    // 仅对非流式请求尝试缓存命中
    let cache_session_id = extract_session_id_from_request(&request).unwrap_or_default();
    let messages_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(request.model.as_bytes());
        for msg in &request.messages {
            hasher.update(msg.role.as_bytes());
            if let Some(content) = &msg.content {
                hasher.update(content.to_string().as_bytes());
            }
        }
        if let Some(tools) = &request.tools {
            hasher.update(serde_json::to_string(tools).unwrap_or_default().as_bytes());
        }
        format!("{:x}", hasher.finalize())
    };
    let cache_message_count = request.messages.len();
    let cache_total_chars: usize = request
        .messages
        .iter()
        .filter_map(|m| m.content.as_ref())
        .map(|c| c.to_string().len())
        .sum();

    if !request.stream {
        let mut cache_guard = state.response_cache.lock().await;
        if let Some(cached) = cache_guard.get(
            &cache_session_id,
            &messages_hash,
            cache_message_count,
            cache_total_chars,
        ) {
            drop(cache_guard);
            log::info!(
                "[响应缓存] 命中! session={}, hash={}, 响应长度={}",
                &cache_session_id[..cache_session_id.len().min(16)],
                &messages_hash[..16],
                cached.response.len()
            );

            // 从缓存构建响应
            if let Ok(cached_response) = serde_json::from_str::<Value>(&cached.response) {
                // 记录缓存命中日志
                let cache_log_context = RequestLogContext {
                    request: Some(&request),
                    ..base_log_context.clone()
                };
                write_request_log(
                    &cache_log_context,
                    StatusCode::OK,
                    "success (cached)",
                    None,
                    None,
                    Some(&cached.response),
                    Some(cached.input_tokens),
                    Some(cached.output_tokens),
                    None,
                    None,
                    None, // 缓存命中不重新计费
                    None, // 缓存命中不重算上下文占用
                    &state,
                );
                return Json(cached_response).into_response();
            }
            // 缓存内容解析失败，继续正常流程
            log::warn!("[响应缓存] 缓存内容解析失败，走正常请求流程");
        } else {
            drop(cache_guard);
        }
    }

    // 创建 log context
    // （原在 upstream 解析前创建；WebSearch 子请求需要在拿到凭证后原地改写 request，
    //  为避免可变借用冲突，把绑定挪到改写完成之后——见下方。）

    let preferred_account_id_ref = preferred_account_id.as_deref();
    let client_ip = client_addr.ip().to_string();
    let upstream = match resolve_upstream_credentials(
        &state.config,
        &state,
        preferred_account_id_ref,
        &client_ip,
    )
    .await
    {
        Ok(creds) => creds,
        Err(message) => {
            let request_log_context = RequestLogContext {
                request: Some(&request),
                ..base_log_context.clone()
            };
            // 如果是 token refresh 429，尝试换一个账号而不是直接返回错误
            // 指定了 x-account-id 时不换号，直接返回限流错误
            if preferred_account_id_ref.is_none()
                && (message.contains("429")
                    || message.to_lowercase().contains("too many requests"))
            {
                log::warn!("[Gateway] Token 刷新被限流，尝试换账号: {}", sanitize_error(&message));
                match resolve_upstream_credentials(&state.config, &state, None, &client_ip).await {
                    Ok(creds) => creds,
                    Err(retry_message) => {
                        let sanitized = sanitize_error(&retry_message);
                        return gateway_error_with_log(
                            &state,
                            format,
                            &request_log_context,
                            GatewayErrorDetails {
                                status: StatusCode::TOO_MANY_REQUESTS,
                                error_type: "rate_limit_error",
                                message: &sanitized,
                                response_body: None,
                            },
                        )
                        .await;
                    }
                }
            } else {
                // 结构化错误：按类型取状态码与 error_type，不再解析字符串前缀。
                // 解析失败（历史遗留的裸字符串）时退回 401，与迁移前行为一致。
                let (status, error_type, display_message) = match classify_gateway_error(&message) {
                    Some(err) => (err.status(), err.error_type(), err.message()),
                    None => (
                        StatusCode::UNAUTHORIZED,
                        "authentication_error",
                        message.clone(),
                    ),
                };
                let display_message = display_message.as_str();

                let sanitized = sanitize_error(&display_message);
                return gateway_error_with_log(
                    &state,
                    format,
                    &request_log_context,
                    GatewayErrorDetails {
                        status,
                        error_type,
                        message: &sanitized,
                        response_body: None,
                    },
                )
                .await;
            }
        }
    };
    let response_id = format!("resp_{}", short_uuid());

    // ===== WebSearch 强制工具子请求（Claude Code 内置搜索）=====
    // 检测 tool_choice 强制 web_search + 服务端工具定义（归一化后服务端工具已剥离，
    // 名单在 server_tool_names）；命中则用当前账号凭证调 Kiro 远程 MCP
    // （runtime /mcp tools/call）执行搜索，把结果注入请求后剥掉工具，
    // 复用既有流式/非流式管线返回纯文本。见 websearch.rs 模块文档。
    if state.config.web_search_enabled
        && super::websearch::is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names,
        )
    {
        super::websearch::handle_forced_web_search(&upstream, &mut request).await;
    }

    // ===== Tool Search 拆分（Anthropic defer_loading 工具发现规范）=====
    // 把 defer_loading 工具挪到 deferred_tools（响应侧代答环的检索池），并
    // 注入合成 tool_search 工具。非流式走下方代答环；流式走缓冲代答环
    // （见流式分支 run_tool_search_buffered_loop），客户端仍收标准 SSE，
    // 仅首包延迟增加缓冲与代答耗时。
    if state.config.tool_search_enabled && !request.deferred_tool_names.is_empty() {
        if let Some(tools) = request.tools.as_mut() {
            request.deferred_tools = super::tool_search::split_deferred_tools(
                tools,
                &request.deferred_tool_names,
                request.tool_choice.as_ref(),
            );
        }
    }

    let request_log_context = RequestLogContext {
        request: Some(&request),
        ..base_log_context.clone()
    };
    let message_id = format!("msg_{}", short_uuid());
    let created_at = chrono::Utc::now().timestamp();

    let upstream_log_context = RequestLogContext {
        upstream: Some(&upstream),
        ..request_log_context.clone()
    };

    // 获取账号可用模型列表（用于模型降级与 effort 档位判定）
    let available_models = match get_available_models_for_upstream(&upstream).await {
        Ok(models) => {
            log::debug!(
                "[Gateway] 账号 {} 可用模型: {:?}",
                upstream.source_label,
                extract_model_ids(&models)
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

    let upstream_payload = match build_kiro_payload_with_tool_limit(
        &state.http,
        &request,
        upstream.profile_arn.clone(),
        available_models.as_deref(),
        state.config.max_tool_description_length,
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

    let original_size = get_payload_size(&payload_value);
    if original_size > MAX_KIRO_PAYLOAD_SIZE {
        log::info!(
            "[网关] Payload 大小 {} 字节超过限制 {} 字节。裁剪历史记录...",
            original_size,
            MAX_KIRO_PAYLOAD_SIZE
        );
        let trimmed = trim_kiro_payload_history(&mut payload_value, MAX_KIRO_PAYLOAD_SIZE);
        if trimmed {
            let final_size = get_payload_size(&payload_value);
            log::info!(
                "[网关] Payload 从 {} 字节裁剪到 {} 字节",
                original_size,
                final_size
            );
        }
    }

    // 方案 3：二次检查 payload 大小，确保裁剪后仍然符合限制
    let mut payload_json = serde_json::to_string(&payload_value).unwrap_or_else(|_| String::new());
    let mut payload_size = payload_json.len();

    if payload_size > MAX_KIRO_PAYLOAD_SIZE {
        log::warn!(
            "[网关] 裁剪后 payload 大小 {} 字节仍超过限制 {} 字节，继续裁剪...",
            payload_size,
            MAX_KIRO_PAYLOAD_SIZE
        );

        // 继续裁剪，直到满足大小限制
        let mut retry_count = 0;
        const MAX_TRIM_RETRIES: u32 = 5;

        while payload_size > MAX_KIRO_PAYLOAD_SIZE && retry_count < MAX_TRIM_RETRIES {
            retry_count += 1;
            let trimmed = trim_kiro_payload_history(&mut payload_value, MAX_KIRO_PAYLOAD_SIZE);

            if !trimmed {
                log::error!(
                    "[网关] 无法继续裁剪 payload（第 {} 次尝试），可能历史记录已为空",
                    retry_count
                );
                break;
            }

            payload_json = serde_json::to_string(&payload_value).unwrap_or_else(|_| String::new());
            let new_size = payload_json.len();

            log::info!(
                "[网关] 第 {} 次裁剪：payload 从 {} 字节减少到 {} 字节",
                retry_count,
                payload_size,
                new_size
            );

            if new_size >= payload_size {
                log::error!(
                    "[网关] 裁剪无效，payload 大小未减少（{} -> {} 字节）",
                    payload_size,
                    new_size
                );
                break;
            }

            payload_size = new_size;
        }

        let final_payload_size = get_payload_size(&payload_value);
        if final_payload_size > MAX_KIRO_PAYLOAD_SIZE {
            log::error!(
                "[网关] 多次裁剪后 payload 大小 {} 字节仍超过限制 {} 字节",
                final_payload_size,
                MAX_KIRO_PAYLOAD_SIZE
            );
        } else {
            log::info!(
                "[网关] 多次裁剪成功，最终 payload 大小 {} 字节",
                final_payload_size
            );
        }
    }

    let upstream_request_body = serde_json::to_string_pretty(&payload_value)
        .unwrap_or_else(|_| "[failed to serialize upstream payload]".to_string());
    let upstream_payload_log_context = RequestLogContext {
        request_body: Some(upstream_request_body.as_str()),
        ..upstream_log_context.clone()
    };

    // 账号重试循环：持续尝试所有账号，直到成功
    // 对于可重试错误（429/402），在尝试完所有账号后等待一段时间再重试
    let mut account_attempt = 0;
    let mut retry_round = 0;
    let mut tried_account_ids: HashSet<String> = HashSet::new();
    let mut token_refreshed_account_ids: HashSet<String> = HashSet::new();
    let mut next_upstream_override: Option<UpstreamCredentials> = None;
    let mut last_retriable_error: Option<(StatusCode, String, String, Option<String>)> = None;
    // 服务端限流响应头 x-amzn-kiro-ratelimit-retry-after 的毫秒值（00 号语料 §2.3）
    let mut rate_limit_retry_after_ms: Option<u64> = None;
    let mut consecutive_auth_failures = 0;
    const MAX_AUTH_FAILURES: u32 = 5; // 连续认证失败次数上限

    // 获取可用账号数量，用于判断何时需要等待
    // 指定了 x-account-id 时固定为 1，避免失败后自动换到其他账号
    let available_account_count = if preferred_account_id_ref.is_some() {
        1
    } else {
        // 只读：match 直接产出计数值，guard 在本块结束时释放，不跨后续 await。
        let store = auth::lock_shared_store(&state.store);
        match state.config.account_mode.as_str() {
            "single" => store
                .accounts
                .iter()
                .filter(|account| {
                    state.config.account_id.as_deref() == Some(account.id.as_str())
                        && account.is_available()
                        && account.enabled
                })
                .count(),
            "group" => store
                .accounts
                .iter()
                .filter(|account| {
                    state.config.group_id.as_deref() == account.group_id.as_deref()
                        && account.is_available()
                        && account.enabled
                })
                .count(),
            "pool" => store
                .accounts
                .iter()
                .filter(|account| {
                    state.config.pool_account_ids.contains(&account.id)
                        && account.is_available()
                        && account.enabled
                })
                .count(),
            _ => 0,
        }
        .max(1) // 至少假设有1个账号
    };

    log::info!(
        "[Gateway] 开始请求，可用账号数: {}{}",
        available_account_count,
        preferred_account_id_ref
            .map(|id| format!(" (指定账号: {})", id))
            .unwrap_or_default()
    );

    let (upstream_resp, successful_upstream) = loop {
        account_attempt += 1;

        // 如果尝试次数超过账号数量，说明本轮所有账号都试过了
        if account_attempt > available_account_count as u32 {
            retry_round += 1;
            account_attempt = 1; // 重置计数器，开始新一轮

            // 如果有可重试错误（429/402/401），等待后重试
            if let Some((status, _, _, _)) = &last_retriable_error {
                if *status == StatusCode::TOO_MANY_REQUESTS || *status == StatusCode::PAYMENT_REQUIRED || *status == StatusCode::UNAUTHORIZED {
                    // 429：优先采用服务端 x-amzn-kiro-ratelimit-retry-after 指定的窗口
                    //（上限 5 分钟防病态值）；402/401 沿用固定递增退避
                    let wait = if *status == StatusCode::TOO_MANY_REQUESTS {
                        rate_limit_retry_after_ms
                            .filter(|ms| *ms > 0)
                            .map(|ms| ms.min(RATE_LIMIT_RETRY_AFTER_MAX_MS))
                            .unwrap_or_else(|| 5_000 * retry_round as u64)
                    } else {
                        5_000 * retry_round as u64
                    };
                    log::warn!(
                        "[Gateway] 所有账号都返回 {} 错误，等{} 秒后重试 (第{} 轮{})",
                        status.as_u16(),
                        wait / 1000,
                        retry_round + 1,
                        if *status == StatusCode::TOO_MANY_REQUESTS && rate_limit_retry_after_ms.is_some() {
                            "（采用服务端 retry-after 窗口）"
                        } else {
                            ""
                        }
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(wait)).await;

                    // 清空已尝试账号列表，重新尝试所有账号
                    tried_account_ids.clear();
                    continue;
                }
            }

            // 如果连续多次认证失败（非429/402/401），可能所有账号都不可用
            consecutive_auth_failures += 1;
            if consecutive_auth_failures >= MAX_AUTH_FAILURES {
                log::error!(
                    "[Gateway] 连续 {} 轮认证失败，停止重试",
                    MAX_AUTH_FAILURES
                );

                // 如果有保存的错误详情，透传；否则返回通用认证错误
                if let Some((status, error_type, message, response_body)) = last_retriable_error {
                    return gateway_error_with_log(
                        &state,
                        format,
                        &upstream_payload_log_context,
                        GatewayErrorDetails {
                            status,
                            error_type: &error_type,
                            message: &message,
                            response_body: response_body.as_deref(),
                        },
                    )
                    .await;
                } else {
                    return gateway_error_with_log(
                        &state,
                        format,
                        &upstream_payload_log_context,
                        GatewayErrorDetails {
                            status: StatusCode::UNAUTHORIZED,
                            error_type: "authentication_error",
                            message: "所有可用账号均无法完成请求，请检查账号状态",
                            response_body: None,
                        },
                    )
                    .await;
                }
            }

            // 清空已尝试账号列表，重新尝试
            tried_account_ids.clear();
        }

        // 如果不是第一次尝试，需要重新选择账号
        let current_upstream = if let Some(creds) = next_upstream_override.take() {
            tried_account_ids.insert(extract_account_id_from_upstream(&creds));
            creds
        } else if account_attempt > 1 {
            match resolve_upstream_credentials(
                &state.config,
                &state,
                preferred_account_id_ref,
                &client_ip,
            )
            .await
            {
                Ok(creds) => {
                    // 检查是否已经尝试过这个账号
                    let account_id = extract_account_id_from_upstream(&creds);
                    if tried_account_ids.contains(&account_id) {
                        log::warn!(
                            "[Gateway] 账号 {} 已尝试过，继续尝试下一个 (尝试: {}/{})",
                            creds.source_label,
                            account_attempt,
                            available_account_count
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
                        available_account_count,
                        sanitized
                    );
                    // 如果是账号不可用，继续尝试
                    if message.contains("未找到符合 kiro2api 配置的可用账号") {
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
        match call_generate_assistant_response(
            &current_upstream,
            &payload_value,
            upstream_payload_log_context.request_index as usize,
            &upstream_payload_log_context.request_id,
        )
        .await
        {
            Ok(resp) => {
                // 请求成功，退出重试循环
                break (resp, current_upstream);
            }
            Err(e) => {
                let (status, error_type, message, upstream_response_body) =
                    (e.status, e.error_type, e.message.clone(), e.response_body.clone());
                let e_rate_limit_retry_after_ms = e.rate_limit_retry_after_ms;
                // 检查是否是 429 错误
                if status == StatusCode::TOO_MANY_REQUESTS {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 记录服务端指定的限流等待窗口（毫秒），全账号试完后优先于固定退避
                    rate_limit_retry_after_ms = e_rate_limit_retry_after_ms;

                    // 保存最后一个 429 错误详情，以便最终透传
                    last_retriable_error = Some((
                        status,
                        error_type.to_string(),
                        message.clone(),
                        upstream_response_body.clone(),
                    ));

                    // 标记账号为速率限制
                    state.load_balancer.mark_rate_limited(&account_id).await;
                    state.load_balancer.record_failure(&account_id).await;

                    log::warn!(
                        "[Gateway] 账号 {} 返回 429 错误，标记为速率限制并切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        available_account_count
                    );

                    // 继续尝试下一个账号
                    continue;
                }

                // 403 + bearer token invalid/expired：先刷新当前账号 token，再用同一账号重试一次。
                if status == StatusCode::FORBIDDEN && error_type == "token_expired_error" {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 注意：token_expired_error 不保存到 last_retriable_error，
                    // 因为它是账号级别的问题，不是临时的限流/配额问题。
                    // 如果所有账号都 token invalid，应该返回认证错误，而不是透传单个账号的错误。

                    if token_refreshed_account_ids.insert(account_id.clone()) {
                        log::warn!(
                            "[Gateway] 账号 {} 返回 token 失效，刷新 token 后重试同一账号 (尝试: {}/{})",
                            current_upstream.source_label,
                            account_attempt,
                            available_account_count
                        );

                        match force_refresh_upstream_credentials(
                            &state.config,
                            &state,
                            &current_upstream,
                        )
                        .await
                        {
                            Ok(refreshed_upstream) => {
                                next_upstream_override = Some(refreshed_upstream);
                                continue;
                            }
                            Err(error) => {
                                state.load_balancer.record_failure(&account_id).await;
                                log::warn!(
                                    "[Gateway] 账号 {} token 刷新失败，切换账号: {}",
                                    current_upstream.source_label,
                                    sanitize_error(&error)
                                );
                                continue;
                            }
                        }
                    }

                    state.load_balancer.record_failure(&account_id).await;
                    log::warn!(
                        "[Gateway] 账号 {} 刷新后仍返回 token 失效，切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        available_account_count
                    );
                    continue;
                }

                // 403 + 封禁：对话接口的 403 是唯一可靠判据——
                // ListAvailableModels / getUsageLimits 的 403 是偶发的，同一次刷新可能漏判。
                // error_type 由 map_upstream_error 统一判定（errors.rs:29-32、:68-70），
                // 这里不再自行解析 body，避免两处判据漂移。
                if status == StatusCode::FORBIDDEN && error_type == "account_banned_error" {
                    let account_id = extract_account_id_from_upstream(&current_upstream);
                    log::warn!(
                        "[Gateway] 账号 {} 返回封禁 403，标记为 banned 并切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        available_account_count
                    );

                    {
                        let mut store = auth::lock_shared_store(&state.store);
                        if let Some(account) =
                            store.accounts.iter_mut().find(|a| a.id == account_id)
                        {
                            // 幂等：已 banned 的账号不重复写盘，避免每次重试都落一次
                            if account.status != "banned" {
                                update_account_status(account, true, false); // is_banned = true
                                if let Err(e) = store.try_save_to_file() {
                                    log::error!("[Gateway] 保存账号封禁状态失败: {}", e);
                                }
                            }
                        }
                    }

                    state.load_balancer.record_failure(&account_id).await;

                    // 封禁是账号级结论，不进重试轮：直接切号
                    continue;
                }

                // 检查是否是 402 配额不足错误
                if status == StatusCode::PAYMENT_REQUIRED {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 保存最后一个配额不足错误详情
                    last_retriable_error = Some((
                        status,
                        error_type.to_string(),
                        message.clone(),
                        upstream_response_body.clone(),
                    ));

                    // 标记账号为配额不足并切换账号
                    state.load_balancer.record_failure(&account_id).await;

                    log::warn!(
                        "[Gateway] 账号 {} 返回 402 配额不足，切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        available_account_count
                    );

                    // 继续尝试下一个账号
                    continue;
                }

                // 检查是否是账户封禁错误 (403 + BANNED: 前缀)
                if status == StatusCode::FORBIDDEN && error_type == "account_banned_error" {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 保存最后一个封禁错误详情
                    last_retriable_error = Some((
                        status,
                        error_type.to_string(),
                        message.clone(),
                        upstream_response_body.clone(),
                    ));

                    // 标记账号为封禁（永久不可用）
                    state.load_balancer.mark_account_banned(&account_id).await;

                    log::warn!(
                        "[Gateway] 账号 {} 被封禁，标记为不可用并切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        available_account_count
                    );

                    // 继续尝试下一个账号
                    continue;
                }

                // 401 认证错误：直接标记账号为 invalid 并切换
                if status == StatusCode::UNAUTHORIZED {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 保存最后一个 401 错误详情
                    last_retriable_error = Some((
                        status,
                        error_type.to_string(),
                        message.clone(),
                        upstream_response_body.clone(),
                    ));

                    // 标记账号为 invalid（不可用）
                    log::warn!(
                        "[Gateway] 账号 {} 返回 401 认证错误，标记为 invalid 并切换账号 (尝试: {}/{})",
                        current_upstream.source_label,
                        account_attempt,
                        available_account_count
                    );

                    // 更新账号状态为 invalid
                    // 401 是账号级问题，落到共享副本上，避免与 Tauri 命令互相覆盖。
                    {
                        let mut store = auth::lock_shared_store(&state.store);
                        if let Some(account) =
                            store.accounts.iter_mut().find(|a| a.id == account_id)
                        {
                            update_account_status(account, false, true); // is_auth_error = true
                            if let Err(e) = store.try_save_to_file() {
                                log::error!("[Gateway] 保存账号状态失败: {}", e);
                            }
                        }
                    }

                    state.load_balancer.record_failure(&account_id).await;

                    if let Some(ref body) = upstream_response_body {
                        log::debug!("[Gateway] 401 完整响应体: {}", body);
                    }

                    // 继续尝试下一个账号
                    continue;
                }

                // 其他错误：记录并切换到下一个账号
                let account_id = extract_account_id_from_upstream(&current_upstream);

                // 保存最后一个错误详情，以便最终透传
                last_retriable_error = Some((
                    status,
                    error_type.to_string(),
                    message.clone(),
                    upstream_response_body.clone(),
                ));

                state.load_balancer.record_failure(&account_id).await;

                log::warn!(
                    "[Gateway] 账号 {} 返回错误 (状态: {}, 类型: {}, 消息: {}), 切换账号 (尝试: {}/{})",
                    current_upstream.source_label,
                    status,
                    error_type,
                    message,
                    account_attempt,
                    available_account_count
                );

                if let Some(ref body) = upstream_response_body {
                    log::debug!("[Gateway] 完整响应体: {}", body);
                }

                // 继续尝试下一个账号
                continue;
            }
        }
    };

    // 连接计数从这里才开始覆盖真实请求周期。
    //
    // 流式路径把 guard 移进 stream_proxy_response 的流式任务，保证活到流结束；
    // 非流式路径由本函数持有到返回。两条路径都覆盖 panic / 提前 return / 断连。
    //
    // 硬约束：**首字节已发出后绝不换号重发**。此处所有换号 / 重试都发生在
    // `break` 拿到 upstream_resp 之前，而流式响应直到 break 之后才开始写客户端，
    // 因此约束由循环结构天然保证。改动这个 loop 前先确认这一点。
    let connection_guard = state
        .load_balancer
        .acquire_connection(&successful_upstream.account_id)
        .await;

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
            upstream_source_hint: Some(successful_upstream.source_label.clone()),
            region_hint: Some(successful_upstream.region.clone()),
            started_at: upstream_payload_log_context.started_at,
            request_body: None,
            request_body_hint: upstream_payload_log_context
                .request_body
                .map(str::to_string),
            model_hint: upstream_payload_log_context.model_hint.clone(),
            is_stream: Some(true),
            client_user_agent: upstream_payload_log_context.client_user_agent.clone(),
            request_id: upstream_payload_log_context.request_id.clone(),
        };

        // ===== Tool Search 缓冲代答环（流式）=====
        // defer_loading 工具存在时：先缓冲整轮上游 EventStream，执行与非流式
        // 同构的代答环（模型 tool_search 调用由网关检索回灌后重发），最后一轮
        // 的原始字节重建为上游响应交既有流式管线增量转换——客户端仍收标准
        // SSE，仅首包延迟包含缓冲与代答耗时。working 为 owned 副本，不与
        // 日志上下文的共享借用冲突。
        let (upstream_resp, stream_messages) = if !request.deferred_tools.is_empty() {
            let mut working = request.clone();
            let resp = match run_tool_search_buffered_loop(
                &state,
                &mut working,
                upstream_resp,
                &successful_upstream,
                available_models.as_deref(),
                request_index,
                &request_id,
            )
            .await
            {
                Ok(resp) => resp,
                Err(message) => {
                    let sanitized = sanitize_error(&message);
                    return gateway_error_with_log(
                        &state,
                        format,
                        &upstream_payload_log_context,
                        GatewayErrorDetails {
                            status: StatusCode::BAD_GATEWAY,
                            error_type: "api_error",
                            message: &sanitized,
                            response_body: None,
                        },
                    )
                    .await;
                }
            };
            (resp, working.messages.clone())
        } else {
            (upstream_resp, request.messages.clone())
        };

        return stream_proxy_response(
            state.clone(),
            upstream_resp,
            format,
            request.model.clone(),
            stream_messages,
            request.tools.clone(),
            request.tool_choice.clone(),
            request.previous_response_id.clone(),
            request.tool_name_map.clone(),
            request.include_usage,
            request.stop.clone(),
            static_log_context,
            connection_guard,
        );
    }

    // 非流式响应也是 EventStream 格式，需要解码
    let raw_bytes = match upstream_resp.bytes().await {
        Ok(bytes) => bytes,
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

    // 添加调试日志：只记录原始响应体大小，不打印响应内容
    log::debug!("[非流式响应] 原始字节大小: {} 字节", raw_bytes.len(),);

    // 解码 EventStream 消息并提取所有 JSON payload
    let mut buffer = raw_bytes.to_vec();
    let mut json_payloads = Vec::new();
    let mut message_count = 0;

    loop {
        match decode_message(&buffer) {
            Ok(Some((msg, consumed_bytes))) => {
                message_count += 1;
                let message_type = msg.headers.get(":message-type").map(String::as_str);
                let event_type = msg.headers.get(":event-type").map(String::as_str);

                log::info!(
                    "[非流式响应] 消息 #{}: type={:?}, event={:?}, payload_size={} 字节",
                    message_count,
                    message_type,
                    event_type,
                    msg.payload.len()
                );

                // 检查错误消息
                if matches!(message_type, Some("error") | Some("exception")) {
                    let error_text = String::from_utf8_lossy(&msg.payload);
                    let detected_error = detect_upstream_error_body(&error_text);
                    let parsed_error_type = detected_error
                        .as_ref()
                        .map(|(_, error_type, _)| *error_type)
                        .unwrap_or("unknown");
                    log::error!(
                        "EventStream 上游错误: message_type={:?}, event_type={:?}, payload_bytes={}, parsed_error_type={}",
                        message_type,
                        event_type,
                        msg.payload.len(),
                        parsed_error_type
                    );

                    if let Some((status, error_type, message)) = detected_error {
                        return gateway_error_with_log(
                            &state,
                            format,
                            &upstream_payload_log_context,
                            GatewayErrorDetails {
                                status,
                                error_type,
                                message: &message,
                                response_body: Some(&error_text),
                            },
                        )
                        .await;
                    }
                }

                // 只处理事件类型的消息
                if matches!(message_type, Some("event")) {
                    let json_text = String::from_utf8_lossy(&msg.payload);
                    let event_name = serde_json::from_str::<Value>(&json_text)
                        .ok()
                        .and_then(|value| {
                            value
                                .as_object()
                                .and_then(|object| object.keys().next().cloned())
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    log::info!(
                        "[Non-Stream Response] Event payload: event={}, payload_bytes={}, payload_chars={}",
                        event_name,
                        msg.payload.len(),
                        json_text.chars().count()
                    );
                    json_payloads.push(json_text.to_string());
                }

                buffer.drain(..consumed_bytes);
            }
            Ok(None) => {
                // 缓冲区数据不足，已处理完所有消息
                log::info!(
                    "[非流式响应] EventStream 解码完成，剩余缓冲区: {} 字节",
                    buffer.len()
                );
                break;
            }
            Err(e) => {
                log::error!(
                    "EventStream 解码失败: {}, 剩余缓冲区: {} 字节",
                    e,
                    buffer.len()
                );
                break;
            }
        }
    }

    // 用于调试日志的拼接字符串
    let mut body = json_payloads.join("");

    // 添加调试日志：只记录解码后的 JSON 数量和长度，不打印内容
    log::info!(
        "[非流式响应] 解码了 {} 条 EventStream 消息, 总 body 长度: {} 字符",
        json_payloads.len(),
        body.len()
    );

    let mut aggregated = stream::aggregate_kiro_response_from_payloads(&json_payloads);

    // ===== Tool Search 代答环（非流式）=====
    // 模型调用合成 tool_search 工具时由网关执行检索，把 tool_use + tool_result
    // 追加进消息后重发上游；最多 3 轮，防止模型反复检索打转。环退出后
    // aggregated 即最终响应，沿用下方既有转换管线（token 估算 / 停止序列 /
    // 协议转换对环无感知：输入 token 估算会自动计上环内追加的消息）。
    // 重发失败按 502 上抛——此时把 tool_search tool_use 透传给客户端只会得到
    // "未知工具"错误，两者都失败，5xx 至少可诊断。
    //
    // 借用约束：request_log_context 持有 &request 直到函数尾，环内**不得
    // 可变借用 request**——所有变更落在 owned 副本（final_messages /
    // re_request），消费点（token 估算 / responses 会话持久化）统一读
    // final_messages。每轮只追加本轮的 tool_use/tool_result 对：跨轮
    // 累积会把同一对消息重复发给上游（重复 tool_use_id）。
    let mut final_messages = request.messages.clone();
    if !request.deferred_tools.is_empty() {
        const TOOL_SEARCH_MAX_ROUNDS: usize = 3;
        for round in 0..TOOL_SEARCH_MAX_ROUNDS {
            let Some((turn_messages, answer)) =
                build_tool_search_turn(&aggregated, &request.deferred_tools)
            else {
                break;
            };
            log::info!(
                "[Tool Search] 第 {} 轮代答：回灌 {} 字节",
                round + 1,
                answer.len()
            );
            // 直接落 final_messages，本轮消息下一轮整份重发，不另做累积容器。
            final_messages.extend(turn_messages);

            // 重发请求视图：原始请求 + 环内追加（owned，不碰 request）
            let mut re_request = request.clone();
            re_request.messages = final_messages.clone();

            // 重建 payload（同账号单次重发，不进限流换号环）
            let re_payload = match build_kiro_payload_with_tool_limit(
                &state.http,
                &re_request,
                successful_upstream.profile_arn.clone(),
                available_models.as_deref(),
                state.config.max_tool_description_length,
            )
            .await
            {
                Ok(payload) => payload,
                Err(message) => {
                    let sanitized = sanitize_error(&message);
                    return gateway_error_with_log(
                        &state,
                        format,
                        &upstream_payload_log_context,
                        GatewayErrorDetails {
                            status: StatusCode::BAD_GATEWAY,
                            error_type: "api_error",
                            message: &sanitized,
                            response_body: None,
                        },
                    )
                    .await;
                }
            };
            let mut re_value = serde_json::to_value(&re_payload).unwrap_or_else(|_| json!({}));
            if get_payload_size(&re_value) > MAX_KIRO_PAYLOAD_SIZE {
                trim_kiro_payload_history(&mut re_value, MAX_KIRO_PAYLOAD_SIZE);
            }

            match call_generate_assistant_response(
                &successful_upstream,
                &re_value,
                request_index as usize,
                &request_id,
            )
            .await
            {
                Ok(re_resp) => match re_resp.bytes().await {
                    Ok(bytes) => {
                        // 重解码 EventStream 并聚合（与首响解码同构；错误路径
                        // 收敛为 break 上浮错误日志，不做换号重试）
                        json_payloads = decode_event_payloads(&bytes);
                        body = json_payloads.join("");
                        aggregated =
                            stream::aggregate_kiro_response_from_payloads(&json_payloads);
                    }
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
                },
                Err(error) => {
                    let message =
                        sanitize_error(&format!("Tool Search 代答重发失败: {}", error.message));
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
            }
        }
    }

    // 直接使用本地估算 token（不依赖响应中的 token 信息）
    log::info!("[非流式响应] 使用本地 token 估算");

    // 估算输入 tokens（从请求消息中）
    let request_text = serde_json::to_string(&final_messages).unwrap_or_default();
    aggregated.input_tokens =
        crate::gateway::token_estimator::estimate_tokens(&request_text, &request.model);

    // 估算输出 tokens（从响应文本中）
    let response_text = format!("{}{}", aggregated.text, aggregated.thinking);
    aggregated.output_tokens =
        crate::gateway::token_estimator::estimate_tokens(&response_text, &request.model);

    log::info!(
        "[非流式响应] 估算的 tokens: input={}, output={} (model={})",
        aggregated.input_tokens,
        aggregated.output_tokens,
        request.model
    );

    // 调试：记录 aggregated 的详细信息
    log::info!(
        "[非流式响应] 聚合详情: text_len={}, thinking_len={}, tool_calls={}, citations={}",
        aggregated.text.len(),
        aggregated.thinking.len(),
        aggregated.tool_calls.len(),
        aggregated.citations.len()
    );

    // Prompt Cache 模拟：如果响应中没有缓存信息，用模拟器填充
    if aggregated.cache_read_input_tokens.is_none()
        && aggregated.cache_creation_input_tokens.is_none()
    {
        let tracker = crate::gateway::prompt_cache::global_prompt_cache_tracker();
        let messages_json: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();
        let tools_json: Option<Vec<serde_json::Value>> = request.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|t| serde_json::to_value(t).unwrap_or_default())
                .collect()
        });

        if let Some(profile) = tracker.build_profile(
            None,
            &messages_json,
            tools_json.as_deref(),
            aggregated.input_tokens as usize,
            &request.model,
        ) {
            let cache_usage = tracker.compute(&request.model, &profile);
            tracker.update(&request.model, &profile);

            if cache_usage.cache_read_input_tokens > 0 {
                aggregated.cache_read_input_tokens =
                    Some(cache_usage.cache_read_input_tokens as i32);
            }
            if cache_usage.cache_creation_input_tokens > 0 {
                aggregated.cache_creation_input_tokens =
                    Some(cache_usage.cache_creation_input_tokens as i32);
            }

            log::info!(
                "[非流式] Prompt Cache 模拟: read={}, creation={}",
                cache_usage.cache_read_input_tokens,
                cache_usage.cache_creation_input_tokens
            );
        }
    }

    // 还原工具名称（sanitized -> original）
    for (_, name, _) in &mut aggregated.tool_calls {
        if let Some(original) = request.tool_name_map.get(name.as_str()) {
            *name = original.clone();
        }
    }

    // stop 序列截断：Kiro 上游无 stop 字段对应物，由网关在响应侧截断
    // （社区 kirocc 同样 "enforces stop_sequences adapter-side"）。
    // 只在可见文本上匹配与截断——thinking 块是模型内部推理，
    // 客户端给的 stop 序列不应在推理内容里生效。
    let mut stop_sequence_hit: Option<String> = None;
    if let Some(stops) = request.stop.as_ref().filter(|list| !list.is_empty()) {
        let visible = stop_sequence::visible_text(&aggregated.text);
        let (truncated, hit) = stop_sequence::truncate_at_stop_sequence(&visible, stops);
        if let Some(sequence) = hit {
            stop_sequence_hit = Some(sequence);
            aggregated.text = truncated;
        }
    }

    let response = match format {
        ResponseFormat::Anthropic => build_anthropic_response_with_stop(
            &request.model,
            &aggregated,
            stop_sequence_hit.as_deref(),
        ),
        ResponseFormat::Responses => build_responses_response_with_ids(
            &request.model,
            &aggregated,
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
            final_messages.clone(),
            request.tools.clone(),
            request.tool_choice.clone(),
            request.previous_response_id.clone(),
            &aggregated,
        )
        .await;
    }
    {
        let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let response_body = serde_json::to_string(&response).unwrap_or_default();
        let body_end = safe_truncate(&response_body, 50000);
        let entry = format!(
            "[{}] kind=client_response idx={} endpoint={} stream=false status=200 bytes={} truncated={} body={}\n",
            chrono::Local::now().format("%H:%M:%S"),
            request_index,
            endpoint,
            response_body.len(),
            body_end < response_body.len(),
            &response_body[..body_end]
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(format!("{}-response.log", get_client_log_prefix)))
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    }
    // ===== 响应缓存：写入（仅非流式成功响应） =====
    {
        let response_json = serde_json::to_string(&response).unwrap_or_default();
        let mut cache_guard = state.response_cache.lock().await;
        cache_guard.put(
            &cache_session_id,
            &messages_hash,
            response_json,
            aggregated.input_tokens,
            aggregated.output_tokens,
            cache_message_count,
            cache_total_chars,
        );
        drop(cache_guard);
        log::debug!(
            "[响应缓存] 已写入: session={}, hash={}",
            &cache_session_id[..cache_session_id.len().min(16)],
            &messages_hash[..16]
        );
    }

    write_request_log(
        &upstream_payload_log_context,
        StatusCode::OK,
        "success",
        None,
        None, // error_type
        Some(body.as_str()),
        Some(aggregated.input_tokens),
        Some(aggregated.output_tokens),
        aggregated.cache_read_input_tokens,
        aggregated.cache_creation_input_tokens,
        aggregated.metering_usage,
        aggregated.context_usage_percentage,
        &state,
    );
    Json(response).into_response()
}

/// 从聚合响应中找出待代答的 tool_search 调用，并生成回灌用的两条消息。
///
/// 非流式代答环与流式缓冲环共用这段语义（流式之前是复制的）：
/// - `find` 只取第一个命中，一个回合内的多次检索由多轮环完成；
/// - assistant 消息保留模型实际调用的名字（可能是官方别名
///   `tool_search_tool_*`），否则历史与模型自述不一致；
/// - tool_result 的 `tool_use_id` 必须与 assistant 的 `id` 相同，上游强校验。
///
/// 返回 `None` 表示本回合没有 tool_search 调用，即环应退出。
pub(crate) fn build_tool_search_turn(
    aggregated: &stream::AggregatedKiroResponse,
    deferred_tools: &[Tool],
) -> Option<(Vec<NormalizedMessage>, String)> {
    let (call_id, call_name, args_json) = aggregated
        .tool_calls
        .iter()
        .find(|(_, name, _)| super::tool_search::is_tool_search_call(name))
        .cloned()?;

    let args: Value = serde_json::from_str(&args_json).unwrap_or_else(|_| json!({}));
    let answer = super::tool_search::execute_tool_search(&args, deferred_tools);

    let messages = vec![
        NormalizedMessage {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(vec![ToolCall {
                id: call_id.clone(),
                call_type: "function".to_string(),
                function: ToolCallFunction {
                    name: call_name,
                    arguments: args_json,
                },
            }]),
            tool_call_id: None,
            metadata: None,
        },
        NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!( [{
                "type": "tool_result",
                "tool_use_id": call_id,
                "content": [{"type": "text", "text": answer}]
            }])),
            tool_calls: None,
            tool_call_id: Some(call_id),
            metadata: None,
        },
    ];
    Some((messages, answer))
}

/// EventStream 字节流 → JSON event payload 列表（只取 `:event-type` 消息）。
/// 非流式代答环与流式缓冲环共用。
fn decode_event_payloads(raw: &[u8]) -> Vec<String> {
    let mut buf = raw.to_vec();
    let mut payloads = Vec::new();
    loop {
        match decode_message(&buf) {
            Ok(Some((msg, consumed))) => {
                if matches!(
                    msg.headers.get(":message-type").map(String::as_str),
                    Some("event")
                ) {
                    payloads.push(String::from_utf8_lossy(&msg.payload).to_string());
                }
                buf.drain(..consumed);
            }
            _ => break,
        }
    }
    payloads
}

/// Tool Search 缓冲代答环（流式）：缓冲整轮上游 EventStream，执行与非流式
/// 同构的代答循环（模型调用合成 tool_search 时由网关检索、回灌、重发），
/// 返回最后一轮原始字节重建的上游响应，交既有流式管线增量转换。
///
/// `working` 为请求的 owned 副本（调用方持有，不与日志上下文的共享借用
/// 冲突）；环内追加的 tool 交互写回其 messages，随流式管线的 token 估算
/// 与日志一起生效。最多 3 轮，防止模型反复检索打转。
async fn run_tool_search_buffered_loop(
    state: &RouterState,
    working: &mut NormalizedRequest,
    first_resp: reqwest::Response,
    upstream: &UpstreamCredentials,
    available_models: Option<&[Value]>,
    request_index: u64,
    request_id: &str,
) -> Result<reqwest::Response, String> {
    const TOOL_SEARCH_MAX_ROUNDS: usize = 3;
    let mut raw = first_resp
        .bytes()
        .await
        .map_err(|e| format!("读取上游响应失败: {e}"))?
        .to_vec();

    for round in 0..TOOL_SEARCH_MAX_ROUNDS {
        let json_payloads = decode_event_payloads(&raw);
        let aggregated = stream::aggregate_kiro_response_from_payloads(&json_payloads);
        let Some((turn_messages, answer)) =
            build_tool_search_turn(&aggregated, &working.deferred_tools)
        else {
            break; // 无 tool_search 调用：当前轮即最终轮
        };
        log::info!(
            "[Tool Search] 流式第 {} 轮代答：回灌 {} 字节",
            round + 1,
            answer.len()
        );
        working.messages.extend(turn_messages);

        // 重建 payload（同账号单次重发，不进限流换号环）
        let re_payload = build_kiro_payload_with_tool_limit(
            &state.http,
            working,
            upstream.profile_arn.clone(),
            available_models,
            state.config.max_tool_description_length,
        )
        .await?;
        let mut re_value = serde_json::to_value(&re_payload).unwrap_or_else(|_| json!({}));
        if get_payload_size(&re_value) > MAX_KIRO_PAYLOAD_SIZE {
            trim_kiro_payload_history(&mut re_value, MAX_KIRO_PAYLOAD_SIZE);
        }

        let re_resp = call_generate_assistant_response(
            upstream,
            &re_value,
            request_index as usize,
            request_id,
        )
        .await
        .map_err(|e| format!("Tool Search 代答重发失败: {}", e.message))?;

        raw = re_resp
            .bytes()
            .await
            .map_err(|e| format!("读取上游响应失败: {e}"))?
            .to_vec();
    }

    // 回放：最后一轮原始字节重建为上游响应，交既有流式管线
    let replay = axum::http::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(
            axum::http::header::CONTENT_TYPE,
            "application/vnd.amazon.eventstream",
        )
        .body(reqwest::Body::from(raw))
        .map_err(|e| format!("重建流式响应失败: {e}"))?;
    Ok(reqwest::Response::from(replay))
}

/// 服务端限流窗口上限（毫秒）：防病态值导致重试环长时间挂起
const RATE_LIMIT_RETRY_AFTER_MAX_MS: u64 = 5 * 60 * 1000;

pub async fn call_generate_assistant_response<T: serde::Serialize + ?Sized>(
    upstream: &UpstreamCredentials,
    upstream_payload: &T,
    request_index: usize,
    request_id: &str,
) -> Result<reqwest::Response, UpstreamRequestError> {
    let upstream_url = build_generate_assistant_response_url(&upstream.region);

    // 追加最新请求到日志文件
    if let Ok(payload_json) = serde_json::to_string(upstream_payload) {
        let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let body_end = safe_truncate(&payload_json, 50000);
        let entry = format!(
            "[{}] kind=kiro_request idx={} request_id={} upstream=generateAssistantResponse bytes={} truncated={} body={}\n",
            chrono::Local::now().format("%H:%M:%S"),
            request_index,
            request_id,
            payload_json.len(),
            body_end < payload_json.len(),
            &payload_json[..body_end]
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("kiro-request.log"))
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    }

    // 重试上界已提到模块级 MAX_RETRIES：amz-sdk-request 头要用同一个数（G-6）
    /// 单次重试退避上限（毫秒）。
    ///
    /// 裸指数退避在最后一次尝试会到 16s，五次累计 31s；封顶后最坏约 19s，
    /// 仍然给上游留出恢复窗口，但不至于让客户端先超时。
    const MAX_RETRY_BACKOFF_MS: u64 = 4_000;
    let mut attempt = 0;

    loop {
        attempt += 1;

        // 有真实 profileArn 就带头：上游 runtime（含 MCP 相关调用）会校验；
        // Enterprise 的真实 ARN 由 ensure_enterprise_profile_arn 在凭证构建时保证
        let include_profile_arn = upstream
            .profile_arn
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        let upstream_resp = add_kiro_upstream_headers(
            upstream.http.post(&upstream_url),
            upstream,
            "application/vnd.amazon.eventstream",
            true,
            true,
            include_profile_arn,
            attempt,
        )
        .json(upstream_payload)
        .send()
        .await
        .map_err(|error| {
            UpstreamRequestError::new(
                StatusCode::BAD_GATEWAY,
                "api_error",
                sanitize_error(&format!("上游请求失败: {error}")),
                None,
            )
        })?;

        let status = upstream_resp.status();

        // 消费 body 前先提取限流头（429 时服务端指定重试窗口，单位毫秒）
        let rate_limit_retry_after_header = upstream_resp
            .headers()
            .get("x-amzn-kiro-ratelimit-retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok());

        if status.is_success() {
            return Ok(upstream_resp);
        }

        let body = upstream_resp.text().await.unwrap_or_default();

        // 追加错误响应到日志文件
        {
            let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
            let body_end = safe_truncate(&body, 50000);
            let entry = format!(
                "[{}] kind=kiro_response idx={} upstream=generateAssistantResponse status={} bytes={} truncated={} body={}\n",
                chrono::Local::now().format("%H:%M:%S"),
                request_index,
                status.as_u16(),
                body.len(),
                body_end < body.len(),
                &body[..body_end]
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_dir.join("kiro-request.log"))
                .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
        }

        // 既保留原始响应体透传，也必须先识别上游语义：
        // 403 bearer token invalid/expired 需要触发账号 token 刷新后重试。
        let (mapped_status, error_type, message) = map_upstream_error(status, &body);

        // 402 配额不足错误不重试，直接返回让外层切换账号
        if mapped_status == StatusCode::PAYMENT_REQUIRED {
            log::warn!("[网关] 上游 402 配额不足，type={}，交给外层切换账号", error_type);
            return Err(UpstreamRequestError::new(mapped_status, error_type, message, Some(body)));
        }

        // 429 限流错误不重试，直接返回让外层切换账号；
        // 提取 Kiro 官方限流头（毫秒）夹带给外层——重试环用它替换固定退避
        if mapped_status == StatusCode::TOO_MANY_REQUESTS {
            log::warn!(
                "[网关] 上游 429 限流，type={}，retry-after={}，交给外层切换账号",
                error_type,
                rate_limit_retry_after_header
                    .map(|ms| format!("{ms}ms"))
                    .unwrap_or_else(|| "无".to_string())
            );
            let mut err = UpstreamRequestError::new(mapped_status, error_type, message, Some(body));
            err.rate_limit_retry_after_ms = rate_limit_retry_after_header;
            return Err(err);
        }

        // 401 认证错误不在 HTTP 层重试；交给外层刷新当前账号 token 或切换账号。
        if mapped_status == StatusCode::UNAUTHORIZED {
            log::warn!("[网关] 上游 401 认证错误，type={}，交给外层处理", error_type);
            return Err(UpstreamRequestError::new(mapped_status, error_type, message, Some(body)));
        }

        // 403 认证错误不在 HTTP 层重试；交给外层刷新当前账号 token 或切换账号。
        if mapped_status == StatusCode::FORBIDDEN {
            log::warn!("[网关] 上游 403 错误，type={}，交给外层处理", error_type);
            return Err(UpstreamRequestError::new(mapped_status, error_type, message, Some(body)));
        }

        // 5xx 服务器错误才重试；4xx（402/429/401/403）全部上交外层换号或刷 token
        let should_retry = attempt < MAX_RETRIES && mapped_status.is_server_error();

        if should_retry {
            // 指数退避但封顶：attempt=5 时裸算是 16s，累计 31s 已能让客户端超时，
            // 再往上加只会把「上游抖动」放大成「用户以为网关挂了」
            let backoff_ms = (1000 * 2u64.pow(attempt - 1)).min(MAX_RETRY_BACKOFF_MS);
            log::warn!(
                "上游请求失败 (状态: {}, 类型: {}, 尝试: {}/{}), {}ms 后重试",
                mapped_status,
                error_type,
                attempt,
                MAX_RETRIES,
                backoff_ms
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            continue;
        }

        // 其他错误也直接返回原始响应（不提取 message，直接透传 JSON）
        return Err(UpstreamRequestError::new(mapped_status, error_type, message, Some(body)));
    }
}

/// 上游 5xx 的最大尝试次数。`amz-sdk-request` 的 max 与此同源（G-6）。
const MAX_RETRIES: u32 = 5;

/// `attempt` 是本次实际尝试序号（从 1 起），与循环里的重试上界 `MAX_RETRIES`
/// 同源，否则 `amz-sdk-request` 声明的 max 与真实行为不一致（G-6）。
pub fn add_kiro_upstream_headers(
    builder: reqwest::RequestBuilder,
    upstream: &UpstreamCredentials,
    accept: &str,
    include_opt_out: bool,
    include_agent_mode: bool,
    include_profile_arn_header: bool,
    attempt: u32,
) -> reqwest::RequestBuilder {
    let invocation_id = uuid::Uuid::new_v4().to_string();
    let x_amz_user_agent = build_kiro_x_amz_user_agent(&upstream.machine_id);
    let sdk_request_header = format!("attempt={attempt}; max={MAX_RETRIES}");

    let mut builder = builder
        .header("Authorization", format!("Bearer {}", upstream.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("host", build_kiro_runtime_host(&upstream.region))
        .header(header::USER_AGENT, upstream.user_agent.clone())
        .header("x-amz-user-agent", x_amz_user_agent)
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", sdk_request_header);

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
