// Auth 相关命令 - 直接存储 usage_data
// Auth 相关命令 - 直接存储 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数
use crate::auth::auth_social;
use crate::auth::providers::{
    cancel_pending_idc_login, create_idc_provider, get_provider_config, AuthMethod, AuthProvider,
};
use crate::auth::User;
use crate::clients::kiro_auth_client::KiroAuthServiceClient;
use crate::commands::common::{
    extract_user_info, find_existing_account_idx, generate_account_machine_id,
    get_usage_by_provider_with_machine_id, lock_store, save_store, update_account_status,
};
use crate::core::account::Account;
use crate::core::protocol_registry;
use crate::state::AppState;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::Value;
use tauri::{Emitter, State};

/// 从 JWT (id_token) 中提取 email 和 user_id
/// Enterprise 账号的 id_token payload 通常包含 email、user_id 或 cognito:username
fn extract_user_info_from_id_token(id_token: &str) -> (Option<String>, Option<String>) {
    // JWT 格式: header.payload.signature
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return (None, None);
    }
    let payload = parts[1];
    // URL-safe base64 解码（补齐 padding）
    let mut padded = payload.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    let decoded = match URL_SAFE_NO_PAD.decode(&padded) {
        Ok(bytes) => bytes,
        Err(_) => return (None, None),
    };
    let json_str = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return (None, None),
    };
    let payload_json: Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let email = payload_json
        .get("email")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    // 优先取 user_id，其次 cognito:username
    let user_id = payload_json
        .get("user_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            payload_json
                .get("cognito:username")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        });
    (email, user_id)
}

fn require_login_email(email: Option<String>) -> Result<String, String> {
    email.ok_or("获取邮箱失败，请检查账号状态".to_string())
}

fn resolve_idc_login_email(
    provider_id: &str,
    email: Option<String>,
    user_id: Option<String>,
) -> Result<Option<String>, String> {
    if provider_id == "Enterprise" {
        // Enterprise 账号允许没有 email 和 userId，都没有时返回 None
        Ok(email.or(user_id))
    } else if provider_id == "BuilderId" {
        // BuilderId 允许没有 email/userId
        Ok(email.or(user_id).or_else(|| Some("builderid_unknown".to_string())))
    } else {
        require_login_email(email).map(Some)
    }
}

fn social_callback_redirect_uri() -> String {
    crate::core::deep_link_handler::DeepLinkCallbackWaiter::get_redirect_uri()
}

fn prepare_pending_social_login(provider: &str, machineid: String) -> crate::state::PendingLogin {
    crate::state::PendingLogin {
        provider: provider.to_string(),
        code_verifier: auth_social::generate_code_verifier_social(),
        state: uuid::Uuid::new_v4().to_string(),
        machineid,
    }
}

#[tauri::command]
pub fn get_current_user(state: State<AppState>) -> Option<User> {
    match lock_store(&state.auth.user, "auth user") {
        Ok(user) => user.clone(),
        Err(_) => {
            log::error!("[auth_cmd] Failed to get current user");
            None
        }
    }
}

#[tauri::command]
pub fn logout(state: State<AppState>) {
    clear_auth_state(&state.auth);
}

fn clear_auth_state(auth: &crate::auth::AuthState) {
    if let Ok(mut user) = lock_store(&auth.user, "auth user") {
        *user = None;
    }
    if let Ok(mut access_token) = lock_store(&auth.access_token, "auth access_token") {
        *access_token = None;
    }
    if let Ok(mut refresh_token) = lock_store(&auth.refresh_token, "auth refresh_token") {
        *refresh_token = None;
    }
}

#[tauri::command]
pub fn cancel_kiro_login(state: State<'_, AppState>) -> bool {
    let cancelled_social = crate::core::deep_link_handler::cancel_waiter();
    let cancelled_idc = cancel_pending_idc_login();
    match lock_store(&state.pending_login, "pending_login") {
        Ok(mut pending_login) => {
            *pending_login = None;
        }
        Err(_) => {
            log::error!("[auth_cmd] Failed to cancel login");
        }
    }
    cancelled_social || cancelled_idc
}

