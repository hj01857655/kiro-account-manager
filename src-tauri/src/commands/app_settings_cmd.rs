// 应用自身设置命令 (存到 ~/.kiro-account-manager/app-settings.json)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: Option<String>,
    pub locale: Option<String>, // 界面语言
    pub lock_model: Option<bool>,
    pub locked_model: Option<String>,
    pub auto_refresh: Option<bool>,
    pub auto_refresh_interval: Option<i32>,
    pub auto_change_machine_id: Option<bool>, // 切换账号时是否更换机器码（默认 true）
    pub browser_path: Option<String>,
    // 账户机器码绑定功能
    pub bind_machine_id_to_account: Option<bool>, // true=绑定模式（每个账号固定机器码），false=随机模式
    // 隐私模式：脱敏显示邮箱
    pub privacy_mode: Option<bool>,
    // 自动换号设置
    pub auto_switch_enabled: Option<bool>,
    pub auto_switch_threshold: Option<f64>,
    pub auto_switch_interval: Option<i32>,
    // Kiro IDE 开关设置（用户偏好）
    pub enable_codebase_indexing: Option<bool>,
    pub enable_tab_autocomplete: Option<bool>,
    pub usage_summary: Option<bool>,
    pub code_references: Option<bool>,
    pub enable_debug_logs: Option<bool>,
    pub notify_action_required: Option<bool>,
    pub notify_failure: Option<bool>,
    pub notify_success: Option<bool>,
    pub notify_billing: Option<bool>,
    // 新增 Kiro IDE 设置
    pub trusted_tools: Option<Vec<String>>,
    pub reference_tracker: Option<bool>,
    pub configure_mcp: Option<String>,
    pub telemetry_content_collection: Option<bool>,
    pub telemetry_usage_analytics: Option<bool>,
    pub telemetry_edit_stats: Option<bool>,
    pub telemetry_feedback: Option<bool>,
}

// 兼容旧配置文件中的 redeem_server 字段（已废弃）
// 读取时忽略，不再写入

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Some("dark".to_string()),
            locale: Some("zh-CN".to_string()),
            lock_model: Some(false),
            locked_model: None,
            auto_refresh: Some(true),
            auto_refresh_interval: Some(50),
            auto_change_machine_id: Some(true), // 默认开启
            browser_path: None,
            bind_machine_id_to_account: Some(true),
            privacy_mode: Some(true), // 默认开启
            // 自动换号默认值
            auto_switch_enabled: Some(false),
            auto_switch_threshold: Some(1.0),
            auto_switch_interval: Some(5),
            // Kiro IDE 开关默认值
            enable_codebase_indexing: Some(true),
            enable_tab_autocomplete: Some(true),
            usage_summary: Some(true),
            code_references: Some(true),
            enable_debug_logs: Some(false),
            notify_action_required: Some(true),
            notify_failure: Some(true),
            notify_success: Some(true),
            notify_billing: Some(true),
            trusted_tools: None,
            reference_tracker: Some(false),
            configure_mcp: Some("Enabled".to_string()),
            telemetry_content_collection: Some(false),
            telemetry_usage_analytics: Some(false),
            telemetry_edit_stats: Some(false),
            telemetry_feedback: Some(false),
        }
    }
}

impl AppSettings {
    fn apply_updates(&mut self, updates: Self) {
        macro_rules! apply_if_some {
            ($field:ident) => {
                if updates.$field.is_some() {
                    self.$field = updates.$field;
                }
            };
        }

        apply_if_some!(theme);
        apply_if_some!(locale);
        apply_if_some!(lock_model);
        apply_if_some!(locked_model);
        apply_if_some!(auto_refresh);
        apply_if_some!(auto_refresh_interval);
        apply_if_some!(auto_change_machine_id);
        apply_if_some!(browser_path);
        apply_if_some!(bind_machine_id_to_account);
        apply_if_some!(privacy_mode);
        apply_if_some!(auto_switch_enabled);
        apply_if_some!(auto_switch_threshold);
        apply_if_some!(auto_switch_interval);
        apply_if_some!(enable_codebase_indexing);
        apply_if_some!(enable_tab_autocomplete);
        apply_if_some!(usage_summary);
        apply_if_some!(code_references);
        apply_if_some!(enable_debug_logs);
        apply_if_some!(notify_action_required);
        apply_if_some!(notify_failure);
        apply_if_some!(notify_success);
        apply_if_some!(notify_billing);
        apply_if_some!(trusted_tools);
        apply_if_some!(reference_tracker);
        apply_if_some!(configure_mcp);
        apply_if_some!(telemetry_content_collection);
        apply_if_some!(telemetry_usage_analytics);
        apply_if_some!(telemetry_edit_stats);
        apply_if_some!(telemetry_feedback);
    }
}

fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        })
        .join(".kiro-account-manager")
}

fn get_app_settings_path() -> PathBuf {
    get_data_dir().join("app-settings.json")
}

fn ensure_parent_dir(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    Ok(())
}

async fn run_blocking_io<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

pub fn get_app_settings_inner() -> Result<AppSettings, String> {
    let path = get_app_settings_path();
    if !path.exists() {
        // 首次启动：创建并保存默认值
        let default_settings = AppSettings::default();
        save_settings_to_file(&default_settings)?;
        return Ok(default_settings);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取设置失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析设置失败: {e}"))
}

pub fn save_settings_to_file(settings: &AppSettings) -> Result<(), String> {
    let path = get_app_settings_path();
    ensure_parent_dir(&path)?;
    let content = serde_json::to_string_pretty(settings).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))
}

fn save_app_settings_inner(updates: AppSettings) -> Result<(), String> {
    let mut current = get_app_settings_inner().unwrap_or_default();

    current.apply_updates(updates);

    save_settings_to_file(&current)
}

#[tauri::command]
pub async fn get_app_settings() -> Result<AppSettings, String> {
    run_blocking_io(get_app_settings_inner).await
}

#[tauri::command]
pub async fn save_app_settings(settings: AppSettings) -> Result<(), String> {
    run_blocking_io(move || save_app_settings_inner(settings)).await
}

/// 获取自定义浏览器路径（供打开浏览器时使用）
pub fn get_browser_path() -> Option<String> {
    get_app_settings_inner()
        .ok()
        .and_then(|s| s.browser_path)
        .filter(|p| !p.is_empty())
}

#[cfg(windows)]
fn write_protocol_mapping(exe_path: &str) -> Result<String, String> {
    use std::path::Path;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let normalized = exe_path.trim().trim_matches('"');
    if normalized.is_empty() {
        return Err("协议映射路径不能为空".to_string());
    }
    if !Path::new(normalized).exists() {
        return Err(format!("目标文件不存在: {normalized}"));
    }

    let command = format!("\"{normalized}\" \"%1\"");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for scheme in ["kiro", "kiro-account-manager"] {
        let class_path = format!("Software\\Classes\\{scheme}");
        let (class_key, _) = hkcu
            .create_subkey(&class_path)
            .map_err(|e| format!("创建协议键失败（{scheme}）: {e}"))?;
        class_key
            .set_value("", &format!("URL:{scheme} Protocol"))
            .map_err(|e| format!("设置协议标题失败（{scheme}）: {e}"))?;
        class_key
            .set_value("URL Protocol", &"")
            .map_err(|e| format!("设置 URL Protocol 失败（{scheme}）: {e}"))?;

        let (cmd_key, _) = hkcu
            .create_subkey(format!("{class_path}\\shell\\open\\command"))
            .map_err(|e| format!("创建命令键失败（{scheme}）: {e}"))?;
        cmd_key
            .set_value("", &command)
            .map_err(|e| format!("写入命令失败（{scheme}）: {e}"))?;
    }

    Ok(command)
}

#[cfg(windows)]
fn get_protocol_command_inner() -> Result<String, String> {
    use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(cmd_key) = hkcu.open_subkey("Software\\Classes\\kiro\\shell\\open\\command") {
        let command: String = cmd_key.get_value("").unwrap_or_default();
        if !command.trim().is_empty() {
            return Ok(command);
        }
    }

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    if let Ok(cmd_key) = hkcr.open_subkey("kiro\\shell\\open\\command") {
        let command: String = cmd_key.get_value("").unwrap_or_default();
        if !command.trim().is_empty() {
            return Ok(command);
        }
    }

    Err("未找到 kiro 协议映射".to_string())
}

#[cfg(not(windows))]
fn get_protocol_command_inner() -> Result<String, String> {
    Err("当前平台不支持 kiro 协议映射设置".to_string())
}

