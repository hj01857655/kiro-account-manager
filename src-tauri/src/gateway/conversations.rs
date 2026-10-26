//! 对话捕获（opt-in）：把经网关的每轮对话（输入/输出分片）落到
//! `logs/gateway-conversations/<日期>/<会话哈希>/turns.jsonl`，供会话页回放。
//!
//! 边界（超限截断并置 cut，不丢结构）：
//! - 单请求 body 上限 8MB；单轮单方向 64KB 预算；单个 part 64KB（按字符安全截）
//! - 保留 7 天，按日期目录过期；全库总量上限 256MB，超限按目录从旧到新删
//! - 隐私：入库文本必须先过遮蔽引擎占位符化（调用方职责），本模块不做真值处理
//! - generation：清空时自增，在途捕获保存前校验，防止已删除数据被复活

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// 单请求 body 上限
pub const BODY_MAX: usize = 8 << 20;
/// 单轮单方向（input/output）字符预算
const DIRECTION_MAX: usize = 64 * 1024;
/// 单个 part 字符上限（按字符截，防 UTF-8 断裂）
const PART_MAX_CHARS: usize = 64 * 1024;
/// 保留天数
pub const RETENTION_DAYS: i64 = 7;
/// 全库总量上限（字节）
pub const STORE_MAX_BYTES: u64 = 256 << 20;

/// 一条对话分片。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TurnPart {
    /// system / user / assistant / tool
    pub role: String,
    /// context（系统提示/工具定义/注入内容）/ text / tool_use / tool_result / image
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub text: String,
    /// 被截掉的字符数（0 = 未截断）
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cut: usize,
}

fn is_zero(v: &usize) -> bool {
    *v == 0
}

/// 一轮对话（一次网关请求）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationTurn {
    pub occurred_at: String,
    pub request_id: String,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub status_code: u16,
    pub input: Vec<TurnPart>,
    pub output: Vec<TurnPart>,
    #[serde(default)]
    pub cut: bool,
}

/// 按字符安全截断（UTF-8 边界），返回 (text, 被截字符数)。
fn truncate_chars(s: &str, max: usize) -> (String, usize) {
    if s.chars().count() <= max {
        return (s.to_string(), 0);
    }
    let cut: String = s.chars().take(max).collect();
    (cut, s.chars().count() - max)
}

/// 限长后加入 part：超过方向预算则丢弃并置 cut。
fn add_part(out: &mut Vec<TurnPart>, direction_total: &mut usize, p: TurnPart, cut: &mut bool) {
    if *direction_total >= DIRECTION_MAX {
        *cut = true;
        return;
    }
    *direction_total += p.text.chars().count();
    out.push(p);
}

/// 会话目录名：sha256("agent\x00session") 前 16 位 hex——原始 id 不进路径。
pub fn session_dir_name(agent: Option<&str>, session_id: &str) -> String {
    let mut h = Sha256::new();
    h.update(agent.unwrap_or("-").as_bytes());
    h.update(b"\x00");
    h.update(session_id.as_bytes());
    let bytes = h.finalize();
    bytes[..8].iter().map(|b| format!("{:02x}", b)).collect()
}

