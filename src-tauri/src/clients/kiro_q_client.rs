// AWS Q Service 客户端 - 统一的 REST API 接口
// 支持 getUsageLimits、ListAvailableModels、MCP、setUserPreference

use crate::clients::http_client::{
    build_http_client, build_http_client_with_timeout_for_account, build_kiro_custom_user_agent,
    build_q_service_url, get_usage_probe_regions,
};
use crate::core::account::Account;
use reqwest::RequestBuilder;
use uuid::Uuid;

pub struct KiroQClient {
    client: reqwest::Client,
}

/// 给 RequestBuilder 加上 Kiro Q API 通用 headers
///
/// 包含 Authorization、UA、AWS SDK 中间件需要的请求 ID/重试头。
/// 配合 `with_kiro_q_accept` / `with_kiro_q_content_type` 使用。
fn with_kiro_q_headers(req: RequestBuilder, access_token: &str, machine_id: &str) -> RequestBuilder {
    let user_agent = build_kiro_custom_user_agent(machine_id);
    let invocation_id = Uuid::new_v4().to_string();
    req.header("Authorization", format!("Bearer {access_token}"))
        .header("user-agent", user_agent.clone())
        .header("x-amz-user-agent", user_agent)
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", "attempt=1; max=1")
}

/// 把非 2xx 响应统一映射成项目约定的错误字符串
///
/// 错误前缀含义：
/// - `AUTH_ERROR: ...`  调用方应触发 token 刷新或登录态失效流程
/// - `BANNED: ...`      账号被封禁/临时停用，不应再请求
/// - 其他保留原始 HTTP 状态码 + body 摘要
async fn classify_kiro_q_error(api: &str, resp: reqwest::Response) -> String {
    let status_code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();

    if status_code == 401 {
        return format!("AUTH_ERROR: {api} 401: {body}");
    }

    if status_code == 403 {
        let body_lower = body.to_lowercase();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
            let reason = parsed.get("reason").and_then(|r| r.as_str()).unwrap_or("");
            if reason == "TemporarilySuspended" {
                let message = parsed
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("账号已被封禁");
                return format!("BANNED: {message}");
            }
        }
        // 检查消息中是否包含 "suspended" 关键词
        if body_lower.contains("suspended") {
            return format!("BANNED: {body}");
        }
        return format!("AUTH_ERROR: {api} 403: {body}");
    }

    if status_code == 423 {
        return "BANNED: Account suspended".to_string();
    }

    format!("{api} failed - HTTP {status_code}: {body}")
}

impl KiroQClient {
    pub fn new() -> Result<Self, String> {
        let client = build_http_client()?;
        Ok(Self { client })
    }

    pub fn for_account(account: &Account) -> Result<Self, String> {
        let client = build_http_client_with_timeout_for_account(account, 30, 10)?;
        Ok(Self { client })
    }

    pub fn from_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// 统一的 getUsageLimits 接口（支持所有账号类型）
    pub async fn get_usage_limits(
        &self,
        access_token: &str,
        machine_id: &str,
        region: &str,
        profile_arn: Option<&str>,
        _auth_method: Option<&str>,
        _provider: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        // 构建 URL，参数顺序：isEmailRequired → origin → profileArn → resourceType
        let url = if let Some(arn) = profile_arn.filter(|s| !s.trim().is_empty()) {
            format!(
                "{}/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&profileArn={}&resourceType=AGENTIC_REQUEST",
                build_q_service_url(region),
                urlencoding::encode(arn)
            )
        } else {
            format!(
                "{}/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&resourceType=AGENTIC_REQUEST",
                build_q_service_url(region)
            )
        };

        let request = with_kiro_q_headers(self.client.get(&url), access_token, machine_id)
            .header("accept", "application/json");

        let response = request
            .send()
            .await
            .map_err(|e| format!("getUsageLimits 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_q_error("getUsageLimits", response).await);
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))
    }

    /// 多区域探测获取企业账号的 usage 数据
    pub async fn get_usage_limits_with_region_probe(
        &self,
        access_token: &str,
        machine_id: &str,
    ) -> Result<(serde_json::Value, String), String> {
        let regions = get_usage_probe_regions();

        for region in regions {
            match self
                .get_usage_limits(access_token, machine_id, region, None, None, None)
                .await
            {
                Ok(data) => return Ok((data, region.to_string())),
                Err(e) if e.starts_with("AUTH_ERROR") && e.contains("403") => continue,
                Err(e) => return Err(e),
            }
        }

        Err("Failed to find account in any region (all returned 403)".to_string())
    }

    /// ListAvailableModels 接口
    #[allow(dead_code)]
    pub async fn list_available_models(
        &self,
        access_token: &str,
        machine_id: &str,
        region: &str,
        profile_arn: Option<&str>,
        model_provider: Option<&str>,
        next_token: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut url = format!(
            "{}/ListAvailableModels?origin=AI_EDITOR&maxResults=50",
            build_q_service_url(region)
        );

        if let Some(arn) = profile_arn.filter(|s| !s.trim().is_empty()) {
            url = format!("{url}&profileArn={}", urlencoding::encode(arn));
        }
        if let Some(mp) = model_provider.filter(|s| !s.trim().is_empty()) {
            url = format!("{url}&modelProvider={}", urlencoding::encode(mp));
        }
        if let Some(token) = next_token.filter(|s| !s.trim().is_empty()) {
            url = format!("{url}&nextToken={}", urlencoding::encode(token));
        }

        let request = with_kiro_q_headers(self.client.get(&url), access_token, machine_id)
            .header("accept", "application/json");

        let response = request
            .send()
            .await
            .map_err(|e| format!("ListAvailableModels 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_q_error("ListAvailableModels", response).await);
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))
    }

    /// MCP 接口 - JSON-RPC 2.0 格式
    #[allow(dead_code)]
    pub async fn call_mcp(
        &self,
        access_token: &str,
        machine_id: &str,
        region: &str,
        profile_arn: Option<&str>,
        json_rpc_request: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/mcp", build_q_service_url(region));

        let mut request = with_kiro_q_headers(self.client.post(&url), access_token, machine_id)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        if let Some(arn) = profile_arn.filter(|s| !s.trim().is_empty()) {
            request = request.header("x-amzn-kiro-profilearn", arn);
        }

        let response = request
            .json(&json_rpc_request)
            .send()
            .await
            .map_err(|e| format!("MCP 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_q_error("MCP", response).await);
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))
    }

    /// setUserPreference 接口 - 设置用户偏好（超额开关）
    pub async fn set_user_preference(
        &self,
        access_token: &str,
        machine_id: &str,
        region: &str,
        profile_arn: Option<&str>,
        overage_status: &str,
    ) -> Result<(), String> {
        let url = format!("{}/setUserPreference", build_q_service_url(region));

        let mut body = serde_json::json!({
            "overageConfiguration": { "overageStatus": overage_status },
        });
        if let Some(profile_arn) = profile_arn.filter(|value| !value.trim().is_empty()) {
            body["profileArn"] = serde_json::json!(profile_arn);
        }

        let request = with_kiro_q_headers(self.client.post(&url), access_token, machine_id)
            .header("content-type", "application/json")
            .json(&body);

        let response = request
            .send()
            .await
            .map_err(|e| format!("setUserPreference 请求失败: {e} (URL: {url})"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_q_error("setUserPreference", response).await);
        }

        Ok(())
    }
}