#[tauri::command]
pub async fn kiro_login(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    provider: String,
    start_url: Option<String>, // 新增：支持自定义 start_url（Enterprise 用）
    region: Option<String>,    // 新增：支持自定义 region（Enterprise 用）
) -> Result<String, String> {
    let mut config = get_provider_config(&provider)
        .ok_or_else(|| format!("Unsupported provider: {provider}"))?;

    // 如果传入了自定义 start_url，覆盖默认值
    if let Some(url) = start_url {
        config.start_url = Some(url);
    }

    // 如果传入了自定义 region，覆盖默认值
    if let Some(reg) = region {
        config.region = reg;
    }

    match config.auth_method {
        AuthMethod::Social => login_social(app_handle, state, &config).await,
        AuthMethod::Idc => login_idc(app_handle, state, &config).await,
    }
}

async fn login_social(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: &crate::auth::providers::ProviderConfig,
) -> Result<String, String> {
    // 确保协议注册指向当前应用（解决多版本/移动应用的问题）
    protocol_registry::ensure_protocol_registration()?;

    let provider_id = config.provider_id.clone();
    let pending = prepare_pending_social_login(&provider_id, generate_account_machine_id());
    let redirect_uri = social_callback_redirect_uri();
    let code_challenge = auth_social::generate_code_challenge_social(&pending.code_verifier);
    let client = KiroAuthServiceClient::new(&pending.machineid)?;

    *lock_store(&state.pending_login, "pending_login")? = Some(pending.clone());

    // 1. 注册 deep link 回调等待器
    let waiter = crate::core::deep_link_handler::register_waiter(&pending.state);

    // 2. 打开浏览器授权
    if let Err(err) = client
        .login(&provider_id, &redirect_uri, &code_challenge, &pending.state)
        .await
    {
        *lock_store(&state.pending_login, "pending_login")? = None;
        return Err(err);
    }

    println!("\n[social] LOGIN STARTED: {provider_id}");

    // 3. 等待回调（阻塞直到用户完成授权或超时）
    let callback_result = waiter.wait_for_callback()?;

    // 4. 用 code 换 token
    let token_result: crate::auth::providers::SocialTokenResponse = client
        .create_token(
            &callback_result.code,
            &pending.code_verifier,
            &redirect_uri,
            None, // invitation_code
        )
        .await?;

    // 5. 获取配额信息
    let usage_result = get_usage_by_provider_with_machine_id(
        &provider_id,
        &token_result.access_token,
        &pending.machineid,
    )
    .await?;

    // 封禁账号直接报错
    if usage_result.is_banned {
        *lock_store(&state.pending_login, "pending_login")? = None;
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    let final_email = new_email.or(user_id.clone()).unwrap_or_else(|| {
        format!(
            "{}_{}",
            provider_id.to_lowercase(),
            &token_result.refresh_token[..8]
        )
    });

    // 6. 保存账号
    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        Some(&final_email),
        &provider_id,
        &token_result.refresh_token,
        user_id.as_ref(),
    );

    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        existing.access_token = Some(token_result.access_token.clone());
        existing.refresh_token = Some(token_result.refresh_token.clone());
        existing.profile_arn = token_result.profile_arn.clone();
        existing.user_id = user_id;
        existing.usage_data = Some(usage_result.usage_data);
        if existing
            .machine_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
        {
            existing.machine_id = Some(pending.machineid.clone());
        }
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {provider_id} 账号"));
        account.access_token = Some(token_result.access_token.clone());
        account.refresh_token = Some(token_result.refresh_token.clone());
        account.profile_arn = token_result.profile_arn.clone();
        account.provider = Some(provider_id.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        update_account_status(
            &mut account,
            usage_result.is_banned,
            usage_result.is_auth_error,
        );
        account.machine_id = Some(pending.machineid.clone());
        store.accounts.insert(0, account.clone());
        account
    };

    save_store(&store)?;
    drop(store);

    // 7. 更新认证状态（失败不影响账号已保存）
    let user = crate::auth::User {
        id: uuid::Uuid::new_v4().to_string(),
        email: account.email.clone(),
        name: account
            .email
            .as_ref()
            .and_then(|e| e.split('@').next())
            .unwrap_or("User")
            .to_string(),
        avatar: None,
        provider: provider_id.clone(),
    };
    let _ = lock_store(&state.auth.user, "auth user").map(|mut u| *u = Some(user));
    let _ = lock_store(&state.auth.access_token, "auth access_token")
        .map(|mut t| *t = Some(token_result.access_token));

    *lock_store(&state.pending_login, "pending_login")? = None;

    println!("\n[social] LOGIN SUCCESS: {}", account.get_display_id());
    let _ = app_handle.emit("login-success", account.id.clone());

    Ok(format!("Successfully logged in with {provider_id}"))
}

