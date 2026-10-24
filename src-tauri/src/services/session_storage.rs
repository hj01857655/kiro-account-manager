use crate::models::ide_session::{IdeSession, SessionSummary};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

// 安全限制
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB

/// 会话文件的解析结果缓存。
///
/// 会话文件随对话不断变长（实测单文件最大 18MB，整个目录 100MB），而列表只需要
/// 标题与消息数。按 (文件大小, 修改时间) 判断文件是否未变，未变则复用上次解析结果，
/// 避免每次刷新列表都反序列化整份 history。
struct CachedSummary {
    file_len: u64,
    modified: Option<SystemTime>,
    summary: SessionSummary,
}

pub struct SessionStorage {
    base_path: PathBuf,
    summaries: Mutex<HashMap<PathBuf, CachedSummary>>,
}

impl SessionStorage {
    pub fn new() -> Result<Self> {
        let base_path = Self::get_storage_path()?;
        Ok(Self {
            base_path,
            summaries: Mutex::new(HashMap::new()),
        })
    }

    fn invalidate(&self, path: Option<&PathBuf>) {
        if let Ok(mut cache) = self.summaries.lock() {
            match path {
                Some(path) => {
                    cache.remove(path);
                }
                None => cache.clear(),
            }
        }
    }

    /// 验证路径组件是否安全（防止路径遍历）
    fn is_safe_path_component(component: &str) -> bool {
        // 只允许字母、数字、下划线、连字符和点号
        // 不允许路径分隔符和特殊字符
        !component.is_empty()
            && !component.contains("..")
            && !component.contains('/')
            && !component.contains('\\')
            && component
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    }

