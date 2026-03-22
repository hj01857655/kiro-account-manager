// 账号相关命令 - 直接存储原始 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递 State
#![allow(clippy::too_many_lines)] // 命令文件包含多个函数

use tauri::State;
use crate::state::AppState;
use crate::account::{Account, AvailableModelsCacheEntry};
use crate::auth::{User, refresh_token_desktop};
use crate::commands::machine_guid::get_machine_id;
use crate::providers::{AuthProvider, IdcProvider, RefreshMetadata, KiroPortalClient};
use crate::commands::common::{
    get_usage_by_provider, RefreshResult, refresh_token_by_provider,
    calc_expires_at, calc_status, extract_user_info, find_existing_account_idx, is_auth_error_message
};
use crate::http_client::build_http_client_with_user_agent;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

const AVAILABLE_MODELS_CACHE_TTL_SECONDS: i64 = 30 * 60;

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
}

// ===== 辅助函数 =====

/// 从 clientSecret JWT 中提取 startUrl
fn extract_start_url_from_client_secret(client_secret: &str) -> Option<String> {
    use base64::{Engine as _, engine::general_purpose};
    
    // JWT 格式：header.payload.signature
    let parts: Vec<&str> = client_secret.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    
    // Base64 解码 payload
    let payload = parts[1];
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    let payload_str = String::from_utf8(decoded).ok()?;
    
    // 解析 JSON
    let payload_json: serde_json::Value = serde_json::from_str(&payload_str).ok()?;
    let serialized_str = payload_json.get("serialized")?.as_str()?;
    let serialized: serde_json::Value = serde_json::from_str(serialized_str).ok()?;
    
    // 提取 initiateLoginUri
    serialized.get("initiateLoginUri")?.as_str().map(|s| s.to_string())
}

/// 根据 startUrl 计算 clientIdHash（与 Kiro IDE 源码一致）
fn calculate_client_id_hash(start_url: &str) -> String {
    let input = format!(r#"{{"startUrl":"{start_url}"}}"#);
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

// ===== 数据结构 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyAccountResponse {
    #[serde(rename = "usageData")]
    pub usage_data: serde_json::Value,  // 直接返回原始数据，前端解析
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
    pub is_new: bool,  // true = 新增，false = 更新
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelTokenLimits {
    pub max_input_tokens: Option<i64>,
    pub max_output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelPromptCaching {
    pub maximum_cache_checkpoints_per_request: Option<i64>,
    pub minimum_tokens_per_cache_checkpoint: Option<i64>,
    pub supports_prompt_caching: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModel {
    pub model_id: String,
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub description: String,
    pub provider: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub context_window: Option<i64>,
    pub is_default: Option<bool>,
    pub rate_multiplier: Option<f64>,
    pub rate_unit: Option<String>,
    pub prompt_caching: Option<AvailableModelPromptCaching>,
    #[serde(default)]
    pub supported_input_types: Vec<String>,
    pub token_limits: Option<AvailableModelTokenLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAvailableModelsResponse {
    #[serde(default)]
    pub models: Vec<AvailableModel>,
    pub next_token: Option<String>,
    pub default_model: Option<AvailableModel>,
}

fn resolve_q_service_endpoint(region: Option<&str>) -> &'static str {
    if region.is_some_and(|value| value.starts_with("eu-")) {
        "https://q.eu-central-1.amazonaws.com"
    } else {
        "https://q.us-east-1.amazonaws.com"
    }
}

fn build_list_available_models_url(
    base_url: &str,
    profile_arn: Option<&str>,
    model_provider: Option<&str>,
    next_token: Option<&str>,
) -> Result<String, String> {
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|error| format!("ListAvailableModels base URL 无效: {error}"))?;
    url.set_path("ListAvailableModels");

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("origin", "AI_EDITOR");
        pairs.append_pair("maxResults", "50");
        if let Some(profile_arn) = profile_arn.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("profileArn", profile_arn);
        }
        if let Some(model_provider) = model_provider.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("modelProvider", model_provider);
        }
        if let Some(next_token) = next_token.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("nextToken", next_token);
        }
    }

    Ok(url.into())
}

fn build_kiro_models_user_agent(machine_id: &str) -> String {
    format!("KiroIDE-0.6.18-{machine_id}")
}

fn now_unix_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn is_available_models_cache_fresh(cached_at: i64, now: i64) -> bool {
    now.saturating_sub(cached_at) <= AVAILABLE_MODELS_CACHE_TTL_SECONDS
}

fn read_available_models_cache(
    account: &Account,
    model_provider: Option<&str>,
    force_refresh: bool,
) -> Option<ListAvailableModelsResponse> {
    if force_refresh {
        return None;
    }
    let cache = account.available_models_cache.as_ref()?;
    if !is_available_models_cache_fresh(cache.cached_at, now_unix_timestamp()) {
        return None;
    }
    if cache.model_provider.as_deref() != model_provider {
        return None;
    }
    serde_json::from_value(cache.response.clone()).ok()
}

fn write_available_models_cache(
    account: &mut Account,
    model_provider: Option<&str>,
    response: &ListAvailableModelsResponse,
) -> Result<(), String> {
    let response_value = serde_json::to_value(response)
        .map_err(|error| format!("序列化模型缓存失败: {error}"))?;
    account.available_models_cache = Some(AvailableModelsCacheEntry {
        response: response_value,
        cached_at: now_unix_timestamp(),
        model_provider: model_provider.map(str::to_string),
    });
    Ok(())
}

