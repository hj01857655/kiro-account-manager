// Kiro IDE 设置命令 (读写 Kiro IDE 的 settings.json)

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数
#![allow(clippy::too_many_lines)] // 设置命令文件包含多个函数

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_SAFE_TRUSTED_COMMANDS: &[&str] = &[
    "npm run *",
    "npm test *",
    "pnpm run *",
    "pnpm test *",
    "yarn run *",
    "yarn test *",
    "bun run *",
    "bun test *",
    "cargo check *",
    "cargo test *",
    "cargo build *",
    "cargo clippy *",
    "cargo fmt *",
    "git status",
    "git diff *",
    "git log *",
    "git show *",
    "git branch *",
    "git rev-parse *",
    "cat *",
    "ls *",
    "dir *",
    "pwd",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)] // 设置结构体需要多个布尔字段来表示不同的开关选项
pub struct KiroSettings {
    pub http_proxy: Option<String>,
    pub model_selection: Option<String>,
    pub enable_codebase_indexing: bool,
    pub trusted_commands_mode: Option<String>,
    pub custom_trusted_commands: Option<String>,
    // Agent 设置
    pub agent_autonomy: Option<String>,
    pub enable_tab_autocomplete: bool,
    pub usage_summary: bool,
    pub code_references: bool,
    pub enable_debug_logs: bool,
    // 通知设置
    pub notify_action_required: bool,
    pub notify_failure: bool,
    pub notify_success: bool,
    pub notify_billing: bool,
    // 新增设置
    pub trusted_tools: Vec<String>,
    pub reference_tracker: bool,
    pub configure_mcp: String, // "Enabled" | "Disabled"
    // 遥测设置
    pub telemetry_content_collection: bool,
    pub telemetry_usage_analytics: bool,
    pub telemetry_edit_stats: bool,
    pub telemetry_feedback: bool,
}

impl Default for KiroSettings {
    fn default() -> Self {
        Self {
            http_proxy: None,
            model_selection: Some("claude-sonnet-4.5".to_string()),
            enable_codebase_indexing: true,
            trusted_commands_mode: Some("none".to_string()),
            custom_trusted_commands: None,
            agent_autonomy: Some("Supervised".to_string()),
            enable_tab_autocomplete: true,
            usage_summary: true,
            code_references: true,
            enable_debug_logs: false,
            notify_action_required: true,
            notify_failure: true,
            notify_success: true,
            notify_billing: true,
            trusted_tools: vec![],
            reference_tracker: false,
            configure_mcp: "Enabled".to_string(),
            telemetry_content_collection: false,
            telemetry_usage_analytics: false,
            telemetry_edit_stats: false,
            telemetry_feedback: false,
        }
    }
}

fn get_kiro_settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata).join("Kiro").join("User").join("settings.json")
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

