// 代理检测命令

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数

#[cfg(target_os = "windows")]
use crate::utils::cmd_output::decode_cmd_output;
use crate::clients::{
    http_client::build_http_client_for_proxy_test,
    kiro_client::build_generate_assistant_response_url,
};
use crate::core::account::AccountProxyConfig;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProxyInfo {
    pub enabled: bool,
    pub proxy_server: Option<String>,
    pub http_proxy: Option<String>,
    pub tun_mode: bool,
    pub tun_interface: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProxyTestResult {
    pub success: bool,
    pub latency_ms: u128,
    pub status: Option<u16>,
    pub message: String,
}

// ============================================================
// Windows: 从注册表读取系统代理
// ============================================================

fn normalize_proxy_url(proxy: &str, default_scheme: &str) -> Option<String> {
    let proxy = proxy.trim();
    if proxy.is_empty() {
        return None;
    }

    if proxy.contains("://") {
        Some(proxy.to_string())
    } else {
        Some(format!("{default_scheme}://{proxy}"))
    }
}

#[cfg(target_os = "windows")]
const WINDOWS_PROXY_PROTOCOL_PRIORITY: &[(&str, &str)] =
    &[("http", "http"), ("https", "http"), ("socks", "socks5")];

#[cfg(target_os = "windows")]
fn select_windows_proxy_endpoint(proxy_server: &str) -> (&str, &str) {
    WINDOWS_PROXY_PROTOCOL_PRIORITY
        .iter()
        .find_map(|(protocol, default_scheme)| {
            proxy_server
                .split(';')
                .filter_map(|segment| segment.trim().split_once('='))
                .find(|(key, _)| key.eq_ignore_ascii_case(protocol))
                .map(|(_, value)| (value, *default_scheme))
        })
        .unwrap_or((proxy_server, "http"))
}

#[cfg(target_os = "windows")]
fn extract_windows_http_proxy(enabled: bool, proxy_server: &str) -> Option<String> {
    if !enabled {
        return None;
    }

    let (proxy, default_scheme) = select_windows_proxy_endpoint(proxy_server);
    normalize_proxy_url(proxy, default_scheme)
}

#[cfg(target_os = "windows")]
fn detect_tun_mode() -> (bool, Option<String>) {
    use std::process::Command;

    // 方法1：使用 Get-NetAdapter 检测 TUN 类型网卡（更可靠）
    let output = Command::new("powershell")
        .args(["-Command", "Get-NetAdapter | Where-Object {$_.InterfaceDescription -like '*TAP*' -or $_.InterfaceDescription -like '*TUN*' -or $_.InterfaceDescription -like '*WireGuard*' -or $_.InterfaceDescription -like '*Wintun*'} | Where-Object {$_.Status -eq 'Up'} | Select-Object -ExpandProperty Name"])
        .output();

    if let Ok(output) = output {
        let stdout = decode_cmd_output(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            // 取第一个活跃的 TUN 网卡
            if let Some(name) = stdout.lines().next() {
                return (true, Some(name.to_string()));
            }
        }
    }

    // 方法2：使用 netsh 检查已连接的非物理网卡
    let output = Command::new("netsh")
        .args(["interface", "show", "interface"])
        .output();

    if let Ok(output) = output {
        let stdout = decode_cmd_output(&output.stdout);

        // 排除常见物理网卡名
        let physical_names = [
            "以太网",
            "ethernet",
            "wlan",
            "wi-fi",
            "bluetooth",
            "蓝牙",
            "本地连接",
        ];

        for line in stdout.lines() {
            let line_lower = line.to_lowercase();
            // 检查是否已连接
            if line.contains("已连接") || line.contains("Connected") {
                // 排除物理网卡
                let is_physical = physical_names.iter().any(|p| line_lower.contains(p));
                if !is_physical {
                    // 提取网卡名（最后一列）
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let name = parts[parts.len() - 1];
                        return (true, Some(name.to_string()));
                    }
                }
            }
        }
    }

    (false, None)
}

#[cfg(target_os = "windows")]
pub(crate) fn detect_system_proxy_inner() -> Result<SystemProxyInfo, String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .map_err(|e| format!("无法打开注册表: {e}"))?;

    let proxy_enable: u32 = internet_settings.get_value("ProxyEnable").unwrap_or(0);
    let proxy_server: String = internet_settings
        .get_value("ProxyServer")
        .unwrap_or_default();

    let enabled = proxy_enable == 1;
    let http_proxy = extract_windows_http_proxy(enabled, &proxy_server);

    let (tun_mode, tun_interface) = detect_tun_mode();

    Ok(SystemProxyInfo {
        enabled,
        proxy_server: if proxy_server.is_empty() {
            None
        } else {
            Some(proxy_server)
        },
        http_proxy,
        tun_mode,
        tun_interface,
    })
}

