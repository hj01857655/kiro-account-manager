// Custom Agents 管理（读取/编辑 ~/.kiro/agents/*.json 和 <project>/.kiro/agents/*.json）
//
// 版本兼容：Kiro IDE 1.0 起 custom agent 从 Markdown 改为 JSON 对象，字段为
// name / description / model / tools / allowedTools / resources /
// includeMcpJson / hooks / prompt。此处同时接受 .md 与 .json，
// 使 IDE 0.x 遗留的 .md agent 仍可被列出与编辑。
// 同目录下的 <name>.lock 由 IDE 自行维护，扩展名不匹配，不会被扫描。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgentFile {
    pub file_name: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<String>,
    /// "user" 或 "project"
    pub scope: String,
}

pub struct CustomAgentsManager;

impl CustomAgentsManager {
    pub fn user_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("agents"))
    }

    pub fn project_dir(project_dir: &str) -> PathBuf {
        PathBuf::from(project_dir).join(".kiro").join("agents")
    }

    fn load_from_dir(dir: &PathBuf, scope: &str) -> Result<Vec<CustomAgentFile>, String> {
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut files = vec![];

        for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();

            // IDE 1.0 起为 .json；保留 .md 以兼容 IDE 0.x 遗留文件
            if path.extension().is_some_and(|e| e == "md" || e == "json") {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let metadata = fs::metadata(&path).ok();
                let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
                let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
                    let datetime: chrono::DateTime<chrono::Local> = t.into();
                    datetime.format("%Y/%m/%d %H:%M:%S").to_string()
                });

                let content = fs::read_to_string(&path).unwrap_or_default();

                files.push(CustomAgentFile {
                    file_name,
                    content,
                    size,
                    modified_at,
                    scope: scope.to_string(),
                });
            }
        }

        files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        Ok(files)
    }

    fn resolve_dir(scope: &str, project_dir: Option<&str>) -> Result<PathBuf, String> {
        match scope {
            "project" => {
                let pd = project_dir.ok_or("项目级操作需要提供项目目录")?;
                Ok(Self::project_dir(pd))
            }
            _ => Self::user_dir().ok_or_else(|| "无法获取用户目录".to_string()),
        }
    }

    fn validate_file_name(file_name: &str) -> Result<(), String> {
        if file_name.is_empty() {
            return Err("文件名不能为空".to_string());
        }
        if file_name.contains('/') || file_name.contains('\\') {
            return Err("文件名不能包含路径分隔符".to_string());
        }
        if file_name.contains("..") {
            return Err("文件名不能包含 ..".to_string());
        }

        let path = Path::new(file_name);
        for comp in path.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("文件名非法".to_string());
            }
        }
        Ok(())
    }

    fn safe_agent_path(base_dir: &Path, file_name: &str) -> Result<PathBuf, String> {
        Self::validate_file_name(file_name)?;
        let candidate = base_dir.join(file_name);

        if !candidate.starts_with(base_dir) {
            return Err("非法路径".to_string());
        }

        Ok(candidate)
    }

    pub fn load_all(project_dir: Option<&str>) -> Result<Vec<CustomAgentFile>, String> {
        let mut all_files = vec![];

        if let Some(dir) = Self::user_dir() {
            all_files.extend(Self::load_from_dir(&dir, "user")?);
        }

        if let Some(pd) = project_dir {
            let dir = Self::project_dir(pd);
            all_files.extend(Self::load_from_dir(&dir, "project")?);
        }

        Ok(all_files)
    }

    pub fn load(
        file_name: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<CustomAgentFile, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let path = Self::safe_agent_path(&dir, file_name)?;

        if !path.exists() {
            return Err(format!("Agent 文件不存在: {file_name}"));
        }

        let content = fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {e}"))?;

        let metadata = fs::metadata(&path).ok();
        let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y/%m/%d %H:%M:%S").to_string()
        });

        Ok(CustomAgentFile {
            file_name: file_name.to_string(),
            content,
            size,
            modified_at,
            scope: scope.to_string(),
        })
    }

    pub fn save(
        file_name: &str,
        content: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        fs::create_dir_all(&dir).ok();

        let path = Self::safe_agent_path(&dir, file_name)?;
        fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
        // .json agent 保存不阻塞草稿，但把 IDE 会解析失败的问题带回前端展示
        Ok(Self::diagnose_content(file_name, content))
    }

    /// .json agent 内容诊断（对齐 1.1.70 bundle ProfileLoader 行为）：
    /// - JSON 解析失败 → IDE 抛 AgentFileFormatError（invalid_config），**该 agent 不加载**
    /// - `allowedTools` / `toolsSettings` 是 CLI-only 字段（bundle gpr）：
    ///   无其他有效字段时整个文件被按 cli_only_agent 跳过，有则忽略字段并警告
    /// - `.md` agent 走 frontmatter 路径（vZ），无 frontmatter 也会解析失败——仅诊断不拦截
    fn diagnose_content(file_name: &str, content: &str) -> Vec<String> {
        let mut issues = Vec::new();
        if file_name.ends_with(".json") {
            let parsed: serde_json::Value = match serde_json::from_str(content) {
                Ok(value) => value,
                Err(e) => {
                    issues.push(format!("JSON 解析失败（IDE 将不加载该 agent）: {e}"));
                    return issues;
                }
            };
            let Some(obj) = parsed.as_object() else {
                issues.push("agent 文件根节点必须是 JSON 对象（IDE 将不加载）".to_string());
                return issues;
            };
            if obj.contains_key("allowedTools") || obj.contains_key("toolsSettings") {
                issues.push(
                    "allowedTools / toolsSettings 是 kiro-cli 专属字段，IDE 侧会忽略；若没有 IDE 侧字段（name/prompt/tools…），整个 agent 会被跳过"
                        .to_string(),
                );
            }
            if !obj.keys().any(|k| k != "allowedTools" && k != "toolsSettings") {
                issues.push("文件只包含 CLI 专属字段，IDE 将按 cli_only_agent 跳过".to_string());
            }
            if let Some(name) = obj.get("name").and_then(serde_json::Value::as_str) {
                if name.trim().is_empty() {
                    issues.push("name 为空字符串（IDE 侧 name 用于 agent 列表显示）".to_string());
                }
            }
        } else if file_name.ends_with(".md") && !content.trim_start().starts_with("---") {
            issues.push(
                ".md agent 需要 YAML frontmatter（name/prompt 等），缺失时 IDE 解析会失败".to_string(),
            );
        }
        issues
    }

    pub fn delete(file_name: &str, scope: &str, project_dir: Option<&str>) -> Result<(), String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let path = Self::safe_agent_path(&dir, file_name)?;

        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("删除失败: {e}"))?;
        }

        Ok(())
    }

    pub fn create(
        file_name: &str,
        content: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<CustomAgentFile, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        fs::create_dir_all(&dir).ok();

        let path = Self::safe_agent_path(&dir, file_name)?;

        if path.exists() {
            return Err(format!("文件已存在: {file_name}"));
        }

        // 创建即入库：解析失败的 agent IDE 不加载，这里硬拦截
        let issues = Self::diagnose_content(file_name, content);
        if !issues.is_empty() {
            return Err(issues.join("；"));
        }

        fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;

        Self::load(file_name, scope, project_dir)
    }
}

