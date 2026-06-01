// 账号相关命令 - 直接存储原始 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递 State
#![allow(clippy::too_many_lines)] // 命令文件包含多个函数

use crate::core::account::{Account, AccountProxyConfig};
use crate::auth::{refresh_token_desktop, User};
use crate::commands::account_models::{
    clear_available_models_cache, fetch_all_available_models, read_available_models_cache,
    write_available_models_cache, ListAvailableModelsResponse,
};
use crate::commands::common::{
    calc_expires_at, extract_user_info, find_account_by_id,
    find_existing_account_idx, get_enterprise_usage_with_region_probe, get_usage_by_account,
    get_usage_by_provider, is_auth_error_message,
    is_token_expired, is_token_expiring_soon,
    lock_store, refresh_token_by_provider, save_store, token_needs_refresh, update_account_status, RefreshResult,
};
use crate::auth::providers::{AuthProvider, IdcProvider, RefreshMetadata};
use crate::state::AppState;
use crate::utils::client_id_hash::{extract_start_url_from_client_secret, normalize_start_url};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[derive(Serialize)]
pub struct SyncAccountResult {
    pub account: Account,
    pub warning: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountParams {
    pub id: String,
    pub label: Option<String>,
    pub status: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub machine_id: Option<String>,
    pub enabled: Option<bool>,
    pub proxy_config: Option<AccountProxyConfig>,
}

// ===== 数据结构 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyAccountResponse {
    #[serde(rename = "usageData")]
    pub usage_data: serde_json::Value, // 直接返回原始数据，前端解析
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

/// 添加账号的返回结果（包含是否新增）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAccountResult {
    pub account: Account,
    #[serde(rename = "isNew")]
    pub is_new: bool, // true = 新增，false = 更新
}

/// `verify_account` 命令参数
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyAccountParams {
    #[allow(dead_code)]
    pub access_token: String,
    pub refresh_token: String,
    pub provider: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
}

/// account_cmd 的扩展：在通用 token 应用之上额外做缓存清理 / status 重置
fn apply_refreshed_account_tokens(account: &mut Account, refresh: &RefreshResult) {
    clear_available_models_cache(account);
    crate::commands::common::apply_refreshed_account_tokens(account, refresh);
    account.status = "active".to_string();
}

#[tauri::command]
pub fn get_accounts(state: State<AppState>) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(mut store) => {
            // 每次获取前重新从文件加载，确保数据最新
            store.reload();
            store.get_all()
        }
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

#[tauri::command]
pub fn delete_account(state: State<AppState>, id: &str) -> bool {
    match lock_store(&state.store, "store") {
        Ok(mut store) => store.delete(id).unwrap_or_else(|err| {
            eprintln!("[account_cmd] {err}");
            false
        }),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            false
        }
    }
}

#[tauri::command]
pub fn delete_accounts(state: State<AppState>, ids: Vec<String>) -> usize {
    match lock_store(&state.store, "store") {
        Ok(mut store) => store.delete_many(&ids).unwrap_or_else(|err| {
            eprintln!("[account_cmd] {err}");
            0
        }),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            0
        }
    }
}

#[tauri::command]
pub async fn sync_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<SyncAccountResult, String> {
    let account = find_account_by_id(&state, &id)?;

    let provider_str = account.provider.as_deref().unwrap_or("Google");
    let access_token = account.access_token.as_ref().ok_or("No access token")?;
    let is_enterprise = provider_str == "Enterprise";

    // 如果账号缺少 machine_id，自动生成一个（所有账号都需要）
    let mut account = account.clone();
    if account.machine_id.is_none() {
        use crate::commands::machine_guid::get_machine_id;
        let machine_id = get_machine_id();
        account.machine_id = Some(machine_id);
        log::info!("Generated machine_id for account: {}", account.id);
    }

    // 先尝试用现有 token 获取配额
    let mut usage_result = if is_enterprise {
        let machine_id = account
            .machine_id
            .as_ref()
            .ok_or("Enterprise account missing machine_id")?;
        get_enterprise_usage_with_region_probe(access_token, machine_id)
            .await
            .map(|(result, _region)| result)
    } else {
        get_usage_by_provider(provider_str, access_token).await
    };

    let mut refresh_result: Option<RefreshResult> = None;
    let mut detected_region: Option<String> = None;

    // 如果是认证错误，刷新 token 后重试
    let needs_refresh = match &usage_result {
        Ok(r) => r.is_auth_error,
        Err(_) => false,
    };

    if needs_refresh {
        match refresh_token_by_provider(&account).await {
            Ok(refreshed) => {
                usage_result = if is_enterprise {
                    let machine_id = account
                        .machine_id
                        .as_ref()
                        .ok_or("Enterprise account missing machine_id")?;
                    match get_enterprise_usage_with_region_probe(&refreshed.access_token, machine_id).await {
                        Ok((result, region)) => {
                            detected_region = Some(region);
                            Ok(result)
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    get_usage_by_provider(provider_str, &refreshed.access_token).await
                };
                refresh_result = Some(refreshed);
            }
            Err(e) => {
                if e.starts_with("BANNED:") || is_auth_error_message(&e) {
                    let mut store = lock_store(&state.store, "store")?;
                    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                        a.status = if e.starts_with("BANNED:") {
                            "banned".to_string()
                        } else {
                            "invalid".to_string()
                        };
                        a.enabled = false;
                        save_store(&store)?;
                    }
                }
                return Err(e);
            }
        }
    }

    // 获取配额失败时容错处理：只更新 token，不更新 usageData
    let (usage, warning) = match usage_result {
        Ok(u) => (Some(u), None),
        Err(e) => {
            // 获取配额失败，不打印日志，直接返回错误信息
            (None, Some(format!("获取配额失败: {e}")))
        }
    };

    let mut store = lock_store(&state.store, "store")?;
    let result = if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        // 如果生成了新的 machine_id，保存它（所有账号都需要）
        // 如果生成了新的 machine_id，保存它（所有账号都需要）
        if account.machine_id.is_some() && a.machine_id.is_none() {
            a.machine_id = account.machine_id.clone();
            log::info!("Saved machine_id for account: {}", a.id);
        }

        // 如果刷新了 token，更新 token 相关字段
        if let Some(ref result) = refresh_result {
            clear_available_models_cache(a);

            let email_display = a
                .email
                .as_deref()
                .or(a.user_id.as_deref())
                .unwrap_or("Unknown");

            // 刷新 Token 成功，更新账号信息
            a.access_token = Some(result.access_token.clone());
            if let Some(ref refresh_token) = result.refresh_token {
                a.refresh_token = Some(refresh_token.clone());
            }
            a.profile_arn = result.profile_arn.clone();
            a.id_token = result.id_token.clone();
            a.sso_session_id = result.sso_session_id.clone();
            a.expires_at = Some(calc_expires_at(result.expires_in));

            log::info!("Token refreshed successfully for account: {}", email_display);
        }
        // 如果探测到了新的区域，更新账户的 region 字段
        if let Some(region) = detected_region {
            a.region = Some(region);
        }

        // 只有成功获取配额时才更新 usage_data 和 status
        if let Some(usage_data) = usage {
            // 直接移动所有权，避免 clone
            a.usage_data = Some(usage_data.usage_data);
            update_account_status(a, usage_data.is_banned, usage_data.is_auth_error);

            // 从 usage_data 中提取并更新 email 和 user_id
            if let Some(user_info) = a.usage_data.as_ref().and_then(|d| d.get("userInfo")) {
                if let Some(email) = user_info.get("email").and_then(|v| v.as_str()) {
                    if !email.is_empty() {
                        a.email = Some(email.to_string());
                    }
                }
                if let Some(user_id) = user_info.get("userId").and_then(|v| v.as_str()) {
                    a.user_id = Some(user_id.to_string());
                }
            }
        } else if refresh_result.is_some() {
            // 获取配额失败，但 token 刷新成功了，说明 token 是有效的
            // 将状态设置为 active（避免显示为失效状态）
            if !matches!(a.status.as_str(), "banned" | "封禁" | "已封禁") {
                a.status = "active".to_string();
            }
        }

        // 克隆结果（这个必须 clone，因为要返回给前端）
        Some(a.clone())
    } else {
        None
    };

    // 保存文件
    save_store(&store)?;

    match result {
        Some(account) => Ok(SyncAccountResult { account, warning }),
        None => Err("Account not found after update".to_string()),
    }
}

