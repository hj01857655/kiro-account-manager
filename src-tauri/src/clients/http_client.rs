//! HTTP 客户端公共模块
//! 提供统一的 HTTP 客户端构建，支持代理配置
use crate::commands::app_settings_cmd::get_app_settings_inner;
use crate::core::account::{Account, AccountProxyConfig};
use reqwest::{Client, ClientBuilder, Proxy};
use serde_json::Value;
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use std::{path::PathBuf, sync::LazyLock, time::Duration};

const KIRO_APP_VERSION_FALLBACK: &str = "0.0.0";
const KIRO_NODE_VERSION_FALLBACK: &str = "22.22.0";
static KIRO_UA_OS_RELEASE: LazyLock<String> = LazyLock::new(detect_os_release_for_user_agent);
static KIRO_UA_NODE_VERSION: LazyLock<String> = LazyLock::new(|| {
    std::env::var("KIRO_NODE_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| KIRO_NODE_VERSION_FALLBACK.to_string())
});
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

fn kiro_ide_user_agent_suffix(machine_id: &str) -> String {
    format!("KiroIDE-{}-{}", get_kiro_app_version(), machine_id.trim())
}

fn js_os_platform_for_user_agent() -> &'static str {
    match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        "linux" => "linux",
        other => other,
    }
}

fn detect_os_release_for_user_agent() -> String {
    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

        let release = (|| -> Option<String> {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let key = hklm
                .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
                .ok()?;
            let major = key
                .get_value::<u32, _>("CurrentMajorVersionNumber")
                .unwrap_or(10);
            let minor = key
                .get_value::<u32, _>("CurrentMinorVersionNumber")
                .unwrap_or(0);
            let build = key
                .get_value::<String, _>("CurrentBuildNumber")
                .or_else(|_| key.get_value::<String, _>("CurrentBuild"))
                .ok()?;
            Some(format!("{major}.{minor}.{build}"))
        })();

        return release.unwrap_or_else(|| "10.0.0".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "0.0.0".to_string())
    }
}

pub fn build_kiro_x_amz_user_agent(machine_id: &str) -> String {
    format!(
        "aws-sdk-js/1.0.39 {}",
        kiro_ide_user_agent_suffix(machine_id)
    )
}

pub fn build_kiro_custom_user_agent(machine_id: &str) -> String {
    format!(
        "aws-sdk-js/1.0.39 ua/2.1 os/{}#{} lang/js md/nodejs#{} api/codewhispererstreaming#1.0.39 m/N {}",
        js_os_platform_for_user_agent(),
        KIRO_UA_OS_RELEASE.as_str(),
        KIRO_UA_NODE_VERSION.as_str(),
        kiro_ide_user_agent_suffix(machine_id)
    )
}

pub fn build_kiro_management_user_agent(machine_id: &str) -> String {
    format!(
        "aws-sdk-js/1.0.0 ua/2.1 os/{}#{} lang/js md/nodejs#{} api/codewhispererruntime#1.0.0 m/N,E {}",
        js_os_platform_for_user_agent(),
        KIRO_UA_OS_RELEASE.as_str(),
        KIRO_UA_NODE_VERSION.as_str(),
        kiro_ide_user_agent_suffix(machine_id)
    )
}

pub fn build_kiro_management_x_amz_user_agent(machine_id: &str) -> String {
    format!(
        "aws-sdk-js/1.0.0 {}",
        kiro_ide_user_agent_suffix(machine_id)
    )
}

