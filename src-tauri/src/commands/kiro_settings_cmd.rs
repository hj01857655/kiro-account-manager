// Kiro IDE 设置命令 (读写 Kiro IDE 的 settings.json)

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数
#![allow(clippy::too_many_lines)] // 设置命令文件包含多个函数

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)] // 设置结构体需要多个布尔字段来表示不同的开关选项
pub struct KiroSettings {
    pub http_proxy: Option<String>,
    pub model_selection: Option<String>,
    pub enable_codebase_indexing: bool,
    // Agent 设置
    pub agent_autonomy: Option<String>,
    pub enable_tab_autocomplete: bool,
    pub usage_summary: bool,
    pub enable_debug_logs: bool,
    // 通知设置
    pub notify_action_required: bool,
    pub notify_failure: bool,
    pub notify_success: bool,
    pub notify_billing: bool,
    // 新增设置
    pub reference_tracker: bool,
    pub configure_mcp: String, // "Enabled" | "Disabled"
    // 遥测设置
    pub telemetry_content_collection: bool,
    pub telemetry_usage_analytics: bool,
    pub telemetry_edit_stats: bool,
    pub telemetry_feedback: bool,
    // === Kiro 1.0 Agent 高级设置 ===
    // 说明：与上面各键一样走「双向同步（IDE 优先 + 回写 app-settings.json）」，不再有
    // 透传/非透传之分。默认值对齐 Kiro 1.0 的 configuration 声明
    // （从 IDE 产物 workbench.desktop.main.js / kiro-agent 扩展抠出）。
    pub tool_card_display_mode: Option<String>, // "collapseOnComplete" | "alwaysExpanded"
    pub terminal_command_timeout: Option<i64>,  // 毫秒；None = 用 IDE 内置默认
    pub agent_ignore_files: Vec<String>,        // 忽略模式文件（如 .gitignore）
    pub artifacts_auto_open_panel: bool,        // 产出 artifact 时自动打开面板
    pub trust_default_pattern: Option<String>,  // "full" | "partial" | "base"
    pub trust_default_scope: Option<String>,    // "user" | "workspace" | "session"
    pub mcp_approved_env_vars: Vec<String>,     // 允许在 MCP 配置中展开的环境变量
    pub auto_approve_agent_commands: Vec<String>, // 自动批准的命令
    pub experiments_cloud_config: bool,
    pub experiments_workspace_manager: bool,
    pub editor_actions_prompts: Option<serde_json::Value>, // 自定义编辑器动作提示词（对象）
    // === 启动与遥测 扩展 ===
    // 说明：旧版只覆盖了 4 个遥测键，这里补齐 Kiro configuration 里剩余的遥测项与启动模式。
    pub telemetry_prompt_logging: bool, // 提示词日志上报
    pub telemetry_edit_stats_details: bool, // 编辑统计明细
    pub telemetry_edit_stats_decorations: bool, // 编辑器内编辑统计装饰
    pub telemetry_edit_stats_status_bar: bool, // 状态栏编辑统计
    pub startup_mode: Option<String>, // "code" | "agentFocus"
    // === 远程会话门户 ===
    // `kiroAgent.remoteSessions.endpoint`：Kiro 云门户（Cloud Portal）的 origin 基址。
    // 用途：会话来源为 remote 时，生成「在浏览器打开」的跳转链接
    //   `${endpoint}/session/${sessionId}`。
    // 注意：该键**未在 IDE 的 registerConfiguration 中声明**，属隐藏键，
    // 只能由 config.getValue() 消费；默认值 https://app.kiro.dev 由代码兜底
    // （IDE 会 trim 并去掉尾部斜杠，空值回落默认）。
    pub remote_sessions_endpoint: Option<String>,
}

impl Default for KiroSettings {
    fn default() -> Self {
        Self {
            http_proxy: None,
            model_selection: Some("claude-sonnet-4.5".to_string()),
            // 默认值对齐 Kiro 1.0 configuration 声明：enableCodebaseIndexing=false（实验特性）、
            // agentAutonomy=Autopilot、enableTabAutocomplete=false、notify.failure/success=false。
            enable_codebase_indexing: false,
            agent_autonomy: Some("Autopilot".to_string()),
            enable_tab_autocomplete: false,
            usage_summary: true,
            enable_debug_logs: false,
            notify_action_required: true,
            notify_failure: false,
            notify_success: false,
            notify_billing: true,
            reference_tracker: false,
            configure_mcp: "Enabled".to_string(),
            telemetry_content_collection: false,
            telemetry_usage_analytics: false,
            telemetry_edit_stats: false,
            telemetry_feedback: false,
            tool_card_display_mode: Some("collapseOnComplete".to_string()),
            terminal_command_timeout: None,
            agent_ignore_files: Vec::new(),
            artifacts_auto_open_panel: true,
            trust_default_pattern: Some("base".to_string()),
            trust_default_scope: Some("workspace".to_string()),
            mcp_approved_env_vars: Vec::new(),
            auto_approve_agent_commands: Vec::new(),
            experiments_cloud_config: false,
            experiments_workspace_manager: false,
            editor_actions_prompts: None,
            telemetry_prompt_logging: false,
            telemetry_edit_stats_details: false,
            telemetry_edit_stats_decorations: false,
            telemetry_edit_stats_status_bar: false,
            startup_mode: Some("code".to_string()),
            // 留空表示「用 IDE 内置默认」，不主动写盘该键
            remote_sessions_endpoint: None,
        }
    }
}