/// 只刷新 token，不获取 usage（启动时快速刷新用）
/// 如果 token 还有 5 分钟以上有效期，跳过刷新直接返回
#[tauri::command]
pub async fn refresh_account_token(
    state: State<'_, AppState>,
    id: String,
) -> Result<Account, String> {
    let account = find_account_by_id(&state, &id)?;

    // 检查 token 是否还有 5 分钟以上有效期
    if let Some(expires_at) = &account.expires_at {
        if let Ok(exp) = chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S") {
            let now = chrono::Local::now().naive_local();
            let remaining = exp.signed_duration_since(now);
            if remaining.num_minutes() >= 5 {
                return Ok(account);
            }
        }
    }

    let refresh_result = match refresh_token_by_provider(&account).await {
        Ok(result) => result,
        Err(e) => {
            if e.starts_with("BANNED:") || is_auth_error_message(&e) {
                let mut store = lock_store(&state.store, "store")?;
                if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                    a.status = if e.starts_with("BANNED:") {
                        "banned".to_string()
                    } else {
                        "invalid".to_string()
                    };
                    a.enabled = false;
                    save_store(&store)?;
                }
            }
            return Err(e);
        }
    };

    let mut store = lock_store(&state.store, "store")?;
    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        clear_available_models_cache(a);
        // 直接移动所有权，避免 clone
        a.access_token = Some(refresh_result.access_token);
        a.refresh_token = refresh_result.refresh_token;
        a.expires_at = Some(calc_expires_at(refresh_result.expires_in));
        if matches!(
            a.status.as_str(),
            "invalid" | "失效" | "已失效" | "Token已失效"
        ) {
            a.status = "active".to_string();
        }
        let result = a.clone();
        save_store(&store)?;
        return Ok(result);
    }
    Err("Account not found after update".to_string())
}

#[tauri::command]
pub async fn verify_account(
    state: State<'_, AppState>,
    params: VerifyAccountParams,
) -> Result<VerifyAccountResponse, String> {
    let VerifyAccountParams {
        access_token: _,
        refresh_token,
        provider,
        client_id,
        client_secret,
        region,
    } = params;

    let is_idc = provider == "BuilderId" || provider == "Enterprise";
    let existing_account = {
        let store = lock_store(&state.store, "store")?;
        store
            .accounts
            .iter()
            .find(|a| a.refresh_token.as_ref() == Some(&refresh_token))
            .cloned()
    };

    // 刷新 token
    let (new_access_token, new_refresh_token) = if is_idc {
        let (cid, csec, reg) = if client_id.is_some() && client_secret.is_some() {
            (client_id, client_secret, region)
        } else {
            existing_account
                .as_ref()
                .map_or((None, None, None), |a| {
                    (
                        a.client_id.clone(),
                        a.client_secret.clone(),
                        a.region.clone(),
                    )
                })
        };

        let cid = cid.ok_or("IdC 账号缺少 client_id，请重新添加账号")?;
        let csec = csec.ok_or("IdC 账号缺少 client_secret，请重新添加账号")?;
        let metadata = RefreshMetadata {
            client_id: Some(cid),
            client_secret: Some(csec),
            region: reg.clone(),
            ..Default::default()
        };

        let idc_provider = IdcProvider::new(&provider, reg.as_deref().unwrap_or("us-east-1"), None);
        let auth = idc_provider.refresh_token(&refresh_token, metadata).await?;
        (auth.access_token, auth.refresh_token)
    } else {
        let auth = refresh_token_desktop(&refresh_token).await?;
        (auth.access_token, auth.refresh_token)
    };

    // 获取 usage_data（使用统一的 getUsageLimits 接口）
    let temp_account = {
        let account = existing_account.as_ref().ok_or("Account not found")?;
        let mut temp_account = account.clone();
        temp_account.access_token = Some(new_access_token.clone());
        temp_account
    }; // MutexGuard 在这里被释放

    let usage_result = get_usage_by_account(&temp_account, &new_access_token).await?;
    let usage_data = usage_result.usage_data.clone();

    // 更新数据库（包括状态）
    {
        let mut store = lock_store(&state.store, "store")?;
        if let Some(account) = store
            .accounts
            .iter_mut()
            .find(|a| a.refresh_token.as_ref() == Some(&refresh_token))
        {
            // 更新 token
            account.access_token = Some(new_access_token.clone()); // ✅ 这里必须 clone，因为后面还要用
            account.refresh_token = Some(new_refresh_token.clone()); // ✅ 这里必须 clone，因为后面还要用
            // 更新 usage_data 和状态（检测封禁）
            account.usage_data = Some(usage_result.usage_data);
            update_account_status(account, usage_result.is_banned, usage_result.is_auth_error);
            save_store(&store)?;
        }
    }

    Ok(VerifyAccountResponse {
        usage_data, // 直接返回，前端解析
        access_token: new_access_token,
        refresh_token: new_refresh_token,
    })
}