// ============================================================
// macOS: 从系统偏好设置读取代理
// ============================================================

#[cfg(target_os = "macos")]
fn detect_tun_mode() -> (bool, Option<String>) {
    use std::process::Command;

    // macOS 上 TUN 设备都是 utun 开头
    let output = Command::new("ifconfig").output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // 查找 utun 开头且有 inet 地址的网卡（活跃的 TUN）
        let mut current_iface: Option<String> = None;
        let mut has_inet = false;

        for line in stdout.lines() {
            if line.starts_with("utun") && line.contains(": flags=") {
                // 新的 utun 接口
                if let Some(ref iface) = current_iface {
                    if has_inet {
                        return (true, Some(iface.clone()));
                    }
                }
                current_iface = line.split(':').next().map(std::string::ToString::to_string);
                has_inet = false;
            } else if current_iface.is_some() && line.contains("inet ") && !line.contains("inet6") {
                has_inet = true;
            }
        }

        // 检查最后一个接口
        if let Some(iface) = current_iface {
            if has_inet {
                return (true, Some(iface));
            }
        }
    }

    (false, None)
}

#[cfg(target_os = "macos")]
pub(crate) fn detect_system_proxy_inner() -> Result<SystemProxyInfo, String> {
    use std::process::Command;

    // 获取当前网络服务名称
    let output = Command::new("networksetup")
        .args(["-listallnetworkservices"])
        .output()
        .map_err(|e| format!("执行 networksetup 失败: {}", e))?;

    let services = String::from_utf8_lossy(&output.stdout);

    // 尝试常见的网络服务名称
    let service_names = ["Wi-Fi", "Ethernet", "USB 10/100/1000 LAN"];
    let mut active_service = None;

    for name in &service_names {
        if services.contains(name) {
            active_service = Some(*name);
            break;
        }
    }

    let service = active_service.unwrap_or("Wi-Fi");

    // 获取 HTTP 代理设置
    let output = Command::new("networksetup")
        .args(["-getwebproxy", service])
        .output()
        .map_err(|e| format!("获取代理设置失败: {}", e))?;

    let proxy_info = String::from_utf8_lossy(&output.stdout);

    let mut enabled = false;
    let mut server = String::new();
    let mut port = String::new();

    for line in proxy_info.lines() {
        if line.starts_with("Enabled:") {
            enabled = line.contains("Yes");
        } else if line.starts_with("Server:") {
            server = line.trim_start_matches("Server:").trim().to_string();
        } else if line.starts_with("Port:") {
            port = line.trim_start_matches("Port:").trim().to_string();
        }
    }

    let proxy_server = if !server.is_empty() && server != "0" {
        Some(format!("{}:{}", server, port))
    } else {
        None
    };
    let http_proxy = proxy_server
        .as_deref()
        .filter(|_| enabled)
        .and_then(|proxy| normalize_proxy_url(proxy, "http"));

    let (tun_mode, tun_interface) = detect_tun_mode();

    Ok(SystemProxyInfo {
        enabled,
        proxy_server,
        http_proxy,
        tun_mode,
        tun_interface,
    })
}