    fn get_storage_path() -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let appdata =
                std::env::var("APPDATA").context("Failed to get APPDATA environment variable")?;
            Ok(PathBuf::from(appdata)
                .join("Kiro")
                .join("User")
                .join("globalStorage")
                .join("kiro.kiroagent")
                .join("workspace-sessions"))
        }

        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").context("Failed to get HOME environment variable")?;
            Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Kiro")
                .join("User")
                .join("globalStorage")
                .join("kiro.kiroagent")
                .join("workspace-sessions"))
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").context("Failed to get HOME environment variable")?;
            Ok(PathBuf::from(home)
                .join(".config")
                .join("Kiro")
                .join("User")
                .join("globalStorage")
                .join("kiro.kiroagent")
                .join("workspace-sessions"))
        }
    }

    /// 列出所有 workspace
    pub fn list_workspaces(&self) -> Result<Vec<String>> {
        let mut workspaces = Vec::new();

        if !self.base_path.exists() {
            return Ok(workspaces);
        }

        // 收集工作区及其修改时间
        let mut workspace_with_time: Vec<(String, std::time::SystemTime)> = Vec::new();

        for entry in
            fs::read_dir(&self.base_path).context("Failed to read workspace-sessions directory")?
        {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                // 跳过 Kiro SessionMigration 的迁移标记目录（._migration- / .migrated-）：
                // 它们不是 workspace，迁移触发时会出现在 workspace-sessions 下，
                // 不过滤会污染会话列表
                if name.starts_with("._migration-") || name.starts_with(".migrated-") {
                    continue;
                }
                let modified = entry.metadata()?.modified()?;
                workspace_with_time.push((name, modified));
            }
        }

        // 按修改时间倒序排序（最近使用的在前）
        workspace_with_time.sort_by(|a, b| b.1.cmp(&a.1));

        // 只返回名称
        workspaces = workspace_with_time
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        Ok(workspaces)
    }

    /// 按项目目录查找 workspace：遍历各 workspace 的会话摘要，
    /// 归一化比较 workspaceDirectory（Windows 大小写/斜杠差异不敏感）。
    /// 目录名是 base64 变体、直接编码容易踩填充/字符集差异，路径精确匹配才稳。
    pub fn find_workspace_by_dir(&self, project_dir: &str) -> Result<Option<String>> {
        let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let target = norm(project_dir);
        for hash in self.list_workspaces()? {
            if let Ok(sessions) = self.list_sessions(&hash) {
                if let Some(first) = sessions.first() {
                    if norm(&first.workspace_directory) == target {
                        return Ok(Some(hash));
                    }
                }
            }
        }
        Ok(None)
    }

    /// 列出指定 workspace 的所有 sessions
    pub fn list_sessions(&self, workspace_hash: &str) -> Result<Vec<SessionSummary>> {
        // 安全检查：防止路径遍历攻击
        if !Self::is_safe_path_component(workspace_hash) {
            log::warn!("[安全] 检测到非法的 workspace_hash: {}", workspace_hash);
            return Err(anyhow::anyhow!("Invalid workspace hash"));
        }

        let workspace_path = self.base_path.join(workspace_hash);
        let mut sessions = Vec::new();

        if !workspace_path.exists() {
            log::warn!("Workspace directory does not exist: {}", workspace_hash);
            return Ok(sessions);
        }

        for entry in fs::read_dir(&workspace_path).context(format!(
            "Failed to read workspace directory: {}",
            workspace_hash
        ))? {
            let entry = entry?;
            let path = entry.path();

            // 只处理 .json 文件
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // 跳过 sessions.json（索引文件）
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if filename == "sessions.json" {
                    continue;
                }
            }

            match self.load_session_summary(&path, workspace_hash) {
                Ok(summary) => sessions.push(summary),
                Err(e) => {
                    log::error!("Failed to load session from {:?}: {}", path, e);
                    // 继续处理其他文件
                }
            }
        }

        // 按修改时间倒序排序
        sessions.sort_by(|a, b| b.modified_at.unwrap_or(0).cmp(&a.modified_at.unwrap_or(0)));

        Ok(sessions)
    }

    /// 加载 session 摘要
    fn load_session_summary(&self, path: &PathBuf, workspace_hash: &str) -> Result<SessionSummary> {
        let metadata =
            fs::metadata(path).context(format!("Failed to read metadata for {:?}", path))?;

        // 安全检查：文件大小限制
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("File too large: {} bytes", metadata.len()));
        }

        let file_len = metadata.len();
        let modified = metadata.modified().ok();

        // 文件未变则复用上次解析结果：列表刷新不再反序列化整份 history
        if let Ok(cache) = self.summaries.lock() {
            if let Some(cached) = cache.get(path) {
                if cached.file_len == file_len && cached.modified == modified {
                    return Ok(cached.summary.clone());
                }
            }
        }

        let content =
            fs::read_to_string(path).context(format!("Failed to read file {:?}", path))?;

        let session: IdeSession = serde_json::from_str(&content)
            .map_err(|e| {
                log::error!("JSON parse error for {:?}: {}", path, e);
                // 打印前 500 个字符帮助调试
                log::error!(
                    "File content preview: {}",
                    &content.chars().take(500).collect::<String>()
                );
                e
            })
            .context(format!("Failed to parse JSON from {:?}", path))?;

        let summary = SessionSummary {
            session_id: session.session_id,
            title: session.title,
            session_type: session.session_type,
            workspace_directory: session.workspace_directory,
            workspace_hash: workspace_hash.to_string(),
            message_count: session.history.len(),
            file_size: metadata.len(),
            created_at: metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
        };

        if let Ok(mut cache) = self.summaries.lock() {
            cache.insert(
                path.clone(),
                CachedSummary {
                    file_len,
                    modified,
                    summary: summary.clone(),
                },
            );
        }

        Ok(summary)
    }

    /// 加载完整 session
    pub fn load_session(&self, workspace_hash: &str, session_id: &str) -> Result<IdeSession> {
        // 安全检查：防止路径遍历攻击
        if !Self::is_safe_path_component(workspace_hash)
            || !Self::is_safe_path_component(session_id)
        {
            log::warn!(
                "[安全] 检测到非法的路径参数: workspace_hash={}, session_id={}",
                workspace_hash,
                session_id
            );
            return Err(anyhow::anyhow!("Invalid path parameters"));
        }

        let path = self
            .base_path
            .join(workspace_hash)
            .join(format!("{}.json", session_id));

        // 安全检查：文件大小限制
        let metadata = fs::metadata(&path).context(format!(
            "Failed to read metadata for session: {}",
            session_id
        ))?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!(
                "Session file too large: {} bytes",
                metadata.len()
            ));
        }

        let content = fs::read_to_string(&path)
            .context(format!("Failed to read session file: {}", session_id))?;
        let session = serde_json::from_str(&content).context("Failed to parse session JSON")?;
        Ok(session)
    }

    /// 删除 session
    pub fn delete_session(&self, workspace_hash: &str, session_id: &str) -> Result<()> {
        // 安全检查：防止路径遍历攻击
        if !Self::is_safe_path_component(workspace_hash)
            || !Self::is_safe_path_component(session_id)
        {
            log::warn!(
                "[安全] 检测到非法的路径参数: workspace_hash={}, session_id={}",
                workspace_hash,
                session_id
            );
            return Err(anyhow::anyhow!("Invalid path parameters"));
        }

        let path = self
            .base_path
            .join(workspace_hash)
            .join(format!("{}.json", session_id));

        fs::remove_file(&path).context(format!("Failed to delete session: {}", session_id))?;
        self.invalidate(Some(&path));

        Ok(())
    }

    /// 删除整个工作区目录
    pub fn delete_workspace(&self, workspace_hash: &str) -> Result<()> {
        // 安全检查：防止路径遍历攻击
        if !Self::is_safe_path_component(workspace_hash) {
            log::warn!("[安全] 检测到非法的 workspace_hash: {}", workspace_hash);
            return Err(anyhow::anyhow!("Invalid workspace hash"));
        }

        let workspace_path = self.base_path.join(workspace_hash);

        if workspace_path.exists() {
            fs::remove_dir_all(&workspace_path)
                .context(format!("Failed to delete workspace: {}", workspace_hash))?;
            // 整个工作区没了，缓存里的路径全部作废
            self.invalidate(None);
        }

        Ok(())
    }

    /// 导出 session
    pub fn export_session(
        &self,
        workspace_hash: &str,
        session_id: &str,
        format: ExportFormat,
    ) -> Result<String> {
        // 安全检查已在 load_session 中完成
        let session = self.load_session(workspace_hash, session_id)?;

        match format {
            ExportFormat::Json => serde_json::to_string_pretty(&session)
                .context("Failed to serialize session to JSON"),
            ExportFormat::Markdown => Ok(self.session_to_markdown(&session)),
        }
    }

    fn session_to_markdown(&self, session: &IdeSession) -> String {
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", session.title));
        md.push_str(&format!("- **Session ID**: {}\n", session.session_id));
        md.push_str(&format!("- **Type**: {}\n", session.session_type));
        md.push_str(&format!(
            "- **Workspace**: {}\n",
            session.workspace_directory
        ));
        md.push_str(&format!("- **Messages**: {}\n\n", session.history.len()));
        md.push_str("---\n\n");

        for (i, item) in session.history.iter().enumerate() {
            md.push_str(&format!("## Message {}\n\n", i + 1));
            md.push_str(&format!(
                "**{}**:\n\n",
                if item.message.role == "user" {
                    "User"
                } else {
                    "Assistant"
                }
            ));

            for content in &item.message.content {
                md.push_str(&format!("{}\n\n", content.text));
            }

            md.push_str("---\n\n");
        }

        md
    }
}

pub enum ExportFormat {
    Json,
    Markdown,
}