pub fn build_kiro_control_plane_user_agent() -> String {
    format!(
        "aws-sdk-js/1.0.0 ua/2.1 os/{}#{} lang/js md/nodejs#{} api/kirocontrolplanebearer#1.0.0 m/N,E",
        js_os_platform_for_user_agent(),
        KIRO_UA_OS_RELEASE.as_str(),
        KIRO_UA_NODE_VERSION.as_str()
    )
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

fn normalize_proxy_url(proxy: &str) -> Option<String> {
    let proxy = proxy.trim();
    if proxy.is_empty() {
        return None;
    }

    if proxy.contains("://") {
        Some(proxy.to_string())
    } else {
        Some(format!("http://{proxy}"))
    }
}

/// 获取 Kiro IDE 设置中的代理
fn get_proxy_from_kiro_settings() -> Option<String> {
    read_kiro_settings_json()
        .and_then(|json| get_setting_string(&json, "http.proxy"))
        .and_then(|value| normalize_proxy_url(&value))
}

/// 从系统环境变量获取代理配置
fn get_proxy_from_env() -> Option<String> {
    // 不打印四个变量的探测过程：这个函数在每次构造 HTTP 客户端时都会跑，
    // 而默认环境（四个变量全未设置）下五行输出全是 None，纯粹是噪音。
    // 只在真的读到代理时记一行，并标出是哪个变量生效——
    // 排查「代理没生效」时需要知道命中哪一个，这比逐个列 None 有用。
    let proxy = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
        .iter()
        .find_map(|name| std::env::var(name).ok().map(|value| (*name, value)));
    let Some((source, proxy)) = proxy else {
        return None;
    };
    
    if proxy.trim().is_empty() {
        log::warn!("[HttpClient] 环境变量代理为空字符串");
        return None;
    }
    
    log::debug!("[HttpClient] 使用系统环境变量代理: {}={}", source, proxy);
    normalize_proxy_url(&proxy)
}

/// 系统代理探测结果。
///
/// 区分「探测成功但没有代理」与「探测失败」：两者对调用方都是 `None`（不出站时不套代理），
/// 但对用户是两件事——后者是环境故障（注册表读不到、networksetup 不可用），
/// 得用 warn 报出来，否则用户配了系统代理却直连，日志里什么都看不到。
enum SystemProxyOutcome {
    /// 探测成功，且系统代理已启用。
    Configured(String),
    /// 探测成功，但系统没配代理（含 TUN 模式：`ProxyEnable = 0`）。
    NotConfigured,
    /// 探测本身失败，无法判定有没有代理。
    ProbeFailed(String),
}

/// 从系统（操作系统级）代理设置获取代理。
///
/// Windows 读 WinINET 注册表（`ProxyEnable` / `ProxyServer`），macOS 走
/// `networksetup -getwebproxy`，Linux 读环境变量。复用 `proxy_cmd` 里已有的
/// 跨平台探测，不另写一套注册表读取。
///
/// 只在 `ProxyEnable = 1` 时返回代理。TUN 模式（如 mihomo 的 TUN）不设系统代理，
/// `ProxyEnable` 为 0，这里返回 `NotConfigured`——TUN 在内核路由层接管流量，
/// 应用层看不到也不需要看到。
fn get_proxy_from_system() -> SystemProxyOutcome {
    match crate::commands::proxy_cmd::detect_system_proxy_inner() {
        Ok(info) => match info.http_proxy {
            Some(proxy) => SystemProxyOutcome::Configured(proxy),
            None => {
                // TUN 模式（mihomo / Clash 等）不设系统代理：`ProxyEnable = 0`，
                // 但流量已被内核路由接管。应用层必须直连，否则是双重代理。
                // 打一行日志是为了让日志能解释“为什么没走代理”。
                if info.tun_mode {
                    log::debug!(
                        "[HttpClient] TUN 模式已接管（接口 {}），应用层直连",
                        info.tun_interface.as_deref().unwrap_or("未知")
                    );
                }
                SystemProxyOutcome::NotConfigured
            }
        },
        Err(e) => SystemProxyOutcome::ProbeFailed(e),
    }
}

fn resolve_app_proxy_url() -> Option<String> {
    let mode = match get_app_settings_inner() {
        Ok(settings) => settings.app_proxy_mode,
        Err(e) => {
            log::warn!("[HttpClient] 读取应用代理设置失败: {}, 使用默认模式", e);
            None
        }
    }
    .unwrap_or_else(|| "followKiro".to_string());

    match mode.as_str() {
        "disabled" => {
            // 代理已禁用（生产环境静默）
            None
        }
        "followKiro" | _ => {
            // 优先使用 Kiro IDE 配置的代理
            if let Some(proxy) = get_proxy_from_kiro_settings() {
                log::debug!("[HttpClient] 使用 Kiro IDE 配置的代理: {}", proxy);
                return Some(proxy);
            }

            // 其次是环境变量（CI / 容器里常用这个）
            if let Some(proxy) = get_proxy_from_env() {
                return Some(proxy);
            }

            // 最后跟随操作系统代理。没有这一层时，开了系统代理（ProxyEnable=1）
            // 的机器上浏览器能上网、本应用却直连——这是实际报过的缺口。
            match get_proxy_from_system() {
                SystemProxyOutcome::Configured(proxy) => {
                    log::debug!("[HttpClient] 使用系统代理: {}", proxy);
                    Some(proxy)
                }
                // TUN 模式与「没配代理」在这里合流：都是应用层不该套代理
                SystemProxyOutcome::NotConfigured => None,
                // 探测失败不是「没配代理」：用户可能配了但读不出来。
                // 不能静默直连，否则表现为「浏览器能上网、应用连不上」且无任何线索。
                SystemProxyOutcome::ProbeFailed(reason) => {
                    log::warn!(
                        "[HttpClient] 系统代理探测失败，本次按直连处理（若浏览器可上网而应用不行，请检查系统代理设置）: {reason}"
                    );
                    None
                }
            }
        }
    }
}

pub fn apply_app_proxy(builder: ClientBuilder) -> Result<ClientBuilder, String> {
    match resolve_app_proxy_url() {
        Some(proxy_url) => {
            log::info!("[HttpClient] 应用代理: {}", proxy_url);

            // 使用 Proxy::all 同时设置 HTTP 和 HTTPS 代理
            let proxy = Proxy::all(&proxy_url)
                .map_err(|e| format!("应用接口代理配置错误: {e}"))?;

            // 先禁用系统代理,再应用自定义代理
            Ok(builder.no_proxy().proxy(proxy))
        }
        None => {
            // 未配置代理，直连（生产环境静默）
            Ok(builder)
        }
    }
}

fn base_http_client_builder(timeout_secs: Option<u64>, connect_timeout_secs: u64) -> ClientBuilder {
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

fn apply_account_or_app_proxy(builder: ClientBuilder, account: &Account) -> Result<ClientBuilder, String> {
    if let Some(proxy_config) = account_proxy_config(account) {
        apply_account_proxy(builder, proxy_config)
    } else {
        apply_app_proxy(builder)
    }
}

/// 构建 HTTP 客户端（支持代理、超时配置）
pub fn build_http_client() -> Result<Client, String> {
    build_http_client_with_timeout(30, 10)
}

/// 网关上游专用：显式代理优先，未配置时退回隐式解析链。
/// 显式代理来自网关自身配置（GatewayConfig.upstream_proxy），语义上独立于
/// 账号代理与 Kiro IDE 代理，避免「网关跟 IDE 走」的隐式耦合。
pub fn build_streaming_http_client_with_upstream_proxy(
    upstream_proxy: Option<&str>,
) -> Result<Client, String> {
    let explicit = upstream_proxy.map(str::trim).filter(|value| !value.is_empty());
    let builder = base_http_client_builder(None, 30);
    let builder = match normalize_proxy_url(explicit.unwrap_or_default()) {
        Some(proxy_url) => {
            log::info!("[HttpClient] 网关上游显式代理: {proxy_url}");
            let proxy = Proxy::all(&proxy_url)
                .map_err(|e| format!("网关上游代理配置错误: {e}"))?;
            builder.no_proxy().proxy(proxy)
        }
        None => apply_app_proxy(builder)?,
    };
    builder
        .build()
        .map_err(|e| format!("Failed to create streaming HTTP client: {e}"))
}

pub fn build_streaming_http_client_for_account_with_upstream_proxy(
    account: &Account,
    upstream_proxy: Option<&str>,
) -> Result<Client, String> {
    let explicit = upstream_proxy.map(str::trim).filter(|value| !value.is_empty());
    let builder = base_http_client_builder(None, 30);
    let builder = match normalize_proxy_url(explicit.unwrap_or_default()) {
        Some(proxy_url) => {
            log::info!("[HttpClient] 网关上游显式代理（账号链路）: {proxy_url}");
            let proxy = Proxy::all(&proxy_url)
                .map_err(|e| format!("网关上游代理配置错误: {e}"))?;
            builder.no_proxy().proxy(proxy)
        }
        None => apply_account_or_app_proxy(builder, account)?,
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
    apply_app_proxy(base_http_client_builder(
        Some(timeout_secs),
        connect_timeout_secs,
    ))?
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub fn build_http_client_with_timeout_for_account(
    account: &Account,
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<Client, String> {
    apply_account_or_app_proxy(
        base_http_client_builder(Some(timeout_secs), connect_timeout_secs),
        account,
    )?
    .build()
    .map_err(|e| format!("Failed to create account HTTP client: {e}"))
}

/// 构建 HTTP 客户端（带 User-Agent）
pub fn build_http_client_with_user_agent(user_agent: &str) -> Result<Client, String> {
    apply_app_proxy(base_http_client_builder(Some(30), 10).user_agent(user_agent))?
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub fn build_http_client_with_user_agent_for_account(
    user_agent: &str,
    account: &Account,
) -> Result<Client, String> {
    apply_account_or_app_proxy(
        base_http_client_builder(Some(30), 10).user_agent(user_agent),
        account,
    )?
    .build()
    .map_err(|e| format!("Failed to create account HTTP client: {e}"))
}

pub fn build_http_client_for_proxy_test(
    proxy_config: &AccountProxyConfig,
) -> Result<Client, String> {
    apply_account_proxy(base_http_client_builder(Some(15), 10), proxy_config)?
        .build()
        .map_err(|e| format!("Failed to create proxy test client: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{
        build_kiro_custom_user_agent, build_kiro_x_amz_user_agent, is_external_idp_auth_method,
        is_supported_kiro_region, parse_region_from_profile_arn, resolve_kiro_upstream_region,
        should_add_redirect_for_internal,
    };

    #[test]
    fn kiro_user_agents_match_aws_sdk_js_shape() {
        let user_agent = build_kiro_custom_user_agent("machine-abc");
        let x_amz_user_agent = build_kiro_x_amz_user_agent("machine-abc");

        assert!(user_agent.starts_with("aws-sdk-js/1.0.39 ua/2.1 os/"));
        assert!(user_agent.contains(" lang/js md/nodejs#"));
        assert!(user_agent.contains(" api/codewhispererstreaming#1.0.39 m/N KiroIDE-"));
        assert!(user_agent.ends_with("-machine-abc"));

        assert!(x_amz_user_agent.starts_with("aws-sdk-js/1.0.39 KiroIDE-"));
        assert!(x_amz_user_agent.ends_with("-machine-abc"));
        assert!(!x_amz_user_agent.contains(" ua/2.1 "));
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