pub(crate) fn get_kiro_settings_path() -> Option<PathBuf> {
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

fn load_kiro_settings_json(path: &Path) -> Result<serde_json::Value, String> {
    if path.exists() {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("读取设置文件失败: {e}"))?;
        Ok(serde_json::from_str(&content).unwrap_or(serde_json::json!({})))
    } else {
        Ok(serde_json::json!({}))
    }
}

fn write_kiro_settings_json(path: &Path, settings: &serde_json::Value) -> Result<(), String> {
    let content =
        serde_json::to_string_pretty(settings).map_err(|e| format!("序列化设置失败: {e}"))?;

    std::fs::write(path, content).map_err(|e| format!("写入设置文件失败: {e}"))
}

async fn run_kiro_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

fn upsert_bool_if_changed(json: &mut serde_json::Value, key: &str, desired: bool) -> bool {
    if json.get(key).and_then(serde_json::Value::as_bool) == Some(desired) {
        return false;
    }

    if let Some(obj) = json.as_object_mut() {
        obj.insert(key.to_string(), serde_json::Value::Bool(desired));
        return true;
    }

    false
}

fn upsert_string_if_changed(json: &mut serde_json::Value, key: &str, desired: &str) -> bool {
    if json.get(key).and_then(serde_json::Value::as_str) == Some(desired) {
        return false;
    }

    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            key.to_string(),
            serde_json::Value::String(desired.to_string()),
        );
        return true;
    }

    false
}

fn get_string_value(json: &serde_json::Value, key: &str) -> Option<String> {
    json.get(key)
        .and_then(|value| value.as_str())
        .map(std::string::ToString::to_string)
}

fn get_bool_value(json: &serde_json::Value, key: &str) -> Option<bool> {
    json.get(key).and_then(serde_json::Value::as_bool)
}

fn get_i64_value(json: &serde_json::Value, key: &str) -> Option<i64> {
    json.get(key).and_then(serde_json::Value::as_i64)
}

/// 读取字符串数组（过滤非字符串项），用于 agentIgnoreFiles / mcpApprovedEnvVars 等。
///
/// 能区分「键不存在」(None) 与「空数组」(Some(vec![]))——双向同步需要这个区分：
/// 键不存在时要补写，存在空数组时说明用户就是要空。
fn get_string_array_opt(json: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    json.get(key).and_then(|value| value.as_array()).map(|items| {
        items
            .iter()
            .filter_map(|item| item.as_str().map(String::from))
            .collect()
    })
}

