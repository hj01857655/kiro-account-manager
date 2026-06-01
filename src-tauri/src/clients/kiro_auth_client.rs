use crate::clients::http_client::{
    build_http_client_with_user_agent, build_http_client_with_user_agent_for_account,
};
use crate::core::account::Account;
use crate::utils::browser::open_browser;
use reqwest::Client;
use serde::Deserialize;

/// Kiro Authentication Service Client
/// 负责与 <https://prod.us-east-1.auth.desktop.kiro.dev> 通信
pub struct KiroAuthServiceClient {
    endpoint: String,
    client: Client,
}

impl KiroAuthServiceClient {
    pub fn new(machine_id: &str) -> Result<Self, String> {
        let endpoint = "https://prod.us-east-1.auth.desktop.kiro.dev".to_string();
        let user_agent = format!("KiroIDE-0.6.18-{machine_id}");

        let client = build_http_client_with_user_agent(&user_agent)?;

        Ok(Self { endpoint, client })
    }

    pub fn for_account(machine_id: &str, account: &Account) -> Result<Self, String> {
        let endpoint = "https://prod.us-east-1.auth.desktop.kiro.dev".to_string();
        let user_agent = format!("KiroIDE-0.6.18-{machine_id}");

        let client = build_http_client_with_user_agent_for_account(&user_agent, account)?;

        Ok(Self { endpoint, client })
    }

    fn login_url(&self) -> String {
        let endpoint = &self.endpoint;
        format!("{endpoint}/login")
    }

    fn create_token_url(&self) -> String {
        let endpoint = &self.endpoint;
        format!("{endpoint}/oauth/token")
    }

    fn refresh_token_url(&self) -> String {
        let endpoint = &self.endpoint;
        format!("{endpoint}/refreshToken")
    }

    /// 打开浏览器到登录页面
    #[allow(clippy::unused_async)] // 使用 spawn_blocking 执行同步操作，需要 async 上下文
    pub async fn login(
        &self,
        provider: &str,
        redirect_uri: &str,
        code_challenge: &str,
        state: &str,
    ) -> Result<(), String> {
        let login_url = format!(
            "{}?idp={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
            self.login_url(),
            provider,
            urlencoding::encode(redirect_uri),
            code_challenge,
            state,
        );

        let login_url = login_url.trim().to_string();

        open_browser(&login_url)?;

        Ok(())
    }

    /// 交换授权码为访问令牌
    pub async fn create_token<T: for<'de> Deserialize<'de>>(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        invitation_code: Option<&str>,
    ) -> Result<T, String> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            code: &'a str,
            code_verifier: &'a str,
            redirect_uri: &'a str,
            invitation_code: Option<&'a str>,
        }

        let body = Body {
            code,
            code_verifier,
            redirect_uri,
            invitation_code,
        };

        let resp = self
            .client
            .post(self.create_token_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Kiro Auth Service request failed: {e}"))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Kiro Auth Service read body failed: {e}"))?;

        let body_str = String::from_utf8_lossy(&bytes);

        if !status.is_success() {
            return Err(format!(
                "Kiro Auth Service token creation failed: {status} - {body_str}"
            ));
        }

        serde_json::from_slice::<T>(&bytes)
            .map_err(|e| format!("Kiro Auth Service token creation parse failed: {e}"))
    }

    /// 刷新访问令牌
    pub async fn refresh_token<T: for<'de> Deserialize<'de>>(
        &self,
        refresh_token: &str,
    ) -> Result<T, String> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            #[serde(rename = "refreshToken")]
            refresh_token: &'a str,
        }

        let body = Body { refresh_token };

        let resp = self
            .client
            .post(self.refresh_token_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Kiro Auth Service request failed: {e}"))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Kiro Auth Service read body failed: {e}"))?;

        let body_str = String::from_utf8_lossy(&bytes);

        if !status.is_success() {
            // 401 错误：直接返回服务器的错误信息
            if status.as_u16() == 401 {
                return Err(format!("AUTH_ERROR: {}", body_str));
            }
            return Err(format!(
                "Kiro Auth Service token refresh failed: {status} - {body_str}"
            ));
        }

        serde_json::from_slice::<T>(&bytes)
            .map_err(|e| format!("Kiro Auth Service token refresh parse failed: {e}"))
    }
}
