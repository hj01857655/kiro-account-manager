// Skills 管理命令

use crate::commands::common::run_blocking_task;
use crate::kiro::settings::skills::{SkillInfo, SkillsManager};
use tauri::command;

#[command]
pub async fn get_skills(project_dir: Option<String>) -> Result<Vec<SkillInfo>, String> {
    run_blocking_task(move || SkillsManager::load_all(project_dir.as_deref())).await
}

#[command]
pub async fn get_skill(
    name: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<SkillInfo, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    run_blocking_task(move || SkillsManager::load(&name, &scope, project_dir.as_deref())).await
}

#[command]
pub async fn save_skill(
    name: String,
    content: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<Vec<String>, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    // 返回内容级诊断列表（空 = 完全合规）；保存本身不因诊断失败
    run_blocking_task(move || SkillsManager::save(&name, &content, &scope, project_dir.as_deref()))
        .await
}

#[command]
pub async fn delete_skill(
    name: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<(), String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    run_blocking_task(move || SkillsManager::delete(&name, &scope, project_dir.as_deref())).await
}

#[command]
pub async fn create_skill(
    name: String,
    content: String,
    scope: Option<String>,
    project_dir: Option<String>,
) -> Result<SkillInfo, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    run_blocking_task(move || {
        SkillsManager::create(&name, &content, &scope, project_dir.as_deref())
    })
    .await
}

#[command]
pub async fn import_skill_local(
    source_path: String,
    target_name: Option<String>,
    scope: Option<String>,
    project_dir: Option<String>,
    overwrite: Option<bool>,
) -> Result<SkillInfo, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    let overwrite = overwrite.unwrap_or(false);
    run_blocking_task(move || {
        SkillsManager::import_local(
            &source_path,
            target_name.as_deref(),
            &scope,
            project_dir.as_deref(),
            overwrite,
        )
    })
    .await
}

#[command]
pub async fn import_skill_from_github(
    repo_url: String,
    path_in_repo: Option<String>,
    branch: Option<String>,
    target_name: Option<String>,
    scope: Option<String>,
    project_dir: Option<String>,
    overwrite: Option<bool>,
) -> Result<SkillInfo, String> {
    let scope = scope.unwrap_or_else(|| "user".to_string());
    let overwrite = overwrite.unwrap_or(false);
    run_blocking_task(move || {
        SkillsManager::import_from_github(
            &repo_url,
            path_in_repo.as_deref(),
            branch.as_deref(),
            target_name.as_deref(),
            &scope,
            project_dir.as_deref(),
            overwrite,
        )
    })
    .await
}