fn clear_available_models_cache(account: &mut Account) {
    account.available_models_cache = None;
}

async fn fetch_available_models_page(
    account: &Account,
    access_token: &str,
    model_provider: Option<&str>,
    next_token: Option<&str>,
) -> Result<ListAvailableModelsResponse, String> {
    let machine_id = account
        .machine_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(get_machine_id);
    let user_agent = build_kiro_models_user_agent(&machine_id);
    let base_url = resolve_q_service_endpoint(account.region.as_deref());
    let url = build_list_available_models_url(
        base_url,
        account.profile_arn.as_deref(),
        model_provider,
        next_token,
    )?;
    let client = build_http_client_with_user_agent(&user_agent)?;
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/json")
        .header("x-amz-user-agent", &user_agent)
        .header("x-amzn-codewhisperer-optout", "true")
        .send()
        .await
        .map_err(|error| format!("ListAvailableModels 请求失败: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(format!("AUTH_ERROR: ListAvailableModels failed ({status}): {body}"));
        }
        return Err(format!("ListAvailableModels failed ({status}): {body}"));
    }

    response
        .json::<ListAvailableModelsResponse>()
        .await
        .map_err(|error| format!("解析 ListAvailableModels 响应失败: {error}"))
}

fn mark_default_model(models: &mut [AvailableModel], default_model_id: Option<&str>) {
    if let Some(default_id) = default_model_id {
        for model in models {
            if model.model_id == default_id && model.is_default.is_none() {
                model.is_default = Some(true);
            }
        }
    }
}

fn ensure_default_model_present(response: &mut ListAvailableModelsResponse) {
    if let Some(default_model) = response.default_model.clone() {
        if response
            .models
            .iter()
            .all(|model| model.model_id != default_model.model_id)
        {
            response.models.insert(0, default_model);
        }
    }
}

async fn fetch_all_available_models(
    account: &Account,
    access_token: &str,
    model_provider: Option<&str>,
) -> Result<ListAvailableModelsResponse, String> {
    let mut aggregated = ListAvailableModelsResponse {
        models: Vec::new(),
        next_token: None,
        default_model: None,
    };
    let mut next_token: Option<String> = None;

    loop {
        let mut response =
            fetch_available_models_page(account, access_token, model_provider, next_token.as_deref()).await?;

        if aggregated.default_model.is_none() {
            aggregated.default_model = response.default_model.clone();
        }

        let default_model_id = aggregated
            .default_model
            .as_ref()
            .map(|model| model.model_id.as_str());
        mark_default_model(&mut response.models, default_model_id);

        if let Some(default_model) = aggregated.default_model.as_mut() {
            default_model.is_default = Some(true);
        }

        aggregated.models.extend(response.models);
        next_token = response.next_token;
        if next_token.is_none() {
            break;
        }
    }

    ensure_default_model_present(&mut aggregated);
    sort_available_models_for_display(&mut aggregated.models);
    aggregated.next_token = None;

    Ok(aggregated)
}

fn sort_available_models_for_display(models: &mut [AvailableModel]) {
    models.sort_by_key(|model| !model.is_default.unwrap_or(false));
}

fn apply_refreshed_account_tokens(
    account: &mut Account,
    refresh: &RefreshResult,
) {
    clear_available_models_cache(account);
    account.access_token = Some(refresh.access_token.clone());
    if let Some(refresh_token) = refresh.refresh_token.clone() {
        account.refresh_token = Some(refresh_token);
    }
    account.profile_arn = refresh.profile_arn.clone();
    account.id_token = refresh.id_token.clone();
    account.sso_session_id = refresh.sso_session_id.clone();
    account.expires_at = Some(calc_expires_at(refresh.expires_in));
    account.status = "active".to_string();
}

#[tauri::command]
pub fn get_accounts(state: State<AppState>) -> Vec<Account> {
    let mut store = state.store.lock().expect("Failed to acquire store lock");
    // 每次获取前重新从文件加载，确保数据最新
    store.reload();
    store.get_all()
}

#[tauri::command]
pub fn delete_account(state: State<AppState>, id: &str) -> bool {
    state.store.lock().expect("Failed to acquire store lock").delete(id)
}

#[tauri::command]
pub fn delete_accounts(state: State<AppState>, ids: Vec<String>) -> usize {
    state.store.lock().expect("Failed to acquire store lock").delete_many(&ids)
}