/// 规范化远程门户 endpoint：与 Kiro IDE 的 `Cro()` 行为保持一致——
/// 先 trim，再去掉末尾的全部 `/`。
///
/// IDE 侧另有「空值回落 `https://app.kiro.dev`」的兜底，本函数只做规范化，
/// 是否回落默认由各调用处决定（读取处兜底展示、保存处视为清空设置）。
fn normalize_endpoint(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn get_kiro_settings_inner() -> Result<KiroSettings, String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    if !path.exists() {
        return Ok(KiroSettings::default());
    }

    // 读取 app-settings.json（首次启动会使用默认值）
    let mut app_settings = super::app_settings_cmd::get_app_settings_inner().unwrap_or_default();

    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取设置文件失败: {e}"))?;

    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析设置文件失败: {e}"))?;

    let mut ide_modified = false;
    let mut app_dirty = false;

    // === 双向同步核心逻辑（以 IDE settings.json 为准）===
    //  - IDE 有该键 -> 以 IDE 的值为准，与 app-settings 不一致时回写 app-settings；
    //  - IDE 缺该键 -> 用 app-settings（或内置默认值）补写进 IDE。
    // 效果：手动编辑 settings.json 会被 app 采纳；在 app 里改动会同时落到两个文件。
    macro_rules! sync2way_bool {
        ($field:ident, $key:expr, $default:expr) => {{
            let ide = get_bool_value(&json, $key);
            let value = ide.unwrap_or_else(|| app_settings.$field.unwrap_or($default));
            if ide.is_none() && upsert_bool_if_changed(&mut json, $key, value) {
                ide_modified = true;
            }
            if app_settings.$field != Some(value) {
                app_settings.$field = Some(value);
                app_dirty = true;
            }
            value
        }};
    }

    macro_rules! sync2way_string {
        ($field:ident, $key:expr, $default:expr) => {{
            let ide = get_string_value(&json, $key);
            let value = ide
                .clone()
                .or_else(|| app_settings.$field.clone())
                .unwrap_or_else(|| $default.to_string());
            if ide.is_none() && upsert_string_if_changed(&mut json, $key, &value) {
                ide_modified = true;
            }
            if app_settings.$field.as_deref() != Some(value.as_str()) {
                app_settings.$field = Some(value.clone());
                app_dirty = true;
            }
            value
        }};
    }

    // 无内置默认值的字符串键：两边都没有时保持「键不存在」，不凭空造键
    macro_rules! sync2way_string_opt {
        ($field:ident, $key:expr) => {{
            let ide = get_string_value(&json, $key);
            let value = ide.clone().or_else(|| app_settings.$field.clone());
            if ide.is_none() {
                if let Some(v) = &value {
                    if upsert_string_if_changed(&mut json, $key, v) {
                        ide_modified = true;
                    }
                }
            }
            if app_settings.$field != value {
                app_settings.$field = value.clone();
                app_dirty = true;
            }
            value
        }};
    }

    macro_rules! sync2way_list {
        ($field:ident, $key:expr) => {{
            let ide = get_string_array_opt(&json, $key);
            let value = ide
                .clone()
                .or_else(|| app_settings.$field.clone())
                .unwrap_or_default();
            if ide.is_none() {
                let desired = serde_json::Value::Array(
                    value
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                );
                if json.get($key) != Some(&desired) {
                    if let Some(obj) = json.as_object_mut() {
                        obj.insert($key.to_string(), desired);
                        ide_modified = true;
                    }
                }
            }
            if app_settings.$field.as_ref() != Some(&value) {
                app_settings.$field = Some(value.clone());
                app_dirty = true;
            }
            value
        }};
    }

    // --- 代理与模型（同在 settings.json 里，一并纳入双向同步）---
    let http_proxy = sync2way_string_opt!(http_proxy, "http.proxy");
    let model_selection = sync2way_string_opt!(model_selection, "kiroAgent.modelSelection");

    // --- kiroAgent.* ---
    let codebase_indexing = sync2way_bool!(
        enable_codebase_indexing,
        "kiroAgent.enableCodebaseIndexing",
        false
    );
    let tab_autocomplete = sync2way_bool!(
        enable_tab_autocomplete,
        "kiroAgent.enableTabAutocomplete",
        false
    );
    let usage_summary = sync2way_bool!(usage_summary, "kiroAgent.usageSummary", true);
    let debug_logs = sync2way_bool!(enable_debug_logs, "kiroAgent.enableDebugLogs", false);
    let notify_action = sync2way_bool!(
        notify_action_required,
        "kiroAgent.notifications.agent.actionRequired",
        true
    );
    let notify_failure = sync2way_bool!(
        notify_failure,
        "kiroAgent.notifications.agent.failure",
        false
    );
    let notify_success = sync2way_bool!(
        notify_success,
        "kiroAgent.notifications.agent.success",
        false
    );
    let notify_billing = sync2way_bool!(notify_billing, "kiroAgent.notifications.billing", true);
    let reference_tracker = sync2way_bool!(
        reference_tracker,
        "kiroAgent.codeReferences.referenceTracker",
        false
    );
    let agent_autonomy = sync2way_string!(agent_autonomy, "kiroAgent.agentAutonomy", "Autopilot");
    let configure_mcp = sync2way_string!(configure_mcp, "kiroAgent.configureMCP", "Enabled");
    let tool_card_display_mode = sync2way_string!(
        tool_card_display_mode,
        "kiroAgent.toolCardDisplayMode",
        "collapseOnComplete"
    );
    let trust_default_pattern =
        sync2way_string!(trust_default_pattern, "kiroAgent.trust.defaultPattern", "base");
    let trust_default_scope = sync2way_string!(
        trust_default_scope,
        "kiroAgent.trust.defaultScope",
        "workspace"
    );
    let artifacts_auto_open_panel = sync2way_bool!(
        artifacts_auto_open_panel,
        "kiroAgent.artifacts.autoOpenPanel",
        true
    );
    let experiments_cloud_config = sync2way_bool!(
        experiments_cloud_config,
        "kiroAgent.experiments.cloudConfig",
        false
    );
    let experiments_workspace_manager = sync2way_bool!(
        experiments_workspace_manager,
        "kiroAgent.experiments.workspaceManager",
        false
    );

    // remoteSessions.endpoint：隐藏键，IDE 侧默认 https://app.kiro.dev。
    // 刻意**不**把默认值回写 settings.json：该键在 IDE 配置声明中不存在，
    // 凭空写入只会污染用户配置，且 IDE 对空值本就回落默认。
    // 故此处仅做「读 + 展示兜底」：未设置时给 UI 默认值，但不落盘。
    const REMOTE_SESSIONS_DEFAULT: &str = "https://app.kiro.dev";
    let remote_sessions_endpoint = {
        let ide = get_string_value(&json, "kiroAgent.remoteSessions.endpoint")
            .map(|s| normalize_endpoint(&s))
            .filter(|s| !s.is_empty());
        // 仅在用户/IDE 确有值时才同步进 app-settings，None 表示未设置
        let value = ide.or_else(|| app_settings.remote_sessions_endpoint.clone());
        if app_settings.remote_sessions_endpoint != value {
            app_settings.remote_sessions_endpoint = value.clone();
            app_dirty = true;
        }
        // 返回给 UI 的值：未设置时展示 IDE 默认，便于用户知道当前生效目标
        Some(value.unwrap_or_else(|| REMOTE_SESSIONS_DEFAULT.to_string()))
    };
    let agent_ignore_files = sync2way_list!(agent_ignore_files, "kiroAgent.agentIgnoreFiles");
    let mcp_approved_env_vars =
        sync2way_list!(mcp_approved_env_vars, "kiroAgent.mcpApprovedEnvVars");
    let auto_approve_agent_commands =
        sync2way_list!(auto_approve_agent_commands, "kiroAgent.autoApproveAgentCommands");

    // terminalCommandTimeout：数字且无内置默认——两边都没有时保持缺失（不凭空造键）
    let terminal_command_timeout = {
        let ide = get_i64_value(&json, "kiroAgent.terminalCommandTimeout");
        let value = ide.or(app_settings.terminal_command_timeout);
        if ide.is_none() {
            if let Some(v) = value {
                if let Some(obj) = json.as_object_mut() {
                    obj.insert(
                        "kiroAgent.terminalCommandTimeout".to_string(),
                        serde_json::json!(v),
                    );
                    ide_modified = true;
                }
            }
        }
        if app_settings.terminal_command_timeout != value {
            app_settings.terminal_command_timeout = value;
            app_dirty = true;
        }
        value
    };

    // editorActions.prompts：对象，双向同步
    let editor_actions_prompts = {
        let ide = json.get("kiroAgent.editorActions.prompts").cloned();
        let value = ide
            .clone()
            .or_else(|| app_settings.editor_actions_prompts.clone());
        if ide.is_none() {
            if let Some(v) = &value {
                if let Some(obj) = json.as_object_mut() {
                    obj.insert("kiroAgent.editorActions.prompts".to_string(), v.clone());
                    ide_modified = true;
                }
            }
        }
        if app_settings.editor_actions_prompts != value {
            app_settings.editor_actions_prompts = value.clone();
            app_dirty = true;
        }
        value
    };

    // --- telemetry.* ---
    let tele_content = sync2way_bool!(
        telemetry_content_collection,
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement",
        false
    );
    let tele_usage = sync2way_bool!(
        telemetry_usage_analytics,
        "telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics",
        false
    );
    let tele_edit = sync2way_bool!(telemetry_edit_stats, "telemetry.editStats.enabled", false);
    let tele_feedback = sync2way_bool!(telemetry_feedback, "telemetry.feedback.enabled", false);
    let tele_prompt_logging = sync2way_bool!(
        telemetry_prompt_logging,
        "telemetry.dataSharingAndPromptLogging.promptLogging",
        false
    );
    let tele_edit_details = sync2way_bool!(
        telemetry_edit_stats_details,
        "telemetry.editStats.details.enabled",
        false
    );
    let tele_edit_decorations = sync2way_bool!(
        telemetry_edit_stats_decorations,
        "telemetry.editStats.showDecorations",
        false
    );
    let tele_edit_status_bar = sync2way_bool!(
        telemetry_edit_stats_status_bar,
        "telemetry.editStats.showStatusBar",
        false
    );

    // --- kiro.* ---
    let startup_mode = sync2way_string!(startup_mode, "kiro.startupMode", "code");

    // 两边都对齐后落盘：IDE 有补写就写 settings.json，app-settings 有变化就写回
    if ide_modified {
        write_kiro_settings_json(&path, &json)?;
    }
    if app_dirty {
        let _ = super::app_settings_cmd::save_settings_to_file(&app_settings);
    }

    Ok(KiroSettings {
        http_proxy,
        model_selection,
        enable_codebase_indexing: codebase_indexing,
        agent_autonomy: Some(agent_autonomy),
        enable_tab_autocomplete: tab_autocomplete,
        usage_summary,
        enable_debug_logs: debug_logs,
        notify_action_required: notify_action,
        notify_failure,
        notify_success,
        notify_billing,
        reference_tracker,
        configure_mcp,
        telemetry_content_collection: tele_content,
        telemetry_usage_analytics: tele_usage,
        telemetry_edit_stats: tele_edit,
        telemetry_feedback: tele_feedback,
        telemetry_prompt_logging: tele_prompt_logging,
        telemetry_edit_stats_details: tele_edit_details,
        telemetry_edit_stats_decorations: tele_edit_decorations,
        telemetry_edit_stats_status_bar: tele_edit_status_bar,
        tool_card_display_mode: Some(tool_card_display_mode),
        terminal_command_timeout,
        agent_ignore_files,
        artifacts_auto_open_panel,
        trust_default_pattern: Some(trust_default_pattern),
        trust_default_scope: Some(trust_default_scope),
        mcp_approved_env_vars,
        auto_approve_agent_commands,
        experiments_cloud_config,
        experiments_workspace_manager,
        editor_actions_prompts,
        startup_mode: Some(startup_mode),
        remote_sessions_endpoint,
    })
}

