// Memory 管理（Kiro 1.4+ Agent Memory：~/.kiro/memories/memories.db）
//
// 存储为 SQLite（WAL 模式，node:sqlite DatabaseSync 写入），表结构逆向自
// Kiro memory 系统文档（docs/Kiro memory 系统.md §1.3）：
//   memories(id, memory_type, scopes, title, summary, content, why,
//            how_to_apply, session_id, metadata, created_at, updated_at)
//   memory_scopes(memory_id REFERENCES memories(id) ON DELETE CASCADE, scope)
// 只做列表、新建/编辑与删除；外键约束必须开启，删除 memories 行才会级联清 memory_scopes。
//
// 字段校验照搬 IDE（docs/Kiro memory 系统.md §2.5）：
//   title   ≤ 64 且 ^[a-zA-Z0-9-]+$   （IDE 的 title-shape，最硬的一条）
//   summary ≤ 250
//   content ≤ 20000
//   总条数  ≤ 1000（store-full）
// 不校验的话写进去的记忆 IDE 认不出（尤其 title 带中文/空格），等于写脏数据。

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

// ============================================================
// 分页（对齐 IDE 的 limit ≤ 1000 + cursor）
// ============================================================

/// 默认页大小（IDE 的 `Zeu` 默认 100）
pub const PAGE_DEFAULT: u32 = 100;
/// 页大小上限（IDE 的 `qni`）
pub const PAGE_MAX: u32 = 1_000;

/// 一页记忆 + 下一页游标。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPage {
    pub memories: Vec<MemoryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// 当前生效的 scope 过滤（IDE 的 `scopeLabel`），未过滤时为 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_label: Option<String>,
}

/// cursor 载荷，与 IDE 的 `Weu` 一致：`{createdAt, id, scopes}`。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CursorPayload {
    #[serde(rename = "createdAt")]
    created_at: String,
    id: String,
    scopes: Vec<String>,
}

/// IDE 的 `tie`：`createdAt` 降序，相同则 `id` 升序。
///
/// 返回 `Less` 表示 `a` 排在 `b` 前面（即 `a` 更新）。
fn compare_keys(
    a_created: &str,
    a_id: &str,
    b_created: &str,
    b_id: &str,
) -> std::cmp::Ordering {
    match b_created.cmp(a_created) {
        std::cmp::Ordering::Equal => a_id.cmp(b_id),
        other => other,
    }
}

fn compare_recency(a: &MemoryItem, b: &MemoryItem) -> std::cmp::Ordering {
    compare_keys(&a.created_at, &a.id, &b.created_at, &b.id)
}

fn encode_cursor(item: &MemoryItem) -> Result<String, String> {
    encode_cursor_with_scopes(item, &item.scopes)
}

/// 编码 cursor，scope 集合用当前遍历的过滤集（IDE 的 `Weu(p, n)` 第二参是 `n` 即 walkScopes 结果）。
fn encode_cursor_with_scopes(item: &MemoryItem, scopes: &[String]) -> Result<String, String> {
    let payload = CursorPayload {
        created_at: item.created_at.clone(),
        id: item.id.clone(),
        scopes: scopes.to_vec(),
    };
    let json = serde_json::to_vec(&payload).map_err(|e| format!("序列化 cursor 失败: {e}"))?;
    Ok(base64_url_encode(&json))
}

fn decode_cursor(raw: &str) -> Result<CursorPayload, String> {
    let bytes = base64_url_decode(raw).map_err(|_| {
        "cursor 不是本管理端签发的，请重新从头开始翻页".to_string()
    })?;
    let payload: CursorPayload = serde_json::from_slice(&bytes)
        .map_err(|_| "cursor 内容无法解析，请重新从头开始翻页".to_string())?;
    Ok(payload)
}

/// base64url（无填充），与 Node 的 `base64url` 编码一致。
fn base64_url_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[n as usize & 63] as char);
        }
    }
    out
}

fn base64_url_decode(raw: &str) -> Result<Vec<u8>, ()> {
    let mut buf = Vec::with_capacity(raw.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for ch in raw.chars() {
        let value = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32,
            'a'..='z' => ch as u32 - 'a' as u32 + 26,
            '0'..='9' => ch as u32 - '0' as u32 + 52,
            '-' => 62,
            '_' => 63,
            // 容忍标准 base64 的 +/ 与尾部填充
            '+' => 62,
            '/' => 63,
            '=' => continue,
            _ => return Err(()),
        };
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            buf.push((acc >> bits) as u8);
        }
    }
    Ok(buf)
}