#[tauri::command]
pub async fn sync_account(state: State<'_, AppState>, id: String) -> Result<SyncAccountResult, String> {
    let account = {
        let store = state.store.lock().expect("Failed to acquire store lock");
        store.accounts.iter().find(|a| a.id == id).cloned()
    }.ok_or("Account not found")?;

    let provider_str = account.provider.as_deref().unwrap_or("Google");
    let access_token = account.access_token.as_ref().ok_or("No access token")?;
    
    // 先尝试用现有 token 获取配额
    let mut usage_result = get_usage_by_provider(provider_str, access_token).await;
    let mut refresh_result: Option<RefreshResult> = None;
    
    // 如果是认证错误，刷新 token 后重试
    let needs_refresh = match &usage_result {
        Ok(r) => r.is_auth_error,
        Err(_) => false,
    };
    
    if needs_refresh {
        match refresh_token_by_provider(&account).await {
            Ok(refreshed) => {
                usage_result = get_usage_by_provider(provider_str, &refreshed.access_token).await;
                refresh_result = Some(refreshed);
            }
            Err(e) => {
                if e.starts_with("BANNED:") || is_auth_error_message(&e) {
                    let mut store = state.store.lock().expect("Failed to acquire store lock");
                    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                        a.status = if e.starts_with("BANNED:") {
                            "banned".to_string()
                        } else {
                            "invalid".to_string()
                        };
                        store.save_to_file();
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

    let mut store = state.store.lock().expect("Failed to acquire store lock");
    let result = if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        // 如果刷新了 token，更新 token 相关字段
        if let Some(result) = refresh_result {
            clear_available_models_cache(a);
            // 直接移动所有权，避免 clone
            a.access_token = Some(result.access_token);
            a.refresh_token = result.refresh_token;
            a.profile_arn = result.profile_arn;
            a.id_token = result.id_token;
            a.sso_session_id = result.sso_session_id;
            a.expires_at = Some(calc_expires_at(result.expires_in));
        }
        
        // 只有成功获取配额时才更新 usage_data
        if let Some(usage_data) = usage {
            // 直接移动所有权，避免 clone
            a.usage_data = Some(usage_data.usage_data);
            a.status = calc_status(usage_data.is_banned, usage_data.is_auth_error);
            
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
        }
        
        // 克隆结果（这个必须 clone，因为要返回给前端）
        Some(a.clone())
    } else {
        None
    };
    
    // 保存文件
    store.save_to_file();
    
    match result {
        Some(account) => Ok(SyncAccountResult { account, warning }),
        None => Err("Account not found after update".to_string()),
    }
}

/// 只刷新 token，不获取 usage（启动时快速刷新用）
/// 如果 token 还有 5 分钟以上有效期，跳过刷新直接返回
#[tauri::command]
pub async fn refresh_account_token(state: State<'_, AppState>, id: String) -> Result<Account, String> {
    let account = {
        let store = state.store.lock().expect("Failed to acquire store lock");
        store.accounts.iter().find(|a| a.id == id).cloned()
    }.ok_or("Account not found")?;

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
                let mut store = state.store.lock().expect("Failed to acquire store lock");
                if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                    a.status = if e.starts_with("BANNED:") {
                        "banned".to_string()
                    } else {
                        "invalid".to_string()
                    };
                    store.save_to_file();
                }
            }
            return Err(e);
        }
    };

    let mut store = state.store.lock().expect("Failed to acquire store lock");
    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        clear_available_models_cache(a);
        // 直接移动所有权，避免 clone
        a.access_token = Some(refresh_result.access_token);
        a.refresh_token = refresh_result.refresh_token;
        a.expires_at = Some(calc_expires_at(refresh_result.expires_in));
        if matches!(a.status.as_str(), "invalid" | "失效" | "已失效" | "Token已失效") {
            a.status = "active".to_string();
        }
        let result = a.clone();
        store.save_to_file();
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
    
    // 刷新 token
    let (new_access_token, new_refresh_token) = if is_idc {
        let (cid, csec, reg) = if client_id.is_some() && client_secret.is_some() {
            (client_id, client_secret, region)
        } else {
            let store = state.store.lock().expect("Failed to acquire store lock");
            store.accounts.iter().find(|a| a.refresh_token.as_ref() == Some(&refresh_token))
                .map_or((None, None, None), |a| (a.client_id.clone(), a.client_secret.clone(), a.region.clone()))
        };
        
        let cid = cid.ok_or("IdC 账号缺少 client_id，请重新添加账号")?;
        let csec = csec.ok_or("IdC 账号缺少 client_secret，请重新添加账号")?;
        let metadata = RefreshMetadata {
            client_id: Some(cid), client_secret: Some(csec), region: reg.clone(), ..Default::default()
        };
        
        let idc_provider = IdcProvider::new(&provider, reg.as_deref().unwrap_or("us-east-1"), None);
        let auth = idc_provider.refresh_token(&refresh_token, metadata).await?;
        (auth.access_token, auth.refresh_token)
    } else {
        let auth = refresh_token_desktop(&refresh_token).await?;
        (auth.access_token, auth.refresh_token)
    };
    
    // 获取 usage_data
    let client = KiroPortalClient::new()?;
    let usage_data = client.get_user_usage_and_limits(&new_access_token, &provider).await?;
    
    // 更新数据库
    {
        let mut store = state.store.lock().expect("Failed to acquire store lock");
        if let Some(account) = store.accounts.iter_mut().find(|a| a.refresh_token.as_ref() == Some(&refresh_token)) {
            // 直接移动所有权，避免 clone
            account.access_token = Some(new_access_token.clone());  // ✅ 这里必须 clone，因为后面还要用
            account.refresh_token = Some(new_refresh_token.clone());  // ✅ 这里必须 clone，因为后面还要用
            store.save_to_file();
        }
    }
    
    Ok(VerifyAccountResponse {
        usage_data,  // 直接返回，前端解析
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
    let idp = provider.as_deref().unwrap_or("Google").to_string();  // ✅ 避免不必要的 clone
    
    // 先尝试用传入的 access_token 获取配额
    let (final_access_token, final_refresh_token, final_profile_arn, usage_result) = if let Some(at) = access_token {
        match get_usage_by_provider(&idp, &at).await {
            Ok(result) if result.is_auth_error => {
                // 401 了，刷新 token
                let refresh_result = refresh_token_desktop(&refresh_token).await?;
                let new_usage = get_usage_by_provider(&idp, &refresh_result.access_token).await?;
                (refresh_result.access_token, refresh_result.refresh_token, refresh_result.profile_arn, new_usage)
            }
            Ok(result) => {
                // access_token 有效，但没有 profile_arn，需要刷新一次获取
                let refresh_result = refresh_token_desktop(&refresh_token).await?;
                (at, refresh_token.clone(), refresh_result.profile_arn, result)
            }
            Err(e) => return Err(e),
        }
    } else {
        // 没有 access_token，直接刷新
        let refresh_result = refresh_token_desktop(&refresh_token).await?;
        let usage_result = get_usage_by_provider(&idp, &refresh_result.access_token).await?;
        (refresh_result.access_token, refresh_result.refresh_token, refresh_result.profile_arn, usage_result)
    };
    
    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }
    
    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    
    // 获取不到邮箱直接报错
    let final_email = new_email.ok_or("获取邮箱失败，请检查账号状态")?;
    
    // 根据邮箱推断最终 provider
    let idp = provider.unwrap_or_else(|| {
        if final_email.contains("gmail") { "Google".to_string() } 
        else if final_email.contains("github") { "Github".to_string() } 
        else { "Google".to_string() }
    });
    
    let mut store = state.store.lock().expect("Failed to acquire store lock");
    let existing_idx = find_existing_account_idx(&store.accounts, Some(&final_email), &idp, &final_refresh_token, user_id.as_ref());
    
    let is_new = existing_idx.is_none();
    
    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        // 直接移动所有权，避免 clone
        existing.access_token = Some(final_access_token.clone());  // ✅ 后面还要用，必须 clone
        existing.refresh_token = Some(final_refresh_token.clone());  // ✅ 后面还要用，必须 clone
        existing.profile_arn = Some(final_profile_arn.clone());  // ✅ 保存 profile_arn
        existing.user_id = user_id;
        existing.usage_data = Some(usage_result.usage_data);
        existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()  // ✅ 必须 clone，因为要返回给前端
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {idp} 账号"));
        account.access_token = Some(final_access_token.clone());  // ✅ 后面还要用，必须 clone
        account.refresh_token = Some(final_refresh_token.clone());  // ✅ 后面还要用，必须 clone
        account.profile_arn = Some(final_profile_arn.clone());  // ✅ 保存 profile_arn
        account.provider = Some(idp.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
        // 使用传入的 machine_id，没有则自动生成
        account.machine_id = machine_id.or_else(|| Some(uuid::Uuid::new_v4().to_string().to_lowercase()));  // ✅ 避免 clone
        store.accounts.insert(0, account.clone());
        account
    };
    
    store.save_to_file();
    drop(store);
    
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: account.email.clone(),  // ✅ 必须 clone，因为 account 被移动了
        name: account.email.as_ref().and_then(|e| e.split('@').next()).unwrap_or("User").to_string(),
        avatar: None,
        provider: idp,
    };
    *state.auth.user.lock().expect("Failed to acquire auth user lock") = Some(user);
    *state.auth.access_token.lock().expect("Failed to acquire auth access_token lock") = Some(final_access_token);
    
    Ok(AddAccountResult { account, is_new })
}

