// Social Provider - Google/Github 登录
// 参考 kiro-batch-login/src/providers/social-provider.js

use super::{AuthProvider, AuthResult, RefreshMetadata};
use crate::auth::auth_social;
use crate::commands::machine_guid::generate_random_machine_id;
use crate::core::deep_link_handler::{register_waiter, DeepLinkCallbackWaiter};
use crate::clients::kiro_auth_client::KiroAuthServiceClient;
use async_trait::async_trait;
use serde::Deserialize;

/// Social 登录 Token 响应
#[derive(Debug, Deserialize)]
pub struct SocialTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "profileArn")]
    pub profile_arn: Option<String>,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "idToken")]
    pub id_token: Option<String>,
    #[serde(rename = "tokenType")]
    pub token_type: Option<String>,
}

/// Social 刷新 Token 响应
#[derive(Debug, Deserialize)]
struct SocialRefreshResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    #[serde(rename = "profileArn")]
    profile_arn: Option<String>,
    #[serde(rename = "expiresIn")]
    expires_in: i64,
}

pub struct SocialProvider {
    provider_id: String,
}

impl SocialProvider {
    pub fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
        }
    }
}

#[async_trait]
impl AuthProvider for SocialProvider {
    async fn login(&self) -> Result<AuthResult, String> {
        let provider = &self.provider_id;

        // Step 1: 使用 deep link 作为回调 URI
        let redirect_uri = DeepLinkCallbackWaiter::get_redirect_uri();

        // Step 2: 生成 PKCE 参数
        let state = uuid::Uuid::new_v4().to_string();
        let code_verifier = auth_social::generate_code_verifier_social();
        let code_challenge = auth_social::generate_code_challenge_social(&code_verifier);

        // Step 3: 注册回调等待器
        let waiter = register_waiter(&state);

        // Step 4: 打开浏览器登录
        let machine_id = generate_random_machine_id();
        let client = KiroAuthServiceClient::new(&machine_id)?;
        client
            .login(provider, &redirect_uri, &code_challenge, &state)
            .await?;

        // Step 5: 等待 deep link 回调
        let callback = tokio::task::spawn_blocking(move || waiter.wait_for_callback())
            .await
            .map_err(|e| format!("Failed to join callback waiter: {e}"))?
            .map_err(|e| format!("OAuth callback failed: {e}"))?;

        // Step 6: 交换 token
        let token_response: SocialTokenResponse = client
            .create_token(&callback.code, &code_verifier, &redirect_uri, None)
            .await?;

        // Step 7: 构建 AuthResult
        let expires_at =
            chrono::Local::now() + chrono::Duration::seconds(token_response.expires_in);

        Ok(AuthResult {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: expires_at.format("%Y/%m/%d %H:%M:%S").to_string(),
            provider: provider.clone(),
            auth_method: "social".to_string(),
            id_token: token_response.id_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            region: None,
            client_id: None,
            client_secret: None,
            client_id_hash: None,
            sso_session_id: None,
            start_url: None, // Social 不需要 start_url
            profile_arn: token_response.profile_arn,
        })
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
        metadata: RefreshMetadata,
    ) -> Result<AuthResult, String> {
        // 优先使用账号的 machineId，没有则生成账号独立 ID
        let machine_id = metadata.machine_id.unwrap_or_else(generate_random_machine_id);
        let client = KiroAuthServiceClient::new(&machine_id)?;
        let token_response: SocialRefreshResponse = client.refresh_token(refresh_token).await?;

        let expires_at =
            chrono::Local::now() + chrono::Duration::seconds(token_response.expires_in);

        Ok(AuthResult {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: expires_at.format("%Y/%m/%d %H:%M:%S").to_string(),
            provider: self.provider_id.clone(),
            auth_method: "social".to_string(),
            id_token: None,
            token_type: Some("Bearer".to_string()),
            expires_in: token_response.expires_in,
            region: None,
            client_id: None,
            client_secret: None,
            client_id_hash: None,
            sso_session_id: None,
            start_url: None, // Social 不需要 start_url
            profile_arn: metadata.profile_arn.or(token_response.profile_arn),
        })
    }

    fn get_provider_id(&self) -> &str {
        &self.provider_id
    }

    fn get_auth_method(&self) -> &'static str {
        "social"
    }
}
