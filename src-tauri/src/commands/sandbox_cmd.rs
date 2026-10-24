// Command Sandbox 配置命令（Kiro IDE 1.3+ / CLI 2.31+）
//
// 沙箱不是 settings.json 的键，而是独立的 sandbox.json（官方 docs/sandbox）：
//   全局：~/.kiro/settings/sandbox.json
//   项目：<workspace-root>/.kiro/settings/sandbox.json
// 键：enabled(bool) / networkAccess(DEFAULT_BLOCKED|COMMON_DEPENDENCIES|
// DEFAULT_ALLOWED，默认 DEFAULT_BLOCKED) / mcpSandboxing(bool)。
// 全局激活时项目只能收紧；本面板只管全局文件，项目级交给 Kiro 自己的 /sandbox。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_network_access")]
    pub network_access: String,
    #[serde(default)]
    pub mcp_sandboxing: bool,
}

fn default_network_access() -> String {
    "DEFAULT_BLOCKED".to_string()
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            network_access: default_network_access(),
            mcp_sandboxing: false,
        }
    }
}

pub fn sandbox_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kiro").join("settings").join("sandbox.json"))
}

#[tauri::command]
pub async fn get_sandbox_config() -> Result<SandboxConfig, String> {
    let path = sandbox_config_path().ok_or("无法获取用户目录")?;
    if !path.exists() {
        // 未启用过：文件不存在 = 沙箱关闭（官方默认态），不报错
        return Ok(SandboxConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("读取 sandbox.json 失败: {e}"))?;
    serde_json::from_str::<SandboxConfig>(&raw).map_err(|e| format!("解析 sandbox.json 失败: {e}"))
}

#[tauri::command]
pub async fn save_sandbox_config(config: SandboxConfig) -> Result<(), String> {
    if !matches!(
        config.network_access.as_str(),
        "DEFAULT_BLOCKED" | "COMMON_DEPENDENCIES" | "DEFAULT_ALLOWED"
    ) {
        return Err(format!("非法的 networkAccess: {}", config.network_access));
    }
    let path = sandbox_config_path().ok_or("无法获取用户目录")?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let content =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&path, content + "\n").map_err(|e| format!("写入 sandbox.json 失败: {e}"))
}