fn set_kiro_proxy_inner(proxy: String) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        if proxy.is_empty() {
            // 清除代理 = 回到「未手动设置代理」的原始状态，三个键都要还原到 IDE 内置默认：
            // - http.proxy：删除键（IDE 声明该键无默认值，缺失即为未设置）；
            // - http.proxySupport：还原 IDE 默认 "override"（允许走系统代理）。
            //   此前写死 "off"，会让该键永久停在非默认值上，即使代理已清空也不再使用系统代理；
            // - http.proxyStrictSSL：还原 IDE 默认 true。设置代理时被改成 false，
            //   若不还原会永久残留，导致后续任何代理连接都关闭 SSL 校验。
            obj.remove("http.proxy");
            obj.insert(
                "http.proxySupport".to_string(),
                serde_json::Value::String("override".to_string()),
            );
            obj.insert(
                "http.proxyStrictSSL".to_string(),
                serde_json::Value::Bool(true),
            );
        } else {
            // 设置代理时，proxySupport 必须为 on，同时提供代理地址
            obj.insert(
                "http.proxy".to_string(),
                serde_json::Value::String(proxy.clone()),
            );
            obj.insert(
                "http.proxyStrictSSL".to_string(),
                serde_json::Value::Bool(false),
            );
            obj.insert(
                "http.proxySupport".to_string(),
                serde_json::Value::String("on".to_string()),
            );
        }
    }

    write_kiro_settings_json(&path, &settings)?;

    // 同步到 app-settings.json；清空代理时写 null，让 app 侧也一并清掉，
    // 否则下次加载会把旧代理「补写」回 IDE。
    let synced = if proxy.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(proxy)
    };
    sync_to_app_settings("http.proxy", &synced);

    Ok(())
}

