use crate::models::cli_session::{
    CliSession, CliSessionMeta, CliSessionMessage, CliSessionSummary,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

/// 会话摘要缓存：按 (大小, 修改时间) 判断文件未变就复用。
/// CLI 会话统计消息数要把 .jsonl 逐行读一遍，不缓存则每次刷新列表都重读。
struct CachedSummary {
    json_len: u64,
    json_modified: Option<SystemTime>,
    jsonl_len: u64,
    jsonl_modified: Option<SystemTime>,
    summary: CliSessionSummary,
}

pub struct CliSessionStorage {
    base_path: PathBuf,
    summaries: Mutex<HashMap<PathBuf, CachedSummary>>,
}

impl CliSessionStorage {
    pub fn new() -> Result<Self> {
        let base_path = Self::get_storage_path()?;
        Ok(Self {
            base_path,
            summaries: Mutex::new(HashMap::new()),
        })
    }

    fn get_storage_path() -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let home = std::env::var("USERPROFILE")
                .context("Failed to get USERPROFILE environment variable")?;
            Ok(PathBuf::from(home).join(".kiro").join("sessions").join("cli"))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let home =
                std::env::var("HOME").context("Failed to get HOME environment variable")?;
            Ok(PathBuf::from(home).join(".kiro").join("sessions").join("cli"))
        }
    }

    /// 列出所有 CLI sessions
    pub fn list_sessions(&self) -> Result<Vec<CliSessionSummary>> {
        let mut sessions = Vec::new();

        if !self.base_path.exists() {
            return Ok(sessions);
        }

        // 收集所有 .json 文件（不是 .jsonl）
        let mut json_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        for entry in fs::read_dir(&self.base_path).context("Failed to read CLI sessions directory")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let file_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                // 排除 .jsonl 文件（file_stem 已经去掉了 .json 后缀）
                if !file_name.ends_with(".jsonl") {
                    let modified = entry.metadata()?.modified()?;
                    json_files.push((path, modified));
                }
            }
        }

        // 按修改时间倒序
        json_files.sort_by(|a, b| b.1.cmp(&a.1));

        for (path, _) in json_files {
            match self.parse_session_summary(&path) {
                Ok(summary) => sessions.push(summary),
                Err(e) => {
                    log::warn!("Failed to parse CLI session {:?}: {}", path, e);
                }
            }
        }

        Ok(sessions)
    }

    /// 解析 session 摘要
    fn parse_session_summary(&self, json_path: &PathBuf) -> Result<CliSessionSummary> {
        let json_meta = fs::metadata(json_path)?;
        let json_len = json_meta.len();
        let json_modified = json_meta.modified().ok();

        let jsonl_path = json_path.with_extension("jsonl");
        let jsonl_meta = fs::metadata(&jsonl_path).ok();
        let jsonl_len = jsonl_meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let jsonl_modified = jsonl_meta.as_ref().and_then(|m| m.modified().ok());

        // 两个文件都未变则复用上次结果
        if let Ok(cache) = self.summaries.lock() {
            if let Some(cached) = cache.get(json_path) {
                if cached.json_len == json_len
                    && cached.json_modified == json_modified
                    && cached.jsonl_len == jsonl_len
                    && cached.jsonl_modified == jsonl_modified
                {
                    return Ok(cached.summary.clone());
                }
            }
        }

        let content = fs::read_to_string(json_path)
            .context("Failed to read session JSON")?;
        let meta: CliSessionMeta =
            serde_json::from_str(&content).context("Failed to parse session JSON")?;

        let file_size = json_len;

        // 统计 .jsonl 消息数
        let message_count = if jsonl_path.exists() {
            let file = fs::File::open(&jsonl_path)?;
            std::io::BufReader::new(file)
                .lines()
                .filter(|line| line.as_ref().is_ok_and(|l| !l.trim().is_empty()))
                .count()
        } else {
            0
        };

        // 提取模型名称和 agent 名称
        let (model_name, agent_name, context_usage, total_credits) =
            if let Some(ref state) = meta.session_state {
                let model = state
                    .rts_model_state
                    .as_ref()
                    .and_then(|s| s.model_info.as_ref())
                    .and_then(|m| m.model_name.clone());
                let agent = state.agent_name.clone();
                let ctx = state
                    .rts_model_state
                    .as_ref()
                    .and_then(|s| s.context_usage_percentage);
                let credits = state
                    .conversation_metadata
                    .as_ref()
                    .map(|cm| {
                        cm.user_turn_metadatas
                            .iter()
                            .flat_map(|t| &t.metering_usage)
                            .map(|u| u.value)
                            .sum::<f64>()
                    })
                    .unwrap_or(0.0);
                (model, agent, ctx, credits)
            } else {
                (None, None, None, 0.0)
            };

        let title = meta
            .title
            .unwrap_or_else(|| meta.cwd.split(['/', '\\']).last().unwrap_or("untitled").to_string());

        let summary = CliSessionSummary {
            session_id: meta.session_id,
            title,
            cwd: meta.cwd,
            model_name,
            agent_name,
            message_count,
            total_credits,
            context_usage,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            file_size,
        };

        if let Ok(mut cache) = self.summaries.lock() {
            cache.insert(
                json_path.clone(),
                CachedSummary {
                    json_len,
                    json_modified,
                    jsonl_len,
                    jsonl_modified,
                    summary: summary.clone(),
                },
            );
        }

        Ok(summary)
    }

    /// 加载完整 session（包含消息）
    pub fn load_session(&self, session_id: &str) -> Result<CliSession> {
        let json_path = self.base_path.join(format!("{session_id}.json"));
        if !json_path.exists() {
            anyhow::bail!("Session not found: {session_id}");
        }

        let content = fs::read_to_string(&json_path)?;
        let meta: CliSessionMeta = serde_json::from_str(&content)?;

        // 读取消息
        let jsonl_path = self.base_path.join(format!("{session_id}.jsonl"));
        let messages = if jsonl_path.exists() {
            let file = fs::File::open(&jsonl_path)?;
            std::io::BufReader::new(file)
                .lines()
                .filter_map(|line| {
                    line.ok().and_then(|l| {
                        if l.trim().is_empty() {
                            return None;
                        }
                        serde_json::from_str::<CliSessionMessage>(&l).ok()
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        let (model_name, agent_name, context_usage, permissions) =
            if let Some(ref state) = meta.session_state {
                let model = state
                    .rts_model_state
                    .as_ref()
                    .and_then(|s| s.model_info.as_ref())
                    .and_then(|m| m.model_name.clone());
                let agent = state.agent_name.clone();
                let ctx = state
                    .rts_model_state
                    .as_ref()
                    .and_then(|s| s.context_usage_percentage);
                let perms = state.permissions.clone();
                (model, agent, ctx, perms)
            } else {
                (None, None, None, None)
            };

        let title = meta
            .title
            .unwrap_or_else(|| meta.cwd.split(['/', '\\']).last().unwrap_or("untitled").to_string());

        Ok(CliSession {
            session_id: meta.session_id,
            title,
            cwd: meta.cwd,
            model_name,
            agent_name,
            context_usage,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            messages,
            permissions,
        })
    }

    /// 删除 session
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let extensions = ["json", "jsonl", "lock", "history"];
        for ext in extensions {
            let path = self.base_path.join(format!("{session_id}.{ext}"));
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        if let Ok(mut cache) = self.summaries.lock() {
            cache.remove(&self.base_path.join(format!("{session_id}.json")));
        }
        Ok(())
    }

    /// 搜索 sessions
    pub fn search_sessions(&self, query: &str) -> Result<Vec<CliSessionSummary>> {
        let all = self.list_sessions()?;
        let query_lower = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|s| {
                s.title.to_lowercase().contains(&query_lower)
                    || s.cwd.to_lowercase().contains(&query_lower)
                    || s.session_id.contains(&query_lower)
            })
            .collect())
    }

    /// 导出 session 为 markdown
    pub fn export_session_markdown(&self, session_id: &str) -> Result<String> {
        let session = self.load_session(session_id)?;
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", session.title));
        md.push_str(&format!("- **Session ID**: {}\n", session.session_id));
        md.push_str(&format!("- **CWD**: {}\n", session.cwd));
        if let Some(ref model) = session.model_name {
            md.push_str(&format!("- **Model**: {}\n", model));
        }
        if let Some(ref created) = session.created_at {
            md.push_str(&format!("- **Created**: {}\n", created));
        }
        md.push_str("\n---\n\n");

        for msg in &session.messages {
            let role = match msg.kind.as_str() {
                "Prompt" => "👤 User",
                "AssistantMessage" => "🤖 Assistant",
                _ => &msg.kind,
            };
            md.push_str(&format!("## {}\n\n", role));
            for content in &msg.data.content {
                md.push_str(&content.data);
                md.push('\n');
            }
            md.push_str("\n---\n\n");
        }

        Ok(md)
    }
}
