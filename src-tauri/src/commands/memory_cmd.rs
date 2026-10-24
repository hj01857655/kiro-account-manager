// Agent Memory 管理命令（~/.kiro/memories/memories.db）
//
// Memory 的启停模式是 Kiro 用户 settings.json（VS Code 风格）里的
// `kiroAgent.memory` 键（{mode: read_write|read_only|disabled, reflection: bool}），
// 官方档位映射（docs/Kiro memory 系统.md）：
//   off                 → {mode:"disabled",  reflection:false}（默认，键不存在）
//   useOnly             → {mode:"read_only", reflection:false}
//   useWithoutLearning  → {mode:"read_write", reflection:false}
//   useAndLearn         → {mode:"read_write", reflection:true}
// 读取走本模块的 get_memory_mode；写入复用既有 set_kiro_agent_setting（任意键）。

use crate::commands::kiro_settings_cmd::get_kiro_settings_path;
use crate::kiro::settings::memories::{
    delete_memory_impl, list_memories_page_impl, preview_memory_injection_impl, save_memory_impl,
    MemoryDraft, MemoryInjectionPreview, MemoryItem, MemoryPage,
};
use serde::Serialize;
use std::fs;

#[tauri::command]
pub async fn list_memories() -> Result<Vec<MemoryItem>, String> {
    crate::kiro::settings::memories::list_memories_impl()
}

/// 分页读取记忆（口径对齐 IDE：`limit` 默认 100、上限 1000，cursor 不透明）。
#[tauri::command]
pub async fn list_memories_page(
    limit: Option<u32>,
    cursor: Option<String>,
    scopes: Option<Vec<String>>,
) -> Result<MemoryPage, String> {
    list_memories_page_impl(limit, cursor, scopes)
}

#[tauri::command]
pub async fn delete_memory(id: String) -> Result<(), String> {
    delete_memory_impl(&id)
}

/// 新建或编辑一条记忆（`draft.id` 为空=新建，否则更新）。返回记忆 id。
///
/// 字段校验（title 形状/长度、summary、content、总条数、scope 形态）
/// 全部在 `save_memory_impl` 里做，与 IDE 同口径。
#[tauri::command]
pub async fn save_memory(draft: MemoryDraft) -> Result<String, String> {
    save_memory_impl(draft)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryModeConfig {
    pub mode: String,
    pub reflection: bool,
}

#[tauri::command]
pub async fn get_memory_mode() -> Result<MemoryModeConfig, String> {
    let path = get_kiro_settings_path().ok_or("无法定位 Kiro settings.json")?;
    if !path.exists() {
        return Ok(MemoryModeConfig { mode: "disabled".into(), reflection: false });
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("读取 settings.json 失败: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("解析 settings.json 失败: {e}"))?;
    let mem = &v["kiroAgent"]["memory"];
    let mode = mem["mode"].as_str().unwrap_or("disabled").to_string();
    let reflection = mem["reflection"].as_bool().unwrap_or(false);
    Ok(MemoryModeConfig { mode, reflection })
}

/// 注入预览：模拟 Kiro buildMemoryBlock 的 resident/manifest 分层与预算（§7.1.11）
#[tauri::command]
pub async fn preview_memory_injection() -> Result<MemoryInjectionPreview, String> {
    let mode = get_memory_mode().await?.mode;
    let enabled = mode != "disabled";
    preview_memory_injection_impl(enabled)
}