fn set_kiro_model_inner(model: String) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        obj.insert(
            "kiroAgent.modelSelection".to_string(),
            serde_json::Value::String(model.clone()),
        );
    }

    write_kiro_settings_json(&path, &settings)?;
    // 同步到 app-settings.json（双向同步的写方向）
    sync_to_app_settings("kiroAgent.modelSelection", &serde_json::json!(model));
    Ok(())
}

#[tauri::command]
pub async fn get_kiro_settings() -> Result<KiroSettings, String> {
    run_kiro_blocking(get_kiro_settings_inner).await
}

/// 纯读取云配置开关，**不触发双向同步写回**。
///
/// KiroConfig 面板需要在进入时判断「云配置是否开启」以挂只读横幅并禁用写操作，
/// 若复用 `get_kiro_settings` 会顺带把缺失键写回 settings.json（污染用户配置），
/// 故单独提供一个只读查询。判定依据：`kiroAgent.experiments.cloudConfig` 为 true。
fn read_cloud_config_enabled_inner() -> Result<bool, String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;
    if !path.exists() {
        return Ok(false);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取设置文件失败: {e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析设置文件失败: {e}"))?;
    Ok(json
        .get("kiroAgent")
        .and_then(|k| k.get("experiments"))
        .and_then(|e| e.get("cloudConfig"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

#[tauri::command]
pub async fn read_cloud_config_enabled() -> Result<bool, String> {
    run_kiro_blocking(read_cloud_config_enabled_inner).await
}

/// 读取会覆盖 `kiroAgent.remoteSessions.endpoint` 的环境变量。
///
/// Kiro 扩展侧（`kiro.kiro-agent`）解析远程会话门户基址时是三级回落：
/// ```text
/// process.env.KIRO_REMOTE_SESSIONS_ENDPOINT  >  settings.json 配置项  >  内置默认
/// ```
/// 即**环境变量优先级高于用户在设置里填的值**。由于该键未在 IDE 的
/// `registerConfiguration` 中声明，用户既看不到它、也不知道有环境变量这一层，
/// 一旦命中就会表现为「改了配置但不生效，且毫无线索」。
///
/// 故单独暴露一个只读查询，供设置面板判断是否需要提示「当前值被环境变量覆盖」。
///
/// 返回：`Some(值)` = 环境变量存在且 trim 后非空（即为实际生效来源）；
///       `None` = 未设置，配置项正常生效。
#[tauri::command]
pub async fn read_remote_sessions_env_override() -> Result<Option<String>, String> {
    // 与扩展侧 `eUt()` 的判定保持一致：先 trim，空串视为未设置
    Ok(std::env::var("KIRO_REMOTE_SESSIONS_ENDPOINT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty()))
}

#[tauri::command]
pub async fn set_kiro_proxy(proxy: String) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_proxy_inner(proxy)).await
}

#[tauri::command]
pub async fn set_kiro_model(model: String) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_model_inner(model)).await
}

fn set_kiro_codebase_indexing_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.enableCodebaseIndexing".to_string(),
        serde_json::json!(enabled),
    )
}

#[tauri::command]
pub async fn set_kiro_codebase_indexing(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_codebase_indexing_inner(enabled)).await
}

