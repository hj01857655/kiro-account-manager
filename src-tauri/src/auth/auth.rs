// Auth 模块 - 当前使用的认证相关代码

use crate::clients::http_client::build_http_client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ============================================================
// User 和 AuthState
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    pub provider: String,
}

pub struct AuthState {
    pub user: Mutex<Option<User>>,
    pub access_token: Mutex<Option<String>>,
    pub refresh_token: Mutex<Option<String>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            user: Mutex::new(None),
            access_token: Mutex::new(None),
            refresh_token: Mutex::new(None),
        }
    }
}

// ============================================================
// API 常量
// ============================================================

pub const DESKTOP_AUTH_API: &str = "https://prod.us-east-1.auth.desktop.kiro.dev";

// ============================================================
// 桌面端 API 响应结构
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub profile_arn: String,
}

// ============================================================
// 桌面端 API 方法
// ============================================================

/// 使用桌面端 API 刷新 Token（只需要 `RefreshToken`）
pub async fn refresh_token_desktop(refresh_token: &str) -> Result<DesktopRefreshResponse, String> {
    let client = build_http_client().map_err(|e| format!("Failed to create client: {e}"))?;
    refresh_token_desktop_with_client(refresh_token, &client).await
}

async fn refresh_token_desktop_with_client(
    refresh_token: &str,
    client: &reqwest::Client,
) -> Result<DesktopRefreshResponse, String> {
    let body = serde_json::json!({
        "refreshToken": refresh_token
    });

    // 重试机制
    let mut last_error = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }

        match client
            .post(format!("{DESKTOP_AUTH_API}/refreshToken"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();

                // 只在调试时输出状态码，不输出敏感的 token 内容
                #[cfg(debug_assertions)]
                println!("[Desktop] RefreshToken Status: {status}");

                if !status.is_success() {
                    // 401 错误：直接返回服务器的错误信息
                    if status.as_u16() == 401 {
                        return Err(format!("AUTH_ERROR: {}", text));
                    }
                    return Err(format!("RefreshToken failed ({status}): {text}"));
                }

                return serde_json::from_str(&text).map_err(|e| format!("Parse failed: {e}"));
            }
            Err(e) => {
                last_error = format!("网络错误: {e}");
            }
        }
    }

    Err(last_error)
}

/// 使用桌面端 API 删除账号（从 AWS 服务端删除）
pub async fn delete_account_desktop(access_token: &str, machine_id: &str) -> Result<(), String> {
    let user_agent = format!("KiroIDE-0.6.18-{machine_id}");

    let client = crate::clients::http_client::build_http_client_with_user_agent(&user_agent)
        .map_err(|e| format!("Failed to create client: {e}"))?;

    // Kiro Desktop 使用 DELETE /account 端点
    let url = format!("{DESKTOP_AUTH_API}/account");

    #[cfg(debug_assertions)]
    println!("[Desktop] DeleteAccount request");

    let response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", &user_agent)
        .send()
        .await
        .map_err(|e| format!("网络错误: {e}"))?;

    let status = response.status();

    #[cfg(debug_assertions)]
    println!("[Desktop] DeleteAccount Status: {status}");

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("删除账号失败 ({status}): {text}"));
    }

    Ok(())
}