#[cfg(target_os = "linux")]
fn detect_tun_mode() -> (bool, Option<String>) {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    // 方法1：使用 ip tuntap show 列出所有 TUN/TAP 设备（最可靠）
    if let Ok(output) = Command::new("ip").args(["tuntap", "show"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // 格式: "tun0: tun" 或 "tap0: tap"
            if let Some(name) = line.split(':').next() {
                let name = name.trim();
                // 检查是否有 IP 地址（活跃状态）
                if let Ok(addr_output) = Command::new("ip").args(["addr", "show", name]).output() {
                    let addr_stdout = String::from_utf8_lossy(&addr_output.stdout);
                    if addr_stdout.contains("inet ") {
                        return (true, Some(name.to_string()));
                    }
                }
            }
        }
    }

    // 方法2：检查 /sys/class/net/ 下的 TUN/TAP 设备
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // 跳过常见物理网卡
            if name.starts_with("eth")
                || name.starts_with("en")
                || name.starts_with("wl")
                || name == "lo"
            {
                continue;
            }

            // 检查 tun_flags 文件（TUN/TAP 设备特有）
            let tun_flags_path = format!("/sys/class/net/{}/tun_flags", name);
            if Path::new(&tun_flags_path).exists() {
                if let Ok(output) = Command::new("ip").args(["addr", "show", &name]).output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("inet ") {
                        return (true, Some(name));
                    }
                }
            }

            // 检查设备类型（65534 = NONE/TUN 点对点设备）
            let type_path = format!("/sys/class/net/{}/type", name);
            if let Ok(type_content) = fs::read_to_string(&type_path) {
                let dev_type = type_content.trim();
                // 65534 = ARPHRD_NONE (TUN), 1 = ARPHRD_ETHER (TAP/物理)
                if dev_type == "65534" {
                    if let Ok(output) = Command::new("ip").args(["addr", "show", &name]).output() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.contains("inet ") {
                            return (true, Some(name));
                        }
                    }
                }
            }
        }
    }

    (false, None)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) fn detect_system_proxy_inner() -> Result<SystemProxyInfo, String> {
    // Linux: 尝试读取环境变量
    let proxy_server = std::env::var("http_proxy")
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .ok();
    let http_proxy = proxy_server
        .as_deref()
        .and_then(|proxy| normalize_proxy_url(proxy, "http"));

    let (tun_mode, tun_interface) = detect_tun_mode();

    Ok(SystemProxyInfo {
        enabled: http_proxy.is_some(),
        proxy_server,
        http_proxy,
        tun_mode,
        tun_interface,
    })
}

// ============================================================
// Tauri Command
// ============================================================

#[tauri::command]
pub async fn detect_system_proxy() -> Result<SystemProxyInfo, String> {
    tokio::task::spawn_blocking(detect_system_proxy_inner)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub async fn test_account_proxy(
    proxy_config: AccountProxyConfig,
) -> Result<AccountProxyTestResult, String> {
    let started_at = Instant::now();
    let client = build_http_client_for_proxy_test(&proxy_config)?;

    let test_url = build_generate_assistant_response_url("us-east-1");
    let response = client
        .get(&test_url)
        .send()
        .await
        .map_err(|error| format!("代理连接失败: {error}"))?;

    let status = response.status();
    let latency_ms = started_at.elapsed().as_millis();

    Ok(AccountProxyTestResult {
        success: true,
        latency_ms,
        status: Some(status.as_u16()),
        message: format!("代理已连通 Kiro runtime 上游，延迟 {latency_ms}ms，上游返回 HTTP {status}"),
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_proxy_url;

    #[test]
    fn normalize_proxy_url_adds_default_scheme_only_when_missing() {
        assert_eq!(
            normalize_proxy_url("127.0.0.1:7890", "http"),
            Some("http://127.0.0.1:7890".to_string())
        );
        assert_eq!(
            normalize_proxy_url("socks5://127.0.0.1:1080", "http"),
            Some("socks5://127.0.0.1:1080".to_string())
        );
        assert_eq!(normalize_proxy_url("", "http"), None);
    }

    #[cfg(target_os = "windows")]
    mod windows {
        use super::super::extract_windows_http_proxy;

        #[test]
        fn extract_windows_http_proxy_supports_simple_and_multi_protocol_values() {
            assert_eq!(
                extract_windows_http_proxy(true, "127.0.0.1:7890"),
                Some("http://127.0.0.1:7890".to_string())
            );
            assert_eq!(
                extract_windows_http_proxy(true, "http=127.0.0.1:7890;https=127.0.0.1:7891"),
                Some("http://127.0.0.1:7890".to_string())
            );
            assert_eq!(
                extract_windows_http_proxy(true, "https=127.0.0.1:7891;socks=127.0.0.1:1080"),
                Some("http://127.0.0.1:7891".to_string())
            );
            assert_eq!(
                extract_windows_http_proxy(true, "socks=127.0.0.1:1080"),
                Some("socks5://127.0.0.1:1080".to_string())
            );
        }

        #[test]
        fn extract_windows_http_proxy_returns_none_when_disabled_or_empty() {
            assert_eq!(extract_windows_http_proxy(false, "127.0.0.1:7890"), None);
            assert_eq!(extract_windows_http_proxy(true, ""), None);
        }
    }
}
