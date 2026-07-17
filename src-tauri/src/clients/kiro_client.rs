// Kiro API 客户端 - 统一的 REST API 接口
// 支持 management 与 runtime API 的 endpoint/header 构造，以及 getUsageLimits、ListAvailableModels、setUserPreference

use crate::clients::http_client::{
    build_http_client, build_kiro_control_plane_user_agent,
    build_kiro_management_user_agent, build_kiro_management_x_amz_user_agent,
};
use reqwest::{Client, RequestBuilder};
use uuid::Uuid;

pub struct KiroClient {
    client: reqwest::Client,
}

pub fn build_kiro_runtime_host(region: &str) -> String {
    format!("runtime.{region}.kiro.dev")
}

fn build_kiro_runtime_service_url(region: &str) -> String {
    format!("https://{}", build_kiro_runtime_host(region))
}

pub fn build_generate_assistant_response_url(region: &str) -> String {
    format!(
        "{}/generateAssistantResponse",
        build_kiro_runtime_service_url(region)
    )
}

/// Kiro runtime MCP endpoint.
///
/// Kiro 0.12.301 capture:
/// `POST https://runtime.{region}.kiro.dev/mcp`
/// with `x-amzn-kiro-profile-arn` supplied by `with_kiro_upstream_headers`.
#[allow(dead_code)]
pub fn build_mcp_url(region: &str) -> String {
    format!("{}/mcp", build_kiro_runtime_service_url(region))
}

fn build_kiro_management_host(region: &str) -> String {
    format!("management.{region}.kiro.dev")
}

fn build_kiro_management_service_url(region: &str) -> String {
    format!("https://{}", build_kiro_management_host(region))
}

fn build_get_usage_limits_url(region: &str, profile_arn: Option<&str>) -> String {
    let base = build_kiro_management_service_url(region);
    // 对齐真实 Kiro IDE：GetUsageLimits 带 profileArn。
    // 企业号必须带 ListAvailableProfiles 返回的真实 ARN；带错（如 BuilderId 默认）会 400/403。
    match profile_arn
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(profile_arn) => format!(
            "{base}/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&profileArn={}&resourceType=AGENTIC_REQUEST",
            urlencoding::encode(profile_arn)
        ),
        None => format!(
            "{base}/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&resourceType=AGENTIC_REQUEST"
        ),
    }
}

/// 从 ListAvailableProfiles 响应中取第一个非空 profile ARN。
pub fn first_profile_arn_from_list_response(value: &serde_json::Value) -> Option<String> {
    value
        .get("profiles")?
        .as_array()?
        .iter()
        .find_map(|profile| {
            profile
                .get("arn")
                .and_then(|arn| arn.as_str())
                .map(str::trim)
                .filter(|arn| !arn.is_empty())
                .map(str::to_string)
        })
}

/// 构造 getUsageLimits 的 region 尝试顺序：优先账号 region，企业号再回退常见 region。
pub fn usage_limits_region_candidates(preferred: &str, is_enterprise: bool) -> Vec<String> {
    let mut regions = Vec::new();
    let preferred = preferred.trim();
    if !preferred.is_empty() {
        regions.push(preferred.to_string());
    }
    if is_enterprise {
        for fallback in ["us-east-1", "eu-central-1"] {
            if !regions.iter().any(|r| r == fallback) {
                regions.push(fallback.to_string());
            }
        }
    }
    if regions.is_empty() {
        regions.push("us-east-1".to_string());
    }
    regions
}

fn build_list_available_models_body(profile_arn: Option<&str>) -> serde_json::Value {
    // ListAvailableModels 仍可选 profileArn；企业号解析结果为 None，不会带上
    let mut body = serde_json::json!({
        "origin": "AI_EDITOR",
    });
    if let Some(profile_arn) = profile_arn.filter(|value| !value.trim().is_empty()) {
        body["profileArn"] = serde_json::json!(profile_arn);
    }
    body
}

fn build_list_available_profiles_body() -> serde_json::Value {
    serde_json::json!({})
}

/// 给 RequestBuilder 加上 Kiro Management API 通用 headers
///
/// 包含 Authorization、UA、AWS SDK 中间件需要的请求 ID/重试头。
fn with_kiro_runtime_management_headers(
    req: RequestBuilder,
    access_token: &str,
    machine_id: &str,
    region: &str,
) -> RequestBuilder {
    let user_agent = build_kiro_management_user_agent(machine_id);
    let x_amz_user_agent = build_kiro_management_x_amz_user_agent(machine_id);
    let invocation_id = Uuid::new_v4().to_string();
    req.header("Authorization", format!("Bearer {access_token}"))
        .header("host", build_kiro_management_host(region))
        .header("user-agent", user_agent.clone())
        .header("x-amz-user-agent", x_amz_user_agent)
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", "attempt=1; max=1")
        .header("connection", "close")
}

