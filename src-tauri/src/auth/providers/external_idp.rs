// External IdP Provider - 微软 Entra ID / Azure AD 登录账号的 Token 刷新
//
// external_idp 号带微软 clientId/(可选)clientSecret + 微软 tokenEndpoint/issuerUrl，
// 刷新走 OAuth2 refresh_token grant（form 表单）POST 到微软 token_endpoint，
// 而不是 AWS OIDC（oidc.*.amazonaws.com）—— 后者不认微软的 clientId，会 400 invalid_request。

use super::{AuthProvider, AuthResult, RefreshMetadata};
use crate::clients::http_client::{
    build_http_client_with_timeout, build_http_client_with_timeout_for_account,
};
use async_trait::async_trait;
use serde::Deserialize;

/// External IdP 刷新 Token 响应（微软 Entra ID / OAuth2 form flow）
#[derive(Debug, Deserialize)]
struct ExternalIdpRefreshResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

pub struct ExternalIdpProvider;

impl ExternalIdpProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExternalIdpProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// 校验 External IdP 的 token_endpoint 只能指向合法的 Microsoft 登录域。
///
/// token_endpoint/issuer_url 来自凭据（可被写凭据的来源污染），刷新时会直接向其 POST
/// （含 refresh_token/client_secret）。若不校验，可被诱导 SSRF 或把凭据发往攻击者域。
/// 强制：
/// - scheme 必须是 https；
/// - host 必须是 `login.microsoftonline.com` / `.us` / 中国域（或其子域）；
/// - 拒绝 userinfo(`@`) 混淆。
pub fn validate_microsoft_token_endpoint(endpoint: &str) -> Result<(), String> {
    // 关键(SSRF)：必须用 url::Url 解析取 host，而不是手工切分字符串。手工按 `/ ? #`
    // 切 authority 会漏掉反斜杠——https 是 WHATWG special scheme，url crate(及 reqwest
    // 底层)会把 `\` 规范化成 `/`。若手工校验：`https://evil.com\.login.microsoftonline.com`
    // 的 authority 被算成 `evil.com\.login.microsoftonline.com`（结尾匹配白名单而放行），
    // 但 reqwest 实际连 `evil.com` → 凭据(client_id/secret/refresh_token)被 POST 到攻击者
    // 域，或用 `169.254.169.254\...` 打云元数据/内网。用同一个解析器消除这种不一致。
    let parsed = url::Url::parse(endpoint)
        .map_err(|e| format!("External IdP token_endpoint 非法: {e}"))?;

    if parsed.scheme() != "https" {
        return Err("External IdP token_endpoint 必须为 https".to_string());
    }
    // 拒绝 userinfo 混淆（https://user:pass@evil.com）
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!(
            "External IdP token_endpoint 含非法 userinfo: {endpoint}"
        ));
    }
    let host = parsed
        .host_str()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if host.is_empty() {
        return Err(format!("External IdP token_endpoint 缺少主机: {endpoint}"));
    }
    const ALLOWED_SUFFIXES: &[&str] = &[
        "login.microsoftonline.com",
        "login.microsoftonline.us",
        "login.partner.microsoftonline.cn",
        "login.chinacloudapi.cn",
    ];
    let ok = ALLOWED_SUFFIXES
        .iter()
        .any(|s| host == *s || host.ends_with(&format!(".{s}")));
    if !ok {
        return Err(format!(
            "External IdP token_endpoint 主机不在 Microsoft 登录域白名单内: {host}"
        ));
    }
    Ok(())
}