#[tauri::command]
pub async fn add_account_by_social(
    state: State<'_, AppState>,
    refresh_token: String,
    provider: Option<String>,
    machine_id: Option<String>,
    access_token: Option<String>,
) -> Result<AddAccountResult, String> {
    let idp = provider.as_deref().unwrap_or("Google").to_string(); // ✅ 避免不必要的 clone

    // 先尝试用传入的 access_token 获取配额
    let (final_access_token, final_refresh_token, final_profile_arn, usage_result) =
        if let Some(at) = access_token {
            match get_usage_by_provider(&idp, &at).await {
                Ok(result) if result.is_auth_error => {
                    // 401 了，刷新 token
                    let refresh_result = refresh_token_desktop(&refresh_token).await?;
                    let new_usage =
                        get_usage_by_provider(&idp, &refresh_result.access_token).await?;
                    (
                        refresh_result.access_token,
                        refresh_result.refresh_token,
                        refresh_result.profile_arn,
                        new_usage,
                    )
                }
                Ok(result) => {
                    // access_token 有效，但没有 profile_arn，需要刷新一次获取
                    let refresh_result = refresh_token_desktop(&refresh_token).await?;
                    (
                        at,
                        refresh_token.clone(),
                        refresh_result.profile_arn,
                        result,
                    )
                }
                Err(e) => return Err(e),
            }
        } else {
            // 没有 access_token，直接刷新
            let refresh_result = refresh_token_desktop(&refresh_token).await?;
            let usage_result = get_usage_by_provider(&idp, &refresh_result.access_token).await?;
            (
                refresh_result.access_token,
                refresh_result.refresh_token,
                refresh_result.profile_arn,
                usage_result,
            )
        };

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // BuilderId 账号允许使用 userId 或 email，如果都没有则用 refreshToken 作为标识
    let final_email = new_email
        .or(user_id.clone())
        .unwrap_or_else(|| format!("builderid_{}", &refresh_token[..8]));

    // 根据邮箱推断最终 provider
    let idp = provider.unwrap_or_else(|| {
        if final_email.contains("gmail") {
            "Google".to_string()
        } else if final_email.contains("github") {
            "Github".to_string()
        } else {
            "Google".to_string()
        }
    });

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        Some(&final_email),
        &idp,
        &final_refresh_token,
        user_id.as_ref(),
    );

    let is_new = existing_idx.is_none();

    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        // 直接移动所有权，避免 clone
        existing.access_token = Some(final_access_token.clone()); // ✅ 后面还要用，必须 clone
        existing.refresh_token = Some(final_refresh_token.clone()); // ✅ 后面还要用，必须 clone
        existing.profile_arn = Some(final_profile_arn.clone()); // ✅ 保存 profile_arn
        existing.user_id = user_id;
        existing.usage_data = Some(usage_result.usage_data);
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone() // ✅ 必须 clone，因为要返回给前端
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {idp} 账号"));
        account.access_token = Some(final_access_token.clone()); // ✅ 后面还要用，必须 clone
        account.refresh_token = Some(final_refresh_token.clone()); // ✅ 后面还要用，必须 clone
        account.profile_arn = Some(final_profile_arn.clone()); // ✅ 保存 profile_arn
        account.provider = Some(idp.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        update_account_status(&mut account, usage_result.is_banned, usage_result.is_auth_error);
        // 使用传入的 machine_id，没有则自动生成
        account.machine_id =
            machine_id.or_else(|| Some(uuid::Uuid::new_v4().to_string().to_lowercase())); // ✅ 避免 clone
        store.accounts.insert(0, account.clone());
        account
    };

    save_store(&store)?;
    drop(store);

    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: account.email.clone(), // ✅ 必须 clone，因为 account 被移动了
        name: account
            .email
            .as_ref()
            .and_then(|e| e.split('@').next())
            .unwrap_or("User")
            .to_string(),
        avatar: None,
        provider: idp,
    };
    *lock_store(&state.auth.user, "auth user")? = Some(user);
    *lock_store(&state.auth.access_token, "auth access_token")? = Some(final_access_token);

    Ok(AddAccountResult { account, is_new })
}

#[tauri::command]
pub fn import_accounts(state: State<AppState>, json: &str) -> Result<usize, String> {
    let mut store = lock_store(&state.store, "store")?;
    store.import_from_json(json)
}

#[tauri::command]
pub fn export_accounts(state: State<AppState>, ids: Option<Vec<String>>) -> String {
    let store = match lock_store(&state.store, "store") {
        Ok(store) => store,
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            return "[]".to_string();
        }
    };

    // 修复账号数据
    let fix_account = |mut account: Account| -> Account {
        // 1. 修复 provider 为 null
        if account.provider.is_none() && account.auth_method.as_deref() == Some("IdC") {
            // IdC 账号但 provider 为 null，根据 start_url 或 client_secret 判断
            if let Some(ref start_url) = account.start_url {
                if start_url.contains("awsapps.com") {
                    account.provider = Some("Enterprise".to_string());
                } else {
                    account.provider = Some("BuilderId".to_string());
                }
            } else if let Some(ref client_secret) = account.client_secret {
                if client_secret.contains("initiateLoginUri") {
                    account.provider = Some("Enterprise".to_string());
                } else {
                    account.provider = Some("BuilderId".to_string());
                }
            } else {
                // 默认 BuilderId
                account.provider = Some("BuilderId".to_string());
            }
        } else if account.provider.is_none() && account.auth_method.as_deref() == Some("social") {
            // Social 账号但 provider 为 null，根据邮箱判断
            if let Some(ref email) = account.email {
                if email.contains("gmail") {
                    account.provider = Some("Google".to_string());
                } else if email.contains("github") {
                    account.provider = Some("Github".to_string());
                } else {
                    account.provider = Some("Google".to_string());
                }
            } else {
                account.provider = Some("Google".to_string());
            }
        }

        // 2. 修复 authMethod 为 null
        if account.auth_method.is_none() {
            if account.client_id.is_some() && account.client_secret.is_some() {
                account.auth_method = Some("IdC".to_string());
            } else {
                account.auth_method = Some("social".to_string());
            }
        }

        account
    };

    match ids {
        Some(id_list) if !id_list.is_empty() => {
            // 导出选中的账号
            let selected: Vec<Account> = store
                .accounts
                .iter()
                .filter(|a| id_list.contains(&a.id))
                .cloned()
                .map(fix_account)
                .collect();
            serde_json::to_string_pretty(&selected).unwrap_or_else(|_| "[]".to_string())
        }
        _ => {
            // 没有选中任何账号，返回空数组
            "[]".to_string()
        }
    }
}