#[cfg(test)]
mod diagnose_tests {
    use super::CustomAgentsManager;

    #[test]
    fn agent_json_diagnosis_matches_profile_loader_behavior() {
        // 合法 json agent
        let ok = r#"{"name":"plan","prompt":"You are a planner","tools":["read_file"]}"#;
        assert!(CustomAgentsManager::diagnose_content("plan.json", ok).is_empty());

        // 坏 JSON → IDE 解析失败不加载
        let issues = CustomAgentsManager::diagnose_content("broken.json", "{oops");
        assert!(issues.iter().any(|i| i.contains("JSON 解析失败")));

        // 根不是对象
        let issues = CustomAgentsManager::diagnose_content("arr.json", "[]");
        assert!(issues.iter().any(|i| i.contains("对象")));

        // CLI-only 字段（bundle gpr：allowedTools/toolsSettings）
        let cli_only = r#"{"allowedTools":["read_file"],"toolsSettings":{}}"#;
        let issues = CustomAgentsManager::diagnose_content("cli.json", cli_only);
        assert!(issues.iter().any(|i| i.contains("cli_only_agent") || i.contains("CLI 专属")));

        // 混合：有 IDE 字段 + CLI 字段 → 只提示忽略
        let mixed = r#"{"name":"a","prompt":"p","allowedTools":["read_file"]}"#;
        let issues = CustomAgentsManager::diagnose_content("mixed.json", mixed);
        assert!(issues.len() == 1 && issues[0].contains("忽略"));

        // .md 无 frontmatter
        let issues = CustomAgentsManager::diagnose_content("legacy.md", "plain body");
        assert!(issues.iter().any(|i| i.contains("frontmatter")));
    }
}
