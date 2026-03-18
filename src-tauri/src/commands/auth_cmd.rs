// Auth 相关命令 - 直接存储 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递 State

use tauri::{Emitter, State};
use crate::state::AppState;
use crate::account::Account;
use crate::auth::User;
use crate::auth_social;
use crate::providers::{AuthMethod, AuthProvider, cancel_pending_idc_login, get_provider_config, create_social_provider, create_idc_provider};
use crate::commands::common::{get_usage_by_provider, extract_user_info, find_existing_account_idx, calc_status};

#[tauri::command]
pub fn get_current_user(state: State<AppState>) -> Option<User> {
    state.auth.user.lock().expect("Failed to acquire lock").clone()
}

#[tauri::command]
pub fn logout(state: State<AppState>) {
    *state.auth.user.lock().expect("Failed to acquire lock") = None;
    *state.auth.access_token.lock().expect("Failed to acquire lock") = None;
}

#[tauri::command]
pub fn cancel_kiro_login(state: State<'_, AppState>) -> bool {
    let cancelled_social = crate::deep_link_handler::cancel_waiter();
    let cancelled_idc = cancel_pending_idc_login();
    *state.pending_login.lock().expect("Failed to acquire lock") = None;
    cancelled_social || cancelled_idc
}

#[tauri::command]
pub async fn kiro_login(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    provider: String,
    start_url: Option<String>, // 新增：支持自定义 start_url（Enterprise 用）
    region: Option<String>, // 新增：支持自定义 region（Enterprise 用）
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
    config: &crate::providers::ProviderConfig,
) -> Result<String, String> {
    let social_provider = create_social_provider(config);
    let provider_id = social_provider.get_provider_id().to_string();
    let auth_method = social_provider.get_auth_method();
    
    let auth_result = social_provider.login().await?;
    
    let usage_result = get_usage_by_provider(&provider_id, &auth_result.access_token).await?;
    
    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }
    
    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    
    // 获取不到邮箱直接报错
    let _final_email = new_email.clone().ok_or("获取邮箱失败，请检查账号状态")?;

    let mut store = state.store.lock().expect("Failed to acquire lock");
    let existing_idx = find_existing_account_idx(&store.accounts, new_email.as_ref(), &provider_id, &auth_result.refresh_token, user_id.as_ref());
    
    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        existing.access_token = Some(auth_result.access_token.clone());
        existing.refresh_token = Some(auth_result.refresh_token.clone());
        existing.email.clone_from(&new_email);
        existing.user_id.clone_from(&user_id);
        existing.expires_at = Some(auth_result.expires_at.clone());
        existing.profile_arn.clone_from(&auth_result.profile_arn);
        existing.label = format!("Kiro {provider_id} 账号");
        existing.usage_data = Some(usage_result.usage_data);
        existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let final_email = new_email.ok_or("获取邮箱失败")?;
        let mut account = Account::new(final_email.clone(), format!("Kiro {provider_id} 账号"));
        account.access_token = Some(auth_result.access_token.clone());
        account.refresh_token = Some(auth_result.refresh_token.clone());
        account.provider = Some(provider_id.clone());
        account.auth_method = Some("social".to_string());
        account.user_id.clone_from(&user_id);
        account.expires_at = Some(auth_result.expires_at.clone());
        account.profile_arn = auth_result.profile_arn;
        account.usage_data = Some(usage_result.usage_data);
        account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        store.accounts.insert(0, account.clone());
        account
    };
    
    store.save_to_file();
    drop(store);

    let display_id = account.get_display_id();
    update_auth_state(&state, account.email.as_ref(), &provider_id, &auth_result.access_token, &auth_result.refresh_token);
    println!("\n[{auth_method}] LOGIN SUCCESS: {display_id}");

    let _ = app_handle.emit("login-success", account.id.clone());
    Ok(format!("{auth_method} login completed for {provider_id}"))
}

