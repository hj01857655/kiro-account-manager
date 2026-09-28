// Custom Agents 管理命令

use crate::commands::common::run_blocking_task;
use crate::kiro::settings::custom_agents::{CustomAgentFile, CustomAgentsManager};
use tauri::command;

#[command]
pub async fn get_custom_agents(
    project_dir: Option<String>,
) -> Result<Vec<CustomAgentFile>, String> {
    run_blocking_task(move || CustomAgentsManager::load_all(project_dir.as_deref())).await
}

#[command]
pub async fn get_custom_agent(
    file_name: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<CustomAgentFile, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    run_blocking_task(move || CustomAgentsManager::load(&file_name, &scope, project_dir.as_deref()))
        .await
}

#[command]
pub async fn save_custom_agent(
    file_name: String,
    content: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<Vec<String>, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    // 返回内容级诊断列表（空 = 通过）；保存本身不因诊断失败
    run_blocking_task(move || {
        CustomAgentsManager::save(&file_name, &content, &scope, project_dir.as_deref())
    })
    .await
}

#[command]
pub async fn delete_custom_agent(
    file_name: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<(), String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    run_blocking_task(move || {
        CustomAgentsManager::delete(&file_name, &scope, project_dir.as_deref())
    })
    .await
}

#[command]
pub async fn create_custom_agent(
    file_name: String,
    content: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<CustomAgentFile, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    run_blocking_task(move || {
        CustomAgentsManager::create(&file_name, &content, &scope, project_dir.as_deref())
    })
    .await
}