// 设置 Agent 自主模式
fn set_kiro_agent_autonomy_inner(autonomy: String) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.agentAutonomy".to_string(),
        serde_json::json!(autonomy),
    )
}

#[tauri::command]
pub async fn set_kiro_agent_autonomy(autonomy: String) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_agent_autonomy_inner(autonomy)).await
}

// 设置 Tab 自动补全
fn set_kiro_tab_autocomplete_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.enableTabAutocomplete".to_string(),
        serde_json::json!(enabled),
    )
}

#[tauri::command]
pub async fn set_kiro_tab_autocomplete(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_tab_autocomplete_inner(enabled)).await
}

// 设置使用统计
fn set_kiro_usage_summary_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.usageSummary".to_string(),
        serde_json::json!(enabled),
    )
}

#[tauri::command]
pub async fn set_kiro_usage_summary(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_usage_summary_inner(enabled)).await
}

// 设置调试日志
fn set_kiro_debug_logs_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.enableDebugLogs".to_string(),
        serde_json::json!(enabled),
    )
}

#[tauri::command]
pub async fn set_kiro_debug_logs(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_debug_logs_inner(enabled)).await
}

// 设置通知选项
fn set_kiro_notification_inner(key: String, enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(key, serde_json::json!(enabled))
}

#[tauri::command]
pub async fn set_kiro_notification(key: String, enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_notification_inner(key, enabled)).await
}

// ===== 通用设置写入 =====

/// 通用写入 Kiro IDE settings.json（支持 bool / string / string[] 类型）
/// 同时同步到 app-settings.json
fn set_kiro_generic_inner(key: String, value: serde_json::Value) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        if value.is_null() {
            // null 表示清除该键（例如 terminalCommandTimeout 恢复为 IDE 内置默认）
            obj.remove(&key);
        } else {
            obj.insert(key.clone(), value.clone());
        }
    }

    write_kiro_settings_json(&path, &settings)?;

    // 同步到 app-settings.json
    sync_to_app_settings(&key, &value);

    Ok(())
}

/// 把 JSON 数组转成 `Option<Vec<String>>`（null / 非数组 -> None，用于表示「该键已删除」）。
fn as_string_list(value: &serde_json::Value) -> Option<Vec<String>> {
    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(|item| item.as_str().map(String::from))
            .collect()
    })
}