/// 分页读取记忆，口径对齐 IDE 的 `Poi.list`：
/// scope 过滤 → 按 recency 排序 → cursor 定位 → 切片。
///
/// `limit` 为 `None` 时用 100，超出 1..=1000 直接报错（与 IDE 的 `badPageSize` 同语义）。
/// `scopes` 为 `None` 或空时不过滤（IDE 的 `walkScopes` 返回全量）。
pub fn list_memories_page_impl(
    limit: Option<u32>,
    cursor: Option<String>,
    scopes: Option<Vec<String>>,
) -> Result<MemoryPage, String> {
    let limit = match limit {
        None => PAGE_DEFAULT,
        Some(n) if (1..=PAGE_MAX).contains(&n) => n,
        Some(n) => {
            return Err(format!(
                "limit 必须是 1 到 {PAGE_MAX} 之间的整数（收到 {n}）"
            ))
        }
    };

    // IDE 的 walkScopes：有 cursor 时必须与 cursor 里记的 scopes 一致，
    // 否则报 badCursor（不能中途换过滤条件，否则分页会错乱）。
    let payload = match cursor.as_deref().filter(|c| !c.is_empty()) {
        None => None,
        Some(raw) => {
            let p = decode_cursor(raw)?;
            let requested: Vec<String> = scopes.clone().unwrap_or_default();
            if !requested.is_empty() && requested != p.scopes {
                return Err(format!(
                    "cursor 继续的是 [{}] 的遍历，与本次 scopes [{}] 不一致；\
                     要换过滤条件请去掉 cursor 重新开始",
                    p.scopes.join(", "),
                    requested.join(", ")
                ));
            }
            Some(p)
        }
    };

    // 生效的 scope 集合：优先用 cursor 里的（保证翻页一致）
    let effective_scopes: Vec<String> = match (&payload, &scopes) {
        (Some(p), _) if !p.scopes.is_empty() => p.scopes.clone(),
        (_, Some(s)) if !s.is_empty() => s.clone(),
        _ => Vec::new(),
    };

    let mut all = list_memories_impl()?;
    if !effective_scopes.is_empty() {
        // IDE 的过滤口径：取主 scope（scopes[0]）匹配，不是“包含任一”
        let wanted: std::collections::HashSet<&str> =
            effective_scopes.iter().map(|s| s.as_str()).collect();
        all.retain(|m| wanted.contains(m.scopes.first().map(|s| s.as_str()).unwrap_or("")));
    }
    all.sort_by(compare_recency);

    let start = match &payload {
        None => 0,
        Some(p) => all
            .iter()
            .position(|item| {
                compare_keys(&p.created_at, &p.id, &item.created_at, &item.id)
                    == std::cmp::Ordering::Less
            })
            .unwrap_or(all.len()),
    };

    let end = (start + limit as usize).min(all.len());
    let page: Vec<MemoryItem> = all[start..end].to_vec();
    let next_cursor = if end < all.len() {
        page.last()
            .map(|item| encode_cursor_with_scopes(item, &effective_scopes))
            .transpose()?
    } else {
        None
    };

    Ok(MemoryPage {
        memories: page,
        next_cursor,
        scope_label: if effective_scopes.is_empty() {
            None
        } else {
            Some(effective_scopes.join(", "))
        },
    })
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

// ============================================================
// 新建 / 编辑
// ============================================================

/// IDE 侧的硬上限（`docs/Kiro memory 系统.md` §2.5）
pub const TITLE_MAX: usize = 64;
pub const SUMMARY_MAX: usize = 250;
pub const CONTENT_MAX: usize = 20_000;
pub const STORE_MAX: usize = 1_000;

/// 写入/更新时允许的 memoryType。IDE 的写侧只接受这两个值，
/// `UNKNOWN` 只是读侧对脏数据的兜底，不能写进去。
const WRITABLE_TYPES: [&str; 2] = ["USER_PREFERENCE", "SEMANTIC"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDraft {
    #[serde(default)]
    pub id: Option<String>,
    pub memory_type: String,
    pub scopes: Vec<String>,
    pub title: String,
    pub summary: String,
    pub content: String,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub how_to_apply: Option<String>,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// title 只允许字母、数字、连字符（IDE 的 `title-shape` 校验）。
fn validate_title(title: &str) -> Result<(), String> {
    if title.is_empty() {
        return Err("title 不能为空".to_string());
    }
    if title.chars().count() > TITLE_MAX {
        return Err(format!(
            "title 超过 {TITLE_MAX} 字符上限，建议缩到 50 字以内"
        ));
    }
    if !title
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(
            "title 只能包含字母、数字和连字符（不能有空格、下划线、标点或中文）".to_string(),
        );
    }
    Ok(())
}

/// 校验三个 scope 形态（IDE §2.12）：`Global` / `workspace:<name>` / `repo:<p>/<o>/<n>`。
fn validate_scope(scope: &str) -> Result<(), String> {
    if scope == "Global" {
        return Ok(());
    }
    if let Some(rest) = scope.strip_prefix("workspace:") {
        if rest.is_empty() || rest.contains('/') {
            return Err(format!("非法的 workspace scope: {scope}"));
        }
        return Ok(());
    }
    if let Some(rest) = scope.strip_prefix("repo:") {
        // repo scope 至少要 3 段：provider/owner/name
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() < 3 || parts.iter().any(|p| p.is_empty()) {
            return Err(format!(
                "repo scope 需为 repo:<provider>/<owner>/<name>，得到: {scope}"
            ));
        }
        return Ok(());
    }
    Err(format!(
        "scope 只能是 Global、workspace:<name> 或 repo:<provider>/<owner>/<name>，得到: {scope}"
    ))
}

fn validate_draft(draft: &MemoryDraft) -> Result<(), String> {
    if !WRITABLE_TYPES.contains(&draft.memory_type.as_str()) {
        return Err(format!(
            "memoryType 只能是 USER_PREFERENCE 或 SEMANTIC，得到: {}",
            draft.memory_type
        ));
    }
    if draft.scopes.is_empty() {
        return Err("至少需要一个 scope".to_string());
    }
    for scope in &draft.scopes {
        validate_scope(scope)?;
    }
    validate_title(&draft.title)?;
    if draft.summary.is_empty() {
        return Err("summary 不能为空".to_string());
    }
    if draft.summary.chars().count() > SUMMARY_MAX {
        return Err(format!("summary 超过 {SUMMARY_MAX} 字符上限"));
    }
    if draft.content.is_empty() {
        return Err("content 不能为空".to_string());
    }
    if draft.content.chars().count() > CONTENT_MAX {
        return Err(format!("content 超过 {CONTENT_MAX} 字符上限"));
    }
    Ok(())
}

/// 新建或更新一条记忆。`draft.id` 为空时新建，否则更新。
///
/// 两条路径都在同一个事务里写 memories + memory_scopes，
/// 避免出现「主行写了、scope 行没写」的牛体。
pub fn save_memory_impl(draft: MemoryDraft) -> Result<String, String> {
    validate_draft(&draft)?;
    let mut conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {e}"))?;

    // title 唯一性（归一化后比较，对齐 §2.3 的匹配语义）：
    // 存储层允许重名，但重复 title 会让 IDE 的按名查询报 ambiguous——
    // 管理端写时就拒绝，避免造出这种状态。排除自身 id（编辑不改名时放行）。
    // title 已被形状校验限定为 ASCII [a-zA-Z0-9-]，SQL lower/trim 与 Rust 侧一致。
    let norm_title = draft.title.trim().to_lowercase();
    let dup: Option<String> = tx
        .query_row(
            "SELECT id FROM memories WHERE lower(trim(title)) = ?1 AND id != ?2 LIMIT 1",
            rusqlite::params![norm_title, draft.id.as_deref().unwrap_or("")],
            |r| r.get(0),
        )
        .ok();
    if let Some(other) = dup {
        return Err(format!(
            "同名记忆已存在（title 与 {other} 归一化后冲突），换一个 title 或直接编辑那条"
        ));
    }

    let scopes_json = serde_json::to_string(&draft.scopes)
        .map_err(|e| format!("序列化 scopes 失败: {e}"))?;
    let now = now_rfc3339();

    let id = match draft.id.as_deref().filter(|s| !s.is_empty()) {
        Some(existing) => {
            let n = tx
                .execute(
                    "UPDATE memories SET memory_type=?1, scopes=?2, title=?3, summary=?4,
                            content=?5, why=?6, how_to_apply=?7, updated_at=?8
                     WHERE id=?9",
                    rusqlite::params![
                        draft.memory_type,
                        scopes_json,
                        draft.title,
                        draft.summary,
                        draft.content,
                        draft.why,
                        draft.how_to_apply,
                        now,
                        existing
                    ],
                )
                .map_err(|e| format!("更新 memory 失败: {e}"))?;
            if n == 0 {
                return Err(format!("memory {existing} 不存在（可能已被并发删除）"));
            }
            // scope 是整体替换：先清后写，保持与 memories.scopes 一致
            tx.execute("DELETE FROM memory_scopes WHERE memory_id = ?1", [existing])
                .map_err(|e| format!("清理旧 scope 失败: {e}"))?;
            existing.to_string()
        }
        None => {
            // 容量检查对齐 IDE：写满 1000 条后拒绝新增
            let count: i64 = tx
                .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
                .map_err(|e| format!("统计记忆条数失败: {e}"))?;
            if count >= STORE_MAX as i64 {
                return Err(format!(
                    "记忆库已满（上限 {STORE_MAX} 条），请先删除一条再新增"
                ));
            }
            let new_id = uuid::Uuid::new_v4().to_string();
            // session_id 是 NOT NULL，手动创建统一记为 "manual"（与 IDE 一致）
            tx.execute(
                "INSERT INTO memories
                    (id, memory_type, scopes, title, summary, content, why,
                     how_to_apply, session_id, metadata, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'manual', NULL, ?9, ?9)",
                rusqlite::params![
                    new_id,
                    draft.memory_type,
                    scopes_json,
                    draft.title,
                    draft.summary,
                    draft.content,
                    draft.why,
                    draft.how_to_apply,
                    now
                ],
            )
            .map_err(|e| format!("新建 memory 失败: {e}"))?;
            new_id
        }
    };

    for scope in &draft.scopes {
        tx.execute(
            "INSERT OR IGNORE INTO memory_scopes (memory_id, scope) VALUES (?1, ?2)",
            rusqlite::params![id, scope],
        )
        .map_err(|e| format!("写入 scope 失败: {e}"))?;
    }

    tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(title: &str, summary: &str, content: &str) -> MemoryDraft {
        MemoryDraft {
            id: None,
            memory_type: "USER_PREFERENCE".to_string(),
            scopes: vec!["Global".to_string()],
            title: title.to_string(),
            summary: summary.to_string(),
            content: content.to_string(),
            why: None,
            how_to_apply: None,
        }
    }

    /// title 只允许字母、数字、连字符（IDE 的 title-shape）。
    /// 中文标题在 IDE 里也是非法的，管理端必须同样拒绝，
    /// 否则写进去的记忆 IDE 读不出来。
    #[test]
    fn title_rejects_chinese_space_and_underscore() {
        for bad in ["中文标题", "has space", "under_score", "dot.dot", ""] {
            assert!(
                validate_title(bad).is_err(),
                "title {bad:?} should be rejected"
            );
        }
        for good in ["use-pnpm", "abc123", "A-B-c-1"] {
            assert!(validate_title(good).is_ok(), "title {good:?} should pass");
        }
    }

    #[test]
    fn title_rejects_over_64_chars() {
        let long = "a".repeat(TITLE_MAX + 1);
        assert!(validate_title(&long).is_err());
        let ok = "a".repeat(TITLE_MAX);
        assert!(validate_title(&ok).is_ok());
    }

    #[test]
    fn scope_accepts_only_the_three_official_shapes() {
        for good in [
            "Global",
            "workspace:my-project",
            "repo:github/owner/name",
        ] {
            assert!(validate_scope(good).is_ok(), "scope {good:?} should pass");
        }
        for bad in [
            "global",              // 大小写敏感，只有字面量 Global
            "workspace:",          // 空名
            "workspace:a/b",       // workspace 不能带斜杠
            "repo:github/owner",   // repo 至少 3 段
            "repo:a/b/",           // 空段
            "random-scope",
        ] {
            assert!(validate_scope(bad).is_err(), "scope {bad:?} should be rejected");
        }
    }

    #[test]
    fn draft_rejects_unknown_memory_type_and_empty_scopes() {
        let mut d = draft("ok-title", "s", "c");
        d.memory_type = "UNKNOWN".to_string();
        assert!(validate_draft(&d).is_err(), "UNKNOWN must not be writable");

        let mut d = draft("ok-title", "s", "c");
        d.scopes.clear();
        assert!(validate_draft(&d).is_err(), "empty scopes must be rejected");
    }

    #[test]
    fn draft_enforces_summary_and_content_limits() {
        let ok_summary = "s".repeat(SUMMARY_MAX);
        assert!(validate_draft(&draft("t", &ok_summary, "c")).is_ok());
        let long_summary = "s".repeat(SUMMARY_MAX + 1);
        assert!(validate_draft(&draft("t", &long_summary, "c")).is_err());

        let ok_content = "c".repeat(CONTENT_MAX);
        assert!(validate_draft(&draft("t", "s", &ok_content)).is_ok());
        let long_content = "c".repeat(CONTENT_MAX + 1);
        assert!(validate_draft(&draft("t", "s", &long_content)).is_err());
    }

    fn item(id: &str, created_at: &str) -> MemoryItem {
        MemoryItem {
            id: id.to_string(),
            memory_type: "SEMANTIC".to_string(),
            scopes: vec!["Global".to_string()],
            title: id.to_string(),
            summary: String::new(),
            content: String::new(),
            why: String::new(),
            how_to_apply: String::new(),
            session_id: String::new(),
            created_at: created_at.to_string(),
            updated_at: created_at.to_string(),
        }
    }

    /// IDE 的 `tie`：createdAt 降序，相同则 id 升序。
    #[test]
    fn recency_order_matches_ide() {
        let newer = item("a", "2026-10-24T10:00:00.000Z");
        let older = item("b", "2026-10-23T10:00:00.000Z");
        assert_eq!(compare_recency(&newer, &older), std::cmp::Ordering::Less);

        // createdAt 相同 → 按 id 升序
        let a = item("a", "2026-10-24T10:00:00.000Z");
        let b = item("b", "2026-10-24T10:00:00.000Z");
        assert_eq!(compare_recency(&a, &b), std::cmp::Ordering::Less);
        assert_eq!(compare_recency(&b, &a), std::cmp::Ordering::Greater);
    }

    /// cursor 往返：编码后能解回同样的三元组（与 IDE 的 base64url JSON 同形）。
    #[test]
    fn cursor_round_trips() {
        let m = item("mem-1", "2026-10-24T10:00:00.000Z");
        let encoded = encode_cursor(&m).expect("encode");
        let decoded = decode_cursor(&encoded).expect("decode");
        assert_eq!(decoded.id, "mem-1");
        assert_eq!(decoded.created_at, "2026-10-24T10:00:00.000Z");
        assert_eq!(decoded.scopes, vec!["Global".to_string()]);
        // base64url 不应出现 + / = 字符
        assert!(!encoded.contains('+') && !encoded.contains('/') && !encoded.contains('='));
    }

    #[test]
    fn cursor_rejects_garbage() {
        assert!(decode_cursor("!!!not-base64!!!").is_err());
        // 合法 base64 但不是我们的 JSON
        assert!(decode_cursor(&base64_url_encode(b"not json")).is_err());
    }

    /// limit 校验对齐 IDE 的 badPageSize：1..=1000，越界报错。
    #[test]
    fn page_limit_bounds() {
        assert_eq!(PAGE_DEFAULT, 100);
        assert_eq!(PAGE_MAX, 1_000);
        // 0 与 1001 都在区间外；这里只验证常量语义，实际报错路径需要数据库
        assert!(!(1..=PAGE_MAX).contains(&0));
        assert!(!(1..=PAGE_MAX).contains(&(PAGE_MAX + 1)));
        assert!((1..=PAGE_MAX).contains(&1));
        assert!((1..=PAGE_MAX).contains(&PAGE_MAX));
    }
}
