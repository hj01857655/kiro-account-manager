//! 上游凭据解析：单账号 / 账号池取号、token 刷新、失败账号停用与用量阈值判断。

use super::*;

#[derive(Debug, Clone)]
pub struct UpstreamCredentials {
    pub(super) account_id: String,
    pub(super) access_token: String,
    pub(super) machine_id: String,
    /// 发送正式 Kiro 请求时使用的 profileArn；BuilderId/Social 会按 provider 兜底。
    pub(super) profile_arn: Option<String>,
    /// ListAvailableModels 探测使用的 profileArn。
    ///
    /// BuilderId 账号本地常见为 `profileArn=null`，但真实 IDE 抓包会带固定
    /// BuilderId profileArn；不带时上游会返回 `Invalid profileArn`。
    /// 因此这里使用有效 profileArn（账号/刷新返回值优先，否则 provider 默认值），
    /// 但 machineId 必须仍使用账号自己的 machineId。
    pub(super) available_models_profile_arn: Option<String>,
    pub(super) provider: Option<String>,
    pub(super) region: String,
    pub(super) source_label: String,
    pub(super) user_agent: String,
    #[allow(dead_code)]
    pub(super) auth_method: Option<String>,
    pub(super) send_opt_out: bool,
    pub(super) http: Client,
}

