use crate::models::ide_session::{IdeSession, SessionSummary};
use crate::services::session_storage::{ExportFormat, SessionStorage};
use tauri::State;

#[tauri::command]
pub async fn list_workspaces(storage: State<'_, SessionStorage>) -> Result<Vec<String>, String> {
    storage.list_workspaces().map_err(|e| e.to_string())
}

/// 按项目目录查找 workspace_hash（会话摘要里记录了原始 workspace 路径）
#[tauri::command]
pub async fn find_workspace_by_dir(
    project_dir: String,
    storage: State<'_, SessionStorage>,
) -> Result<Option<String>, String> {
    storage
        .find_workspace_by_dir(&project_dir)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_sessions(
    workspace_hash: String,
    storage: State<'_, SessionStorage>,
) -> Result<Vec<SessionSummary>, String> {
    storage
        .list_sessions(&workspace_hash)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_session(
    workspace_hash: String,
    session_id: String,
    storage: State<'_, SessionStorage>,
) -> Result<IdeSession, String> {
    storage
        .load_session(&workspace_hash, &session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_session(
    workspace_hash: String,
    session_id: String,
    storage: State<'_, SessionStorage>,
) -> Result<(), String> {
    storage
        .delete_session(&workspace_hash, &session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_workspace(
    workspace_hash: String,
    storage: State<'_, SessionStorage>,
) -> Result<(), String> {
    storage
        .delete_workspace(&workspace_hash)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_session(
    workspace_hash: String,
    session_id: String,
    format: String,
    storage: State<'_, SessionStorage>,
) -> Result<String, String> {
    let export_format = match format.as_str() {
        "json" => ExportFormat::Json,
        "markdown" => ExportFormat::Markdown,
        _ => return Err("Invalid format".to_string()),
    };

    storage
        .export_session(&workspace_hash, &session_id, export_format)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_sessions(
    query: String,
    storage: State<'_, SessionStorage>,
) -> Result<Vec<SessionSummary>, String> {
    let workspaces = storage.list_workspaces().map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    for workspace in workspaces {
        let sessions = storage
            .list_sessions(&workspace)
            .map_err(|e| e.to_string())?;

        for session in sessions {
            if session.title.to_lowercase().contains(&query_lower) {
                results.push(session);
            }
        }
    }

    Ok(results)
}
