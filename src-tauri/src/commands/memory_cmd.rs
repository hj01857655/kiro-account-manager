// Agent Memory 管理命令（~/.kiro/memories/memories.db）
//
// Memory 的启停模式是 Kiro 用户 settings.json（VS Code 风格）里的
// `kiroAgent.memory` 键（{mode: read_write|read_only|disabled, reflection: bool}），
// 官方档位映射（docs/Kiro memory 系统事实.md）：
//   off                 → {mode:"disabled",  reflection:false}（默认，键不存在）
//   useOnly             → {mode:"read_only", reflection:false}
//   useWithoutLearning  → {mode:"read_write", reflection:false}
//   useAndLearn         → {mode:"read_write", reflection:true}
// 读取走本模块的 get_memory_mode；写入复用既有 set_kiro_agent_setting（任意键）。

use crate::commands::kiro_settings_cmd::get_kiro_settings_path;
use crate::kiro::settings::memories::{delete_memory_impl, list_memories_impl, MemoryItem};
use serde::Serialize;
use std::fs;

#[tauri::command]
pub async fn list_memories() -> Result<Vec<MemoryItem>, String> {
    list_memories_impl()
}

#[tauri::command]
pub async fn delete_memory(id: String) -> Result<(), String> {
    delete_memory_impl(&id)
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