pub fn ip_matches_allowlist(ip: IpAddr, allowlist: &[String]) -> bool {
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

pub fn verify_client_auth(headers: &HeaderMap, config: &GatewayConfig) -> Result<(), String> {
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

/// 从请求头读取可选的指定账号 ID（API Playground / 客户端调试用）
pub fn extract_preferred_account_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-account-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

pub async fn resolve_upstream_credentials(
    config: &GatewayConfig,
    state: &RouterState,
    preferred_account_id: Option<&str>,
) -> Result<UpstreamCredentials, String> {
    match config.account_mode.as_str() {
        "single" | "group" | "pool" => {
            resolve_managed_account_credentials(config, state, preferred_account_id).await
        }
        "local" => Err("2API不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string()),
        _ => Err("accountMode 必须是 single/group/pool".to_string()),
    }
}

pub async fn resolve_managed_account_credentials(
    config: &GatewayConfig,
    state: &RouterState,
    preferred_account_id: Option<&str>,
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
                .filter(|account| config.pool_account_ids.contains(&account.id))
                .collect();

            !pool_accounts.is_empty()
                && pool_accounts
                    .iter()
                    .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures"))
        }
        _ => false,
    };

    if all_disabled_by_failures {
        for account in store.accounts.iter_mut() {
            if account.disabled_reason.as_deref() == Some("TooManyFailures") {
                account.failure_count = 0;
                account.status = "active".to_string();
                account.disabled_reason = None;
            }
        }
        let _ = store.save_to_file();
    }

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
                config.group_id.as_deref() == account.group_id.as_deref()
                    && account.is_available()
                    && account.enabled
            })
            .cloned()
            .collect::<Vec<_>>(),
        "pool" => store
            .accounts
            .iter()
            .filter(|account| {
                config.pool_account_ids.contains(&account.id)
                    && account.is_available()
                    && account.enabled
            })
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    // x-account-id：强制使用指定账号，且必须在当前路由可用集合内
    if let Some(preferred) = preferred_account_id {
        accounts.retain(|account| account.id == preferred);
        if accounts.is_empty() {
            return Err(format!(
                "__400__指定账号 {} 不在当前2API路由可用账号中，请检查账号模式/分组/账号池配置",
                preferred
            ));
        }
        log::info!("[网关] 使用指定账号: {}", preferred);
    }

    if accounts.is_empty() {
        return Err("__402__未找到符合2API配置的可用账号".to_string());
    }

    // 使用 LoadBalancer 选择账号
    let selected_account = state.load_balancer.select_account(&accounts).await;

    let Some(account) = selected_account else {
        return Err("__402__LoadBalancer 未能选择可用账号".to_string());
    };

    // 增加连接计数
    state.load_balancer.increment_connections(&account.id).await;
    let request_start = Instant::now();

    // 检查 token 是否真正过期（不再提前刷新，避免和定时器/IDE 冲突导致 429）
    // 定时器会提前 10 分钟刷新，网关只在 token 真正过期时才刷新
    let need_refresh = match &account.expires_at {
        Some(expires_at) => is_token_expired(expires_at),
        None => true, // 没有过期时间，强制刷新
    };

    // 如果 token 没过期且有 access_token，直接使用
    if !need_refresh {
        if let Some(access_token) = &account.access_token {
            if !access_token.is_empty() {
                // token 未过期，不需要 refresh，释放连接计数
                state.load_balancer.decrement_connections(&account.id).await;
                let ctx = crate::commands::common::resolve_kiro_call_context(
                    &account,
                    &state.config.region,
                );
                let available_models_profile_arn = ctx.profile_arn.clone();
                let http = match build_streaming_http_client_for_account(&account) {
                    Ok(http) => http,
                    Err(error) => {
                        state.load_balancer.decrement_connections(&account.id).await;
                        return Err(format!(
                            "创建账号 {} 的2API HTTP 客户端失败: {}",
                            account.label,
                            sanitize_error(&error)
                        ));
                    }
                };
                let creds = UpstreamCredentials {
                    account_id: account.id.clone(),
                    access_token: access_token.clone(),
                    machine_id: ctx.machine_id.clone(),
                    profile_arn: ctx.profile_arn,
                    available_models_profile_arn,
                    provider: account.provider.clone(),
                    region: ctx.region,
                    source_label: format_managed_upstream_source(&state.config, &account),
                    user_agent: build_kiro_custom_user_agent(&ctx.machine_id),
                    auth_method: account.auth_method.clone(),
                    send_opt_out: should_send_codewhisperer_optout(),
                    http,
                };
                return ensure_enterprise_profile_arn(&account, creds).await;
            }
        }
    }

    match refresh_token_by_provider_with_account_proxy(&account).await {
        Ok(refresh) => {
            let usage_result = get_usage_by_account(&account, &refresh.access_token).await;
            let mut usage_data = None;
            let mut is_banned = false;
            let mut is_auth_error = false;
            let mut resolved_profile_arn = None;

            if let Ok(usage) = usage_result {
                usage_data = Some(usage.usage_data);
                is_banned = usage.is_banned;
                is_auth_error = usage.is_auth_error;
                resolved_profile_arn = usage.resolved_profile_arn;
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
            if let Some(ref arn) = resolved_profile_arn {
                persist_account_profile_arn(&account.id, arn);
            }

            // 减少连接计数
            state.load_balancer.decrement_connections(&account.id).await;

            if is_banned || is_auth_error {
                // 记录失败
                state.load_balancer.record_failure(&account.id).await;
                return Err(format!("账号 {} 已不可用", account.label));
            }

            if let Some(usage_data) = &usage_data {
                if usage_exceeds_threshold(usage_data, config.threshold) {
                    // 配额超阈值，直接禁用账号
                    state.load_balancer.record_failure(&account.id).await;
                    disable_account_by_id(&account.id, "配额已满");
                    return Err(format!("账号 {} 配额已满，已自动禁用", account.label));
                } else {
                    // 配额已恢复，检查是否需要自动启用账号
                    // 仅当账号因配额满被自动禁用时才自动启用
                    if !account.enabled && account.disabled_reason.as_deref() == Some("配额已满")
                    {
                        enable_account_by_id(&account.id);
                    }
                }
            }

            // 记录成功
            let response_time_ms = request_start.elapsed().as_millis() as u64;
            state
                .load_balancer
                .record_success(&account.id, response_time_ms)
                .await;

            let mut account_for_creds = account.clone();
            if let Some(arn) = resolved_profile_arn {
                account_for_creds.profile_arn = Some(arn);
            }
            let creds = build_upstream_credentials_from_refresh(config, &account_for_creds, refresh)?;
            ensure_enterprise_profile_arn(&account_for_creds, creds).await
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

pub async fn force_refresh_upstream_credentials(
    config: &GatewayConfig,
    state: &RouterState,
    upstream: &UpstreamCredentials,
) -> Result<UpstreamCredentials, String> {
    let mut store = AccountStore::new();
    store.reload();

    let account = store
        .accounts
        .iter()
        .find(|candidate| candidate.id == upstream.account_id)
        .cloned()
        .ok_or_else(|| format!("账号 {} 不存在，无法刷新 Token", upstream.source_label))?;

    let refresh = refresh_token_by_provider_with_account_proxy(&account)
        .await
        .map_err(|error| {
            format!(
                "刷新账号 {} 失败: {}",
                account.label,
                sanitize_error(&error)
            )
        })?;

    let usage_result = get_usage_by_account(&account, &refresh.access_token).await;
    let mut usage_data = None;
    let mut is_banned = false;
    let mut is_auth_error = false;
    let mut resolved_profile_arn = None;

    if let Ok(usage) = usage_result {
        usage_data = Some(usage.usage_data);
        is_banned = usage.is_banned;
        is_auth_error = usage.is_auth_error;
        resolved_profile_arn = usage.resolved_profile_arn;
    }

    persist_account_refresh(
        &account,
        &refresh,
        usage_data.clone(),
        is_banned,
        is_auth_error,
        is_banned || is_auth_error,
    );
    if let Some(ref arn) = resolved_profile_arn {
        persist_account_profile_arn(&account.id, arn);
    }

    if is_banned || is_auth_error {
        state.load_balancer.record_failure(&account.id).await;
        return Err(format!("账号 {} 刷新后仍不可用", account.label));
    }

    if let Some(usage_data) = &usage_data {
        if usage_exceeds_threshold(usage_data, config.threshold) {
            state.load_balancer.record_failure(&account.id).await;
            disable_account_by_id(&account.id, "配额已满");
            return Err(format!("账号 {} 配额已满，已自动禁用", account.label));
        }
    }

    let mut account_for_creds = account.clone();
    if let Some(arn) = resolved_profile_arn {
        account_for_creds.profile_arn = Some(arn);
    }
    let creds = build_upstream_credentials_from_refresh(config, &account_for_creds, refresh)?;
    ensure_enterprise_profile_arn(&account_for_creds, creds).await
}

pub fn build_upstream_credentials_from_refresh(
    config: &GatewayConfig,
    account: &Account,
    refresh: RefreshResult,
) -> Result<UpstreamCredentials, String> {
    let machine_id = account_machine_id_or_new(&account.machine_id);
    let profile_arn = resolve_profile_arn_from_candidates(
        refresh.profile_arn.as_deref(),
        account.profile_arn.as_deref(),
        account.provider.as_deref(),
    );
    let region = resolve_kiro_upstream_region(
        profile_arn.as_deref(),
        account.region.as_deref(),
        &config.region,
    );

    let http = build_streaming_http_client_for_account(account).map_err(|error| {
        format!(
            "创建账号 {} 的2API HTTP 客户端失败: {}",
            account.label,
            sanitize_error(&error)
        )
    })?;

    Ok(UpstreamCredentials {
        account_id: account.id.clone(),
        access_token: refresh.access_token,
        machine_id: machine_id.clone(),
        profile_arn: profile_arn.clone(),
        available_models_profile_arn: profile_arn,
        provider: account.provider.clone(),
        region,
        source_label: format_managed_upstream_source(config, account),
        user_agent: build_kiro_custom_user_agent(&machine_id),
        auth_method: account.auth_method.clone(),
        send_opt_out: should_send_codewhisperer_optout(),
        http,
    })
}
/// Enterprise 上游调用必须有真实 profileArn：账号已存优先，否则 ListAvailableProfiles 发现并落库。
async fn ensure_enterprise_profile_arn(
    account: &Account,
    mut creds: UpstreamCredentials,
) -> Result<UpstreamCredentials, String> {
    let has_arn = creds
        .profile_arn
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    if has_arn {
        return Ok(creds);
    }
    if account.provider.as_deref() != Some("Enterprise") {
        return Ok(creds);
    }

    use crate::clients::kiro_client::{usage_limits_region_candidates, KiroClient};

    let client = KiroClient::from_client(creds.http.clone());
    let regions = usage_limits_region_candidates(&creds.region, true);
    let arn = client
        .resolve_enterprise_profile_arn(&creds.access_token, &regions)
        .await?
        .ok_or_else(|| {
            format!(
                "Enterprise 账号 {} 未解析到 profileArn（ListAvailableProfiles 为空）",
                account.label
            )
        })?;

    persist_account_profile_arn(&account.id, &arn);
    log::info!(
        "[网关] Enterprise 账号 {} 解析到 profileArn: {}",
        account.label,
        arn
    );
    creds.profile_arn = Some(arn.clone());
    creds.available_models_profile_arn = Some(arn);
    Ok(creds)
}

fn persist_account_profile_arn(account_id: &str, profile_arn: &str) {
    let mut store = AccountStore::new();
    if let Some(account) = store.accounts.iter_mut().find(|a| a.id == account_id) {
        let empty = account
            .profile_arn
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty();
        if empty || account.profile_arn.as_deref() != Some(profile_arn) {
            account.profile_arn = Some(profile_arn.to_string());
            let _ = store.save_to_file();
        }
    }
}

pub fn format_managed_upstream_source(config: &GatewayConfig, account: &Account) -> String {
    let account_label = account
        .email
        .as_deref()
        .or(account.user_id.as_deref())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim())
        .unwrap_or("unknown");

    match config.account_mode.as_str() {
        "single" => format!("single:{account_label}"),
        "group" => format!(
            "group:{}:{account_label}",
            config.group_id.as_deref().unwrap_or("unknown")
        ),
        "pool" => format!("pool:{account_label}"),
        _ => account_label.to_string(),
    }
}

/// 禁用指定账号（配额满时自动调用）
pub fn disable_account_by_id(account_id: &str, reason: &str) {
    let mut store = AccountStore::new();
    if let Some(account) = store.accounts.iter_mut().find(|a| a.id == account_id) {
        account.enabled = false;
        account.disabled_reason = Some(reason.to_string());
        store.save_to_file();
        log::info!("[网关] 账号 {} 已自动禁用: {}", account_id, reason);
    }
}

/// 启用指定账号（配额恢复时自动调用）
pub fn enable_account_by_id(account_id: &str) {
    let mut store = AccountStore::new();
    if let Some(account) = store.accounts.iter_mut().find(|a| a.id == account_id) {
        account.enabled = true;
        account.disabled_reason = None;
        store.save_to_file();
        log::info!("[网关] 账号 {} 配额已恢复，已自动启用", account_id);
    }
}

pub fn persist_account_refresh(
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
        // 应用 token 字段更新（Option 字段仅在新值存在时覆盖，避免清空已有值）
        crate::commands::common::apply_refreshed_account_tokens(target, &refresh);
        if let Some(data) = usage_data {
            target.usage_data = Some(data);
        }
        update_account_status(target, is_banned, is_auth_error);

        // 失败追踪逻辑
        if should_increment_failure {
            target.failure_count += 1;
            target.last_failure_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

            // 如果失败次数达到阈值，自动禁用账号
            if target.failure_count >= MAX_FAILURES_PER_ACCOUNT {
                target.status = "disabled".to_string();
                target.disabled_reason = Some("TooManyFailures".to_string());
                log::warn!(
                    "[Gateway] 账号 {} 失败次数达到 {}，自动禁用",
                    target.label,
                    MAX_FAILURES_PER_ACCOUNT
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

pub fn usage_exceeds_threshold(usage_data: &Value, threshold: i32) -> bool {
    crate::core::usage::usage_exceeds_threshold(Some(usage_data), f64::from(threshold))
}

pub fn extract_account_id_from_upstream(upstream: &UpstreamCredentials) -> String {
    upstream.account_id.clone()
}