/// 添加本地 Kiro IDE 账号
#[tauri::command]
pub async fn add_local_kiro_account(
    state: State<'_, AppState>,
) -> Result<AddAccountResult, String> {
    use crate::kiro::ide::{get_client_registration, get_kiro_local_token};

    let local_token = get_kiro_local_token()
        .await
        .ok_or("未找到本地 Kiro 账号，请先在 Kiro IDE 中登录")?;

    let refresh_token = local_token
        .refresh_token
        .ok_or("本地账号缺少 refresh_token")?;

    let auth_method = local_token.auth_method.as_deref().unwrap_or("social");
    let provider = local_token
        .provider
        .clone()
        .unwrap_or_else(|| "Google".to_string());

    // 根据 auth_method 调用对应的添加函数
    if auth_method == "IdC" {
        let hash = local_token
            .client_id_hash
            .clone()
            .ok_or("IdC 账号缺少 clientIdHash")?;
        let region = local_token
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string());

        let client_reg = get_client_registration(&hash)
            .await
            .ok_or(format!("未找到客户端注册信息: {hash}.json"))?;

        // 统一调用 add_account_by_idc（展开参数）
        add_account_by_idc(
            state,
            Some(provider),                   // provider: BuilderId 或 Enterprise
            refresh_token,                    // refresh_token
            client_reg.client_id,             // client_id
            client_reg.client_secret,         // client_secret
            Some(region),                     // region
            None,                             // machine_id: 本地导入不指定，自动生成
            local_token.access_token.clone(), // access_token
            None,                             // password: 本地导入无密码
            None,                             // start_url: 本地导入无 start_url
            Some(hash),                       // client_id_hash: 直接使用 Kiro IDE 提供的
        )
        .await
    } else {
        add_account_by_social(
            state,
            refresh_token,
            Some(provider),
            None,                             // 本地导入不指定 machine_id，自动生成
            local_token.access_token.clone(), // 传入 access_token
        )
        .await
    }
}

/// 添加 IdC 账号（BuilderId 或 Enterprise）
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令签名需要显式参数，避免前后端调用契约破坏
pub async fn add_account_by_idc(
    state: State<'_, AppState>,
    provider: Option<String>,
    refresh_token: String,
    client_id: String,
    client_secret: String,
    region: Option<String>,
    machine_id: Option<String>,
    access_token: Option<String>,
    password: Option<String>,
    start_url: Option<String>,
    client_id_hash: Option<String>,
) -> Result<AddAccountResult, String> {
    // 从参数中获取 provider，默认为 BuilderId
    let provider_id = provider.unwrap_or_else(|| "BuilderId".to_string());

    // 验证 provider 是否合法
    if provider_id != "BuilderId" && provider_id != "Enterprise" {
        return Err(format!("不支持的 provider: {}", provider_id));
    }

    add_account_by_idc_internal(
        state,
        IdcAccountParams {
            refresh_token,
            client_id,
            client_secret,
            region,
            machine_id,
            access_token,
            password,
            provider_id,
            start_url,
            client_id_hash,
        },
    )
    .await
}

/// `IdC` 账号添加参数
struct IdcAccountParams {
    refresh_token: String,
    client_id: String,
    client_secret: String,
    region: Option<String>,
    machine_id: Option<String>,
    access_token: Option<String>,
    password: Option<String>,
    provider_id: String,
    start_url: Option<String>,
    client_id_hash: Option<String>,
}

