// 公共工具函数 - 提取重复逻辑

use crate::account::Account;
use crate::providers::{AuthProvider, IdcProvider, RefreshMetadata, SocialProvider, KiroPortalClient};

/// Token 刷新结果
pub struct RefreshResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub profile_arn: Option<String>,
    pub id_token: Option<String>,
    pub sso_session_id: Option<String>,
}

/// Usage 获取结果
pub struct UsageResult {
    pub usage_data: serde_json::Value,
    pub is_banned: bool,
    pub is_auth_error: bool,
}

/// 根据 provider 刷新 token
pub async fn refresh_token_by_provider(
    account: &Account,
) -> Result<RefreshResult, String> {
    let provider = account.provider.as_deref().unwrap_or("Google");
    let refresh_token = account.refresh_token.as_ref().ok_or("No refresh token")?;

    if provider == "BuilderId" || provider == "Enterprise" {
        let metadata = RefreshMetadata {
            client_id: account.client_id.clone(),
            client_secret: account.client_secret.clone(),
            region: account.region.clone(),
            ..Default::default()
        };
        let region = metadata.region.as_deref().unwrap_or("us-east-1");
        // Enterprise 使用保存的 start_url
        let start_url = if provider == "Enterprise" {
            account.start_url.clone()
        } else {
            None
        };
        let idc_provider = IdcProvider::new(provider, region, start_url);
        let auth = idc_provider.refresh_token(refresh_token, metadata).await?;
        Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            profile_arn: None,
            id_token: auth.id_token,
            sso_session_id: auth.sso_session_id,
        })
    } else {
        let metadata = RefreshMetadata {
            profile_arn: account.profile_arn.clone(),
            machine_id: account.machine_id.clone(),
            ..Default::default()
        };
        let social_provider = SocialProvider::new(provider);
        let auth = social_provider.refresh_token(refresh_token, metadata).await?;
        Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            profile_arn: auth.profile_arn,
            id_token: None,
            sso_session_id: None,
        })
    }
}

/// 根据 provider 获取 usage 数据（统一使用 Web Portal 接口）
pub async fn get_usage_by_provider(
    provider: &str,
    access_token: &str,
) -> Result<UsageResult, String> {
    // 统一使用 KiroPortalClient 的 GetUserUsageAndLimits 接口
    // provider 即 idp: Google / Github / BuilderId
    let client = KiroPortalClient::new()?;
    let usage_call = client.get_user_usage_and_limits(access_token, provider).await;
    parse_usage_result(usage_call)
}

/// 解析 usage 结果，提取封禁状态和认证错误
fn parse_usage_result(
    result: Result<serde_json::Value, String>,
) -> Result<UsageResult, String> {
    match result {
        Ok(usage_data) => Ok(UsageResult {
            usage_data,  // 直接使用 JSON Value
            is_banned: false,
            is_auth_error: false,
        }),
        Err(e) if e.starts_with("BANNED:") => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: true,
            is_auth_error: false,
        }),
        // 401 或认证相关错误（包括 403 + token invalid）
        Err(e) if is_auth_error_message(&e) => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: false,
            is_auth_error: true,
        }),
        // 其他错误直接抛出
        Err(e) => Err(e),
    }
}

pub fn is_auth_error_message(error: &str) -> bool {
    let lower = error.to_lowercase();
    error.starts_with("AUTH_ERROR:")
        || error.contains("401")
        || error.contains("Unauthorized")
        || lower.contains("expired")
        || lower.contains("invalid")
}

/// 计算过期时间字符串
pub fn calc_expires_at(expires_in: i64) -> String {
    let expires_at = chrono::Local::now() + chrono::Duration::seconds(expires_in);
    expires_at.format("%Y/%m/%d %H:%M:%S").to_string()
}

/// 根据 `usage_result` 计算账号状态
pub fn calc_status(is_banned: bool, is_auth_error: bool) -> String {
    if is_banned {
        "banned".to_string()
    } else if is_auth_error {
        "invalid".to_string()
    } else {
        "active".to_string()
    }
}

/// 从 `usage_data` 中简单提取 `email` 和 `user_id`（不依赖结构体）
pub fn extract_user_info(usage_data: &serde_json::Value) -> (Option<String>, Option<String>) {
    let user_info = usage_data.get("userInfo");
    
    let email = user_info
        .and_then(|u| u.get("email"))
        .and_then(|e| e.as_str())
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);
    
    let user_id = user_info
        .and_then(|u| u.get("userId"))
        .and_then(|id| id.as_str())
        .map(std::string::ToString::to_string);
    
    (email, user_id)
}

/// 查找已存在的账号索引
/// 使用 `user_id` 去重（最简单直接的策略）
pub fn find_existing_account_idx(
    accounts: &[Account],
    email: Option<&String>,
    _provider: &str,
    _refresh_token: &str,
    user_id: Option<&String>,
) -> Option<usize> {
    // 只用 user_id 去重
    if let Some(uid) = user_id {
        return accounts.iter().position(|a| {
            a.user_id.as_ref() == Some(uid)
        });
    }
    
    // 如果没有 user_id，用 email 兜底
    if let Some(e) = email {
        return accounts.iter().position(|a| {
            a.email.as_ref() == Some(e)
        });
    }
    
    None
}