async fn login_idc(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: &crate::auth::providers::ProviderConfig,
) -> Result<String, String> {
    let idc_provider = create_idc_provider(config);
    let provider_id = idc_provider.get_provider_id().to_string();
    let auth_method = idc_provider.get_auth_method();

    let auth_result = idc_provider.login().await?;

    // 先查找已存在账号，优先使用已有的 machine_id
    let existing_machine_id = {
        let store = lock_store(&state.store, "store")?;
        store.accounts.iter().find(|acc| {
            // 通过 start_url + client_id_hash 匹配 Enterprise 账号
            if provider_id == "Enterprise" {
                if let (Some(ref acc_start_url), Some(ref acc_hash), Some(ref auth_start_url), Some(ref auth_hash)) =
                    (&acc.start_url, &acc.client_id_hash, &auth_result.start_url, &auth_result.client_id_hash)
                {
                    return acc_start_url == auth_start_url && acc_hash == auth_hash;
                }
            }
            // BuilderId 通过 refresh_token 匹配
            if let Some(ref acc_rt) = acc.refresh_token {
                return acc_rt == &auth_result.refresh_token;
            }
            false
        }).and_then(|acc| acc.machine_id.clone())
    }; // store 在此作用域结束时自动释放

    let account_machine_id = existing_machine_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(generate_account_machine_id);

    let usage_result = match get_usage_by_provider_with_machine_id(
        &provider_id,
        &auth_result.access_token,
        &account_machine_id,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            log::warn!("Failed to get usage for {}: {}", provider_id, e);
            // 即使 getUsageLimits 失败，也能保存账号
            crate::commands::common::UsageResult::empty()
        }
    };

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // 调试：输出 Enterprise 账号的 userInfo 对象
    if provider_id == "Enterprise" {
        log::info!("Enterprise userInfo: {}", usage_result.usage_data.get("userInfo").unwrap_or(&serde_json::json!(null)));
        log::info!("Extracted email: {:?}, user_id: {:?}", new_email, user_id);
    }

    // Enterprise 账号：如果 getUsageLimits 没返回 userInfo，尝试从 id_token (JWT) 解析
    let (new_email, user_id) = if provider_id == "Enterprise" && new_email.is_none() && user_id.is_none() {
        if let Some(ref id_token) = auth_result.id_token {
            log::info!("Enterprise account: getUsageLimits returned no userInfo, trying to decode id_token");
            let (email_from_token, user_id_from_token) = extract_user_info_from_id_token(id_token);
            if email_from_token.is_some() || user_id_from_token.is_some() {
                log::info!("Decoded from id_token - email: {:?}, user_id: {:?}", email_from_token, user_id_from_token);
                (email_from_token, user_id_from_token)
            } else {
                (new_email, user_id)
            }
        } else {
            (new_email, user_id)
        }
    } else {
        (new_email, user_id)
    };

    // Enterprise 账号允许没有 email,使用 userId 作为标识
    let final_email = resolve_idc_login_email(&provider_id, new_email.clone(), user_id.clone())?;
    log::info!("final_email for {} account: {:?}", provider_id, final_email);

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        new_email.as_ref(),
        &provider_id,
        &auth_result.refresh_token,
        user_id.as_ref(),
    );
    log::info!("existing_idx: {:?}, will create new account: {}", existing_idx, existing_idx.is_none());

    let account = if let Some(idx) = existing_idx {
        log::info!("Updating existing account at index {}", idx);
        let existing = &mut store.accounts[idx];
        existing.access_token = Some(auth_result.access_token.clone());
        existing.refresh_token = Some(auth_result.refresh_token.clone());
        existing.email.clone_from(&new_email);
        existing.user_id.clone_from(&user_id);
        existing.provider = Some(provider_id.clone()); // 确保 provider 不变
        existing.expires_at = Some(auth_result.expires_at.clone());
        existing.client_id_hash = auth_result.client_id_hash;
        existing.client_id = auth_result.client_id;
        existing.client_secret = auth_result.client_secret;
        existing.region = auth_result.region;
        existing.start_url.clone_from(&auth_result.start_url); // 保存 start_url
        existing.sso_session_id = auth_result.sso_session_id;
        existing.id_token = auth_result.id_token;
        existing.profile_arn = auth_result.profile_arn;
        existing.usage_data = Some(usage_result.usage_data);
        // machine_id 应该已经存在且被复用了，这里仅作兜底
        if existing.machine_id.as_ref().is_none_or(|id| id.trim().is_empty()) {
            existing.machine_id = Some(account_machine_id.clone());
        }
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(
            final_email.clone().unwrap_or_else(|| format!("{provider_id}_account")),
            format!("Kiro {provider_id} 账号"),
        );
        // 如果 Enterprise 账号没有 email，设回 None
        if provider_id == "Enterprise" && final_email.is_none() {
            account.email = None;
        }
        account.access_token = Some(auth_result.access_token.clone());
        account.refresh_token = Some(auth_result.refresh_token.clone());
        account.provider = Some(provider_id.clone());
        account.auth_method = Some("IdC".to_string());
        account.user_id = user_id;
        account.expires_at = Some(auth_result.expires_at.clone());
        account.client_id_hash = auth_result.client_id_hash;
        account.client_id = auth_result.client_id;
        account.client_secret = auth_result.client_secret;
        account.region = auth_result.region;
        account.start_url.clone_from(&auth_result.start_url); // 保存 start_url
        account.sso_session_id = auth_result.sso_session_id;
        account.id_token = auth_result.id_token;
        account.profile_arn = auth_result.profile_arn;
        account.usage_data = Some(usage_result.usage_data);
        update_account_status(
            &mut account,
            usage_result.is_banned,
            usage_result.is_auth_error,
        );

        // 为所有新账号生成唯一的 machine_id（每个账号独立 UUID，避免隐私泄露）
        account.machine_id = Some(account_machine_id);
        log::info!(
            "Generated unique machine_id for new {} account",
            provider_id
        );

        store.accounts.insert(0, account.clone());
        log::info!("Inserted new {} account into store", provider_id);
        account
    };

    log::info!("Saving store with {} accounts...", store.accounts.len());
    save_store(&store)?;
    log::info!("Store saved successfully");
    drop(store);

    let display_id = account.get_display_id();
    update_auth_state(
        &state,
        account.email.as_ref(),
        &provider_id,
        &auth_result.access_token,
        &auth_result.refresh_token,
    )?;
    println!("\n[{auth_method}] LOGIN SUCCESS: {display_id}");

    log::info!("Emitting login-success event for account: {}", account.id);
    let _ = app_handle.emit("login-success", account.id.clone());
    let _ = app_handle.emit("accounts-updated", ());
    log::info!("Emitted login-success and accounts-updated events");
    log::info!("Returning success response");
    Ok(format!("{auth_method} login completed for {display_id}"))
}

