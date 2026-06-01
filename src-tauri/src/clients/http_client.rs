//! HTTP 客户端公共模块
//! 提供统一的 HTTP 客户端构建，支持代理配置
use crate::core::account::{Account, AccountProxyConfig};
use reqwest::{Client, ClientBuilder, Proxy};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};

const KIRO_APP_VERSION_FALLBACK: &str = "0.0.0";
const SUPPORTED_KIRO_REGIONS: &[&str] = &[
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eu-central-1",
    "eu-central-2",
    "eu-north-1",
    "eu-south-1",
    "eu-south-2",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-northeast-3",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-southeast-3",
    "ap-southeast-4",
    "ap-southeast-5",
    "ap-southeast-7",
    "ap-south-1",
    "ap-south-2",
    "ap-east-1",
    "ca-central-1",
    "ca-west-1",
    "sa-east-1",
    "me-south-1",
    "me-central-1",
    "il-central-1",
    "mx-central-1",
    "af-south-1",
    "us-gov-west-1",
    "us-gov-east-1",
    "cn-north-1",
    "cn-northwest-1",
];

// 企业账号多区域探测优先级列表（按使用频率排序）
//
// 不能再单独维护一份精简列表 —— issue #103 那位 ap-southeast-2 的企业用户被这层挡掉，
// 报 502。SUPPORTED_KIRO_REGIONS 收 38 个、这里却只有 10 个，肯定漏。
// 现在覆盖跟 SUPPORTED_KIRO_REGIONS 同等的全集，前面留高频 region 不破坏探测命中率，
// 后面追加低频 region 兜底，尽可能首次探测就命中、不必用户手动改 region 字段。
const USAGE_PROBE_REGIONS: &[&str] = &[
    // 高频
    "us-east-1",
    "eu-central-1",
    "us-west-2",
    "ap-northeast-1",
    "us-east-2",
    "eu-west-1",
    "ap-southeast-1",
    "us-west-1",
    "eu-west-2",
    "ap-northeast-2",
    // 兜底（低频但受支持）
    "ap-southeast-2",
    "ap-southeast-3",
    "ap-southeast-4",
    "ap-southeast-5",
    "ap-southeast-7",
    "ap-northeast-3",
    "ap-south-1",
    "ap-south-2",
    "ap-east-1",
    "eu-west-3",
    "eu-north-1",
    "eu-south-1",
    "eu-south-2",
    "eu-central-2",
    "ca-central-1",
    "ca-west-1",
    "sa-east-1",
    "me-south-1",
    "me-central-1",
    "il-central-1",
    "mx-central-1",
    "af-south-1",
    "us-gov-west-1",
    "us-gov-east-1",
    "cn-north-1",
    "cn-northwest-1",
];

fn normalize_kiro_region(region: Option<&str>) -> Option<String> {
    let region = region?.trim();
    if region.is_empty() || !SUPPORTED_KIRO_REGIONS.contains(&region) {
        return None;
    }
    Some(region.to_string())
}

fn get_kiro_settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata)
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

fn read_kiro_settings_json() -> Option<Value> {
    let path = get_kiro_settings_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn get_setting_bool(json: &Value, key: &str) -> Option<bool> {
    if let Some(value) = json.get(key).and_then(Value::as_bool) {
        return Some(value);
    }

    let mut current = json;
    for segment in key.split('.') {
        current = current.get(segment)?;
    }
    current.as_bool()
}

fn get_setting_string(json: &Value, key: &str) -> Option<String> {
    if let Some(value) = json.get(key).and_then(Value::as_str) {
        return Some(value.to_string());
    }

    let mut current = json;
    for segment in key.split('.') {
        current = current.get(segment)?;
    }
    current.as_str().map(str::to_string)
}

fn get_kiro_product_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let root = PathBuf::from(local_app_data)
                .join("Programs")
                .join("Kiro")
                .join("resources")
                .join("app");
            paths.push(root.join("product.json"));
            paths.push(root.join("package.json"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let root = PathBuf::from("/Applications")
            .join("Kiro.app")
            .join("Contents")
            .join("Resources")
            .join("app");
        paths.push(root.join("product.json"));
        paths.push(root.join("package.json"));
    }

    #[cfg(target_os = "linux")]
    {
        for root in [
            PathBuf::from("/opt/Kiro/resources/app"),
            std::env::var("HOME")
                .ok()
                .map(|home| {
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join("Kiro")
                        .join("resources")
                        .join("app")
                })
                .unwrap_or_default(),
        ] {
            if !root.as_os_str().is_empty() {
                paths.push(root.join("product.json"));
                paths.push(root.join("package.json"));
            }
        }
    }

    paths
}

pub fn get_kiro_app_version() -> String {
    get_kiro_product_paths()
        .into_iter()
        .find_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let json: Value = serde_json::from_str(&content).ok()?;
            json.get("version")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| KIRO_APP_VERSION_FALLBACK.to_string())
}