fn get_kiro_settings_inner() -> Result<KiroSettings, String> {
    let path = get_kiro_settings_path()
        .ok_or("无法获取 Kiro 设置路径")?;
    
    if !path.exists() {
        return Ok(KiroSettings::default());
    }
    
    // 读取 app-settings.json（首次启动会使用默认值）
    let app_settings = super::app_settings_cmd::get_app_settings_inner().unwrap_or_default();
    
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取设置文件失败: {e}"))?;
    
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析设置文件失败: {e}"))?;
    
    let mut ide_modified = false;
    
    // 核心逻辑：以 app-settings.json 为准，强制同步到 Kiro IDE
    // 首次启动时，app_settings 使用默认值，会将默认值写入 IDE
    
    // enableCodebaseIndexing
    let codebase_indexing = app_settings.enable_codebase_indexing.unwrap_or(true);
    if json.get("kiroAgent.enableCodebaseIndexing").and_then(serde_json::Value::as_bool) != Some(codebase_indexing) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.enableCodebaseIndexing".to_string(), serde_json::Value::Bool(codebase_indexing));
            ide_modified = true;
        }
    }
    
    // enableTabAutocomplete
    let tab_autocomplete = app_settings.enable_tab_autocomplete.unwrap_or(true);
    if json.get("kiroAgent.enableTabAutocomplete").and_then(serde_json::Value::as_bool) != Some(tab_autocomplete) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.enableTabAutocomplete".to_string(), serde_json::Value::Bool(tab_autocomplete));
            ide_modified = true;
        }
    }
    
    // usageSummary
    let usage_summary = app_settings.usage_summary.unwrap_or(true);
    if json.get("kiroAgent.usageSummary").and_then(serde_json::Value::as_bool) != Some(usage_summary) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.usageSummary".to_string(), serde_json::Value::Bool(usage_summary));
            ide_modified = true;
        }
    }
    
    // codeReferences
    let code_references = app_settings.code_references.unwrap_or(true);
    if json.get("kiroAgent.codeReferences").and_then(serde_json::Value::as_bool) != Some(code_references) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.codeReferences".to_string(), serde_json::Value::Bool(code_references));
            ide_modified = true;
        }
    }
    
    // enableDebugLogs
    let debug_logs = app_settings.enable_debug_logs.unwrap_or(false);
    if json.get("kiroAgent.enableDebugLogs").and_then(serde_json::Value::as_bool) != Some(debug_logs) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.enableDebugLogs".to_string(), serde_json::Value::Bool(debug_logs));
            ide_modified = true;
        }
    }
    
    // notifyActionRequired
    let notify_action = app_settings.notify_action_required.unwrap_or(true);
    if json.get("kiroAgent.notifications.agent.actionRequired").and_then(serde_json::Value::as_bool) != Some(notify_action) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.notifications.agent.actionRequired".to_string(), serde_json::Value::Bool(notify_action));
            ide_modified = true;
        }
    }
    
    // notifyFailure
    let notify_failure = app_settings.notify_failure.unwrap_or(true);
    if json.get("kiroAgent.notifications.agent.failure").and_then(serde_json::Value::as_bool) != Some(notify_failure) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.notifications.agent.failure".to_string(), serde_json::Value::Bool(notify_failure));
            ide_modified = true;
        }
    }
    
    // notifySuccess
    let notify_success = app_settings.notify_success.unwrap_or(true);
    if json.get("kiroAgent.notifications.agent.success").and_then(serde_json::Value::as_bool) != Some(notify_success) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.notifications.agent.success".to_string(), serde_json::Value::Bool(notify_success));
            ide_modified = true;
        }
    }
    
    // notifyBilling
    let notify_billing = app_settings.notify_billing.unwrap_or(true);
    if json.get("kiroAgent.notifications.billing").and_then(serde_json::Value::as_bool) != Some(notify_billing) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.notifications.billing".to_string(), serde_json::Value::Bool(notify_billing));
            ide_modified = true;
        }
    }
    
    // trustedTools
    if let Some(ref app_tools) = app_settings.trusted_tools {
        let ide_tools: Vec<String> = json.get("kiroAgent.trustedTools")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|item| item.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if *app_tools != ide_tools {
            if let Some(obj) = json.as_object_mut() {
                obj.insert("kiroAgent.trustedTools".to_string(), serde_json::json!(app_tools));
                ide_modified = true;
            }
        }
    }

    // referenceTracker
    let reference_tracker = app_settings.reference_tracker.unwrap_or(false);
    if json.get("kiroAgent.codeReferences.referenceTracker").and_then(serde_json::Value::as_bool) != Some(reference_tracker) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("kiroAgent.codeReferences.referenceTracker".to_string(), serde_json::Value::Bool(reference_tracker));
            ide_modified = true;
        }
    }

    // configureMCP
    if let Some(ref mcp_mode) = app_settings.configure_mcp {
        let ide_mcp = json.get("kiroAgent.configureMCP").and_then(|v| v.as_str()).unwrap_or("Enabled");
        if mcp_mode != ide_mcp {
            if let Some(obj) = json.as_object_mut() {
                obj.insert("kiroAgent.configureMCP".to_string(), serde_json::Value::String(mcp_mode.clone()));
                ide_modified = true;
            }
        }
    }

    // telemetry: contentCollectionForServiceImprovement
    let tele_content = app_settings.telemetry_content_collection.unwrap_or(false);
    if json.get("telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement").and_then(serde_json::Value::as_bool) != Some(tele_content) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement".to_string(), serde_json::Value::Bool(tele_content));
            ide_modified = true;
        }
    }

    // telemetry: usageAnalyticsAndPerformanceMetrics
    let tele_usage = app_settings.telemetry_usage_analytics.unwrap_or(false);
    if json.get("telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics").and_then(serde_json::Value::as_bool) != Some(tele_usage) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics".to_string(), serde_json::Value::Bool(tele_usage));
            ide_modified = true;
        }
    }

    // telemetry: editStats
    let tele_edit = app_settings.telemetry_edit_stats.unwrap_or(false);
    if json.get("telemetry.editStats.enabled").and_then(serde_json::Value::as_bool) != Some(tele_edit) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("telemetry.editStats.enabled".to_string(), serde_json::Value::Bool(tele_edit));
            ide_modified = true;
        }
    }

    // telemetry: feedback
    let tele_feedback = app_settings.telemetry_feedback.unwrap_or(false);
    if json.get("telemetry.feedback.enabled").and_then(serde_json::Value::as_bool) != Some(tele_feedback) {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("telemetry.feedback.enabled".to_string(), serde_json::Value::Bool(tele_feedback));
            ide_modified = true;
        }
    }

    // 如果有修改，写入 Kiro IDE settings.json
    if ide_modified {
        let content = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("序列化设置失败: {e}"))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("写入设置文件失败: {e}"))?;
    }
    
    Ok(KiroSettings {
        http_proxy: json.get("http.proxy").and_then(|v| v.as_str()).map(std::string::ToString::to_string),
        model_selection: json.get("kiroAgent.modelSelection").and_then(|v| v.as_str()).map(std::string::ToString::to_string),
        enable_codebase_indexing: codebase_indexing,
        trusted_commands_mode: json.get("kiroAgent.trustedCommands")
            .and_then(|v| v.as_array())
            .map(|arr| {
                if arr.iter().any(|item| item.as_str() == Some("*")) {
                    "all".to_string()
                } else if arr.is_empty() {
                    "none".to_string()
                } else {
                    "common".to_string()
                }
            }),
        custom_trusted_commands: json.get("kiroAgent.trustedCommands")
            .and_then(|v| v.as_array())
            .filter(|arr| !arr.iter().any(|item| item.as_str() == Some("*")) && !arr.is_empty())
            .map(|arr| arr.iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
                .join("\n")),
        agent_autonomy: json.get("kiroAgent.agentAutonomy").and_then(|v| v.as_str()).map(std::string::ToString::to_string),
        enable_tab_autocomplete: tab_autocomplete,
        usage_summary,
        code_references,
        enable_debug_logs: debug_logs,
        notify_action_required: notify_action,
        notify_failure,
        notify_success,
        notify_billing,
        trusted_tools: app_settings.trusted_tools.unwrap_or_else(|| {
            json.get("kiroAgent.trustedTools")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|item| item.as_str().map(String::from)).collect())
                .unwrap_or_default()
        }),
        reference_tracker,
        configure_mcp: app_settings.configure_mcp.unwrap_or_else(|| {
            json.get("kiroAgent.configureMCP")
                .and_then(|v| v.as_str()).unwrap_or("Enabled").to_string()
        }),
        telemetry_content_collection: tele_content,
        telemetry_usage_analytics: tele_usage,
        telemetry_edit_stats: tele_edit,
        telemetry_feedback: tele_feedback,
    })
}