/// 拼接微软 token_endpoint：优先 metadata.token_endpoint；否则从 issuer_url 派生。
/// issuer 去尾斜杠后，若以 `/v2.0` 结尾则 `{issuer}/token`，否则 `{issuer}/oauth2/v2.0/token`。
fn resolve_token_endpoint(metadata: &RefreshMetadata) -> Result<String, String> {
    if let Some(endpoint) = metadata
        .token_endpoint
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        return Ok(endpoint.to_string());
    }
    let issuer = metadata
        .issuer_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "External IdP 刷新需要 tokenEndpoint 或 issuerUrl".to_string())?
        .trim_end_matches('/');
    if issuer.ends_with("/v2.0") {
        Ok(format!("{issuer}/token"))
    } else {
        Ok(format!("{issuer}/oauth2/v2.0/token"))
    }
}

#[async_trait]
impl AuthProvider for ExternalIdpProvider {
    async fn login(&self) -> Result<AuthResult, String> {
        // external_idp 号通过导入引入（微软/Azure AD 的登录在 Kiro 外完成），
        // 应用内不发起交互式登录。
        Err("External IdP 账号不支持应用内交互式登录，请通过导入添加".to_string())
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
        metadata: RefreshMetadata,
    ) -> Result<AuthResult, String> {
        let client_id = metadata
            .client_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| "External IdP 刷新需要 clientId".to_string())?
            .to_string();

        let token_endpoint = resolve_token_endpoint(&metadata)?;

        // 安全（SSRF）：token_endpoint / issuer_url 来自凭据，刷新时会直接 POST 它。
        // 限制只能指向合法的 Microsoft 登录域，防止被诱导把 client_id/refresh_token
        // 发到攻击者服务器，或拿本机当跳板打内网。
        validate_microsoft_token_endpoint(&token_endpoint)?;

