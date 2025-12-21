// 应用自身设置命令 (存到 ~/.kiro-account-manager/app-settings.json)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::portable;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: Option<String>,
    pub lock_model: Option<bool>,
    pub locked_model: Option<String>,
    pub auto_refresh: Option<bool>,
    pub auto_refresh_interval: Option<i32>,
    pub auto_change_machine_id: Option<bool>,
    pub browser_path: Option<String>,
    // 账户机器码绑定功能
    pub bind_machine_id_to_account: Option<bool>,  // 是否启用账户绑定机器码
    pub use_bound_machine_id: Option<bool>,        // 切换时使用绑定的机器码（否则随机生成）
    pub account_machine_ids: Option<std::collections::HashMap<String, String>>,  // 账户ID -> 机器码映射
}

fn get_app_settings_path() -> PathBuf {
    portable::get_app_data_dir().join("app-settings.json")
}

fn get_app_settings_inner() -> Result<AppSettings, String> {
    let path = get_app_settings_path();
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取设置失败: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析设置失败: {}", e))
}

fn save_app_settings_inner(updates: AppSettings) -> Result<(), String> {
    let path = get_app_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    // 读取现有设置，合并更新
    let mut current = get_app_settings_inner().unwrap_or_default();
    
    // 只更新传入的非 None 字段
    if updates.theme.is_some() { current.theme = updates.theme; }
    if updates.lock_model.is_some() { current.lock_model = updates.lock_model; }
    if updates.locked_model.is_some() { current.locked_model = updates.locked_model; }
    if updates.auto_refresh.is_some() { current.auto_refresh = updates.auto_refresh; }
    if updates.auto_refresh_interval.is_some() { current.auto_refresh_interval = updates.auto_refresh_interval; }
    if updates.auto_change_machine_id.is_some() { current.auto_change_machine_id = updates.auto_change_machine_id; }
    if updates.browser_path.is_some() { current.browser_path = updates.browser_path; }
    if updates.bind_machine_id_to_account.is_some() { current.bind_machine_id_to_account = updates.bind_machine_id_to_account; }
    if updates.use_bound_machine_id.is_some() { current.use_bound_machine_id = updates.use_bound_machine_id; }
    if updates.account_machine_ids.is_some() { current.account_machine_ids = updates.account_machine_ids; }
    
    let content = serde_json::to_string_pretty(&current)
        .map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_app_settings() -> Result<AppSettings, String> {
    tokio::task::spawn_blocking(get_app_settings_inner)
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn save_app_settings(settings: AppSettings) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_app_settings_inner(settings))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

/// 获取自定义浏览器路径（供打开浏览器时使用）
pub fn get_browser_path() -> Option<String> {
    let path = get_app_settings_path();
    println!("[Settings] App settings path: {:?}", path);
    
    let result = get_app_settings_inner();
    println!("[Settings] get_app_settings_inner result: {:?}", result);
    
    let browser_path = result.ok().and_then(|s| s.browser_path).filter(|p| !p.is_empty());
    println!("[Settings] browser_path: {:?}", browser_path);
    
    browser_path
}

// ============================================================
// 账号绑定机器码功能
// ============================================================

/// 绑定机器码到账号
fn bind_machine_id_inner(account_id: String, machine_id: String) -> Result<(), String> {
    let path = get_app_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    let mut current = get_app_settings_inner().unwrap_or_default();
    let mut map = current.account_machine_ids.unwrap_or_default();
    map.insert(account_id, machine_id);
    current.account_machine_ids = Some(map);
    
    let content = serde_json::to_string_pretty(&current)
        .map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

/// 解绑账号的机器码
fn unbind_machine_id_inner(account_id: String) -> Result<(), String> {
    let path = get_app_settings_path();
    let mut current = get_app_settings_inner().unwrap_or_default();
    
    if let Some(ref mut map) = current.account_machine_ids {
        map.remove(&account_id);
    }
    
    let content = serde_json::to_string_pretty(&current)
        .map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

/// 获取账号绑定的机器码
fn get_bound_machine_id_inner(account_id: String) -> Result<Option<String>, String> {
    let current = get_app_settings_inner().unwrap_or_default();
    Ok(current.account_machine_ids
        .and_then(|map| map.get(&account_id).cloned()))
}

/// 获取所有账号绑定的机器码
fn get_all_bound_machine_ids_inner() -> Result<std::collections::HashMap<String, String>, String> {
    let current = get_app_settings_inner().unwrap_or_default();
    Ok(current.account_machine_ids.unwrap_or_default())
}

#[tauri::command]
pub async fn bind_machine_id_to_account(account_id: String, machine_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || bind_machine_id_inner(account_id, machine_id))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn unbind_machine_id_from_account(account_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || unbind_machine_id_inner(account_id))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn get_bound_machine_id(account_id: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || get_bound_machine_id_inner(account_id))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn get_all_bound_machine_ids() -> Result<std::collections::HashMap<String, String>, String> {
    tokio::task::spawn_blocking(get_all_bound_machine_ids_inner)
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}