fn set_kiro_proxy_inner(proxy: String) -> Result<(), String> {
    let path = get_kiro_settings_path()
        .ok_or("无法获取 Kiro 设置路径")?;
    
    let mut settings: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取设置文件失败: {e}"))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    if let Some(obj) = settings.as_object_mut() {
        if proxy.is_empty() {
            // 清除代理时，必须把 proxySupport 设为 off，否则 Kiro 会尝试连接系统代理
            obj.remove("http.proxy");
            obj.insert("http.proxySupport".to_string(), serde_json::Value::String("off".to_string()));
        } else {
            // 设置代理时，proxySupport 必须为 on，同时提供代理地址
            obj.insert("http.proxy".to_string(), serde_json::Value::String(proxy));
            obj.insert("http.proxyStrictSSL".to_string(), serde_json::Value::Bool(false));
            obj.insert("http.proxySupport".to_string(), serde_json::Value::String("on".to_string()));
        }
    }
    
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化设置失败: {e}"))?;
    
    std::fs::write(&path, content)
        .map_err(|e| format!("写入设置文件失败: {e}"))?;
    
    Ok(())
}

fn set_kiro_model_inner(model: String) -> Result<(), String> {
    let path = get_kiro_settings_path()
        .ok_or("无法获取 Kiro 设置路径")?;
    
    let mut settings: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取设置文件失败: {e}"))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("kiroAgent.modelSelection".to_string(), serde_json::Value::String(model));
    }
    
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化设置失败: {e}"))?;
    
    std::fs::write(&path, content)
        .map_err(|e| format!("写入设置文件失败: {e}"))?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_kiro_settings() -> Result<KiroSettings, String> {
    tokio::task::spawn_blocking(get_kiro_settings_inner)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub async fn set_kiro_proxy(proxy: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_proxy_inner(proxy))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub async fn set_kiro_model(model: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_model_inner(model))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

fn set_kiro_codebase_indexing_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner("kiroAgent.enableCodebaseIndexing".to_string(), serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_codebase_indexing(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_codebase_indexing_inner(enabled))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

fn set_kiro_trusted_commands_inner(mode: String, custom_commands: Option<String>) -> Result<(), String> {
    let path = get_kiro_settings_path()
        .ok_or("无法获取 Kiro 设置路径")?;
    
    let mut settings: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取设置文件失败: {e}"))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    if let Some(obj) = settings.as_object_mut() {
        let commands = match mode.as_str() {
            "all" => serde_json::json!(["*"]),
            "common" => {
                // 如果有自定义命令，解析它；否则使用默认列表
                if let Some(ref custom) = custom_commands {
                    if custom.trim().is_empty() {
                        serde_json::json!(DEFAULT_SAFE_TRUSTED_COMMANDS)
                    } else {
                        let cmds: Vec<&str> = custom.lines()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .collect();
                        if cmds.iter().any(|cmd| *cmd == "*") {
                            return Err("common 模式不允许使用 *，如需全部信任请切换到“全部信任”".to_string());
                        }
                        serde_json::json!(cmds)
                    }
                } else {
                    serde_json::json!(DEFAULT_SAFE_TRUSTED_COMMANDS)
                }
            },
            "none" => serde_json::json!([]),
            _ => return Err(format!("不支持的 trusted commands 模式: {mode}")),
        };
        obj.insert("kiroAgent.trustedCommands".to_string(), commands);
    }
    
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化设置失败: {e}"))?;
    
    std::fs::write(&path, content)
        .map_err(|e| format!("写入设置文件失败: {e}"))?;
    
    Ok(())
}

#[tauri::command]
pub async fn set_kiro_trusted_commands(mode: String, custom_commands: Option<String>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_trusted_commands_inner(mode, custom_commands))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}