#[tauri::command]
pub fn import_accounts(state: State<AppState>, json: &str) -> Result<usize, String> {
    state.store.lock().expect("Failed to acquire store lock").import_from_json(json)
}

#[tauri::command]
pub fn export_accounts(state: State<AppState>, ids: Option<Vec<String>>) -> String {
    let store = state.store.lock().expect("Failed to acquire store lock");
    
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
            let selected: Vec<Account> = store.accounts.iter()
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
pub async fn add_local_kiro_account(state: State<'_, AppState>) -> Result<AddAccountResult, String> {
    use crate::kiro::{get_kiro_local_token, get_client_registration};
    
    let local_token = get_kiro_local_token().await
        .ok_or("未找到本地 Kiro 账号，请先在 Kiro IDE 中登录")?;
    
    let refresh_token = local_token.refresh_token
        .ok_or("本地账号缺少 refresh_token")?;
    
    let auth_method = local_token.auth_method.as_deref().unwrap_or("social");
    let provider = local_token.provider.clone().unwrap_or_else(|| "Google".to_string());
    
    // 根据 auth_method 调用对应的添加函数
    if auth_method == "IdC" {
        let hash = local_token.client_id_hash.clone()
            .ok_or("IdC 账号缺少 clientIdHash")?;
        let region = local_token.region.clone().unwrap_or_else(|| "us-east-1".to_string());
        
        let client_reg = get_client_registration(&hash).await
            .ok_or(format!("未找到客户端注册信息: {hash}.json"))?;
        
        // 统一调用 add_account_by_idc（展开参数）
        add_account_by_idc(
            state,
            Some(provider),  // provider: BuilderId 或 Enterprise
            refresh_token,   // refresh_token
            client_reg.client_id,  // client_id
            client_reg.client_secret,  // client_secret
            Some(region),    // region
            None,            // machine_id: 本地导入不指定，自动生成
            local_token.access_token.clone(),  // access_token
            None,            // password: 本地导入无密码
            None,            // start_url: 本地导入无 start_url
            Some(hash),      // client_id_hash: 直接使用 Kiro IDE 提供的
        ).await
    } else {
        add_account_by_social(
            state,
            refresh_token,
            Some(provider),
            None, // 本地导入不指定 machine_id，自动生成
            local_token.access_token.clone(), // 传入 access_token
        ).await
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
    ).await
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
    
    // 从 clientSecret JWT 中提取 startUrl（如果未提供）
    let start_url = if params.start_url.is_some() {
        params.start_url.clone()
    } else if is_enterprise {
        extract_start_url_from_client_secret(&params.client_secret)
    } else {
        None
    };
    
    // BuilderId 和 Enterprise 都使用默认 region（如果未提供）
    let region = params.region.unwrap_or_else(|| "us-east-1".to_string());
    
    // 先尝试用传入的 access_token 获取配额
    let (final_access_token, final_refresh_token, usage_result, expires_at, id_token, sso_session_id) =
        if let Some(at) = params.access_token {
            match get_usage_by_provider(&params.provider_id, &at).await {
                Ok(result) if result.is_auth_error => {
                    // 401 了，刷新 token
                    let metadata = RefreshMetadata {
                        client_id: Some(params.client_id.clone()),
                        client_secret: Some(params.client_secret.clone()),
                        region: Some(region.clone()),
                        ..Default::default()
                    };
                    let idc_provider = IdcProvider::new(&params.provider_id, &region, start_url.clone());
                    let auth_result = idc_provider.refresh_token(&params.refresh_token, metadata).await?;
                    let new_usage = get_usage_by_provider(&params.provider_id, &auth_result.access_token).await?;
                    let expires_at = calc_expires_at(auth_result.expires_in);
                    (auth_result.access_token, auth_result.refresh_token, new_usage, expires_at, auth_result.id_token, auth_result.sso_session_id)
                }
                Ok(result) => {
                    // access_token 有效，不需要刷新
                    (at, params.refresh_token.clone(), result, String::new(), None, None)
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
            let auth_result = idc_provider.refresh_token(&params.refresh_token, metadata).await?;
            let usage_result = get_usage_by_provider(&params.provider_id, &auth_result.access_token).await?;
            let expires_at = calc_expires_at(auth_result.expires_in);
            (auth_result.access_token, auth_result.refresh_token, usage_result, expires_at, auth_result.id_token, auth_result.sso_session_id)
        };
    
    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }
    
    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);
    
    // ========== Enterprise 和 BuilderId 分开处理 ==========
    
    if is_enterprise {
        // Enterprise 账号：必须有 user_id，email 可选
        let user_id = user_id.ok_or("Enterprise 账号缺少 userId")?;
        
        // 计算 client_id_hash（可选）
        let client_id_hash = if let Some(hash) = params.client_id_hash.clone() {
            Some(hash)  // 如果提供了 clientIdHash，直接使用
        } else {
            start_url
                .as_ref()
                .map(|url| calculate_client_id_hash(url))  // 如果提取到了 startUrl，计算
        };
        
        let mut store = state.store.lock().expect("Failed to acquire store lock");
        let existing_idx = find_existing_account_idx(&store.accounts, new_email.as_ref(), &params.provider_id, &final_refresh_token, Some(&user_id));
        
        let is_new = existing_idx.is_none();
        
        let account = if let Some(idx) = existing_idx {
            // 更新已存在的账号
            let existing = &mut store.accounts[idx];
            existing.access_token = Some(final_access_token);
            existing.refresh_token = Some(final_refresh_token);
            existing.email = new_email;  // 更新 email（可能是 None）
            existing.user_id = Some(user_id);
            existing.provider = Some(params.provider_id.clone());
            existing.auth_method = Some("IdC".to_string());  // 确保 authMethod 正确
            if !expires_at.is_empty() {
                existing.expires_at = Some(expires_at);
            }
            existing.client_id = Some(params.client_id.clone());
            existing.client_secret = Some(params.client_secret.clone());
            existing.region = Some(region.clone());
            existing.client_id_hash = client_id_hash.clone();  // 可能是 None
            existing.start_url = start_url.clone();
            if id_token.is_some() {
                existing.id_token = id_token;
            }
            if sso_session_id.is_some() {
                existing.sso_session_id = sso_session_id;
            }
            existing.usage_data = Some(usage_result.usage_data);
            existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
            existing.clone()
        } else {
            // 创建新的 Enterprise 账号
            let mut account = Account::new_enterprise(user_id.clone(), "Kiro Enterprise 账号".to_string());
            account.access_token = Some(final_access_token);
            account.refresh_token = Some(final_refresh_token);
            account.email = new_email;  // 可能是 None
            if !expires_at.is_empty() {
                account.expires_at = Some(expires_at);
            }
            account.client_id = Some(params.client_id.clone());
            account.client_secret = Some(params.client_secret.clone());
            account.region = Some(region.clone());
            account.client_id_hash = client_id_hash;  // 可能是 None
            account.start_url = start_url.clone();
            account.id_token = id_token;
            account.sso_session_id = sso_session_id;
            account.usage_data = Some(usage_result.usage_data);
            account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
            account.machine_id = Some(params.machine_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string().to_lowercase()));
            account.password.clone_from(&params.password);
            store.accounts.insert(0, account.clone());
            account
        };
        
        store.save_to_file();
        Ok(AddAccountResult { account, is_new })
        
    } else {
        // BuilderId 账号：必须有 email
        let email = new_email.ok_or("BuilderId 账号缺少 email")?;
        
        // 计算 client_id_hash（可选）
        let client_id_hash = if let Some(hash) = params.client_id_hash {
            Some(hash)  // 如果提供了 clientIdHash，直接使用
        } else if params.start_url.is_some() {
            Some(calculate_client_id_hash(params.start_url.as_ref().unwrap()))  // 如果提供了 startUrl，计算
        } else {
            Some(calculate_client_id_hash("https://view.awsapps.com/start"))  // BuilderId 使用默认 URL
        };
        
        let mut store = state.store.lock().expect("Failed to acquire store lock");
        let existing_idx = find_existing_account_idx(&store.accounts, Some(&email), &params.provider_id, &final_refresh_token, user_id.as_ref());
        
        let is_new = existing_idx.is_none();
        
        let account = if let Some(idx) = existing_idx {
            // 更新已存在的账号
            let existing = &mut store.accounts[idx];
            existing.access_token = Some(final_access_token);
            existing.refresh_token = Some(final_refresh_token);
            existing.provider = Some(params.provider_id.clone());
            existing.auth_method = Some("IdC".to_string());  // 确保 authMethod 正确
            existing.user_id = user_id;
            if !expires_at.is_empty() {
                existing.expires_at = Some(expires_at);
            }
            existing.client_id = Some(params.client_id.clone());
            existing.client_secret = Some(params.client_secret.clone());
            existing.region = Some(region.clone());
            existing.client_id_hash = client_id_hash.clone();  // 可能是 None
            if id_token.is_some() {
                existing.id_token = id_token;
            }
            if sso_session_id.is_some() {
                existing.sso_session_id = sso_session_id;
            }
            existing.usage_data = Some(usage_result.usage_data);
            existing.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
            existing.clone()
        } else {
            // 创建新的 BuilderId 账号
            let mut account = Account::new(email, "Kiro BuilderId 账号".to_string());
            account.access_token = Some(final_access_token);
            account.refresh_token = Some(final_refresh_token);
            account.provider = Some(params.provider_id.clone());
            account.auth_method = Some("IdC".to_string());
            account.user_id = user_id;
            if !expires_at.is_empty() {
                account.expires_at = Some(expires_at);
            }
            account.client_id = Some(params.client_id.clone());
            account.client_secret = Some(params.client_secret.clone());
            account.region = Some(region.clone());
            account.client_id_hash = client_id_hash;  // 可能是 None
            account.id_token = id_token;
            account.sso_session_id = sso_session_id;
            account.usage_data = Some(usage_result.usage_data);
            account.status = calc_status(usage_result.is_banned, usage_result.is_auth_error);
            account.machine_id = Some(params.machine_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string().to_lowercase()));
            account.password = params.password;
            store.accounts.insert(0, account.clone());
            account
        };
        
        store.save_to_file();
        Ok(AddAccountResult { account, is_new })
    }
}

