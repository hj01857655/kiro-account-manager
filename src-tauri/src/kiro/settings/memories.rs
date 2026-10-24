// Memory 管理（Kiro 1.4+ Agent Memory：~/.kiro/memories/memories.db）
//
// 存储为 SQLite（WAL 模式，node:sqlite DatabaseSync 写入），表结构逆向自
// Kiro memory 系统文档（docs/Kiro memory 系统事实.md §1.3）：
//   memories(id, memory_type, scopes, title, summary, content, why,
//            how_to_apply, session_id, metadata, created_at, updated_at)
//   memory_scopes(memory_id REFERENCES memories(id) ON DELETE CASCADE, scope)
// 只做列表与删除；外键约束必须开启，删除 memories 行才会级联清 memory_scopes。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 一条 Agent Memory（camelCase 直出前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub id: String,
    pub memory_type: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub why: String,
    #[serde(default)]
    pub how_to_apply: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

pub fn memories_db_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kiro").join("memories").join("memories.db"))
}

fn open_db() -> Result<rusqlite::Connection, String> {
    let path = memories_db_path().ok_or("无法获取用户目录")?;
    if !path.exists() {
        return Err("memories.db 不存在（Memory 功能从未启用过）".to_string());
    }
    let conn = rusqlite::Connection::open(&path).map_err(|e| format!("打开 memories.db 失败: {e}"))?;
    // 与 Kiro 写入侧一致：busy_timeout 防止 IDE/CLI 同时持锁报错，
    // foreign_keys 开启才能让删除 memories 时级联清 memory_scopes。
    conn.busy_timeout(std::time::Duration::from_millis(2000))
        .map_err(|e| format!("设置 busy_timeout 失败: {e}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("开启外键约束失败: {e}"))?;
    Ok(conn)
}

/// scopes 列是 TEXT 存的 JSON 数组；解析失败按空数组处理（与 Kiro 读侧一致）
fn parse_scopes(raw: Option<String>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

pub fn list_memories_impl() -> Result<Vec<MemoryItem>, String> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, scopes, title, summary, content, why,
                    how_to_apply, session_id, created_at, updated_at
             FROM memories ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("查询 memories 失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(MemoryItem {
                id: row.get(0)?,
                memory_type: row.get(1)?,
                scopes: parse_scopes(row.get(2)?),
                title: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                summary: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                content: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                why: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                how_to_apply: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                session_id: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                created_at: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                updated_at: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            })
        })
        .map_err(|e| format!("读取 memories 行失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("遍历 memories 失败: {e}"))
}

pub fn delete_memory_impl(id: &str) -> Result<(), String> {
    let conn = open_db()?;
    let n = conn
        .execute("DELETE FROM memories WHERE id = ?1", [id])
        .map_err(|e| format!("删除 memory 失败: {e}"))?;
    if n == 0 {
        return Err(format!("memory {id} 不存在（可能已被并发删除）"));
    }
    Ok(())
}