pub fn build_kiro_custom_user_agent(machine_id: &str) -> String {
    format!("KiroIDE {} {}", get_kiro_app_version(), machine_id)
}

pub fn is_supported_kiro_region(region: &str) -> bool {
    normalize_kiro_region(Some(region)).is_some()
}

pub fn parse_region_from_profile_arn(profile_arn: Option<&str>) -> Option<String> {
    let profile_arn = profile_arn?.trim();
    if profile_arn.is_empty() {
        return None;
    }

    let mut segments = profile_arn.split(':');
    let arn = segments.next()?;
    let partition = segments.next()?;
    let service = segments.next()?;
    let region = segments.next()?;

    if arn != "arn" || partition.is_empty() || service != "codewhisperer" {
        return None;
    }

    normalize_kiro_region(Some(region))
}

pub fn resolve_kiro_upstream_region(
    profile_arn: Option<&str>,
    account_region: Option<&str>,
    fallback_region: &str,
) -> String {
    parse_region_from_profile_arn(profile_arn)
        .or_else(|| normalize_kiro_region(account_region))
        .or_else(|| normalize_kiro_region(Some(fallback_region)))
        .unwrap_or_else(|| "us-east-1".to_string())
}

pub fn get_usage_probe_regions() -> &'static [&'static str] {
    USAGE_PROBE_REGIONS
}

pub fn build_q_service_url(region: &str) -> String {
    format!("https://q.{}.amazonaws.com", region)
}

pub fn should_send_codewhisperer_optout() -> bool {
    let Some(json) = read_kiro_settings_json() else {
        return true;
    };

    let content_collection_enabled = get_setting_bool(
        &json,
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement",
    )
    .or_else(|| {
        get_setting_bool(
            &json,
            "telemetry.dataSharing.contentCollectionForServiceImprovement",
        )
    })
    .unwrap_or(false);

    !content_collection_enabled
}

#[allow(dead_code)]
pub fn is_external_idp_auth_method(auth_method: Option<&str>) -> bool {
    auth_method.is_some_and(|value| {
        let trimmed = value.trim();
        trimmed.eq_ignore_ascii_case("external_idp") || trimmed.eq_ignore_ascii_case("IdC")
    })
}

pub fn should_add_redirect_for_internal(provider: Option<&str>) -> bool {
    provider.is_some_and(|value| value.trim().eq_ignore_ascii_case("Internal"))
}

/// 获取 Kiro IDE 设置中的代理
fn get_proxy_from_kiro_settings() -> Option<String> {
    read_kiro_settings_json().and_then(|json| {
        get_setting_string(&json, "http.proxy").filter(|value| !value.trim().is_empty())
    })
}

fn base_http_client_builder(
    timeout_secs: Option<u64>,
    connect_timeout_secs: u64,
) -> ClientBuilder {
    let builder = Client::builder()
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .pool_idle_timeout(Duration::from_secs(120))
        .pool_max_idle_per_host(20)
        .tcp_keepalive(Duration::from_secs(60))
        .http2_keep_alive_interval(Duration::from_secs(30))
        .http2_keep_alive_timeout(Duration::from_secs(20))
        .http2_keep_alive_while_idle(true);

    if let Some(timeout_secs) = timeout_secs {
        builder.timeout(Duration::from_secs(timeout_secs))
    } else {
        builder
    }
}