fn with_kiro_control_plane_headers(
    req: RequestBuilder,
    access_token: &str,
    region: &str,
) -> RequestBuilder {
    let invocation_id = Uuid::new_v4().to_string();
    req.header("Authorization", format!("Bearer {access_token}"))
        .header("host", build_kiro_management_host(region))
        .header("user-agent", build_kiro_control_plane_user_agent())
        .header("x-amz-user-agent", "aws-sdk-js/1.0.0")
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", "attempt=1; max=3")
        .header("connection", "close")
}

async fn classify_kiro_management_error(api: &str, resp: reqwest::Response) -> String {
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

impl KiroClient {
    pub fn new() -> Result<Self, String> {
        let client = build_http_client()?;
        Ok(Self { client })
    }

    pub fn from_client(client: Client) -> Self {
        Self { client }
    }

    /// 统一的 getUsageLimits 接口（支持所有账号类型）。
    ///
    /// `profile_arn`：BuilderId/Social 用解析后的 ARN；Enterprise 用账号保存值或
    /// `ListAvailableProfiles` 发现的真实 ARN。勿对企业号套 BuilderId 默认 ARN。
    pub async fn get_usage_limits(
        &self,
        access_token: &str,
        machine_id: &str,
        region: &str,
        profile_arn: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let url = build_get_usage_limits_url(region, profile_arn);

        log::info!(
            "[GetUsageLimits] Request - region: {region}, profileArn: {}",
            profile_arn.unwrap_or("(none)")
        );

        let request = with_kiro_runtime_management_headers(
            self.client.get(&url),
            access_token,
            machine_id,
            region,
        )
        .header("accept", "application/json");

        let response = request
            .send()
            .await
            .map_err(|e| format!("getUsageLimits 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_management_error("getUsageLimits", response).await);
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))
    }

    /// 按 region 候选列表依次尝试 getUsageLimits。
    /// AUTH_ERROR / BANNED 直接返回（换 region 无意义）；其它错误继续回退。
    pub async fn get_usage_limits_with_region_fallback(
        &self,
        access_token: &str,
        machine_id: &str,
        regions: &[String],
        profile_arn: Option<&str>,
    ) -> Result<(String, serde_json::Value), String> {
        let mut last_err = String::new();
        for region in regions {
            match self
                .get_usage_limits(access_token, machine_id, region, profile_arn)
                .await
            {
                Ok(v) => return Ok((region.clone(), v)),
                Err(e) if e.starts_with("AUTH_ERROR:") || e.starts_with("BANNED:") => {
                    return Err(e);
                }
                Err(e) => {
                    log::warn!("[GetUsageLimits] region {region} failed: {e}");
                    last_err = e;
                }
            }
        }
        Err(format!("所有 region 均失败: {last_err}"))
    }

    /// 按 region 候选列表解析 Enterprise 真实 profileArn（ListAvailableProfiles）。
    ///
    /// AUTH_ERROR / BANNED 直接返回；空 profiles 或其它错误则继续下一 region。
    pub async fn resolve_enterprise_profile_arn(
        &self,
        access_token: &str,
        regions: &[String],
    ) -> Result<Option<String>, String> {
        for region in regions {
            match self.list_available_profiles(access_token, region).await {
                Ok(value) => {
                    if let Some(arn) = first_profile_arn_from_list_response(&value) {
                        log::info!(
                            "[ListAvailableProfiles] region {region} resolved profileArn: {arn}"
                        );
                        return Ok(Some(arn));
                    }
                    log::info!("[ListAvailableProfiles] region {region} returned empty profiles");
                }
                Err(e) if e.starts_with("AUTH_ERROR:") || e.starts_with("BANNED:") => {
                    return Err(e);
                }
                Err(e) => {
                    log::warn!("[ListAvailableProfiles] region {region} failed: {e}");
                }
            }
        }
        Ok(None)
    }

    /// ListAvailableModels 接口
    pub async fn list_available_models(
        &self,
        access_token: &str,
        _machine_id: &str,
        region: &str,
        profile_arn: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let url = build_kiro_management_service_url(region);
        let body = build_list_available_models_body(profile_arn);

        let request = with_kiro_control_plane_headers(self.client.post(&url), access_token, region)
            .header("content-type", "application/x-amz-json-1.0")
            .header(
                "x-amz-target",
                "KiroControlPlaneBearerService.ListAvailableModels",
            )
            .json(&body);

        let response = request
            .send()
            .await
            .map_err(|e| format!("ListAvailableModels 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_management_error("ListAvailableModels", response).await);
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))
    }

    /// ListAvailableProfiles 接口
    pub async fn list_available_profiles(
        &self,
        access_token: &str,
        region: &str,
    ) -> Result<serde_json::Value, String> {
        let url = build_kiro_management_service_url(region);
        let body = build_list_available_profiles_body();

        let request = with_kiro_control_plane_headers(self.client.post(&url), access_token, region)
            .header("content-type", "application/x-amz-json-1.0")
            .header(
                "x-amz-target",
                "KiroControlPlaneBearerService.ListAvailableProfiles",
            )
            .json(&body);

        let response = request
            .send()
            .await
            .map_err(|e| format!("ListAvailableProfiles 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_management_error("ListAvailableProfiles", response).await);
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))
    }

    /// 通用 GET 请求到 Kiro Management API（用于 getAccount 等自定义端点）
    pub async fn get_management_api(
        &self,
        access_token: &str,
        machine_id: &str,
        region: &str,
        path: &str,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", build_kiro_management_service_url(region), path);

        let request = with_kiro_runtime_management_headers(
            self.client.get(&url),
            access_token,
            machine_id,
            region,
        )
        .header("accept", "application/json");

        let response = request
            .send()
            .await
            .map_err(|e| format!("GET {path} 请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_management_error(&format!("GET {path}"), response).await);
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
        let url = format!(
            "{}/setUserPreference",
            build_kiro_management_service_url(region)
        );

        let mut body = serde_json::json!({
            "overageConfiguration": { "overageStatus": overage_status },
        });
        if let Some(profile_arn) = profile_arn.filter(|value| !value.trim().is_empty()) {
            body["profileArn"] = serde_json::json!(profile_arn);
        }

        let request = with_kiro_runtime_management_headers(
            self.client.post(&url),
            access_token,
            machine_id,
            region,
        )
        .header("content-type", "application/json")
        .json(&body);

        let response = request
            .send()
            .await
            .map_err(|e| format!("setUserPreference 请求失败: {e} (URL: {url})"))?;

        if !response.status().is_success() {
            return Err(classify_kiro_management_error("setUserPreference", response).await);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_get_usage_limits_url, build_kiro_management_host, build_kiro_management_service_url,
        build_list_available_models_body, build_list_available_profiles_body,
        usage_limits_region_candidates, with_kiro_control_plane_headers,
        with_kiro_runtime_management_headers,
    };

    #[test]
    fn builds_kiro_management_endpoint_for_region() {
        assert_eq!(
            build_kiro_management_host("us-east-1"),
            "management.us-east-1.kiro.dev"
        );
        assert_eq!(
            build_kiro_management_service_url("eu-central-1"),
            "https://management.eu-central-1.kiro.dev"
        );
    }

    #[test]
    fn builds_get_usage_limits_url_with_optional_profile_arn() {
        assert_eq!(
            build_get_usage_limits_url("us-east-1", None),
            "https://management.us-east-1.kiro.dev/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&resourceType=AGENTIC_REQUEST"
        );
        assert_eq!(
            build_get_usage_limits_url(
                "us-east-1",
                Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/AAAACCCCXXXX")
            ),
            "https://management.us-east-1.kiro.dev/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&profileArn=arn%3Aaws%3Acodewhisperer%3Aus-east-1%3A123456789012%3Aprofile%2FAAAACCCCXXXX&resourceType=AGENTIC_REQUEST"
        );
    }

    #[test]
    fn first_profile_arn_from_list_response_picks_first_non_empty() {
        use super::first_profile_arn_from_list_response;

        assert_eq!(
            first_profile_arn_from_list_response(&serde_json::json!({
                "profiles": [
                    { "arn": "   " },
                    { "arn": "arn:aws:codewhisperer:us-east-1:123456789012:profile/AAAACCCCXXXX", "profileName": "KiroProfile-us-east-1" }
                ]
            }))
            .as_deref(),
            Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/AAAACCCCXXXX")
        );
        assert_eq!(
            first_profile_arn_from_list_response(&serde_json::json!({ "profiles": [] })),
            None
        );
    }

    #[test]
    fn usage_limits_region_candidates_prefers_account_region_and_dedupes_enterprise_fallback() {
        assert_eq!(
            usage_limits_region_candidates("eu-central-1", true),
            vec!["eu-central-1".to_string(), "us-east-1".to_string()]
        );
        assert_eq!(
            usage_limits_region_candidates("us-east-1", true),
            vec!["us-east-1".to_string(), "eu-central-1".to_string()]
        );
        assert_eq!(
            usage_limits_region_candidates("ap-southeast-1", false),
            vec!["ap-southeast-1".to_string()]
        );
        assert_eq!(
            usage_limits_region_candidates("", true),
            vec!["us-east-1".to_string(), "eu-central-1".to_string()]
        );
    }

    #[test]
    fn builds_list_available_models_body_like_control_plane_capture() {
        assert_eq!(
            build_list_available_models_body(Some(
                "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX"
            )),
            serde_json::json!({
                "origin": "AI_EDITOR",
                "profileArn": "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX"
            })
        );
        assert_eq!(
            build_list_available_models_body(None),
            serde_json::json!({ "origin": "AI_EDITOR" })
        );
    }

    #[test]
    fn builds_list_available_profiles_body_like_control_plane_schema() {
        assert_eq!(build_list_available_profiles_body(), serde_json::json!({}));
    }

    #[test]
    fn with_kiro_control_plane_headers_matches_list_available_profiles_capture_shape() {
        let request = with_kiro_control_plane_headers(
            reqwest::Client::new().post("https://management.us-east-1.kiro.dev/"),
            "token-cp",
            "us-east-1",
        )
        .header("content-type", "application/x-amz-json-1.0")
        .header(
            "x-amz-target",
            "KiroControlPlaneBearerService.ListAvailableProfiles",
        )
        .json(&build_list_available_profiles_body())
        .build()
        .expect("request should build");

        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://management.us-east-1.kiro.dev/"
        );
        assert_eq!(
            request
                .headers()
                .get("x-amz-target")
                .and_then(|value| value.to_str().ok()),
            Some("KiroControlPlaneBearerService.ListAvailableProfiles")
        );
    }

    #[test]
    fn with_kiro_runtime_management_headers_uses_kiro_aws_sdk_js_user_agents() {
        let request = with_kiro_runtime_management_headers(
            reqwest::Client::new().get(
                "https://management.eu-central-1.kiro.dev/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&resourceType=AGENTIC_REQUEST",
            ),
            "token-ua",
            "machine-ua",
            "eu-central-1",
        )
        .build()
        .expect("request should build");

        let user_agent = request
            .headers()
            .get(reqwest::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .expect("user-agent header");
        let x_amz_user_agent = request
            .headers()
            .get("x-amz-user-agent")
            .and_then(|value| value.to_str().ok())
            .expect("x-amz-user-agent header");

        assert!(user_agent.starts_with("aws-sdk-js/1.0.0 ua/2.1 os/"));
        assert!(user_agent.contains(" api/codewhispererruntime#1.0.0 m/N,E KiroIDE-"));
        assert!(user_agent.ends_with("-machine-ua"));
        assert!(x_amz_user_agent.starts_with("aws-sdk-js/1.0.0 KiroIDE-"));
        assert!(x_amz_user_agent.ends_with("-machine-ua"));
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::HOST)
                .and_then(|value| value.to_str().ok()),
            Some("management.eu-central-1.kiro.dev")
        );
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::CONNECTION)
                .and_then(|value| value.to_str().ok()),
            Some("close")
        );
    }

    #[test]
    fn with_kiro_control_plane_headers_matches_list_available_models_capture_shape() {
        let request = with_kiro_control_plane_headers(
            reqwest::Client::new().post("https://management.us-east-1.kiro.dev/"),
            "token-cp",
            "us-east-1",
        )
        .header("content-type", "application/x-amz-json-1.0")
        .header(
            "x-amz-target",
            "KiroControlPlaneBearerService.ListAvailableModels",
        )
        .build()
        .expect("request should build");

        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://management.us-east-1.kiro.dev/"
        );
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::HOST)
                .and_then(|value| value.to_str().ok()),
            Some("management.us-east-1.kiro.dev")
        );
        assert_eq!(
            request
                .headers()
                .get("x-amz-target")
                .and_then(|value| value.to_str().ok()),
            Some("KiroControlPlaneBearerService.ListAvailableModels")
        );
        assert_eq!(
            request
                .headers()
                .get("x-amz-user-agent")
                .and_then(|value| value.to_str().ok()),
            Some("aws-sdk-js/1.0.0")
        );
        assert!(request
            .headers()
            .get(reqwest::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("api/kirocontrolplanebearer#1.0.0")));
        assert_eq!(
            request
                .headers()
                .get("amz-sdk-request")
                .and_then(|value| value.to_str().ok()),
            Some("attempt=1; max=3")
        );
    }
}