async fn login_idc(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: &crate::providers::ProviderConfig,
) -> Result<String, String> {
    let idc_provider = create_idc_provider(config);
    let provider_id = idc_provider.get_provider_id().to_string();
    let auth_method = idc_provider.get_auth_method();
    
    let auth_result = idc_provider.login().await?;

    let usage_result = get_usage_by_provider(&provider_id, &auth_result.access_token).await?;
    
    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }
    
    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    
    // Enterprise 账号允许没有 email,使用 userId 作为标识
    let final_email = if provider_id == "Enterprise" {
        new_email.clone().or_else(|| user_id.clone()).ok_or("Enterprise 账号缺少 email 和 userId")?
    } else {
        new_email.clone().ok_or("获取邮箱失败，请检查账号状态")?
    };

    let mut store = state.store.lock().expect("Failed to acquire lock");
    let existing_idx = find_existing_account_idx(&store.accounts, new_email.as_ref(), &provider_id, &auth_result.refresh_token, user_id.as_ref());
    
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
        existing.start_url.clone_from(&auth_result.start_url);  // 保存 start_url
        existing.sso_session_id = auth_result.sso_session_id;
        existing.id_token = auth_result.id_token;
        existing.profile_arn = auth_result.profile_arn;
        existing.usage_data = Some(usage_result.usage_data);
        existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
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
        account.start_url.clone_from(&auth_result.start_url);  // 保存 start_url
        account.sso_session_id = auth_result.sso_session_id;
        account.id_token = auth_result.id_token;
        account.profile_arn = auth_result.profile_arn;
        account.usage_data = Some(usage_result.usage_data);
        account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        store.accounts.insert(0, account.clone());
        account
    };
    
    store.save_to_file();
    drop(store);

    let display_id = account.get_display_id();
    update_auth_state(&state, account.email.as_ref(), &provider_id, &auth_result.access_token, &auth_result.refresh_token);
    println!("\n[{auth_method}] LOGIN SUCCESS: {display_id}");

    let _ = app_handle.emit("login-success", account.id.clone());
    Ok(format!("{auth_method} login completed for {display_id}"))
}

fn update_auth_state(state: &State<'_, AppState>, email: Option<&String>, provider: &str, access_token: &str, refresh_token: &str) {
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: email.cloned(),
        name: email.and_then(|e| e.split('@').next()).unwrap_or("User").to_string(),
        avatar: None,
        provider: provider.to_string(),
    };
    *state.auth.user.lock().expect("Failed to acquire lock") = Some(user);
    *state.auth.access_token.lock().expect("Failed to acquire lock") = Some(access_token.to_string());
    *state.auth.refresh_token.lock().expect("Failed to acquire lock") = Some(refresh_token.to_string());
    *state.pending_login.lock().expect("Failed to acquire lock") = None;
}

#[tauri::command]
pub async fn handle_kiro_social_callback(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    code: String,
    callback_state: String,
) -> Result<(), String> {
    let pending = {
        let lock = state.pending_login.lock().expect("Failed to acquire lock");
        lock.clone().ok_or("No pending login found")?
    };
    
    if pending.state != callback_state {
        return Err("State mismatch".to_string());
    }
    
    let redirect_uri = crate::deep_link_handler::DeepLinkCallbackWaiter::get_redirect_uri()
        .replace("/authenticate-success", "/app/callback");
    let token_response = auth_social::exchange_social_code_for_token(
        &code, &pending.code_verifier, &redirect_uri, &pending.machineid,
    ).await?;
    
    let usage_result = get_usage_by_provider(&pending.provider, &token_response.access_token).await?;
    
    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }
    
    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    
    // 获取不到邮箱直接报错
    let final_email = new_email.clone().ok_or("获取邮箱失败，请检查账号状态")?;

    let mut store = state.store.lock().expect("Failed to acquire lock");
    let existing_idx = find_existing_account_idx(&store.accounts, new_email.as_ref(), &pending.provider, &token_response.refresh_token, user_id.as_ref());
    
    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        existing.access_token = Some(token_response.access_token.clone());
        existing.refresh_token = Some(token_response.refresh_token.clone());
        existing.email.clone_from(&new_email);
        existing.user_id.clone_from(&user_id);
        existing.usage_data = Some(usage_result.usage_data);
        existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {} 账号", pending.provider));
        account.access_token = Some(token_response.access_token.clone());
        account.refresh_token = Some(token_response.refresh_token.clone());
        account.provider = Some(pending.provider.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        store.accounts.insert(0, account.clone());
        account
    };
    
    store.save_to_file();
    drop(store);
    
    let display_id = account.get_display_id();
    update_auth_state(&state, account.email.as_ref(), &pending.provider, &token_response.access_token, &token_response.refresh_token);
    let _ = app_handle.emit("login-success", account.id);
    println!("Social callback login completed: {display_id}");
    Ok(())
}

#[tauri::command]
pub fn get_supported_providers() -> Vec<&'static str> {
    crate::providers::get_supported_providers()
}