#[tauri::command]
pub async fn get_kiro_protocol_command() -> Result<String, String> {
    run_blocking_io(get_protocol_command_inner).await
}

#[tauri::command]
pub async fn set_kiro_protocol_executable(path: String) -> Result<String, String> {
    run_blocking_io(move || {
        #[cfg(windows)]
        {
            write_protocol_mapping(&path)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err("当前平台不支持 kiro 协议映射设置".to_string())
        }
    })
    .await
}

#[tauri::command]
pub async fn reset_kiro_protocol_to_current_exe() -> Result<String, String> {
    run_blocking_io(|| {
        #[cfg(windows)]
        {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("无法获取当前程序路径: {e}"))?
                .display()
                .to_string();
            write_protocol_mapping(&exe_path)
        }
        #[cfg(not(windows))]
        {
            Err("当前平台不支持 kiro 协议映射设置".to_string())
        }
    })
    .await
}

// ============================================================
// 账号绑定机器码功能（已废弃，保留空实现兼容旧调用）
// ============================================================

#[tauri::command]
pub async fn bind_machine_id_to_account(
    _account_id: String,
    _machine_id: String,
) -> Result<(), String> {
    // 已废弃：机器码现在存储在账号的 machine_id 字段
    Ok(())
}

#[tauri::command]
pub async fn unbind_machine_id_from_account(_account_id: String) -> Result<(), String> {
    // 已废弃：机器码现在存储在账号的 machine_id 字段
    Ok(())
}

#[tauri::command]
pub async fn get_bound_machine_id(_account_id: String) -> Result<Option<String>, String> {
    // 已废弃：机器码现在存储在账号的 machine_id 字段
    Ok(None)
}

#[tauri::command]
pub async fn get_all_bound_machine_ids() -> Result<std::collections::HashMap<String, String>, String>
{
    // 已废弃：机器码现在存储在账号的 machine_id 字段
    Ok(std::collections::HashMap::new())
}

