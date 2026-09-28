//! 主代理流程：proxy_handler 及其上游调用、上游请求头注入。

use super::*;

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
            "[{}] kind=client_request idx={} endpoint={} bytes={} truncated={} body={}\n",
            chrono::Local::now().format("%H:%M:%S"),
            request_index,
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

    let model_hint = extract_model_from_payload(&raw_request_body);
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
        let message = format!("访问地址 {} 不在2API白名单中", client_addr.ip());
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

    // 可选：通过 x-account-id 指定本次请求使用的账号（须在当前路由可用集合内）
    let preferred_account_id = extract_preferred_account_id(&headers);
    if let Some(ref account_id) = preferred_account_id {
        log::info!("[网关] 请求指定账号: {}", account_id);
    }

    let mut request = match normalize_request(format, &payload) {
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

    // 模型映射：根据规则替换请求的模型名（仅 OpenAI 协议）
    let original_model = request.model.clone();
    if matches!(format, ResponseFormat::OpenAI | ResponseFormat::Responses) {
        request.model = crate::gateway::resolve_model_mapping(&state.config, &request.model);
        if request.model != original_model {
            log::info!("[模型映射] {} → {} (OpenAI 协议)", original_model, request.model);
        }
    } else {
        // Anthropic Messages 协议客户端直接传 Claude 模型名，不做映射
        log::debug!("[模型映射] 跳过 Anthropic Messages 协议 (model={})", request.model);
    }

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
    let upstream = match resolve_upstream_credentials(
        &state.config,
        &state,
        preferred_account_id_ref,
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
                match resolve_upstream_credentials(&state.config, &state, None).await {
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
                // 检查是否是配额不足 / 参数错误（以 __402__ / __400__ 为前缀标记）
                let (status, error_type, display_message) = if message.starts_with("__402__") {
                    (
                        StatusCode::PAYMENT_REQUIRED,
                        "insufficient_quota",
                        message.strip_prefix("__402__").unwrap_or(&message),
                    )
                } else if message.starts_with("__400__") {
                    (
                        StatusCode::BAD_REQUEST,
                        "invalid_request_error",
                        message.strip_prefix("__400__").unwrap_or(&message),
                    )
                } else {
                    (
                        StatusCode::UNAUTHORIZED,
                        "authentication_error",
                        message.as_str(),
                    )
                };

                let sanitized = sanitize_error(display_message);
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
        let mut store = AccountStore::new();
        store.reload();

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
                    if message.contains("未找到符合2API配置的可用账号") {
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
        )
        .await
        {
            Ok(resp) => {
                // 请求成功，退出重试循环
                break (resp, current_upstream);
            }
            Err((status, error_type, message, upstream_response_body)) => {
                // 检查是否是 429 错误
                if status == StatusCode::TOO_MANY_REQUESTS {
                    let account_id = extract_account_id_from_upstream(&current_upstream);

                    // 记录服务端指定的限流等待窗口（毫秒），全账号试完后优先于固定退避
                    rate_limit_retry_after_ms =
                        parse_rate_limit_retry_after_ms(&message);

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
                    let mut store = crate::core::account::AccountStore::new();
                    if let Some(account) = store.accounts.iter_mut().find(|a| a.id == account_id) {
                        update_account_status(account, false, true); // is_auth_error = true
                        if let Err(e) = store.try_save_to_file() {
                            log::error!("[Gateway] 保存账号状态失败: {}", e);
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
            request.tool_name_map.clone(),
            request.include_usage,
            static_log_context,
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
    let body = json_payloads.join("");

    // 添加调试日志：只记录解码后的 JSON 数量和长度，不打印内容
    log::info!(
        "[非流式响应] 解码了 {} 条 EventStream 消息, 总 body 长度: {} 字符",
        json_payloads.len(),
        body.len()
    );

    let mut aggregated = stream::aggregate_kiro_response_from_payloads(&json_payloads);

    // 直接使用本地估算 token（不依赖响应中的 token 信息）
    log::info!("[非流式响应] 使用本地 token 估算");

    // 估算输入 tokens（从请求消息中）
    let request_text = serde_json::to_string(&request.messages).unwrap_or_default();
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

    let response = match format {
        ResponseFormat::Anthropic => build_anthropic_response(&request.model, &aggregated),
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
            request.messages.clone(),
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
        &state,
    );
    Json(response).into_response()
}

/// 服务端限流窗口上限（毫秒）：防病态值导致重试环长时间挂起
const RATE_LIMIT_RETRY_AFTER_MAX_MS: u64 = 5 * 60 * 1000;

/// 从 429 错误消息中解析服务端限流窗口标记 `[retry-after: <ms>ms]`
/// （由 call_generate_assistant_response 在消费响应体前从
/// `x-amzn-kiro-ratelimit-retry-after` 头提取，见 00 号语料 §2.3——单位毫秒）。
fn parse_rate_limit_retry_after_ms(message: &str) -> Option<u64> {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"\[retry-after:\s*(\d{1,9})ms\]").expect("retry-after regex")
    });
    re.captures(message)?
        .get(1)?
        .as_str()
        .parse::<u64>()
        .ok()
}

pub async fn call_generate_assistant_response<T: serde::Serialize + ?Sized>(
    upstream: &UpstreamCredentials,
    upstream_payload: &T,
    request_index: usize,
) -> Result<reqwest::Response, UpstreamRequestError> {
    let upstream_url = build_generate_assistant_response_url(&upstream.region);

    // 追加最新请求到日志文件
    if let Ok(payload_json) = serde_json::to_string(upstream_payload) {
        let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let body_end = safe_truncate(&payload_json, 50000);
        let entry = format!(
            "[{}] kind=kiro_request idx={} upstream=generateAssistantResponse bytes={} truncated={} body={}\n",
            chrono::Local::now().format("%H:%M:%S"),
            request_index,
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

    const MAX_RETRIES: u32 = 5;
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
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 429 限流错误不重试，直接返回让外层切换账号；
        // 提取 Kiro 官方限流头（毫秒）夹带给外层——重试环用它替换固定退避
        if mapped_status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after_ms = rate_limit_retry_after_header;
            let message = match retry_after_ms {
                Some(ms) => format!("{message} [retry-after: {ms}ms]"),
                None => message,
            };
            log::warn!(
                "[网关] 上游 429 限流，type={}，retry-after={}，交给外层切换账号",
                error_type,
                retry_after_ms.map(|ms| format!("{ms}ms")).unwrap_or_else(|| "无".to_string())
            );
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 401 认证错误不在 HTTP 层重试；交给外层刷新当前账号 token 或切换账号。
        if mapped_status == StatusCode::UNAUTHORIZED {
            log::warn!("[网关] 上游 401 认证错误，type={}，交给外层处理", error_type);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 403 认证错误不在 HTTP 层重试；交给外层刷新当前账号 token 或切换账号。
        if mapped_status == StatusCode::FORBIDDEN {
            log::warn!("[网关] 上游 403 错误，type={}，交给外层处理", error_type);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 5xx 服务器错误才重试
        let should_retry = attempt < MAX_RETRIES && mapped_status.is_server_error();

        if should_retry {
            let backoff_ms = 1000 * 2u64.pow(attempt - 1);
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
        return Err((mapped_status, error_type, message, Some(body)));
    }
}

pub fn add_kiro_upstream_headers(
    builder: reqwest::RequestBuilder,
    upstream: &UpstreamCredentials,
    accept: &str,
    include_opt_out: bool,
    include_agent_mode: bool,
    include_profile_arn_header: bool,
) -> reqwest::RequestBuilder {
    let invocation_id = uuid::Uuid::new_v4().to_string();
    let x_amz_user_agent = build_kiro_x_amz_user_agent(&upstream.machine_id);

    let mut builder = builder
        .header("Authorization", format!("Bearer {}", upstream.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("host", build_kiro_runtime_host(&upstream.region))
        .header(header::USER_AGENT, upstream.user_agent.clone())
        .header("x-amz-user-agent", x_amz_user_agent)
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

#[cfg(test)]
mod retry_after_tests {
    use super::parse_rate_limit_retry_after_ms;

    #[test]
    fn parses_retry_after_marker() {
        let msg = "Rate limited. [retry-after: 45000ms]";
        assert_eq!(parse_rate_limit_retry_after_ms(msg), Some(45000));
        // 无标记
        assert_eq!(parse_rate_limit_retry_after_ms("Rate limited."), None);
        // 非数字
        assert_eq!(parse_rate_limit_retry_after_ms("[retry-after: abcms]"), None);
        // 零值保留（由调用方决定是否回退）
        assert_eq!(parse_rate_limit_retry_after_ms("[retry-after: 0ms]"), Some(0));
    }
}