// 设置 Agent 自主模式
fn set_kiro_agent_autonomy_inner(autonomy: String) -> Result<(), String> {
    set_kiro_generic_inner("kiroAgent.agentAutonomy".to_string(), serde_json::json!(autonomy))
}

#[tauri::command]
pub async fn set_kiro_agent_autonomy(autonomy: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_agent_autonomy_inner(autonomy))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

// 设置 Tab 自动补全
fn set_kiro_tab_autocomplete_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner("kiroAgent.enableTabAutocomplete".to_string(), serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_tab_autocomplete(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_tab_autocomplete_inner(enabled))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

// 设置使用统计
fn set_kiro_usage_summary_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner("kiroAgent.usageSummary".to_string(), serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_usage_summary(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_usage_summary_inner(enabled))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

// 设置代码引用
fn set_kiro_code_references_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner("kiroAgent.codeReferences".to_string(), serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_code_references(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_code_references_inner(enabled))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

// 设置调试日志
fn set_kiro_debug_logs_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner("kiroAgent.enableDebugLogs".to_string(), serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_debug_logs(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_debug_logs_inner(enabled))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

// 设置通知选项
fn set_kiro_notification_inner(key: String, enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(key, serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_notification(key: String, enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_kiro_notification_inner(key, enabled))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

// ===== 通用设置写入 =====

/// 通用写入 Kiro IDE settings.json（支持 bool / string / string[] 类型）
/// 同时同步到 app-settings.json
fn set_kiro_generic_inner(key: String, value: serde_json::Value) -> Result<(), String> {
    let path = get_kiro_settings_path()
        .ok_or("无法获取 Kiro 设置路径")?;

    let mut settings: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取设置文件失败: {e}"))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = settings.as_object_mut() {
        obj.insert(key.clone(), value.clone());
    }

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化设置失败: {e}"))?;

    std::fs::write(&path, content)
        .map_err(|e| format!("写入设置文件失败: {e}"))?;

    // 同步到 app-settings.json
    sync_to_app_settings(&key, &value);

    Ok(())
}

/// 将 IDE 设置变更同步到 app-settings.json
fn sync_to_app_settings(key: &str, value: &serde_json::Value) {
    let mut app = super::app_settings_cmd::get_app_settings_inner().unwrap_or_default();
    match key {
        "kiroAgent.trustedTools" => {
            app.trusted_tools = value.as_array().map(|arr|
                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
            );
        }
        "kiroAgent.codeReferences.referenceTracker" => {
            app.reference_tracker = value.as_bool();
        }
        "kiroAgent.configureMCP" => {
            app.configure_mcp = value.as_str().map(String::from);
        }
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement" => {
            app.telemetry_content_collection = value.as_bool();
        }
        "telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics" => {
            app.telemetry_usage_analytics = value.as_bool();
        }
        "telemetry.editStats.enabled" => {
            app.telemetry_edit_stats = value.as_bool();
        }
        "telemetry.feedback.enabled" => {
            app.telemetry_feedback = value.as_bool();
        }
        "kiroAgent.enableCodebaseIndexing" => {
            app.enable_codebase_indexing = value.as_bool();
        }
        "kiroAgent.enableTabAutocomplete" => {
            app.enable_tab_autocomplete = value.as_bool();
        }
        "kiroAgent.usageSummary" => {
            app.usage_summary = value.as_bool();
        }
        "kiroAgent.codeReferences" => {
            app.code_references = value.as_bool();
        }
        "kiroAgent.enableDebugLogs" => {
            app.enable_debug_logs = value.as_bool();
        }
        "kiroAgent.notifications.agent.actionRequired" => {
            app.notify_action_required = value.as_bool();
        }
        "kiroAgent.notifications.agent.failure" => {
            app.notify_failure = value.as_bool();
        }
        "kiroAgent.notifications.agent.success" => {
            app.notify_success = value.as_bool();
        }
        "kiroAgent.notifications.billing" => {
            app.notify_billing = value.as_bool();
        }
        _ => return, // 不需要同步的 key
    }
    let _ = super::app_settings_cmd::save_settings_to_file(&app);
}

/// 设置 trustedTools（字符串数组）
#[tauri::command]
pub async fn set_kiro_trusted_tools(tools: Vec<String>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        set_kiro_generic_inner("kiroAgent.trustedTools".to_string(), serde_json::json!(tools))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// 设置 referenceTracker
#[tauri::command]
pub async fn set_kiro_reference_tracker(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        set_kiro_generic_inner("kiroAgent.codeReferences.referenceTracker".to_string(), serde_json::json!(enabled))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// 设置 configureMCP（"Enabled" / "Disabled"）
#[tauri::command]
pub async fn set_kiro_configure_mcp(mode: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        set_kiro_generic_inner("kiroAgent.configureMCP".to_string(), serde_json::json!(mode))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// 设置遥测选项（通用 bool，key 由前端传入）
#[tauri::command]
pub async fn set_kiro_telemetry(key: String, enabled: bool) -> Result<(), String> {
    // 白名单校验，防止任意 key 写入
    let allowed = [
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement",
        "telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics",
        "telemetry.editStats.enabled",
        "telemetry.feedback.enabled",
    ];
    if !allowed.contains(&key.as_str()) {
        return Err(format!("不允许的遥测 key: {key}"));
    }
    tokio::task::spawn_blocking(move || {
        set_kiro_generic_inner(key, serde_json::json!(enabled))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}