// ============================================================
// 使用量历史记录功能
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistoryEntry {
    pub date: String, // YYYY-MM-DD
    pub total_quota: i32,
    pub total_used: i32,
    pub account_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistory {
    pub entries: Vec<UsageHistoryEntry>,
}

fn get_usage_history_path() -> PathBuf {
    get_data_dir().join("usage-history.json")
}

fn get_usage_history_inner() -> Result<UsageHistory, String> {
    let path = get_usage_history_path();
    if !path.exists() {
        return Ok(UsageHistory::default());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取历史记录失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析历史记录失败: {e}"))
}

fn merge_usage_history_entry(history: &mut UsageHistory, entry: UsageHistoryEntry) {
    // 如果当天已有记录，则更新；否则添加新记录
    if let Some(existing) = history.entries.iter_mut().find(|e| e.date == entry.date) {
        existing.total_quota = entry.total_quota;
        existing.total_used = entry.total_used;
        existing.account_count = entry.account_count;
    } else {
        history.entries.push(entry);
    }

    // 只保留最近 30 天的记录
    history.entries.sort_by(|a, b| a.date.cmp(&b.date));
    if history.entries.len() > 30 {
        let skip_count = history.entries.len() - 30;
        history.entries.drain(..skip_count);
    }
}

fn save_usage_history_entry_inner(entry: UsageHistoryEntry) -> Result<(), String> {
    let path = get_usage_history_path();
    ensure_parent_dir(&path)?;

    let mut history = get_usage_history_inner().unwrap_or_default();
    merge_usage_history_entry(&mut history, entry);

    let content = serde_json::to_string_pretty(&history).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn get_usage_history() -> Result<UsageHistory, String> {
    run_blocking_io(get_usage_history_inner).await
}

#[tauri::command]
pub async fn save_usage_history_entry(entry: UsageHistoryEntry) -> Result<(), String> {
    run_blocking_io(move || save_usage_history_entry_inner(entry)).await
}

#[cfg(test)]
mod tests {
    use super::{merge_usage_history_entry, AppSettings, UsageHistory, UsageHistoryEntry};

    #[test]
    fn apply_updates_only_overwrites_fields_provided_as_some() {
        let mut current = AppSettings {
            theme: Some("dark".to_string()),
            locale: Some("zh-CN".to_string()),
            lock_model: Some(true),
            locked_model: Some("claude-opus-4.5".to_string()),
            auto_refresh: Some(true),
            auto_refresh_interval: Some(50),
            auto_change_machine_id: Some(true),
            browser_path: Some("C:/browser.exe".to_string()),
            bind_machine_id_to_account: Some(true),
            privacy_mode: Some(true),
            auto_switch_enabled: Some(false),
            auto_switch_threshold: Some(1.0),
            auto_switch_interval: Some(5),
            enable_codebase_indexing: Some(true),
            enable_tab_autocomplete: Some(true),
            usage_summary: Some(true),
            code_references: Some(true),
            enable_debug_logs: Some(false),
            notify_action_required: Some(true),
            notify_failure: Some(true),
            notify_success: Some(true),
            notify_billing: Some(true),
            trusted_tools: Some(vec!["tool-a".to_string()]),
            reference_tracker: Some(false),
            configure_mcp: Some("Enabled".to_string()),
            telemetry_content_collection: Some(false),
            telemetry_usage_analytics: Some(false),
            telemetry_edit_stats: Some(false),
            telemetry_feedback: Some(false),
        };

        let updates = AppSettings {
            theme: Some("light".to_string()),
            locale: None,
            lock_model: Some(false),
            locked_model: None,
            auto_refresh: None,
            auto_refresh_interval: Some(30),
            auto_change_machine_id: None,
            browser_path: Some("D:/portable/browser.exe".to_string()),
            bind_machine_id_to_account: None,
            privacy_mode: Some(false),
            auto_switch_enabled: Some(true),
            auto_switch_threshold: None,
            auto_switch_interval: None,
            enable_codebase_indexing: Some(false),
            enable_tab_autocomplete: None,
            usage_summary: None,
            code_references: Some(false),
            enable_debug_logs: Some(true),
            notify_action_required: None,
            notify_failure: None,
            notify_success: None,
            notify_billing: Some(false),
            trusted_tools: Some(vec!["tool-b".to_string(), "tool-c".to_string()]),
            reference_tracker: Some(true),
            configure_mcp: Some("Disabled".to_string()),
            telemetry_content_collection: None,
            telemetry_usage_analytics: Some(true),
            telemetry_edit_stats: None,
            telemetry_feedback: Some(true),
        };

        current.apply_updates(updates);

        assert_eq!(current.theme.as_deref(), Some("light"));
        assert_eq!(current.locale.as_deref(), Some("zh-CN"));
        assert_eq!(current.lock_model, Some(false));
        assert_eq!(current.locked_model.as_deref(), Some("claude-opus-4.5"));
        assert_eq!(current.auto_refresh, Some(true));
        assert_eq!(current.auto_refresh_interval, Some(30));
        assert_eq!(
            current.browser_path.as_deref(),
            Some("D:/portable/browser.exe")
        );
        assert_eq!(current.privacy_mode, Some(false));
        assert_eq!(current.auto_switch_enabled, Some(true));
        assert_eq!(current.auto_switch_threshold, Some(1.0));
        assert_eq!(current.enable_codebase_indexing, Some(false));
        assert_eq!(current.code_references, Some(false));
        assert_eq!(current.enable_debug_logs, Some(true));
        assert_eq!(current.notify_billing, Some(false));
        assert_eq!(
            current.trusted_tools,
            Some(vec!["tool-b".to_string(), "tool-c".to_string()])
        );
        assert_eq!(current.reference_tracker, Some(true));
        assert_eq!(current.configure_mcp.as_deref(), Some("Disabled"));
        assert_eq!(current.telemetry_content_collection, Some(false));
        assert_eq!(current.telemetry_usage_analytics, Some(true));
        assert_eq!(current.telemetry_edit_stats, Some(false));
        assert_eq!(current.telemetry_feedback, Some(true));
    }

    #[test]
    fn apply_updates_with_all_none_keeps_existing_values() {
        let mut current = AppSettings::default();
        let before = current.clone();

        current.apply_updates(AppSettings {
            theme: None,
            locale: None,
            lock_model: None,
            locked_model: None,
            auto_refresh: None,
            auto_refresh_interval: None,
            auto_change_machine_id: None,
            browser_path: None,
            bind_machine_id_to_account: None,
            privacy_mode: None,
            auto_switch_enabled: None,
            auto_switch_threshold: None,
            auto_switch_interval: None,
            enable_codebase_indexing: None,
            enable_tab_autocomplete: None,
            usage_summary: None,
            code_references: None,
            enable_debug_logs: None,
            notify_action_required: None,
            notify_failure: None,
            notify_success: None,
            notify_billing: None,
            trusted_tools: None,
            reference_tracker: None,
            configure_mcp: None,
            telemetry_content_collection: None,
            telemetry_usage_analytics: None,
            telemetry_edit_stats: None,
            telemetry_feedback: None,
        });

        assert_eq!(current.theme, before.theme);
        assert_eq!(current.locale, before.locale);
        assert_eq!(current.lock_model, before.lock_model);
        assert_eq!(current.locked_model, before.locked_model);
        assert_eq!(current.auto_refresh, before.auto_refresh);
        assert_eq!(current.auto_refresh_interval, before.auto_refresh_interval);
        assert_eq!(
            current.auto_change_machine_id,
            before.auto_change_machine_id
        );
        assert_eq!(current.browser_path, before.browser_path);
        assert_eq!(
            current.bind_machine_id_to_account,
            before.bind_machine_id_to_account
        );
        assert_eq!(current.privacy_mode, before.privacy_mode);
        assert_eq!(current.auto_switch_enabled, before.auto_switch_enabled);
        assert_eq!(current.auto_switch_threshold, before.auto_switch_threshold);
        assert_eq!(current.auto_switch_interval, before.auto_switch_interval);
        assert_eq!(
            current.enable_codebase_indexing,
            before.enable_codebase_indexing
        );
        assert_eq!(
            current.enable_tab_autocomplete,
            before.enable_tab_autocomplete
        );
        assert_eq!(current.usage_summary, before.usage_summary);
        assert_eq!(current.code_references, before.code_references);
        assert_eq!(current.enable_debug_logs, before.enable_debug_logs);
        assert_eq!(
            current.notify_action_required,
            before.notify_action_required
        );
        assert_eq!(current.notify_failure, before.notify_failure);
        assert_eq!(current.notify_success, before.notify_success);
        assert_eq!(current.notify_billing, before.notify_billing);
        assert_eq!(current.trusted_tools, before.trusted_tools);
        assert_eq!(current.reference_tracker, before.reference_tracker);
        assert_eq!(current.configure_mcp, before.configure_mcp);
        assert_eq!(
            current.telemetry_content_collection,
            before.telemetry_content_collection
        );
        assert_eq!(
            current.telemetry_usage_analytics,
            before.telemetry_usage_analytics
        );
        assert_eq!(current.telemetry_edit_stats, before.telemetry_edit_stats);
        assert_eq!(current.telemetry_feedback, before.telemetry_feedback);
    }

    #[test]
    fn merge_usage_history_entry_replaces_same_day_entry() {
        let mut history = UsageHistory {
            entries: vec![UsageHistoryEntry {
                date: "2026-03-30".to_string(),
                total_quota: 100,
                total_used: 40,
                account_count: 2,
            }],
        };

        merge_usage_history_entry(
            &mut history,
            UsageHistoryEntry {
                date: "2026-03-30".to_string(),
                total_quota: 120,
                total_used: 55,
                account_count: 3,
            },
        );

        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].date, "2026-03-30");
        assert_eq!(history.entries[0].total_quota, 120);
        assert_eq!(history.entries[0].total_used, 55);
        assert_eq!(history.entries[0].account_count, 3);
    }

    #[test]
    fn merge_usage_history_entry_keeps_only_latest_thirty_sorted_days() {
        let mut history = UsageHistory {
            entries: (1..=30)
                .map(|day| UsageHistoryEntry {
                    date: format!("2026-03-{day:02}"),
                    total_quota: day,
                    total_used: day,
                    account_count: 1,
                })
                .collect(),
        };

        merge_usage_history_entry(
            &mut history,
            UsageHistoryEntry {
                date: "2026-03-31".to_string(),
                total_quota: 31,
                total_used: 31,
                account_count: 1,
            },
        );

        assert_eq!(history.entries.len(), 30);
        assert_eq!(
            history.entries.first().map(|entry| entry.date.as_str()),
            Some("2026-03-02")
        );
        assert_eq!(
            history.entries.last().map(|entry| entry.date.as_str()),
            Some("2026-03-31")
        );
    }
}