/// 更新账号信息（支持修改 label、token、SSO Client ID/Secret、machineId）
#[tauri::command]
pub fn update_account(
    state: State<AppState>,
    params: UpdateAccountParams,
) -> Result<Account, String> {
    let mut store = state.store.lock().expect("Failed to acquire store lock");
    
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
        let result = store.accounts[idx].clone();
        store.save_to_file();
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
    let account = {
        let store = state.store.lock().expect("Failed to acquire store lock");
        store.accounts.iter().find(|a| a.id == id).cloned()
    }.ok_or("账号不存在")?;
    
    // 检查 provider
    let provider = account.provider.as_deref().unwrap_or("Google");
    if provider == "Enterprise" {
        return Err("Enterprise 账号不支持远程删除".to_string());
    }
    if provider == "BuilderId" {
        return Err("BuilderId 账号不支持远程删除".to_string());
    }
    
    let access_token = account.access_token.as_ref()
        .ok_or("账号缺少 access_token，请先刷新")?;
    
    // Google/Github 账号使用 Desktop API
    let machine_id = get_machine_id();
    delete_account_desktop(access_token, &machine_id).await?;
    
    // 如果需要同时删除本地记录
    if delete_local {
        let mut store = state.store.lock().expect("Failed to acquire store lock");
        store.delete(&id);
    }
    
    Ok(format!("账号 {} 已从服务端删除", account.get_display_id()))
}