/// 内部函数：添加 `IdC` 账号（BuilderId 或 Enterprise）
async fn add_account_by_idc_internal(
    state: State<'_, AppState>,
    params: IdcAccountParams,
) -> Result<AddAccountResult, String> {
    let is_enterprise = params.provider_id == "Enterprise";

    // 从 clientSecret JWT 中提取 startUrl（如果未提供）。
    // 两条来源都过 normalize_start_url 去尾斜杠：params.start_url 可能由前端带斜杠传入，
    // JWT 真相源虽已规范化，这里统一兜底，保证落进 account.start_url 的永远是无斜杠规范形。
    let start_url = if let Some(ref url) = params.start_url {
        Some(normalize_start_url(url))
    } else if is_enterprise {
        extract_start_url_from_client_secret(&params.client_secret)
    } else {
        None
    };

    // BuilderId 和 Enterprise 都使用默认 region（如果未提供）
    let mut region = params.region.unwrap_or_else(|| "us-east-1".to_string());

    // 获取 machine_id（企业账号多区域探测需要）
    let machine_id = params
        .machine_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string().to_lowercase());

    // 企业账号导入时强制刷新 token（导入时的 access_token 很可能已过期）
    let (
        final_access_token,
        final_refresh_token,
        usage_result,
        expires_at,
        id_token,
        sso_session_id,
    ) = if is_enterprise || params.access_token.is_none() {
        // 企业账号或没有 access_token 时，直接刷新
        let metadata = RefreshMetadata {
            client_id: Some(params.client_id.clone()),
            client_secret: Some(params.client_secret.clone()),
            region: Some(region.clone()),
            ..Default::default()
        };
        let idc_provider = IdcProvider::new(&params.provider_id, &region, start_url.clone());
        let auth_result = idc_provider
            .refresh_token(&params.refresh_token, metadata)
            .await?;

        // 企业账号使用多区域探测
        let usage_result = if is_enterprise {
            let (result, detected_region) = get_enterprise_usage_with_region_probe(&auth_result.access_token, &machine_id).await?;
            region = detected_region;
            result
        } else {
            get_usage_by_provider(&params.provider_id, &auth_result.access_token).await?
        };

        let expires_at = calc_expires_at(auth_result.expires_in);
        (
            auth_result.access_token,
            auth_result.refresh_token,
            usage_result,
            expires_at,
            auth_result.id_token,
            auth_result.sso_session_id,
        )
    } else if let Some(at) = params.access_token {
        // BuilderId 且有 access_token 时，先尝试使用
            // BuilderId 使用原有逻辑
            match get_usage_by_provider(&params.provider_id, &at).await {
                Ok(result) if result.is_auth_error => {
                    // 401 了，刷新 token
                    let metadata = RefreshMetadata {
                        client_id: Some(params.client_id.clone()),
                        client_secret: Some(params.client_secret.clone()),
                        region: Some(region.clone()),
                        ..Default::default()
                    };
                    let idc_provider =
                        IdcProvider::new(&params.provider_id, &region, start_url.clone());
                    let auth_result = idc_provider
                        .refresh_token(&params.refresh_token, metadata)
                        .await?;
                    let new_usage =
                        get_usage_by_provider(&params.provider_id, &auth_result.access_token).await?;
                    let expires_at = calc_expires_at(auth_result.expires_in);
                    (
                        auth_result.access_token,
                        auth_result.refresh_token,
                        new_usage,
                        expires_at,
                        auth_result.id_token,
                        auth_result.sso_session_id,
                    )
                }
                Ok(result) => {
                    // access_token 有效，不需要刷新
                    (
                        at,
                        params.refresh_token.clone(),
                        result,
                        String::new(),
                        None,
                        None,
                    )
                }
                Err(e) => return Err(e),
            }
    } else {
        // 没有 access_token，直接刷新
        let metadata = RefreshMetadata {
            client_id: Some(params.client_id.clone()),
            client_secret: Some(params.client_secret.clone()),
            region: Some(region.clone()),
            ..Default::default()
        };
        let idc_provider = IdcProvider::new(&params.provider_id, &region, start_url.clone());
        let auth_result = idc_provider
            .refresh_token(&params.refresh_token, metadata)
            .await?;

        // 企业账号使用多区域探测
        let usage_result = if is_enterprise {
            let (result, detected_region) = get_enterprise_usage_with_region_probe(&auth_result.access_token, &machine_id).await?;
            region = detected_region;
            result
        } else {
            get_usage_by_provider(&params.provider_id, &auth_result.access_token).await?
        };

        let expires_at = calc_expires_at(auth_result.expires_in);
        (
            auth_result.access_token,
            auth_result.refresh_token,
            usage_result,
            expires_at,
            auth_result.id_token,
            auth_result.sso_session_id,
        )
    };

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // ========== Enterprise 和 BuilderId 分开处理 ==========

    if is_enterprise {
        // Enterprise 账号：必须有 user_id，email 可选
        let user_id = user_id.ok_or_else(|| {
            format!(
                "Enterprise 账号缺少 userId。API 返回的数据：\n{}",
                serde_json::to_string_pretty(&usage_result.usage_data).unwrap_or_default()
            )
        })?;

        // 解析 client_id_hash：走统一裁决点，Enterprise 会在此硬校验（issue #119）
        let client_id_hash = Some(crate::commands::common::resolve_idc_client_id_hash(
            &params.provider_id,
            params.client_id_hash.as_deref(),
            start_url.as_deref(),
        )?);

        let mut store = lock_store(&state.store, "store")?;
        let existing_idx = find_existing_account_idx(
            &store.accounts,
            new_email.as_ref(),
            &params.provider_id,
            &final_refresh_token,
            Some(&user_id),
        );

        let is_new = existing_idx.is_none();

        let account = if let Some(idx) = existing_idx {
            // 更新已存在的账号
            let existing = &mut store.accounts[idx];
            existing.access_token = Some(final_access_token);
            existing.refresh_token = Some(final_refresh_token);
            existing.email = new_email; // 更新 email（可能是 None）
            existing.user_id = Some(user_id);
            existing.provider = Some(params.provider_id.clone());
            existing.auth_method = Some("IdC".to_string()); // 确保 authMethod 正确
            if !expires_at.is_empty() {
                existing.expires_at = Some(expires_at);
            }
            existing.client_id = Some(params.client_id.clone());
            existing.client_secret = Some(params.client_secret.clone());
            existing.region = Some(region.clone());
            existing.client_id_hash = client_id_hash.clone(); // 可能是 None
            existing.start_url = start_url.clone();
            if id_token.is_some() {
                existing.id_token = id_token;
            }
            if sso_session_id.is_some() {
                existing.sso_session_id = sso_session_id;
            }
            existing.usage_data = Some(usage_result.usage_data);
            update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
            existing.clone()
        } else {
            // 创建新的 Enterprise 账号
            let mut account =
                Account::new_enterprise(user_id.clone(), "Kiro Enterprise 账号".to_string());
            account.access_token = Some(final_access_token);
            account.refresh_token = Some(final_refresh_token);
            account.email = new_email; // 可能是 None
            if !expires_at.is_empty() {
                account.expires_at = Some(expires_at);
            }
            account.client_id = Some(params.client_id.clone());
            account.client_secret = Some(params.client_secret.clone());
            account.region = Some(region.clone());
            account.client_id_hash = client_id_hash; // 可能是 None
            account.start_url = start_url.clone();
            account.id_token = id_token;
            account.sso_session_id = sso_session_id;
            account.usage_data = Some(usage_result.usage_data);
            update_account_status(&mut account, usage_result.is_banned, usage_result.is_auth_error);
            account.machine_id = Some(
                params
                    .machine_id
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string().to_lowercase()),
            );
            account.password.clone_from(&params.password);
            store.accounts.insert(0, account.clone());
            account
        };

        save_store(&store)?;
        Ok(AddAccountResult { account, is_new })
    } else {
        // BuilderId 账号：允许没有 userId/email，用 refreshToken 去重

        // 解析 client_id_hash：走统一裁决点（BuilderId 缺 startUrl 时用常量兜底）
        let client_id_hash = Some(crate::commands::common::resolve_idc_client_id_hash(
            &params.provider_id,
            params.client_id_hash.as_deref(),
            params.start_url.as_deref(),
        )?);

        let mut store = lock_store(&state.store, "store")?;
        let existing_idx = find_existing_account_idx(
            &store.accounts,
            new_email.as_ref(),
            &params.provider_id,
            &final_refresh_token,
            user_id.as_ref(),
        );

        let is_new = existing_idx.is_none();

        let account = if let Some(idx) = existing_idx {
            // 更新已存在的账号
            let existing = &mut store.accounts[idx];
            existing.access_token = Some(final_access_token);
            existing.refresh_token = Some(final_refresh_token);
            existing.provider = Some(params.provider_id.clone());
            existing.auth_method = Some("IdC".to_string()); // 确保 authMethod 正确
            existing.user_id = user_id;
            if !expires_at.is_empty() {
                existing.expires_at = Some(expires_at);
            }
            existing.client_id = Some(params.client_id.clone());
            existing.client_secret = Some(params.client_secret.clone());
            existing.region = Some(region.clone());
            existing.client_id_hash = client_id_hash.clone(); // 可能是 None
            if id_token.is_some() {
                existing.id_token = id_token;
            }
            if sso_session_id.is_some() {
                existing.sso_session_id = sso_session_id;
            }
            existing.usage_data = Some(usage_result.usage_data);
            update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
            existing.clone()
        } else {
            // 创建新的 BuilderId 账号
            // 使用 user_id 或 email 作为标识
            let display_id = new_email
                .clone()
                .or_else(|| user_id.clone())
                .unwrap_or_else(|| "BuilderId 账号".to_string());

            let mut account = Account::new(display_id.clone(), "Kiro BuilderId 账号".to_string());
            account.access_token = Some(final_access_token);
            account.refresh_token = Some(final_refresh_token);
            account.provider = Some(params.provider_id.clone());
            account.auth_method = Some("IdC".to_string());
            account.email = new_email; // 可能是 None
            account.user_id = user_id; // 可能是 None
            if !expires_at.is_empty() {
                account.expires_at = Some(expires_at);
            }
            account.client_id = Some(params.client_id.clone());
            account.client_secret = Some(params.client_secret.clone());
            account.region = Some(region.clone());
            account.client_id_hash = client_id_hash; // 可能是 None
            account.id_token = id_token;
            account.sso_session_id = sso_session_id;
            account.usage_data = Some(usage_result.usage_data);
            update_account_status(&mut account, usage_result.is_banned, usage_result.is_auth_error);
            account.machine_id = Some(
                params
                    .machine_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string().to_lowercase()),
            );
            account.password = params.password;
            store.accounts.insert(0, account.clone());
            account
        };

        save_store(&store)?;
        Ok(AddAccountResult { account, is_new })
    }
}

