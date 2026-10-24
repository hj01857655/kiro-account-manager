// Command Sandbox 配置命令（Kiro IDE 1.3+ / CLI 2.31+）
//
// 沙箱不是 settings.json 的键，而是独立的 sandbox.json（官方 docs/sandbox）：
//   全局：~/.kiro/settings/sandbox.json
//   项目：<workspace-root>/.kiro/settings/sandbox.json
// 键：enabled(bool) / networkAccess(DEFAULT_BLOCKED|COMMON_DEPENDENCIES|
// DEFAULT_ALLOWED，默认 DEFAULT_BLOCKED) / mcpSandboxing(bool)。
//
// 两级合并规则（对齐 IDE 的 tGf）：
//   - 全局 enabled=false 时项目配置不生效（全局是总闸）
//   - 项目 networkAccess 不能比全局宽松，宽松则忽略并 warn
//   - 全局开了 mcpSandboxing 时项目不能关，关了就忽略并 warn
// 即项目只能收紧不能放宽。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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

/// 项目级 sandbox.json：`<workspace-root>/.kiro/settings/sandbox.json`。
pub fn project_sandbox_config_path(workspace_root: &str) -> PathBuf {
    Path::new(workspace_root)
        .join(".kiro")
        .join("settings")
        .join("sandbox.json")
}

/// 出网模式的宽松程度：数字越大越宽松（IDE 的 `iAa`）。
fn network_rank(mode: &str) -> u8 {
    match mode {
        "DEFAULT_BLOCKED" => 3,
        "COMMON_DEPENDENCIES" => 2,
        "DEFAULT_ALLOWED" => 1,
        _ => 3,
    }
}

fn read_sandbox_file(path: &Path) -> Result<Option<SandboxConfig>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str::<SandboxConfig>(&raw)
        .map(Some)
        .map_err(|e| format!("解析 {} 失败: {e}", path.display()))
}

/// 全局 + 项目两级的生效配置。
///
/// 项目配置只能在全局基础上收紧：更宽松的 `networkAccess` 被忽略，
/// 全局开启的 `mcpSandboxing` 不能被项目关掉。两者都通过 `notes` 告知用户。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSandbox {
    pub global: SandboxConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<SandboxConfig>,
    /// 合并后的最终生效值
    pub effective: SandboxConfig,
    /// 项目配置被收紧/忽略的说明（空 = 无冲突）
    pub notes: Vec<String>,
}

#[tauri::command]
pub async fn get_effective_sandbox(workspace_root: Option<String>) -> Result<EffectiveSandbox, String> {
    let global = get_sandbox_config().await?;
    let project = match workspace_root.as_deref().filter(|s| !s.is_empty()) {
        Some(root) => read_sandbox_file(&project_sandbox_config_path(root))?,
        None => None,
    };

    let mut notes = Vec::new();
    let mut effective = global.clone();

    if let Some(p) = &project {
        // 全局关着 → 项目配置不生效
        if !global.enabled {
            if p.enabled {
                notes.push(
                    "项目启用了沙箱，但全局未启用——全局是总闸，项目配置不生效".to_string(),
                );
            }
        } else if p.enabled {
            // 出网模式只能收紧
            if network_rank(&p.network_access) > network_rank(&global.network_access) {
                notes.push(format!(
                    "项目 networkAccess '{}' 比全局 '{}' 更宽松，已忽略（项目只能收紧）",
                    p.network_access, global.network_access
                ));
            } else {
                effective.network_access = p.network_access.clone();
            }
            // 全局开的 mcpSandboxing 不能被项目关掉
            if global.mcp_sandboxing && !p.mcp_sandboxing {
                notes.push("项目关闭了 mcpSandboxing，但全局已开启，已忽略（项目只能收紧）".to_string());
            } else {
                effective.mcp_sandboxing = p.mcp_sandboxing;
            }
        }
    }

    Ok(EffectiveSandbox {
        global,
        project,
        effective,
        notes,
    })
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
    save_sandbox_config_at(sandbox_config_path().ok_or("无法获取用户目录")?, config).await
}

/// 保存项目级 sandbox.json（`<workspace-root>/.kiro/settings/sandbox.json`）。
///
/// 项目配置只能收紧：这里不校验，交给 `get_effective_sandbox` 在读取时
/// 按全局值裁剪并给出 notes——写入侧保留用户原值，用户才能看到自己写了什么。
#[tauri::command]
pub async fn save_project_sandbox_config(
    workspace_root: String,
    config: SandboxConfig,
) -> Result<(), String> {
    if workspace_root.trim().is_empty() {
        return Err("workspace_root 不能为空".to_string());
    }
    save_sandbox_config_at(project_sandbox_config_path(&workspace_root), config).await
}

async fn save_sandbox_config_at(path: PathBuf, config: SandboxConfig) -> Result<(), String> {
    if !matches!(
        config.network_access.as_str(),
        "DEFAULT_BLOCKED" | "COMMON_DEPENDENCIES" | "DEFAULT_ALLOWED"
    ) {
        return Err(format!("非法的 networkAccess: {}", config.network_access));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
    }

    // 读-改-写而不是整体覆写：sandbox.json 是 IDE 与 CLI 共用的文件，
    // 直接 to_string_pretty(&config) 会把本面板不认识的键（IDE 后续新增的）全部抹掉。
    // 只覆盖我们管理的那三个键，其余原样保留。
    let mut object = match fs::read_to_string(&path) {
        Ok(raw) if !raw.trim().is_empty() => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(serde_json::Value::Object(map)) => map,
            Ok(_) => {
                return Err("sandbox.json 顶层不是对象，拒绝覆盖（请检查文件内容）".to_string())
            }
            Err(e) => return Err(format!("解析现有 sandbox.json 失败，拒绝覆盖: {e}")),
        },
        // 文件不存在或为空：从空对象开始
        _ => serde_json::Map::new(),
    };
    let managed = serde_json::to_value(&config).map_err(|e| format!("序列化失败: {e}"))?;
    if let serde_json::Value::Object(managed) = managed {
        for (key, value) in managed {
            object.insert(key, value);
        }
    }

    let content = serde_json::to_string_pretty(&serde_json::Value::Object(object))
        .map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&path, content + "\n").map_err(|e| format!("写入 sandbox.json 失败: {e}"))
}