// ============================================================
// 筛选查询命令
// ============================================================

/// 获取可用账号列表（用于自动换号）
#[tauri::command]
pub fn get_available_accounts(state: State<AppState>) -> Vec<Account> {
    let store = state.store.lock().expect("Failed to acquire store lock");
    store.get_available_accounts().into_iter().cloned().collect()
}

/// 按分组筛选账号
#[tauri::command]
pub fn get_accounts_by_group(state: State<AppState>, group_id: String) -> Vec<Account> {
    let store = state.store.lock().expect("Failed to acquire store lock");
    store.get_accounts_by_group(&group_id).into_iter().cloned().collect()
}

/// 按标签筛选账号
#[tauri::command]
pub fn get_accounts_by_tag(state: State<AppState>, tag_id: String) -> Vec<Account> {
    let store = state.store.lock().expect("Failed to acquire store lock");
    store.get_accounts_by_tag(&tag_id).into_iter().cloned().collect()
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
    let client = KiroPortalClient::new()?;
    client.get_user_usage_and_limits(&access_token, provider_str).await
}

#[tauri::command]
pub async fn list_available_models(
    state: State<'_, AppState>,
    id: String,
    model_provider: Option<String>,
    force_refresh: Option<bool>,
) -> Result<ListAvailableModelsResponse, String> {
    let mut account = {
        let store = state.store.lock().expect("Failed to acquire store lock");
        store.accounts.iter().find(|item| item.id == id).cloned()
    }
    .ok_or("账号不存在")?;

    if let Some(cached_response) =
        read_available_models_cache(&account, model_provider.as_deref(), force_refresh.unwrap_or(false))
    {
        return Ok(cached_response);
    }

    let initial_access_token = account
        .access_token
        .clone()
        .ok_or("账号缺少 access_token，请先刷新 Token")?;

    match fetch_all_available_models(&account, &initial_access_token, model_provider.as_deref()).await {
        Ok(response) => {
            let mut store = state.store.lock().expect("Failed to acquire store lock");
            if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                write_available_models_cache(stored_account, model_provider.as_deref(), &response)?;
                store.save_to_file();
            }
            Ok(response)
        }
        Err(error) if is_auth_error_message(&error) => {
            let refresh = refresh_token_by_provider(&account).await?;
            apply_refreshed_account_tokens(&mut account, &refresh);

            {
                let mut store = state.store.lock().expect("Failed to acquire store lock");
                let stored_account = store
                    .accounts
                    .iter_mut()
                    .find(|item| item.id == id)
                    .ok_or("账号不存在")?;
                apply_refreshed_account_tokens(stored_account, &refresh);
                store.save_to_file();
            }
            let response =
                fetch_all_available_models(&account, &refresh.access_token, model_provider.as_deref()).await?;
            {
                let mut store = state.store.lock().expect("Failed to acquire store lock");
                if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                    write_available_models_cache(stored_account, model_provider.as_deref(), &response)?;
                    store.save_to_file();
                }
            }
            Ok(response)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_list_available_models_url, clear_available_models_cache, ensure_default_model_present,
        is_available_models_cache_fresh, mark_default_model, read_available_models_cache,
        resolve_q_service_endpoint, sort_available_models_for_display, write_available_models_cache,
        AvailableModel, ListAvailableModelsResponse, AVAILABLE_MODELS_CACHE_TTL_SECONDS,
    };
    use crate::account::Account;

    #[test]
    fn resolve_q_service_endpoint_matches_upstream_region_rule() {
        assert_eq!(
            resolve_q_service_endpoint(Some("eu-west-1")),
            "https://q.eu-central-1.amazonaws.com"
        );
        assert_eq!(
            resolve_q_service_endpoint(Some("us-east-1")),
            "https://q.us-east-1.amazonaws.com"
        );
        assert_eq!(
            resolve_q_service_endpoint(None),
            "https://q.us-east-1.amazonaws.com"
        );
    }

    #[test]
    fn build_list_available_models_url_keeps_expected_query_shape() {
        let url = build_list_available_models_url(
            "https://q.us-east-1.amazonaws.com",
            Some("arn:aws:codewhisperer:::profile/test"),
            Some("anthropic"),
            Some("next-token"),
        )
        .expect("url should build");
        let parsed = reqwest::Url::parse(&url).expect("url should parse");
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(parsed.path(), "/ListAvailableModels");
        assert_eq!(params.get("origin").map(String::as_str), Some("AI_EDITOR"));
        assert_eq!(params.get("maxResults").map(String::as_str), Some("50"));
        assert_eq!(
            params.get("profileArn").map(String::as_str),
            Some("arn:aws:codewhisperer:::profile/test")
        );
        assert_eq!(
            params.get("modelProvider").map(String::as_str),
            Some("anthropic")
        );
        assert_eq!(params.get("nextToken").map(String::as_str), Some("next-token"));
    }

    #[test]
    fn deserialize_list_available_models_response_supports_known_fields() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5",
                    "description": "The Claude Sonnet 4.5 model",
                    "rateMultiplier": 1.3,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 200000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "nextToken": "page-2"
        }))
        .expect("response should deserialize");

        assert_eq!(response.models.len(), 1);
        assert_eq!(
            response.models[0].model_id,
            "claude-sonnet-4.5"
        );
        assert_eq!(response.models[0].model_name, "Claude Sonnet 4.5");
        assert_eq!(
            response.models[0].supported_input_types,
            vec!["TEXT".to_string(), "IMAGE".to_string()]
        );
        assert_eq!(
            response.models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_input_tokens),
            Some(200000)
        );
        assert_eq!(
            response.models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
        assert_eq!(response.next_token.as_deref(), Some("page-2"));
    }

    #[test]
    fn deserialize_list_available_models_response_supports_full_default_model_shape() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4",
                    "modelName": "Claude Sonnet 4",
                    "description": "Hybrid reasoning and coding for regular use",
                    "isDefault": true,
                    "promptCaching": {
                        "maximumCacheCheckpointsPerRequest": 4,
                        "minimumTokensPerCacheCheckpoint": 1024,
                        "supportsPromptCaching": true
                    },
                    "rateMultiplier": 1.3,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 200000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "defaultModel": {
                "modelId": "claude-sonnet-4",
                "modelName": "Claude Sonnet 4",
                "description": "Hybrid reasoning and coding for regular use",
                "promptCaching": {
                    "maximumCacheCheckpointsPerRequest": 4,
                    "minimumTokensPerCacheCheckpoint": 1024,
                    "supportsPromptCaching": true
                },
                "rateMultiplier": 1.3,
                "rateUnit": "Credit",
                "supportedInputTypes": ["TEXT", "IMAGE"],
                "tokenLimits": {
                    "maxInputTokens": 200000,
                    "maxOutputTokens": 64000
                }
            }
        }))
        .expect("full response should deserialize");

        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].model_id, "claude-sonnet-4");
        assert_eq!(response.models[0].model_name, "Claude Sonnet 4");
        assert_eq!(
            response.models[0].description,
            "Hybrid reasoning and coding for regular use"
        );
        assert_eq!(response.models[0].is_default, Some(true));
        assert_eq!(
            response.models[0]
                .prompt_caching
                .as_ref()
                .and_then(|value| value.supports_prompt_caching),
            Some(true)
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .map(|model| model.model_id.as_str()),
            Some("claude-sonnet-4")
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.prompt_caching.as_ref())
                .and_then(|value| value.minimum_tokens_per_cache_checkpoint),
            Some(1024)
        );
    }

    #[test]
    fn deserialize_list_available_models_response_supports_live_default_model_shape() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "description": "Models chosen by task for optimal usage and consistent quality",
                "modelId": "auto",
                "modelName": "Auto",
                "promptCaching": {
                    "maximumCacheCheckpointsPerRequest": 4,
                    "minimumTokensPerCacheCheckpoint": 1024,
                    "supportsPromptCaching": true
                },
                "rateMultiplier": 1.0,
                "rateUnit": "Credit",
                "supportedInputTypes": ["TEXT", "IMAGE"],
                "tokenLimits": {
                    "maxInputTokens": 200000,
                    "maxOutputTokens": 64000
                }
            },
            "models": [
                {
                    "description": "Models chosen by task for optimal usage and consistent quality",
                    "modelId": "auto",
                    "modelName": "Auto"
                }
            ],
            "nextToken": null
        }))
        .expect("live response shape should deserialize");

        let default_model = response
            .default_model
            .as_ref()
            .expect("default model should exist");
        assert_eq!(default_model.model_id, "auto");
        assert_eq!(default_model.model_name, "Auto");
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.supports_prompt_caching),
            Some(true)
        );
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.maximum_cache_checkpoints_per_request),
            Some(4)
        );
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.minimum_tokens_per_cache_checkpoint),
            Some(1024)
        );
        assert_eq!(
            default_model
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
    }

    #[test]
    fn sort_available_models_for_display_prioritizes_default_models() {
        let mut models: Vec<AvailableModel> = serde_json::from_value(serde_json::json!([
            {
                "modelId": "claude-sonnet-4.5",
                "modelName": "Claude Sonnet 4.5"
            },
            {
                "modelId": "auto",
                "modelName": "Auto",
                "isDefault": true
            },
            {
                "modelId": "claude-sonnet-4",
                "modelName": "Claude Sonnet 4"
            }
        ]))
        .expect("models should deserialize");

        sort_available_models_for_display(&mut models);

        let ordered_ids: Vec<_> = models.iter().map(|model| model.model_id.as_str()).collect();
        assert_eq!(ordered_ids, vec!["auto", "claude-sonnet-4.5", "claude-sonnet-4"]);
    }

    #[test]
    fn mark_default_model_sets_matching_entry() {
        let mut models: Vec<AvailableModel> = serde_json::from_value(serde_json::json!([
            { "modelId": "claude-sonnet-4.5", "modelName": "Claude Sonnet 4.5" },
            { "modelId": "auto", "modelName": "Auto" }
        ]))
        .expect("models should deserialize");

        mark_default_model(&mut models, Some("auto"));

        assert_eq!(models[0].is_default, None);
        assert_eq!(models[1].is_default, Some(true));
    }

    #[test]
    fn ensure_default_model_present_inserts_only_once() {
        let mut response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize");

        ensure_default_model_present(&mut response);
        ensure_default_model_present(&mut response);

        let auto_count = response
            .models
            .iter()
            .filter(|model| model.model_id == "auto")
            .count();
        assert_eq!(auto_count, 1);
        assert_eq!(
            response.models.first().map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn available_models_cache_round_trips_response() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [
                {
                    "modelId": "auto",
                    "modelName": "Auto"
                },
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, Some("anthropic"), &response)
            .expect("cache write should succeed");
        let cached = read_available_models_cache(&account, Some("anthropic"), false)
            .expect("cache should be readable");

        assert_eq!(cached.models.len(), 2);
        assert_eq!(
            cached.default_model.as_ref().map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn available_models_cache_expires_after_ttl() {
        assert!(is_available_models_cache_fresh(
            100,
            100 + AVAILABLE_MODELS_CACHE_TTL_SECONDS
        ));
        assert!(!is_available_models_cache_fresh(
            100,
            101 + AVAILABLE_MODELS_CACHE_TTL_SECONDS
        ));
    }

    #[test]
    fn clear_available_models_cache_removes_cached_response() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, None, &response)
            .expect("cache write should succeed");
        clear_available_models_cache(&mut account);

        assert!(read_available_models_cache(&account, None, false).is_none());
    }

    #[test]
    fn available_models_cache_misses_when_model_provider_differs() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, Some("anthropic"), &response)
            .expect("cache write should succeed");

        assert!(read_available_models_cache(&account, Some("openai"), false).is_none());
        assert!(read_available_models_cache(&account, None, false).is_none());
        assert!(read_available_models_cache(&account, Some("anthropic"), false).is_some());
    }

    #[test]
    fn available_models_cache_skips_when_force_refresh_enabled() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, None, &response)
            .expect("cache write should succeed");

        assert!(read_available_models_cache(&account, None, true).is_none());
    }
}
