// Base Provider - 认证提供者接口和结果结构
// 参考 kiro-batch-login/src/providers/base-provider.js

use crate::core::account::Account;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 认证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResult {
    // 通用字段
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
    pub expires_in: i64,
    pub provider: String,
    pub auth_method: String, // "social" / "IdC"
    pub token_type: Option<String>,

    // IdC (BuilderId) 专用
    pub id_token: Option<String>,
    pub region: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub client_id_hash: Option<String>,
    pub sso_session_id: Option<String>,
    pub start_url: Option<String>, // Enterprise 的 Start URL

    // Social (Google/Github) 专用
    pub profile_arn: Option<String>,
    #[serde(default)]
    pub machine_id: Option<String>,

    // external_idp（企业 SSO / Entra ID）专用：仅当登录流程经企业 IdP 完成时填充。
    // 写入 Account 后刷新链路据此 POST 微软 token_endpoint 而非 AWS OIDC。
    pub token_endpoint: Option<String>,
    pub issuer_url: Option<String>,
    pub idp_scopes: Option<String>,
}

/// 刷新 Token 所需的元数据
#[derive(Debug, Clone, Default)]
pub struct RefreshMetadata {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
    pub client_id_hash: Option<String>,
    pub profile_arn: Option<String>,
    pub machine_id: Option<String>,
    pub account: Option<Account>,
    // external_idp（微软 / Azure AD）专用
    pub token_endpoint: Option<String>,
    pub issuer_url: Option<String>,
    pub scopes: Option<String>,
}

/// 认证提供者接口
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// 执行登录认证
    async fn login(&self) -> Result<AuthResult, String>;

    /// 刷新 Token
    async fn refresh_token(
        &self,
        refresh_token: &str,
        metadata: RefreshMetadata,
    ) -> Result<AuthResult, String>;

    /// 获取 Provider ID
    fn get_provider_id(&self) -> &str;

    /// 获取认证方式
    #[must_use]
    fn get_auth_method(&self) -> &'static str;
}