/// 更新账号信息（支持修改 label、token、SSO Client ID/Secret、machineId）
#[tauri::command]
pub fn update_account(
    state: State<AppState>,
    params: UpdateAccountParams,
) -> Result<Account, String> {
    let mut store = lock_store(&state.store, "store")?;

    // 先找到索引，避免借用冲突
    let idx = store.accounts.iter().position(|a| a.id == params.id);

    if let Some(idx) = idx {
        if let Some(l) = params.label {
            store.accounts[idx].label = l;
        }
        if let Some(status) = params.status {
            store.accounts[idx].status = status;
        }
        if let Some(at) = params.access_token {
            store.accounts[idx].access_token = Some(at);
        }
        if let Some(rt) = params.refresh_token {
            store.accounts[idx].refresh_token = Some(rt);
        }
        // BuilderId SSO 字段
        if let Some(cid) = params.client_id {
            store.accounts[idx].client_id = Some(cid);
        }
        if let Some(csec) = params.client_secret {
            store.accounts[idx].client_secret = Some(csec);
        }
        // 机器码
        if let Some(mid) = params.machine_id {
            store.accounts[idx].machine_id = Some(mid);
        }
        // 启用/禁用
        if let Some(enabled) = params.enabled {
            store.accounts[idx].enabled = enabled;
        }
        if let Some(proxy_config) = params.proxy_config {
            let has_proxy_values = !proxy_config.host.trim().is_empty()
                || proxy_config.port > 0
                || proxy_config
                    .username
                    .as_deref()
                    .map_or(false, |value| !value.trim().is_empty())
                || proxy_config
                    .password
                    .as_deref()
                    .map_or(false, |value| !value.is_empty());
            let next_proxy_config = if proxy_config.enabled || has_proxy_values {
                Some(proxy_config)
            } else {
                None
            };
            if store.accounts[idx].proxy_config != next_proxy_config {
                store.accounts[idx].proxy_config = next_proxy_config;
                clear_available_models_cache(&mut store.accounts[idx]);
            }
        }
        let result = store.accounts[idx].clone();
        save_store(&store)?;
        Ok(result)
    } else {
        Err("账号不存在".to_string())
    }
}

/// 从 AWS 服务端删除账号（注销账号）
/// 仅支持 Google、Github，不支持 `BuilderId` 和 `Enterprise`
#[tauri::command]
pub async fn delete_account_remote(
    state: State<'_, AppState>,
    id: String,
    delete_local: bool,
) -> Result<String, String> {
    use crate::auth::delete_account_desktop;
    use crate::commands::machine_guid::get_machine_id;

    // 获取账号信息
    let account = find_account_by_id(&state, &id)?;

    // 检查 provider
    let provider = account.provider.as_deref().unwrap_or("Google");
    if provider == "Enterprise" {
        return Err("Enterprise 账号不支持远程删除".to_string());
    }
    if provider == "BuilderId" {
        return Err("BuilderId 账号不支持远程删除".to_string());
    }

    let access_token = account
        .access_token
        .as_ref()
        .ok_or("账号缺少 access_token，请先刷新")?;

    // Google/Github 账号使用 Desktop API
    let machine_id = get_machine_id();
    delete_account_desktop(access_token, &machine_id).await?;

    // 如果需要同时删除本地记录
    if delete_local {
        let mut store = lock_store(&state.store, "store")?;
        store.delete(&id)?;
    }

    Ok(format!("账号 {} 已从服务端删除", account.get_display_id()))
}

// ============================================================
// 筛选查询命令
// ============================================================

/// 获取可用账号列表（用于自动换号）
#[tauri::command]
pub fn get_available_accounts(state: State<AppState>) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(store) => store
            .get_available_accounts()
            .into_iter()
            .cloned()
            .collect(),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

/// 按分组筛选账号
#[tauri::command]
pub fn get_accounts_by_group(state: State<AppState>, group_id: String) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(store) => store
            .get_accounts_by_group(&group_id)
            .into_iter()
            .cloned()
            .collect(),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

/// 按标签筛选账号
#[tauri::command]
pub fn get_accounts_by_tag(state: State<AppState>, tag_id: String) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(store) => store
            .get_accounts_by_tag(&tag_id)
            .into_iter()
            .cloned()
            .collect(),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

// ============================================================
// 配额查询接口
// ============================================================

