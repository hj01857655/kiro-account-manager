//! HTTP 客户端公共模块
//! 提供统一的 HTTP 客户端构建，支持代理配置

use reqwest::{Client, Proxy};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};

const KIRO_APP_VERSION_FALLBACK: &str = "0.0.0";
const SUPPORTED_KIRO_REGIONS: &[&str] = &[
    "us-east-1",
    "eu-central-1",
    "us-west-2",
    "ap-northeast-1",
    "ap-southeast-1",
    "us-gov-west-1",
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

pub fn resolve_q_service_endpoint(region: Option<&str>) -> &'static str {
    if normalize_kiro_region(region).is_some_and(|value| value.starts_with("eu-")) {
        "https://q.eu-central-1.amazonaws.com"
    } else {
        "https://q.us-east-1.amazonaws.com"
    }
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

/// 获取 Kiro IDE 设置中的代理
fn get_proxy_from_kiro_settings() -> Option<String> {
    read_kiro_settings_json().and_then(|json| {
        get_setting_string(&json, "http.proxy").filter(|value| !value.trim().is_empty())
    })
}

/// 构建 HTTP 客户端（支持代理、超时配置）
pub fn build_http_client() -> Result<Client, String> {
    build_http_client_with_timeout(30, 10)
}

/// 构建 HTTP 客户端（自定义超时）
pub fn build_http_client_with_timeout(
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(connect_timeout_secs));

    // 尝试从 Kiro 设置获取代理
    if let Some(proxy_url) = get_proxy_from_kiro_settings() {
        if let Ok(proxy) = Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }

    builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

/// 构建 HTTP 客户端（带 User-Agent）
pub fn build_http_client_with_user_agent(user_agent: &str) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent(user_agent);

    // 尝试从 Kiro 设置获取代理
    if let Some(proxy_url) = get_proxy_from_kiro_settings() {
        if let Ok(proxy) = Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }

    builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{
        is_supported_kiro_region, parse_region_from_profile_arn, resolve_kiro_upstream_region,
        resolve_q_service_endpoint,
    };

    #[test]
    fn resolve_q_service_endpoint_matches_upstream_region_rule() {
        assert_eq!(
            resolve_q_service_endpoint(Some("eu-west-1")),
            "https://q.us-east-1.amazonaws.com"
        );
        assert_eq!(
            resolve_q_service_endpoint(Some("eu-central-1")),
            "https://q.eu-central-1.amazonaws.com"
        );
        assert_eq!(
            resolve_q_service_endpoint(Some("us-east-1")),
            "https://q.us-east-1.amazonaws.com"
        );
        assert_eq!(
            resolve_q_service_endpoint(None),
            "https://q.us-east-1.amazonaws.com"
        );
    }

    #[test]
    fn parse_region_from_profile_arn_accepts_supported_regions_only() {
        assert_eq!(
            parse_region_from_profile_arn(Some(
                "arn:aws:codewhisperer:eu-central-1:123456789012:profile/test"
            ))
            .as_deref(),
            Some("eu-central-1")
        );
        assert_eq!(
            parse_region_from_profile_arn(Some(
                "arn:aws:codewhisperer:eu-west-1:123456789012:profile/test"
            )),
            None
        );
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
        assert_eq!(
            resolve_kiro_upstream_region(None, Some("eu-west-1"), "us-west-2"),
            "us-west-2"
        );
    }

    #[test]
    fn supported_region_helper_matches_gateway_allow_list() {
        assert!(is_supported_kiro_region("us-east-1"));
        assert!(is_supported_kiro_region("us-gov-west-1"));
        assert!(!is_supported_kiro_region("eu-west-1"));
    }
}