        let mut form: Vec<(&str, String)> = vec![
            ("client_id", client_id.clone()),
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.to_string()),
        ];
        if let Some(scopes) = metadata.scopes.as_ref().filter(|s| !s.trim().is_empty()) {
            form.push(("scope", scopes.to_string()));
        }
        if let Some(client_secret) = metadata
            .client_secret
            .as_ref()
            .filter(|s| !s.trim().is_empty())
        {
            form.push(("client_secret", client_secret.to_string()));
        }

        let client = if let Some(account) = metadata.account.as_ref() {
            build_http_client_with_timeout_for_account(account, 60, 10)?
        } else {
            build_http_client_with_timeout(60, 10)?
        };

        let response = client
            .post(&token_endpoint)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("External IdP Token 刷新请求失败: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            // 400 + invalid_grant → refreshToken 永久失效
            if status.as_u16() == 400 && body_text.contains("invalid_grant") {
                return Err(format!(
                    "External IdP refreshToken 已失效 (invalid_grant): {body_text}"
                ));
            }
            let error_msg = match status.as_u16() {
                401 => "External IdP 凭证已过期或无效，需要重新认证",
                403 => "External IdP 权限不足，无法刷新 Token",
                429 => "External IdP 请求过于频繁，已被限流",
                500..=599 => "External IdP 服务暂时不可用",
                _ => "External IdP Token 刷新失败",
            };
            return Err(format!("{error_msg}: {status} {body_text}"));
        }

        let data: ExternalIdpRefreshResponse = response
            .json()
            .await
            .map_err(|e| format!("解析 External IdP 刷新响应失败: {e}"))?;

        let expires_in = data.expires_in.unwrap_or(3600);
        let expires_at = chrono::Local::now() + chrono::Duration::seconds(expires_in);

        Ok(AuthResult {
            access_token: data.access_token,
            // 微软不一定回新的 refresh_token，缺省时沿用旧值（避免把已有的清空）
            refresh_token: data
                .refresh_token
                .unwrap_or_else(|| refresh_token.to_string()),
            expires_at: expires_at.format("%Y/%m/%d %H:%M:%S").to_string(),
            provider: "external_idp".to_string(),
            auth_method: "external_idp".to_string(),
            id_token: None,
            token_type: Some("Bearer".to_string()),
            expires_in,
            region: metadata.region.clone(),
            client_id: Some(client_id),
            client_secret: metadata.client_secret.clone(),
            client_id_hash: None,
            sso_session_id: None,
            start_url: None,
            // profile_arn 原样保留（external_idp 号必须透传真实 arn）
            profile_arn: metadata.profile_arn.clone(),
            machine_id: None,
            // 刷新走的元数据原样回传，保持凭据自描述
            token_endpoint: metadata.token_endpoint.clone(),
            issuer_url: metadata.issuer_url.clone(),
            idp_scopes: metadata.scopes.clone(),
        })
    }

    fn get_provider_id(&self) -> &str {
        "external_idp"
    }

    fn get_auth_method(&self) -> &'static str {
        "external_idp"
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_token_endpoint, validate_microsoft_token_endpoint};
    use crate::auth::providers::RefreshMetadata;

    #[test]
    fn validate_accepts_microsoft_login_domains() {
        assert!(validate_microsoft_token_endpoint(
            "https://login.microsoftonline.com/common/oauth2/v2.0/token"
        )
        .is_ok());
        assert!(validate_microsoft_token_endpoint(
            "https://login.microsoftonline.us/tenant/oauth2/v2.0/token"
        )
        .is_ok());
        assert!(validate_microsoft_token_endpoint(
            "https://login.partner.microsoftonline.cn/t/oauth2/v2.0/token"
        )
        .is_ok());
    }

    #[test]
    fn validate_rejects_non_https() {
        assert!(validate_microsoft_token_endpoint(
            "http://login.microsoftonline.com/common/oauth2/v2.0/token"
        )
        .is_err());
    }

    #[test]
    fn validate_rejects_non_microsoft_hosts() {
        assert!(validate_microsoft_token_endpoint("https://evil.com/token").is_err());
        // 后缀伪装：evil 域伪装成微软子串但主机不匹配
        assert!(
            validate_microsoft_token_endpoint("https://login.microsoftonline.com.evil.com/token")
                .is_err()
        );
    }

    #[test]
    fn validate_rejects_userinfo_confusion() {
        assert!(validate_microsoft_token_endpoint(
            "https://login.microsoftonline.com@evil.com/token"
        )
        .is_err());
    }

    #[test]
    fn validate_rejects_backslash_ssrf_bypass() {
        // SSRF 回归：https 下 url crate 把 `\` 规范化成 `/`，真实 host 是 evil.com。
        // 手工按 / ? # 切分会漏掉 `\` 而误放行 → 凭据外泄。必须拒绝。
        assert!(validate_microsoft_token_endpoint(
            "https://evil.com\\.login.microsoftonline.com/token"
        )
        .is_err());
        assert!(validate_microsoft_token_endpoint(
            "https://evil.com\\@login.microsoftonline.com/token"
        )
        .is_err());
        // 元数据端点伪装
        assert!(validate_microsoft_token_endpoint(
            "https://169.254.169.254\\.login.microsoftonline.com/latest/meta-data"
        )
        .is_err());
    }

    #[test]
    fn resolve_prefers_explicit_token_endpoint() {
        let metadata = RefreshMetadata {
            token_endpoint: Some(
                "https://login.microsoftonline.com/tenant/oauth2/v2.0/token".to_string(),
            ),
            issuer_url: Some("https://login.microsoftonline.com/tenant/v2.0".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_token_endpoint(&metadata).unwrap(),
            "https://login.microsoftonline.com/tenant/oauth2/v2.0/token"
        );
    }

    #[test]
    fn resolve_derives_from_v2_issuer() {
        let metadata = RefreshMetadata {
            issuer_url: Some("https://login.microsoftonline.com/tenant/v2.0/".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_token_endpoint(&metadata).unwrap(),
            "https://login.microsoftonline.com/tenant/v2.0/token"
        );
    }

    #[test]
    fn resolve_derives_from_non_v2_issuer() {
        let metadata = RefreshMetadata {
            issuer_url: Some("https://login.microsoftonline.com/tenant".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_token_endpoint(&metadata).unwrap(),
            "https://login.microsoftonline.com/tenant/oauth2/v2.0/token"
        );
    }
}