/// 获取账号配额信息（不刷新 token，不更新数据库）
#[tauri::command]
pub async fn get_account_usage(
    access_token: String,
    provider: Option<String>,
) -> Result<serde_json::Value, String> {
    let provider_str = provider.as_deref().unwrap_or("Google");
    let usage_result = get_usage_by_provider(provider_str, &access_token).await?;
    Ok(usage_result.usage_data)
}

#[tauri::command]
pub async fn list_available_models(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
    model_provider: Option<String>,
    force_refresh: Option<bool>,
) -> Result<ListAvailableModelsResponse, String> {
    let mut account = find_account_by_id(&state, &id)?;

    if let Some(cached_response) = read_available_models_cache(
        &account,
        model_provider.as_deref(),
        force_refresh.unwrap_or(false),
    ) {
        return Ok(cached_response);
    }

    let initial_access_token = account
        .access_token
        .clone()
        .ok_or("账号缺少 access_token，请先刷新 Token")?;

    match fetch_all_available_models(&account, &initial_access_token, model_provider.as_deref())
        .await
    {
        Ok(response) => {
            let mut store = lock_store(&state.store, "store")?;
            if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                write_available_models_cache(stored_account, model_provider.as_deref(), &response)?;
                save_store(&store)?;
            }
            Ok(response)
        }
        Err(error) if is_auth_error_message(&error) => {
            let refresh = refresh_token_by_provider(&account).await?;
            apply_refreshed_account_tokens(&mut account, &refresh);

            {
                let mut store = lock_store(&state.store, "store")?;
                let stored_account = store
                    .accounts
                    .iter_mut()
                    .find(|item| item.id == id)
                    .ok_or("账号不存在")?;
                apply_refreshed_account_tokens(stored_account, &refresh);
                save_store(&store)?;
            }
            let response = fetch_all_available_models(
                &account,
                &refresh.access_token,
                model_provider.as_deref(),
            )
            .await?;
            {
                let mut store = lock_store(&state.store, "store")?;
                if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                    write_available_models_cache(
                        stored_account,
                        model_provider.as_deref(),
                        &response,
                    )?;
                    save_store(&store)?;
                }
            }
            Ok(response)
        }
        Err(error) if error.starts_with("BANNED:") => {
            // 更新账号状态为封禁
            let mut store = lock_store(&state.store, "store")?;
            if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                stored_account.status = "banned".to_string();
                stored_account.enabled = false;
                save_store(&store)?;
                // 通知前端刷新账号列表
                let _ = app.emit("accounts-updated", ());
            }
            Err(error)
        }
        Err(error) => Err(error),
    }
}

// ============================================================
// Token 状态检查接口（参考 Kiro IDE 源码）
// ============================================================

/// Token 状态检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenStatusResponse {
    pub status: String, // "active" | "expiring_soon" | "expired" | "invalid"
    pub expires_at: String,
    pub expires_in_seconds: i64,
    pub needs_refresh: bool,
}

/// Token 状态汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStatusSummary {
    pub total: usize,
    pub active: usize,
    pub expiring_soon: usize,
    pub expired: usize,
    pub invalid: usize,
    pub accounts_need_refresh: Vec<AccountRefreshInfo>,
}

/// 需要刷新的账号信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRefreshInfo {
    pub id: String,
    pub email: Option<String>,
    pub provider: String,
    pub expires_at: String,
    pub expires_in_seconds: i64,
}

/// 批量刷新响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshAllResponse {
    pub total_attempted: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<RefreshResultItem>,
}

/// 单个刷新结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResultItem {
    pub id: String,
    pub email: Option<String>,
    pub success: bool,
    pub message: String,
}

/// 计算剩余秒数
fn calculate_expires_in_seconds(expires_at: &str) -> Result<i64, String> {
    let expires = chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S")
        .map_err(|e| format!("Failed to parse expires_at: {}", e))?;
    let now = chrono::Local::now().naive_local();
    let remaining = expires.signed_duration_since(now);
    Ok(remaining.num_seconds())
}

/// 检查单个账号的 Token 状态
#[tauri::command]
pub fn check_token_status(
    state: State<AppState>,
    id: String,
) -> Result<CheckTokenStatusResponse, String> {
    let store = lock_store(&state.store, "store")?;
    let account = store
        .accounts
        .iter()
        .find(|a| a.id == id)
        .ok_or("Account not found")?;

    let expires_at = account
        .expires_at
        .as_ref()
        .ok_or("No expiration time")?;

    let status = if is_token_expired(expires_at) {
        "expired"
    } else if is_token_expiring_soon(expires_at) {
        "expiring_soon"
    } else if account.status == "invalid" {
        "invalid"
    } else {
        "active"
    };

    let expires_in_seconds = calculate_expires_in_seconds(expires_at)?;

    Ok(CheckTokenStatusResponse {
        status: status.to_string(),
        expires_at: expires_at.clone(),
        expires_in_seconds,
        needs_refresh: is_token_expiring_soon(expires_at),
    })
}

/// 批量检查所有账号的 Token 状态
#[tauri::command]
pub fn check_all_tokens_status(state: State<AppState>) -> Result<TokenStatusSummary, String> {
    let store = lock_store(&state.store, "store")?;
    let accounts = &store.accounts;

    let mut summary = TokenStatusSummary {
        total: accounts.len(),
        active: 0,
        expiring_soon: 0,
        expired: 0,
        invalid: 0,
        accounts_need_refresh: Vec::new(),
    };

    for account in accounts {
        if let Some(ref expires_at) = account.expires_at {
            if is_token_expired(expires_at) {
                summary.expired += 1;
            } else if is_token_expiring_soon(expires_at) {
                summary.expiring_soon += 1;
                if let Ok(expires_in_seconds) = calculate_expires_in_seconds(expires_at) {
                    summary.accounts_need_refresh.push(AccountRefreshInfo {
                        id: account.id.clone(),
                        email: account.email.clone(),
                        provider: account.provider.clone().unwrap_or_default(),
                        expires_at: expires_at.clone(),
                        expires_in_seconds,
                    });
                }
            } else if account.status == "invalid" {
                summary.invalid += 1;
            } else {
                summary.active += 1;
            }
        } else {
            summary.invalid += 1;
        }
    }

    Ok(summary)
}

