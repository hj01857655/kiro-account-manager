// Auth 相关命令 - 直接存储 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递 State

use crate::core::account::Account;
use crate::auth::User;
use crate::auth::auth_social;
use crate::commands::common::{
    calc_status, extract_user_info, find_existing_account_idx, get_usage_by_provider,
};
use crate::commands::machine_guid::get_machine_id;
use crate::clients::kiro_auth_client::KiroAuthServiceClient;
use crate::auth::providers::{
    cancel_pending_idc_login, create_idc_provider, get_provider_config, AuthMethod, AuthProvider,
};
use crate::state::AppState;
use std::sync::{Mutex, MutexGuard};
use tauri::{Emitter, State};

#[cfg(windows)]
fn ensure_kiro_protocol_points_to_current_app() -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current exe path: {e}"))?
        .display()
        .to_string();
    let command = format!("\"{exe_path}\" \"%1\"");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    for scheme in ["kiro", "kiro-account-manager"] {
        let class_path = format!("Software\\Classes\\{scheme}");
        let (class_key, _) = hkcu
            .create_subkey(&class_path)
            .map_err(|e| format!("Failed to create protocol key `{scheme}`: {e}"))?;
        class_key
            .set_value("", &format!("URL:{scheme} Protocol"))
            .map_err(|e| format!("Failed to set protocol title `{scheme}`: {e}"))?;
        class_key
            .set_value("URL Protocol", &"")
            .map_err(|e| format!("Failed to set URL Protocol flag `{scheme}`: {e}"))?;

        let (cmd_key, _) = hkcu
            .create_subkey(format!("{class_path}\\shell\\open\\command"))
            .map_err(|e| format!("Failed to create command key `{scheme}`: {e}"))?;
        cmd_key
            .set_value("", &command)
            .map_err(|e| format!("Failed to set protocol command `{scheme}`: {e}"))?;
    }

    Ok(())
}

fn lock_state<'a, T>(mutex: &'a Mutex<T>, label: &str) -> Result<MutexGuard<'a, T>, String> {
    mutex
        .lock()
        .map_err(|_| format!("Failed to acquire {label} lock"))
}

fn save_store(store: &crate::core::account::AccountStore) -> Result<(), String> {
    if store.save_to_file() {
        Ok(())
    } else {
        Err("保存账号数据失败".to_string())
    }
}

fn require_login_email(email: Option<String>) -> Result<String, String> {
    email.ok_or("获取邮箱失败，请检查账号状态".to_string())
}

fn resolve_idc_login_email(
    provider_id: &str,
    email: Option<String>,
    user_id: Option<String>,
) -> Result<String, String> {
    if provider_id == "Enterprise" || provider_id == "BuilderId" {
        email
            .or(user_id)
            .ok_or_else(|| format!("{} 账号缺少 userId 或 email", provider_id))
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
    match lock_state(&state.auth.user, "auth user") {
        Ok(user) => user.clone(),
        Err(err) => {
            eprintln!("[auth_cmd] {err}");
            None
        }
    }
}

#[tauri::command]
pub fn logout(state: State<AppState>) {
    clear_auth_state(&state.auth);
}

fn clear_auth_state(auth: &crate::auth::AuthState) {
    if let Ok(mut user) = lock_state(&auth.user, "auth user") {
        *user = None;
    }
    if let Ok(mut access_token) = lock_state(&auth.access_token, "auth access_token") {
        *access_token = None;
    }
    if let Ok(mut refresh_token) = lock_state(&auth.refresh_token, "auth refresh_token") {
        *refresh_token = None;
    }
}

#[tauri::command]
pub fn cancel_kiro_login(state: State<'_, AppState>) -> bool {
    let cancelled_social = crate::core::deep_link_handler::cancel_waiter();
    let cancelled_idc = cancel_pending_idc_login();
    match lock_state(&state.pending_login, "pending_login") {
        Ok(mut pending_login) => {
            *pending_login = None;
        }
        Err(err) => {
            eprintln!("[auth_cmd] {err}");
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
        AuthMethod::Social => login_social(state, &config).await,
        AuthMethod::Idc => login_idc(app_handle, state, &config).await,
    }
}

async fn login_social(
    state: State<'_, AppState>,
    config: &crate::auth::providers::ProviderConfig,
) -> Result<String, String> {
    #[cfg(windows)]
    ensure_kiro_protocol_points_to_current_app()?;

    let provider_id = config.provider_id.clone();
    let pending = prepare_pending_social_login(&provider_id, get_machine_id());
    let redirect_uri = social_callback_redirect_uri();
    let code_challenge = auth_social::generate_code_challenge_social(&pending.code_verifier);
    let client = KiroAuthServiceClient::new(&pending.machineid)?;

    *lock_state(&state.pending_login, "pending_login")? = Some(pending.clone());

    if let Err(err) = client
        .login(&provider_id, &redirect_uri, &code_challenge, &pending.state)
        .await
    {
        *lock_state(&state.pending_login, "pending_login")? = None;
        return Err(err);
    }

    println!("\n[social] LOGIN STARTED: {provider_id}");
    Ok(format!("social login started for {provider_id}"))
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

    let usage_result = get_usage_by_provider(&provider_id, &auth_result.access_token).await?;

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // Enterprise 账号允许没有 email,使用 userId 作为标识
    let final_email = resolve_idc_login_email(&provider_id, new_email.clone(), user_id.clone())?;

    let mut store = lock_state(&state.store, "store")?;
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
        account.start_url.clone_from(&auth_result.start_url); // 保存 start_url
        account.sso_session_id = auth_result.sso_session_id;
        account.id_token = auth_result.id_token;
        account.profile_arn = auth_result.profile_arn;
        account.usage_data = Some(usage_result.usage_data);
        account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
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
    *lock_state(&state.auth.user, "auth user")? = Some(user);
    *lock_state(&state.auth.access_token, "auth access_token")? = Some(access_token.to_string());
    *lock_state(&state.auth.refresh_token, "auth refresh_token")? = Some(refresh_token.to_string());
    *lock_state(&state.pending_login, "pending_login")? = None;
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
        let lock = lock_state(&state.pending_login, "pending_login")?;
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

    let usage_result =
        get_usage_by_provider(&pending.provider, &token_response.access_token).await?;

    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    let final_email = require_login_email(new_email.clone())?;

    let mut store = lock_state(&state.store, "store")?;
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
        existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
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
        account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
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
        clear_auth_state, prepare_pending_social_login, require_login_email,
        resolve_idc_login_email, social_callback_redirect_uri,
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

    #[test]
    fn prepare_pending_social_login_creates_callback_context() {
        let pending = prepare_pending_social_login("Google", "machine-123".to_string());

        assert_eq!(pending.provider, "Google");
        assert_eq!(pending.machineid, "machine-123");
        assert!(!pending.state.is_empty());
        assert!(!pending.code_verifier.is_empty());
    }

    #[test]
    fn social_callback_redirect_uri_uses_compat_callback_path() {
        let redirect_uri = social_callback_redirect_uri();

        assert!(redirect_uri.starts_with("kiro://"));
        assert!(redirect_uri.ends_with("/authenticate-success"));
    }
}