fn apply_kiro_settings_proxy(mut builder: ClientBuilder) -> ClientBuilder {
    // Preserve the existing generic proxy behavior: invalid settings are ignored.
    if let Some(proxy_url) = get_proxy_from_kiro_settings() {
        if let Ok(proxy) = Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder
}

fn apply_account_proxy(
    builder: ClientBuilder,
    proxy_config: &AccountProxyConfig,
) -> Result<ClientBuilder, String> {
    let proxy_url = proxy_config.to_proxy_url()?;
    let proxy = Proxy::all(&proxy_url)
        .map_err(|error| format!("Invalid account proxy configuration: {error}"))?;

    Ok(builder.no_proxy().proxy(proxy))
}

fn account_proxy_config(account: &Account) -> Option<&AccountProxyConfig> {
    account
        .proxy_config
        .as_ref()
        .filter(|proxy_config| proxy_config.enabled)
}

/// 构建 HTTP 客户端（支持代理、超时配置）
pub fn build_http_client() -> Result<Client, String> {
    build_http_client_with_timeout(30, 10)
}

pub fn build_streaming_http_client_for_account(account: &Account) -> Result<Client, String> {
    let builder = base_http_client_builder(None, 30);
    let builder = if let Some(proxy_config) = account_proxy_config(account) {
        apply_account_proxy(builder, proxy_config)?
    } else {
        apply_kiro_settings_proxy(builder)
    };

    builder
        .build()
        .map_err(|e| format!("Failed to create account streaming HTTP client: {e}"))
}

/// 构建 HTTP 客户端（自定义超时）
pub fn build_http_client_with_timeout(
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<Client, String> {
    apply_kiro_settings_proxy(base_http_client_builder(
        Some(timeout_secs),
        connect_timeout_secs,
    ))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub fn build_http_client_with_timeout_for_account(
    account: &Account,
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<Client, String> {
    let builder = base_http_client_builder(Some(timeout_secs), connect_timeout_secs);
    let builder = if let Some(proxy_config) = account_proxy_config(account) {
        apply_account_proxy(builder, proxy_config)?
    } else {
        apply_kiro_settings_proxy(builder)
    };

    builder
        .build()
        .map_err(|e| format!("Failed to create account HTTP client: {e}"))
}

/// 构建 HTTP 客户端（带 User-Agent）
pub fn build_http_client_with_user_agent(user_agent: &str) -> Result<Client, String> {
    apply_kiro_settings_proxy(base_http_client_builder(Some(30), 10).user_agent(user_agent))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub fn build_http_client_with_user_agent_for_account(
    user_agent: &str,
    account: &Account,
) -> Result<Client, String> {
    let builder = base_http_client_builder(Some(30), 10).user_agent(user_agent);
    let builder = if let Some(proxy_config) = account_proxy_config(account) {
        apply_account_proxy(builder, proxy_config)?
    } else {
        apply_kiro_settings_proxy(builder)
    };

    builder
        .build()
        .map_err(|e| format!("Failed to create account HTTP client: {e}"))
}

pub fn build_http_client_for_proxy_test(
    proxy_config: &AccountProxyConfig,
) -> Result<Client, String> {
    let builder = base_http_client_builder(Some(15), 10);
    apply_account_proxy(builder, proxy_config)?
        .build()
        .map_err(|e| format!("Failed to create proxy test client: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{
        is_external_idp_auth_method, is_supported_kiro_region,
        parse_region_from_profile_arn, resolve_kiro_upstream_region,
        should_add_redirect_for_internal,
    };

    #[test]
    fn parse_region_from_profile_arn_accepts_supported_regions_only() {
        assert_eq!(
            parse_region_from_profile_arn(Some(
                "arn:aws:codewhisperer:eu-central-1:123456789012:profile/test"
            ))
            .as_deref(),
            Some("eu-central-1")
        );
        // eu-west-1 现在是支持的区域
        assert_eq!(
            parse_region_from_profile_arn(Some(
                "arn:aws:codewhisperer:eu-west-1:123456789012:profile/test"
            ))
            .as_deref(),
            Some("eu-west-1")
        );
        // 非 codewhisperer 服务的 ARN 应该返回 None
        assert_eq!(
            parse_region_from_profile_arn(Some("arn:aws:s3:us-east-1:123456789012:bucket/test")),
            None
        );
    }

    #[test]
    fn resolve_kiro_upstream_region_prefers_profile_arn_then_account_then_fallback() {
        assert_eq!(
            resolve_kiro_upstream_region(
                Some("arn:aws:codewhisperer:eu-central-1:123456789012:profile/test"),
                Some("us-east-1"),
                "us-west-2"
            ),
            "eu-central-1"
        );
        assert_eq!(
            resolve_kiro_upstream_region(None, Some("ap-southeast-1"), "us-east-1"),
            "ap-southeast-1"
        );
        // eu-west-1 现在是支持的区域，所以会被使用而不是 fallback
        assert_eq!(
            resolve_kiro_upstream_region(None, Some("eu-west-1"), "us-west-2"),
            "eu-west-1"
        );
    }

    #[test]
    fn supported_region_helper_matches_gateway_allow_list() {
        assert!(is_supported_kiro_region("us-east-1"));
        assert!(is_supported_kiro_region("us-gov-west-1"));
        assert!(is_supported_kiro_region("eu-west-1"));
    }

    #[test]
    fn external_idp_auth_method_check_is_case_insensitive_and_strict() {
        assert!(is_external_idp_auth_method(Some("external_idp")));
        assert!(is_external_idp_auth_method(Some("EXTERNAL_IDP")));
        assert!(is_external_idp_auth_method(Some("IdC")));
        assert!(!is_external_idp_auth_method(Some("social")));
    }

    #[test]
    fn redirect_for_internal_check_is_case_insensitive_and_strict() {
        assert!(should_add_redirect_for_internal(Some("Internal")));
        assert!(should_add_redirect_for_internal(Some("internal")));
        assert!(!should_add_redirect_for_internal(Some("Enterprise")));
        assert!(!should_add_redirect_for_internal(Some("BuilderId")));
    }
}