/// 将 IDE 设置变更同步到 app-settings.json。
///
/// 与 `get_kiro_settings_inner` 的反向同步配对，构成「双向同步」的写方向：
/// app 侧任何写操作都经由 `set_kiro_generic_inner` 落到 IDE，再在这里镜像回 app-settings，
/// 保证两个文件始终一致。`value` 为 null 时对应字段置 None（表示键已删除）。
fn sync_to_app_settings(key: &str, value: &serde_json::Value) {
    let mut app = super::app_settings_cmd::get_app_settings_inner().unwrap_or_default();
    match key {
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
        // === Kiro 1.0 Agent 高级设置（原先只写 IDE，现在一并镜像到 app-settings）===
        "kiroAgent.agentAutonomy" => {
            app.agent_autonomy = value.as_str().map(String::from);
        }
        "kiroAgent.toolCardDisplayMode" => {
            app.tool_card_display_mode = value.as_str().map(String::from);
        }
        "kiroAgent.terminalCommandTimeout" => {
            app.terminal_command_timeout = value.as_i64();
        }
        "kiroAgent.agentIgnoreFiles" => {
            app.agent_ignore_files = as_string_list(value);
        }
        "kiroAgent.artifacts.autoOpenPanel" => {
            app.artifacts_auto_open_panel = value.as_bool();
        }
        "kiroAgent.trust.defaultPattern" => {
            app.trust_default_pattern = value.as_str().map(String::from);
        }
        "kiroAgent.trust.defaultScope" => {
            app.trust_default_scope = value.as_str().map(String::from);
        }
        "kiroAgent.mcpApprovedEnvVars" => {
            app.mcp_approved_env_vars = as_string_list(value);
        }
        "kiroAgent.autoApproveAgentCommands" => {
            app.auto_approve_agent_commands = as_string_list(value);
        }
        "kiroAgent.experiments.cloudConfig" => {
            app.experiments_cloud_config = value.as_bool();
        }
        "kiroAgent.experiments.workspaceManager" => {
            app.experiments_workspace_manager = value.as_bool();
        }
        "kiroAgent.editorActions.prompts" => {
            app.editor_actions_prompts = if value.is_null() {
                None
            } else {
                Some(value.clone())
            };
        }
        // === 模型与代理（同在 settings.json 里）===
        "kiroAgent.modelSelection" => {
            app.model_selection = value.as_str().map(String::from);
        }
        "http.proxy" => {
            app.http_proxy = value.as_str().map(String::from);
        }
        // === 启动与遥测 扩展 ===
        "kiro.startupMode" => {
            app.startup_mode = value.as_str().map(String::from);
        }
        // 远程会话门户基址；null 或空串视为「清空设置」（IDE 将回落默认 http://app.kiro.dev）
        "kiroAgent.remoteSessions.endpoint" => {
            app.remote_sessions_endpoint = value
                .as_str()
                .map(normalize_endpoint)
                .filter(|s| !s.is_empty());
        }
        "telemetry.dataSharingAndPromptLogging.promptLogging" => {
            app.telemetry_prompt_logging = value.as_bool();
        }
        "telemetry.editStats.details.enabled" => {
            app.telemetry_edit_stats_details = value.as_bool();
        }
        "telemetry.editStats.showDecorations" => {
            app.telemetry_edit_stats_decorations = value.as_bool();
        }
        "telemetry.editStats.showStatusBar" => {
            app.telemetry_edit_stats_status_bar = value.as_bool();
        }
        _ => return, // 不需要同步的 key
    }
    let _ = super::app_settings_cmd::save_settings_to_file(&app);
}

/// 设置 referenceTracker
#[tauri::command]
pub async fn set_kiro_reference_tracker(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || {
        set_kiro_generic_inner(
            "kiroAgent.codeReferences.referenceTracker".to_string(),
            serde_json::json!(enabled),
        )
    })
    .await
}

/// 设置 configureMCP（"Enabled" / "Disabled"）
#[tauri::command]
pub async fn set_kiro_configure_mcp(mode: String) -> Result<(), String> {
    run_kiro_blocking(move || {
        set_kiro_generic_inner(
            "kiroAgent.configureMCP".to_string(),
            serde_json::json!(mode),
        )
    })
    .await
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
        "telemetry.dataSharingAndPromptLogging.promptLogging",
        "telemetry.editStats.details.enabled",
        "telemetry.editStats.showDecorations",
        "telemetry.editStats.showStatusBar",
    ];
    if !allowed.contains(&key.as_str()) {
        return Err(format!("不允许的遥测 key: {key}"));
    }
    run_kiro_blocking(move || set_kiro_generic_inner(key, serde_json::json!(enabled))).await
}