fn update_auth_state(
    state: &State<'_, AppState>,
    email: Option<&String>,
    provider: &str,
    access_token: &str,
    refresh_token: &str,
) -> Result<(), String> {
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: email.cloned(),
        name: email
            .and_then(|e| e.split('@').next())
            .unwrap_or("User")
            .to_string(),
        avatar: None,
        provider: provider.to_string(),
    };
    *lock_store(&state.auth.user, "auth user")? = Some(user);
    *lock_store(&state.auth.access_token, "auth access_token")? = Some(access_token.to_string());
    *lock_store(&state.auth.refresh_token, "auth refresh_token")? = Some(refresh_token.to_string());
    *lock_store(&state.pending_login, "pending_login")? = None;
    Ok(())
}

#[tauri::command]
pub async fn handle_kiro_social_callback(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    code: String,
    callback_state: String,
) -> Result<(), String> {
    let pending = {
        let lock = lock_store(&state.pending_login, "pending_login")?;
        lock.clone().ok_or("No pending login found")?
    };

    if pending.state != callback_state {
        return Err("State mismatch".to_string());
    }

    let redirect_uri = social_callback_redirect_uri();
    let token_response = auth_social::exchange_social_code_for_token(
        &code,
        &pending.code_verifier,
        &redirect_uri,
        &pending.machineid,
    )
    .await?;

    let usage_result = get_usage_by_provider_with_machine_id(
        &pending.provider,
        &token_response.access_token,
        &pending.machineid,
    )
    .await?;

    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    let final_email = new_email.or(user_id.clone()).unwrap_or_else(|| {
        format!(
            "{}_{}",
            pending.provider.to_lowercase(),
            &token_response.refresh_token[..8]
        )
    });

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        Some(&final_email),
        &pending.provider,
        &token_response.refresh_token,
        user_id.as_ref(),
    );

    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        existing.access_token = Some(token_response.access_token.clone());
        existing.refresh_token = Some(token_response.refresh_token.clone());
        existing.profile_arn = Some(token_response.profile_arn.clone());
        existing.user_id = user_id;
        existing.usage_data = Some(usage_result.usage_data);
        if existing
            .machine_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
        {
            existing.machine_id = Some(pending.machineid.clone());
        }
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {} 账号", pending.provider));
        account.access_token = Some(token_response.access_token.clone());
        account.refresh_token = Some(token_response.refresh_token.clone());
        account.profile_arn = Some(token_response.profile_arn.clone());
        account.provider = Some(pending.provider.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        update_account_status(
            &mut account,
            usage_result.is_banned,
            usage_result.is_auth_error,
        );
        account.machine_id = Some(pending.machineid.clone());
        store.accounts.insert(0, account.clone());
        account
    };

    save_store(&store)?;
    drop(store);

    let user = crate::auth::User {
        id: uuid::Uuid::new_v4().to_string(),
        email: account.email.clone(),
        name: account
            .email
            .as_ref()
            .and_then(|e| e.split('@').next())
            .unwrap_or("User")
            .to_string(),
        avatar: None,
        provider: pending.provider.clone(),
    };
    let _ = lock_store(&state.auth.user, "auth user").map(|mut u| *u = Some(user));
    let _ = lock_store(&state.auth.access_token, "auth access_token")
        .map(|mut t| *t = Some(token_response.access_token));

    *lock_store(&state.pending_login, "pending_login")? = None;

    println!("\n[social] LOGIN SUCCESS: {}", account.get_display_id());
    let _ = app_handle.emit("login-success", account.id.clone());
    let _ = app_handle.emit("accounts-updated", ());

    Ok(())
}