/// 从客户端请求体解析输入分片（system/工具定义归 context，消息按块分类）。
/// body 非法 JSON 返回 None。
pub fn parse_input(body: &str) -> Option<(Vec<TurnPart>, bool)> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let mut out: Vec<TurnPart> = Vec::new();
    let mut dir_total = 0usize;
    let mut cut = false;

    // system：字符串或 block 数组
    match v.get("system") {
        Some(serde_json::Value::String(s)) => {
            let (t, c) = truncate_chars(s, PART_MAX_CHARS);
            add_part(
                &mut out,
                &mut dir_total,
                TurnPart {
                    role: "system".into(),
                    kind: "context".into(),
                    name: Some("System prompt".into()),
                    text: t,
                    cut: c,
                },
                &mut cut,
            );
        }
        Some(serde_json::Value::Array(blocks)) => {
            let joined: String = blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            let (t, c) = truncate_chars(&joined, PART_MAX_CHARS);
            add_part(
                &mut out,
                &mut dir_total,
                TurnPart {
                    role: "system".into(),
                    kind: "context".into(),
                    name: Some("System prompt".into()),
                    text: t,
                    cut: c,
                },
                &mut cut,
            );
        }
        _ => {}
    }

    // tools：定义 JSON 原文归 context
    if let Some(tools) = v.get("tools").and_then(|t| t.as_array()) {
        if !tools.is_empty() {
            let raw = serde_json::to_string(tools).unwrap_or_default();
            let (t, c) = truncate_chars(&raw, PART_MAX_CHARS);
            add_part(
                &mut out,
                &mut dir_total,
                TurnPart {
                    role: "system".into(),
                    kind: "context".into(),
                    name: Some("Tools".into()),
                    text: t,
                    cut: c,
                },
                &mut cut,
            );
        }
    }

    // messages：逐块分类
    if let Some(messages) = v.get("messages").and_then(|m| m.as_array()) {
        for m in messages {
            let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            match m.get("content") {
                Some(serde_json::Value::String(s)) => {
                    let (t, c) = truncate_chars(s, PART_MAX_CHARS);
                    add_part(
                        &mut out,
                        &mut dir_total,
                        TurnPart {
                            role: role.into(),
                            kind: "text".into(),
                            name: None,
                            text: t,
                            cut: c,
                        },
                        &mut cut,
                    );
                }
                Some(serde_json::Value::Array(blocks)) => {
                    for b in blocks {
                        let (kind, name, text): (&str, Option<&str>, String) =
                            match b.get("type").and_then(|t| t.as_str()) {
                                Some("tool_use") => (
                                    "tool_use",
                                    b.get("name").and_then(|n| n.as_str()),
                                    b.get("input")
                                        .map(|i| serde_json::to_string(i).unwrap_or_default())
                                        .unwrap_or_default(),
                                ),
                                Some("tool_result") => (
                                    "tool_result",
                                    None,
                                    match b.get("content") {
                                        Some(serde_json::Value::String(s)) => s.clone(),
                                        Some(other) => other.to_string(),
                                        None => String::new(),
                                    },
                                ),
                                Some("image") => ("image".into(), None, "[image]".into()),
                                Some("text") => {
                                    let is_inject = b
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .map(|t| {
                                            let s = t.trim_start();
                                            s.starts_with('<')
                                                || s.starts_with("# AGENTS.md")
                                                || s.starts_with("# CLAUDE.md")
                                        })
                                        .unwrap_or(false);
                                    if is_inject {
                                        ("context", None, b.get("text").and_then(|t| t.as_str()).unwrap_or("").into())
                                    } else {
                                        ("text", None, b.get("text").and_then(|t| t.as_str()).unwrap_or("").into())
                                    }
                                }
                                _ => ("text", None, b.to_string()),
                            };
                        let (t, c) = truncate_chars(&text, PART_MAX_CHARS);
                        add_part(
                            &mut out,
                            &mut dir_total,
                            TurnPart {
                                role: if kind == "tool_result" { "tool".into() } else { role.into() },
                                kind: kind.into(),
                                name: name.map(str::to_string),
                                text: t,
                                cut: c,
                            },
                            &mut cut,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    Some((out, cut))
}

/// 从响应体解析输出分片：JSON body 取 content 文本块；非法 JSON 记 cut。
pub fn parse_output(response_body: Option<&str>) -> (Vec<TurnPart>, bool) {
    let Some(body) = response_body else {
        return (Vec::new(), false);
    };
    let mut out: Vec<TurnPart> = Vec::new();
    let mut dir_total = 0usize;
    let mut cut = false;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        let blocks = v
            .get("content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        for b in &blocks {
            let (kind, text) = match b.get("type").and_then(|t| t.as_str()) {
                Some("text") => ("text", b.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string()),
                Some("tool_use") => (
                    "tool_use",
                    format!(
                        "{} {}",
                        b.get("name").and_then(|n| n.as_str()).unwrap_or("tool"),
                        b.get("input").map(|i| serde_json::to_string(i).unwrap_or_default()).unwrap_or_default()
                    ),
                ),
                Some("thinking") => ("text", b.get("thinking").and_then(|t| t.as_str()).unwrap_or("").to_string()),
                _ => ("text", b.to_string()),
            };
            let (t, c) = truncate_chars(&text, PART_MAX_CHARS);
            add_part(
                &mut out,
                &mut dir_total,
                TurnPart {
                    role: "assistant".into(),
                    kind: kind.into(),
                    name: None,
                    text: t,
                    cut: c,
                },
                &mut cut,
            );
        }
    } else if !body.trim().is_empty() {
        cut = true;
    }
    (out, cut)
}

/// 捕获会话的存储根目录。
pub fn store_root() -> PathBuf {
    crate::gateway::config_dir()
        .join("logs")
        .join("gateway-conversations")
}

/// 保存一轮对话。generation 校验失败（录制被清空过）返回 Ok(false)。
pub fn save_turn(
    root: &Path,
    turn: &ConversationTurn,
    expected_generation: u64,
    current_generation: &AtomicU64,
) -> Result<bool, String> {
    if current_generation.load(Ordering::SeqCst) != expected_generation {
        return Ok(false);
    }
    let dir_name = session_dir_name(turn.agent.as_deref(), &turn.session_id);
    let dir = root.join(&turn.occurred_at[..10.min(turn.occurred_at.len())]).join(dir_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建对话目录失败: {e}"))?;
    let mut line = serde_json::to_string(turn).map_err(|e| format!("序列化对话失败: {e}"))?;
    line.push('\n');
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("turns.jsonl"))
        .map_err(|e| format!("打开对话文件失败: {e}"))?;
    f.write_all(line.as_bytes()).map_err(|e| format!("写入对话失败: {e}"))?;
    Ok(true)
}

/// 读取某会话的全部轮次（时间正序）。
pub fn load_turns(root: &Path, agent: Option<&str>, session_id: &str) -> Result<Vec<ConversationTurn>, String> {
    let dir_name = session_dir_name(agent, session_id);
    let mut turns = Vec::new();
    if !root.exists() {
        return Ok(turns);
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(root)
        .map_err(|e| format!("读取对话目录失败: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for day_dir in entries {
        let file = day_dir.join(&dir_name).join("turns.jsonl");
        if !file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&file).map_err(|e| format!("读取对话文件失败: {e}"))?;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(t) = serde_json::from_str::<ConversationTurn>(line) {
                turns.push(t);
            }
        }
    }
    turns.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
    Ok(turns)
}

/// 会话摘要：目录名（会话 id 哈希）+ 轮次数 + 最近活动时间。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvSessionSummary {
    pub dir: String,
    pub turns: usize,
    pub last: String,
}

/// 列出有捕获记录的会话（按轮次数倒序）。
pub fn list_sessions(root: &Path) -> Result<Vec<ConvSessionSummary>, String> {
    let mut map: std::collections::HashMap<String, (usize, String)> =
        std::collections::HashMap::new();
    if !root.exists() {
        return Ok(Vec::new());
    }
    for day in std::fs::read_dir(root).map_err(|e| format!("读取对话目录失败: {e}"))? {
        let day = day.map_err(|e| e.to_string())?.path();
        if !day.is_dir() {
            continue;
        }
        for sess in std::fs::read_dir(&day).map_err(|e| e.to_string())? {
            let sess = sess.map_err(|e| e.to_string())?.path();
            let file = sess.join("turns.jsonl");
            if !file.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&file).unwrap_or_default();
            let lines: Vec<&str> = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect();
            if lines.is_empty() {
                continue;
            }
            let dir_name = sess.file_name().unwrap_or_default().to_string_lossy().to_string();
            let entry = map.entry(dir_name).or_insert_with(|| (0, String::new()));
            entry.0 += lines.len();
            // 最近活动：最后一行的 occurredAt（文件按日期目录组织，直接比字符串）
            if let Ok(last) = serde_json::from_str::<ConversationTurn>(lines[lines.len() - 1]) {
                if last.occurred_at > entry.1 {
                    entry.1 = last.occurred_at;
                }
            }
        }
    }
    let mut out: Vec<ConvSessionSummary> = map
        .into_iter()
        .map(|(dir, (turns, last))| ConvSessionSummary { dir, turns, last })
        .collect();
    out.sort_by(|a, b| b.turns.cmp(&a.turns).then(b.last.cmp(&a.last)));
    Ok(out)
}

/// 按会话目录名读取全部轮次（时间正序）。原始 session id 在每轮的 sessionId 字段里。
pub fn load_turns_by_dir(root: &Path, dir_name: &str) -> Result<Vec<ConversationTurn>, String> {
    let mut turns = Vec::new();
    if !root.exists() {
        return Ok(turns);
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(root)
        .map_err(|e| format!("读取对话目录失败: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for day_dir in entries {
        let file = day_dir.join(dir_name).join("turns.jsonl");
        if !file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&file).map_err(|e| format!("读取对话文件失败: {e}"))?;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(t) = serde_json::from_str::<ConversationTurn>(line) {
                turns.push(t);
            }
        }
    }
    turns.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
    Ok(turns)
}

/// 清空全部捕获（删除根目录）。
pub fn clear(root: &Path) -> Result<(), String> {
    if root.exists() {
        std::fs::remove_dir_all(root).map_err(|e| format!("清空对话捕获失败: {e}"))?;
    }
    Ok(())
}

/// 过期清理：删除 7 天前的日期目录 + 全库超限时从旧目录删到限额内。
pub fn prune(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(RETENTION_DAYS))
        .format("%Y-%m-%d")
        .to_string();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .map_err(|e| format!("读取对话目录失败: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    // 日期目录名即 YYYY-MM-DD，字典序对比即可
    for d in dirs.clone() {
        let name = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.len() == 10 && name.as_str() < cutoff.as_str() {
            let _ = std::fs::remove_dir_all(&d);
        }
    }
    // 全库总量
    fn dir_size(p: &Path) -> u64 {
        let mut s = 0;
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    s += dir_size(&p);
                } else if let Ok(m) = e.metadata() {
                    s += m.len();
                }
            }
        }
        s
    }
    let fresh: Vec<PathBuf> = std::fs::read_dir(root)
        .map_err(|e| format!("读取对话目录失败: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    let mut total = fresh.iter().map(|d| dir_size(d)).sum::<u64>();
    for d in &fresh {
        if total <= STORE_MAX_BYTES {
            break;
        }
        let size = dir_size(d);
        if std::fs::remove_dir_all(d).is_ok() {
            total -= size;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anthropic_body_into_classified_parts() {
        let body = r#"{
            "system": "you are kiro",
            "tools": [{"name": "read_file"}],
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "<system-reminder>remember</system-reminder>"},
                    {"type": "text", "text": "please read the file"},
                    {"type": "tool_use", "name": "read_file", "input": {"path": "a.txt"}},
                    {"type": "tool_result", "content": "file contents here"}
                ]}
            ]
        }"#;
        let (parts, cut) = parse_input(body).unwrap();
        assert!(!cut);
        assert_eq!(parts[0].role, "system");
        assert_eq!(parts[0].name.as_deref(), Some("System prompt"));
        assert!(parts.iter().any(|p| p.name.as_deref() == Some("Tools")));
        assert!(parts.iter().any(|p| p.kind == "context" && p.text.contains("remember")));
        assert!(parts.iter().any(|p| p.kind == "text" && p.text.contains("please read")));
        assert!(parts.iter().any(|p| p.kind == "tool_use" && p.name.as_deref() == Some("read_file")));
        assert!(parts.iter().any(|p| p.kind == "tool_result" && p.role == "tool"));
    }

    #[test]
    fn part_truncation_is_char_safe_and_counted() {
        let s = "好".repeat(PART_MAX_CHARS + 10);
        let (t, cut) = truncate_chars(&s, PART_MAX_CHARS);
        assert_eq!(t.chars().count(), PART_MAX_CHARS);
        assert_eq!(cut, 10);
    }

    #[test]
    fn session_dir_is_stable_and_hides_id() {
        let a = session_dir_name(Some("claude-code"), "sess-1");
        let b = session_dir_name(Some("claude-code"), "sess-1");
        let c = session_dir_name(Some("claude-code"), "sess-2");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(!a.contains("sess-1"));
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn save_load_roundtrip_and_generation_guard() {
        let root = std::env::temp_dir().join(format!("conv-test-{}", std::process::id()));
        let _ = clear(&root);
        let gen = AtomicU64::new(1);
        let turn = ConversationTurn {
            occurred_at: "2026-10-26 10:00:00".into(),
            request_id: "r1".into(),
            session_id: "sess-1".into(),
            agent: Some("claude-code".into()),
            model: Some("m".into()),
            status_code: 200,
            input: vec![TurnPart { role: "user".into(), kind: "text".into(), name: None, text: "hi".into(), cut: 0 }],
            output: vec![TurnPart { role: "assistant".into(), kind: "text".into(), name: None, text: "hello".into(), cut: 0 }],
            cut: false,
        };
        assert!(save_turn(&root, &turn, 1, &gen).unwrap());
        let loaded = load_turns(&root, Some("claude-code"), "sess-1").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].input[0].text, "hi");
        // generation 变更后写入被拒
        gen.store(2, Ordering::SeqCst);
        assert!(!save_turn(&root, &turn, 1, &gen).unwrap());
        let _ = clear(&root);
    }

    #[test]
    fn parse_output_extracts_assistant_text_and_flags_bad_json() {
        let (parts, cut) = parse_output(Some(r#"{"content":[{"type":"text","text":"answer"}]}"#));
        assert!(!cut);
        assert_eq!(parts[0].role, "assistant");
        assert_eq!(parts[0].text, "answer");
        let (_, cut2) = parse_output(Some("not json"));
        assert!(cut2);
        let (empty, cut3) = parse_output(None);
        assert!(empty.is_empty() && !cut3);
    }
}
