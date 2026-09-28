// Workflows 管理命令（Kiro 1.1.14）

use crate::commands::common::run_blocking_task;
use crate::kiro::settings::workflows::{WorkflowFile, WorkflowManager};
use tauri::command;

/// 列出某一级别下的所有 workflow 文件。
#[command]
pub async fn list_workflows(
    scope: String,
    project_dir: Option<String>,
) -> Result<Vec<WorkflowFile>, String> {
    run_blocking_task(move || WorkflowManager::list_workflows(&scope, project_dir.as_deref())).await
}

/// 读取单个 workflow 文件。
#[command]
pub async fn read_workflow(
    scope: String,
    project_dir: Option<String>,
    file_name: String,
) -> Result<WorkflowFile, String> {
    run_blocking_task(move || {
        WorkflowManager::read_workflow(&scope, project_dir.as_deref(), &file_name)
    })
    .await
}

/// 写入单个 workflow 文件。
#[command]
pub async fn save_workflow(
    scope: String,
    project_dir: Option<String>,
    file_name: String,
    content: String,
) -> Result<Vec<String>, String> {
    // 返回 schema 诊断列表（空 = 通过）；保存本身不因诊断失败
    run_blocking_task(move || {
        WorkflowManager::write_workflow(&scope, project_dir.as_deref(), &file_name, &content)
    })
    .await
}

/// 新建一个空 workflow 文件。
#[command]
pub async fn create_workflow(
    scope: String,
    project_dir: Option<String>,
    file_name: String,
) -> Result<(), String> {
    run_blocking_task(move || WorkflowManager::create_workflow(&scope, project_dir.as_deref(), &file_name))
        .await
}

/// 删除单个 workflow 文件。
#[command]
pub async fn delete_workflow(
    scope: String,
    project_dir: Option<String>,
    file_name: String,
) -> Result<(), String> {
    run_blocking_task(move || {
        WorkflowManager::delete_workflow(&scope, project_dir.as_deref(), &file_name)
    })
    .await
}
