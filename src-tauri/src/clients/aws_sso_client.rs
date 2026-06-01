//! AWS SSO OIDC Client
//! 实现 AWS SSO OIDC API 调用，用于 BuilderId/Enterprise 认证
//! 使用 Authorization Code Flow（跟 Kiro Desktop 一致）

use crate::clients::http_client::{build_http_client, build_http_client_with_timeout_for_account};
use crate::core::account::Account;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// 默认 scopes（跟 Kiro 一样）
pub const GRANT_SCOPES: &[&str] = &[
    "codewhisperer:completions",
    "codewhisperer:analysis",
    "codewhisperer:conversations",
    "codewhisperer:transformations",
    "codewhisperer:taskassist",
];

/// AWS SSO OIDC 客户端
pub struct AWSSSOClient {
    base_url: String,
    client: Client,
}

/// 客户端注册响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistration {
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "clientSecret")]
    pub client_secret: String,
    #[serde(rename = "clientIdIssuedAt")]
    pub client_id_issued_at: Option<i64>,
    #[serde(rename = "clientSecretExpiresAt")]
    pub client_secret_expires_at: Option<i64>,
    #[serde(rename = "authorizationEndpoint")]
    pub authorization_endpoint: Option<String>,
    #[serde(rename = "tokenEndpoint")]
    pub token_endpoint: Option<String>,
}

/// Token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "idToken")]
    pub id_token: Option<String>,
    #[serde(rename = "tokenType")]
    pub token_type: Option<String>,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "aws_sso_app_session_id")]
    pub aws_sso_app_session_id: Option<String>,
    #[serde(rename = "issuedTokenType")]
    pub issued_token_type: Option<String>,
    #[serde(rename = "originSessionId")]
    pub origin_session_id: Option<String>,
}

impl AWSSSOClient {
    pub fn new(region: &str) -> Self {
        let base_url = format!("https://oidc.{region}.amazonaws.com");
        let client = build_http_client().expect("Failed to create HTTP client");

        Self { base_url, client }
    }

    pub fn for_account(region: &str, account: &Account) -> Result<Self, String> {
        let base_url = format!("https://oidc.{region}.amazonaws.com");
        let client = build_http_client_with_timeout_for_account(account, 30, 10)?;

        Ok(Self { base_url, client })
    }

    /// 获取 authorize URL
    pub fn get_authorize_url(&self) -> String {
        format!("{}/authorize", self.base_url)
    }

    /// 注册客户端（Authorization Code Flow，跟 Kiro 一样）
    pub async fn register_client(
        &self,
        issuer_url: &str,
        redirect_uri: &str,
        has_user_provided_input: bool,
    ) -> Result<ClientRegistration, String> {
        let url = format!("{}/client/register", self.base_url);

        let scopes: Vec<String> = GRANT_SCOPES
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let body = serde_json::json!({
            "clientName": "Kiro IDE",
            "clientType": "public",
            "scopes": scopes,
            "grantTypes": ["authorization_code", "refresh_token"],
            "redirectUris": [redirect_uri],
            "issuerUrl": issuer_url
        });

        #[cfg(debug_assertions)]
        println!("[AWS SSO] Register Client (Authorization Code Flow)");

        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                // 网络错误可能是 region 不正确
                if e.to_string().contains("dns error") || e.to_string().contains("connection") {
                    format!("无法连接到 AWS SSO 服务。\n\n可能的原因：\n1. Region 选择错误（请确认您的 IAM Identity Center 所在的 Region）\n2. 网络连接问题\n3. Start URL 格式错误\n\n错误详情: {e}")
                } else {
                    format!("Client registration failed: {e}")
                }
            })?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            // 特殊处理：用户提供的 Start URL 无效（跟 Kiro IDE 一样）
            // 参考：extension.js 行 138415-138416
            if has_user_provided_input && status.as_u16() == 400 {
                // 检查错误描述中是否包含 "invalid start url provided"
                if text.to_lowercase().contains("invalid start url provided") {
                    return Err("Start URL 无效。请检查您输入的 IAM Identity Center Start URL 是否正确。\n\n示例格式：https://d-1234567890.awsapps.com/start".to_string());
                }
                // 其他 400 错误可能是 region 不匹配
                return Err(format!("注册客户端失败 (400 Bad Request)\n\n可能的原因：\n1. Region 选择错误（请确认您的 IAM Identity Center 所在的 Region）\n2. Start URL 格式错误\n\n错误详情: {text}"));
            }
            return Err(format!("Client registration failed ({status}): {text}"));
        }

        #[cfg(debug_assertions)]
        println!("[AWS SSO] Client registered successfully");

        serde_json::from_str(&text).map_err(|e| format!("Failed to parse client registration: {e}"))
    }

    /// 使用授权码交换 Token
    pub async fn create_token(
        &self,
        client_id: &str,
        client_secret: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse, String> {
        let url = format!("{}/token", self.base_url);

        let body = serde_json::json!({
            "clientId": client_id,
            "clientSecret": client_secret,
            "grantType": "authorization_code",
            "code": code,
            "codeVerifier": code_verifier,
            "redirectUri": redirect_uri
        });

        #[cfg(debug_assertions)]
        println!("[AWS SSO] Create Token with Authorization Code");

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Token creation failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Token creation failed ({status}): {text}"));
        }

        #[cfg(debug_assertions)]
        log::debug!("[AWS SSO] Token created successfully");

        serde_json::from_str(&text).map_err(|e| format!("Failed to parse token response: {e}"))
    }

    /// 刷新 Token
    pub async fn refresh_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<TokenResponse, String> {
        let url = format!("{}/token", self.base_url);

        let body = serde_json::json!({
            "clientId": client_id,
            "clientSecret": client_secret,
            "grantType": "refresh_token",
            "refreshToken": refresh_token
        });

        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                // 网络错误可能是 region 不正确
                if e.to_string().contains("dns error") || e.to_string().contains("connection") {
                    format!("无法连接到 AWS SSO 服务。\n\n可能的原因：\n1. Region 选择错误（请确认账号注册时使用的 Region）\n2. 网络连接问题\n\n错误详情: {e}")
                } else {
                    format!("Token refresh request failed: {e}")
                }
            })?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        // 只打印非 200 的响应
        if !status.is_success() {
            log::debug!("[AWS SSO] RefreshToken Status: {status}");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                log::debug!(
                    "[AWS SSO] RefreshToken Response:\n{}",
                    serde_json::to_string_pretty(&json).unwrap_or(text.clone())
                );
            } else {
                log::debug!("[AWS SSO] RefreshToken Response: {text}");
            }
        }

        if !status.is_success() {
            // 401 错误：直接返回服务器的错误信息
            if status.as_u16() == 401 {
                return Err(format!("AUTH_ERROR: {}", text));
            }
            if status.as_u16() == 400 {
                // 400 错误可能是 region 不匹配或 refresh token 无效
                if text.to_lowercase().contains("invalid")
                    && text.to_lowercase().contains("refresh")
                {
                    // invalid_grant 错误：静默返回，不显示详细错误信息
                    return Err("AUTH_ERROR: RefreshToken 已失效".to_string());
                }
                return Err(format!("Token refresh failed (400 Bad Request)\n\n可能的原因：\n1. Region 选择错误\n2. RefreshToken 无效\n\n错误详情: {text}"));
            }
            return Err(format!("Token refresh failed ({status}): {text}"));
        }

        serde_json::from_str(&text).map_err(|e| format!("Failed to parse token response: {e}"))
    }
}
