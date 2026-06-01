// Auth 相关命令 - 直接存储 usage_data
// Auth 相关命令 - 直接存储 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递 State
use crate::core::account::Account;
use crate::core::protocol_registry;
use crate::auth::User;
use crate::auth::auth_social;
use crate::commands::common::{
    extract_user_info, find_existing_account_idx, generate_account_machine_id,
    get_usage_by_provider_with_machine_id, lock_store, save_store, update_account_status,
};
use crate::clients::kiro_auth_client::KiroAuthServiceClient;
use crate::auth::providers::{
    cancel_pending_idc_login, create_idc_provider, get_provider_config, AuthMethod, AuthProvider,
};
use crate::state::AppState;
use tauri::{Emitter, State};

fn require_login_email(email: Option<String>) -> Result<String, String> {
    email.ok_or("获取邮箱失败，请检查账号状态".to_string())
}

fn resolve_idc_login_email(
    provider_id: &str,
    email: Option<String>,
    user_id: Option<String>,
) -> Result<String, String> {
    if provider_id == "Enterprise" {
        email
            .or(user_id)
            .ok_or_else(|| format!("{} 账号缺少 userId 或 email", provider_id))
    } else if provider_id == "BuilderId" {
        // BuilderId 允许没有 email/userId
        Ok(email.or(user_id).unwrap_or_else(|| "builderid_unknown".to_string()))
    } else {
        require_login_email(email)
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
    let final_email = new_email
        .or(user_id.clone())
        .unwrap_or_else(|| format!("{}_{}", provider_id.to_lowercase(), &token_result.refresh_token[..8]));

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
        existing.machine_id = Some(pending.machineid.clone());
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
        update_account_status(&mut account, usage_result.is_banned, usage_result.is_auth_error);
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
    let _ = lock_store(&state.auth.access_token, "auth access_token").map(|mut t| *t = Some(token_result.access_token));

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

    let account_machine_id = generate_account_machine_id();
    let usage_result = get_usage_by_provider_with_machine_id(
        &provider_id,
        &auth_result.access_token,
        &account_machine_id,
    )
    .await?;

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // Enterprise 账号允许没有 email,使用 userId 作为标识
    let final_email = resolve_idc_login_email(&provider_id, new_email.clone(), user_id.clone())?;

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        new_email.as_ref(),
        &provider_id,
        &auth_result.refresh_token,
        user_id.as_ref(),
    );

    let account = if let Some(idx) = existing_idx {
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
        if existing.machine_id.as_ref().is_none_or(|id| id.trim().is_empty()) {
            existing.machine_id = Some(account_machine_id.clone());
        }
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {provider_id} 账号"));
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
        update_account_status(&mut account, usage_result.is_banned, usage_result.is_auth_error);

        // 为所有新账号生成唯一的 machine_id（每个账号独立 UUID，避免隐私泄露）
        account.machine_id = Some(account_machine_id);
        log::info!("Generated unique machine_id for new {} account", provider_id);
        
        store.accounts.insert(0, account.clone());
        account
    };

    save_store(&store)?;
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

    let _ = app_handle.emit("login-success", account.id.clone());
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
    let final_email = require_login_email(new_email.clone())?;

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        new_email.as_ref(),
        &pending.provider,
        &token_response.refresh_token,
        user_id.as_ref(),
    );

    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        existing.access_token = Some(token_response.access_token.clone());
        existing.refresh_token = Some(token_response.refresh_token.clone());
        existing.email.clone_from(&new_email);
        existing.user_id.clone_from(&user_id);
        existing.usage_data = Some(usage_result.usage_data);
        existing.machine_id = Some(pending.machineid.clone());
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(
            final_email.clone(),
            format!("Kiro {} 账号", pending.provider),
        );
        account.access_token = Some(token_response.access_token.clone());
        account.refresh_token = Some(token_response.refresh_token.clone());
        account.provider = Some(pending.provider.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        update_account_status(&mut account, usage_result.is_banned, usage_result.is_auth_error);

        // 为所有新账号生成唯一的 machine_id（每个账号独立 UUID，避免隐私泄露）
        account.machine_id = Some(pending.machineid.clone());
        log::info!("Generated unique machine_id for new {} account", pending.provider);
        
        store.accounts.insert(0, account.clone());
        account
    };

    save_store(&store)?;
    drop(store);

    let display_id = account.get_display_id();
    update_auth_state(
        &state,
        account.email.as_ref(),
        &pending.provider,
        &token_response.access_token,
        &token_response.refresh_token,
    )?;
    let _ = app_handle.emit("login-success", account.id);
    println!("Social callback login completed: {display_id}");
    Ok(())
}

#[tauri::command]
pub fn get_supported_providers() -> Vec<&'static str> {
    crate::auth::providers::get_supported_providers()
}

#[cfg(test)]
mod tests {
    use super::{
        clear_auth_state, require_login_email,
        resolve_idc_login_email,
    };
    use crate::auth::AuthState;
    use crate::auth::User;

    #[test]
    fn require_login_email_rejects_missing_email() {
        assert_eq!(
            require_login_email(Some("user@example.com".to_string())).unwrap(),
            "user@example.com".to_string()
        );
        assert_eq!(
            require_login_email(None).unwrap_err(),
            "获取邮箱失败，请检查账号状态".to_string()
        );
    }

    #[test]
    fn resolve_idc_login_email_uses_enterprise_user_id_fallback() {
        assert_eq!(
            resolve_idc_login_email("Enterprise", None, Some("enterprise-user".to_string()))
                .unwrap(),
            "enterprise-user".to_string()
        );
        assert_eq!(
            resolve_idc_login_email("BuilderId", None, Some("builder-user".to_string()))
                .unwrap(),
            "builder-user".to_string()
        );
        assert_eq!(
            resolve_idc_login_email("Enterprise", None, None).unwrap_err(),
            "Enterprise 账号缺少 userId 或 email".to_string()
        );
    }

    #[test]
    fn clear_auth_state_removes_refresh_token_too() {
        let auth = AuthState::new();
        *auth.user.lock().expect("user lock should work") = Some(User {
            id: "user-1".to_string(),
            email: Some("user@example.com".to_string()),
            name: "user".to_string(),
            avatar: None,
            provider: "Google".to_string(),
        });
        *auth
            .access_token
            .lock()
            .expect("access_token lock should work") = Some("access-token".to_string());
        *auth
            .refresh_token
            .lock()
            .expect("refresh_token lock should work") = Some("refresh-token".to_string());

        clear_auth_state(&auth);

        assert!(auth.user.lock().expect("user lock should work").is_none());
        assert!(auth
            .access_token
            .lock()
            .expect("access_token lock should work")
            .is_none());
        assert!(auth
            .refresh_token
            .lock()
            .expect("refresh_token lock should work")
            .is_none());
    }
}