/// 设置 Kiro IDE 设置项（key 由前端传入）。
///
/// 写入 settings.json 后会经 `sync_to_app_settings` 镜像回 app-settings.json（双向同步）。
/// 仅允许白名单内的键，避免任意写入；`value` 为 null 时删除该键（恢复 IDE 内置默认）。
#[tauri::command]
pub async fn set_kiro_agent_setting(key: String, value: serde_json::Value) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "kiroAgent.toolCardDisplayMode",
        "kiroAgent.terminalCommandTimeout",
        "kiroAgent.agentIgnoreFiles",
        "kiroAgent.artifacts.autoOpenPanel",
        "kiroAgent.trust.defaultPattern",
        "kiroAgent.trust.defaultScope",
        "kiroAgent.mcpApprovedEnvVars",
        "kiroAgent.autoApproveAgentCommands",
        "kiroAgent.experiments.cloudConfig",
        "kiroAgent.experiments.workspaceManager",
        "kiroAgent.editorActions.prompts",
        "kiro.startupMode",
        // 隐藏键（未在 IDE registerConfiguration 中声明），但确有实际作用：
        // 决定「在浏览器打开远程会话」的跳转地址
        "kiroAgent.remoteSessions.endpoint",
        // Memory 四档模式（{mode: read_write|read_only|disabled, reflection}），
        // IDE 的 Memory 设置键；默认 Off 时键不存在，记忆面板经此通道写入
        "kiroAgent.memory",
    ];
    if !ALLOWED.contains(&key.as_str()) {
        return Err(format!("不允许的 Kiro Agent key: {key}"));
    }
    run_kiro_blocking(move || set_kiro_generic_inner(key, value)).await
}

/// 用系统默认程序打开 Kiro 配置文件（kind: "settings" | "mcp" | "sandbox"）。
///
/// 文件不存在时会先创建（父目录 + 空对象），保证「打开」始终有目标；
/// 与 `app_data_cmd::open_app_data_dir` 采用同样的平台分支（explorer / open / xdg-open）。
#[tauri::command]
pub fn open_kiro_config_file(kind: String) -> Result<(), String> {
    let path = match kind.as_str() {
        "settings" => get_kiro_settings_path().ok_or("无法定位 Kiro settings.json 路径")?,
        "mcp" => crate::kiro::settings::mcp::McpConfig::config_path()
            .ok_or("无法定位 Kiro mcp.json 路径")?,
        "sandbox" => dirs::home_dir()
            .map(|h| h.join(".kiro").join("settings").join("sandbox.json"))
            .ok_or("无法定位 Kiro sandbox.json 路径")?,
        other => return Err(format!("未知的配置文件类型: {other}")),
    };

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    if !path.exists() {
        std::fs::write(&path, "{\n}\n").map_err(|e| format!("创建配置文件失败: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开配置文件失败: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开配置文件失败: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开配置文件失败: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::upsert_bool_if_changed;

    #[test]
    fn upsert_bool_if_changed_only_marks_when_value_changes() {
        let mut json = serde_json::json!({
            "kiroAgent.enableDebugLogs": false
        });

        assert!(!upsert_bool_if_changed(
            &mut json,
            "kiroAgent.enableDebugLogs",
            false
        ));
        assert!(upsert_bool_if_changed(
            &mut json,
            "kiroAgent.enableDebugLogs",
            true
        ));
        assert_eq!(
            json.get("kiroAgent.enableDebugLogs")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    // 说明：原先覆盖 `resolve_configure_mcp` / `sync_optional_configure_mcp_if_changed`
    // 的两个用例已随函数一并移除——配置同步改为统一的「双向同步（IDE 优先）」后，
    // 这两个 helper 不再存在，其语义由 `get_kiro_settings_inner` 里的 sync2way_* 宏承担。

    #[test]
    fn as_string_list_maps_arrays_and_null() {
        assert_eq!(
            super::as_string_list(&serde_json::json!(["a", "b"])),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(
            super::as_string_list(&serde_json::json!([1, "b", null])),
            Some(vec!["b".to_string()])
        );
        assert_eq!(super::as_string_list(&serde_json::Value::Null), None);
        assert_eq!(super::as_string_list(&serde_json::json!("x")), None);
    }

    #[test]
    fn get_string_array_opt_distinguishes_missing_from_empty() {
        let json = serde_json::json!({
            "kiroAgent.agentIgnoreFiles": [],
            "kiroAgent.mcpApprovedEnvVars": ["A"]
        });
        assert_eq!(
            super::get_string_array_opt(&json, "kiroAgent.agentIgnoreFiles"),
            Some(Vec::new())
        );
        assert_eq!(
            super::get_string_array_opt(&json, "kiroAgent.mcpApprovedEnvVars"),
            Some(vec!["A".to_string()])
        );
        assert_eq!(
            super::get_string_array_opt(&json, "kiroAgent.notThere"),
            None
        );
    }
}