/// 批量刷新即将过期的 Token
#[tauri::command]
pub async fn refresh_all_expiring_tokens(
    state: State<'_, AppState>,
    only_expiring: Option<bool>,
    max_concurrent: Option<usize>,
) -> Result<RefreshAllResponse, String> {
    use tokio::sync::Semaphore;
    use std::sync::Arc;

    let only_expiring = only_expiring.unwrap_or(true);
    let max_concurrent = max_concurrent.unwrap_or(3);

    // 获取需要刷新的账号列表
    let accounts_to_refresh: Vec<Account> = {
        let store = lock_store(&state.store, "store")?;
        store
            .accounts
            .iter()
            .filter(|acc| {
                if only_expiring {
                    acc.expires_at
                        .as_ref()
                        .map(|exp| is_token_expiring_soon(exp))
                        .unwrap_or(false)
                } else {
                    acc.refresh_token.is_some()
                }
            })
            .cloned()
            .collect()
    };

    let total = accounts_to_refresh.len();
    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    // 使用 Semaphore 控制并发
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = Vec::new();

    for account in accounts_to_refresh {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let account_clone = account.clone();

        let task = tokio::spawn(async move {
            let result = refresh_token_by_provider(&account_clone).await;
            drop(permit);
            (account_clone, result)
        });

        tasks.push(task);
    }

    // 等待所有任务完成
    for task in tasks {
        let (account, result) = task.await.unwrap();
        let email_display = account
            .email
            .as_deref()
            .or(account.user_id.as_deref())
            .unwrap_or("Unknown")
            .to_string();

        log::debug!("Token refresh result for {}: {:?}", email_display, result);

        match result {
            Ok(refresh_result) => {
                // 更新数据库
                let mut store = lock_store(&state.store, "store")?;
                if let Some(acc) = store.accounts.iter_mut().find(|a| a.id == account.id) {
                    acc.access_token = Some(refresh_result.access_token);
                    acc.refresh_token = refresh_result.refresh_token;
                    acc.expires_at = Some(calc_expires_at(refresh_result.expires_in));

                    // IdC 账号更新额外字段
                    if let Some(id_token) = refresh_result.id_token {
                        acc.id_token = Some(id_token);
                    }
                    if let Some(sso_session_id) = refresh_result.sso_session_id {
                        acc.sso_session_id = Some(sso_session_id);
                    }
                    // Social 账号更新 profile_arn
                    if let Some(profile_arn) = refresh_result.profile_arn {
                        acc.profile_arn = Some(profile_arn);
                    }

                    save_store(&store)?;
                }

                log::info!("Batch refresh: successfully refreshed token for {}", email_display);
                successful += 1;
                results.push(RefreshResultItem {
                    id: account.id,
                    email: account.email,
                    success: true,
                    message: "Refreshed successfully".to_string(),
                });
            }
            Err(e) => {
                log::error!("Batch refresh: failed to refresh token for {}: {}", email_display, e);
                failed += 1;
                results.push(RefreshResultItem {
                    id: account.id,
                    email: account.email,
                    success: false,
                    message: e,
                });
            }
        }
    }

    Ok(RefreshAllResponse {
        total_attempted: total,
        successful,
        failed,
        skipped: 0,
        results,
    })
}

/// 设置超额开关状态
#[tauri::command]
pub async fn set_overage_status(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    use crate::clients::kiro_q_client::KiroQClient;
    use crate::core::usage::OverageCapability;

    // 1. 从 state 获取账号
    let account = find_account_by_id(&state, &id)?;

    // 资格预检：必须是 OVERAGE_CAPABLE 才能开关超额
    match OverageCapability::from_usage_data(account.usage_data.as_ref()) {
        OverageCapability::Capable => {}
        OverageCapability::Incapable => {
            return Err("此账号订阅级别不支持超额（仅 Pro / Pro+ 可开启）".to_string());
        }
        OverageCapability::Unknown => {
            return Err("无法判断账号超额资格，请先点击「检测」刷新账号信息".to_string());
        }
    }

    let access_token = account.access_token.as_ref().ok_or("No access token")?.clone();

    // 2. 检查 token 是否过期，如果过期则刷新
    let final_access_token = if let Some(expires_at) = &account.expires_at {
        if token_needs_refresh(expires_at) {
            let refresh_result = refresh_token_by_provider(&account).await?;
            // 更新 store 中的 token
            let mut store = lock_store(&state.store, "store")?;
            if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                apply_refreshed_account_tokens(a, &refresh_result);
                save_store(&store)?;
            }
            refresh_result.access_token
        } else {
            access_token
        }
    } else {
        access_token
    };

    // 3. 解析 Kiro 调用上下文
    let ctx = crate::commands::common::resolve_kiro_call_context(&account, "us-east-1");

    // 4. 调用 API（失败后刷新重试）
    let overage_status = if enabled { "ENABLED" } else { "DISABLED" };
    let client = KiroQClient::new()?;

    let result = client
        .set_user_preference(
            &final_access_token,
            &ctx.machine_id,
            &ctx.region,
            ctx.profile_arn.as_deref(),
            overage_status,
        )
        .await;

    // 如果首次调用失败且是认证错误，刷新 token 后重试
    if let Err(ref e) = result {
        if is_auth_error_message(e) {
            log::info!("[set_overage_status] Token 失效，尝试刷新后重试");
            let refresh_result = refresh_token_by_provider(&account).await?;

            // 更新 store 中的 token（在独立作用域内，确保锁在 await 之前释放）
            {
                let mut store = lock_store(&state.store, "store")?;
                if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                    apply_refreshed_account_tokens(a, &refresh_result);
                    save_store(&store)?;
                }
            } // 锁在这里释放

            // 用新 token 重试
            client
                .set_user_preference(
                    &refresh_result.access_token,
                    &ctx.machine_id,
                    &ctx.region,
                    ctx.profile_arn.as_deref(),
                    overage_status,
                )
                .await?;
        } else {
            return Err(e.clone());
        }
    } else {
        result?;
    }

    // 5. 更新本地 usage_data 中的 overageConfiguration.overageStatus
    let mut store = lock_store(&state.store, "store")?;
    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        if let Some(ref mut usage_data) = a.usage_data {
            if let Some(overage_config) = usage_data.get_mut("overageConfiguration") {
                if let Some(obj) = overage_config.as_object_mut() {
                    obj.insert(
                        "overageStatus".to_string(),
                        serde_json::Value::String(overage_status.to_string()),
                    );
                }
            } else {
                // 如果 overageConfiguration 不存在，创建它
                if let Some(obj) = usage_data.as_object_mut() {
                    obj.insert(
                        "overageConfiguration".to_string(),
                        serde_json::json!({
                            "overageStatus": overage_status
                        }),
                    );
                }
            }
        }
        save_store(&store)?;
    }

    Ok(())
}
