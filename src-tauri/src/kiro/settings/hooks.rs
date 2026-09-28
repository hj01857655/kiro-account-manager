// Hooks 管理（<project>/.kiro/hooks/）
//
// 版本兼容：Kiro IDE 1.0 把 hook 格式整体换掉了——
//   IDE 0.x：单文件单 hook，扩展名 .kiro.hook，字段 when.type / then.type
//   IDE 1.0：单文件可含多个 hook，扩展名 .json，顶层 {version, hooks:[...]}，
//            字段 trigger（PascalCase）/ action.type = command | agent，
//            另有 matcher / timeout / enabled / confirm
// 本模块两种格式都接受：按文件内容自动判别，扩展名 .kiro.hook 与 .json 都扫描。
// 注：IDE 0.x 遗留 hook 在 IDE 1.0 中降级为 "Legacy Manual Hook"，仅可手动执行。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookFile {
    pub file_name: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<String>,
    /// "user"（~/.kiro/hooks）或 "project"（<project>/.kiro/hooks）
    pub scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookSchemaForRead {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    enabled: Option<bool>,
    #[allow(dead_code)]
    version: Option<String>,
    when: HookWhen,
    then: HookThen,
    #[allow(dead_code)]
    workspace_folder_name: Option<String>,
    #[allow(dead_code)]
    short_name: Option<String>,
    #[allow(dead_code)]
    file_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookWhen {
    r#type: HookWhenType,
    #[allow(dead_code)]
    file_pattern: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum HookWhenType {
    UserTriggered,
    FileCreated,
    FileEdited,
    FileDeleted,
    PromptSubmit,
    AgentStop,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum HookThen {
    AskAgent {
        prompt: String,
    },
    RunShellCommand {
        command: String,
        #[allow(dead_code)]
        args: Option<Vec<String>>,
        #[allow(dead_code)]
        cwd: Option<String>,
    },
}

/// IDE 1.0 新格式：单文件可含多个 hook，顶层 {version, hooks: [...]}
#[derive(Debug, Deserialize)]
struct HookFileV1 {
    #[allow(dead_code)]
    version: Option<String>,
    hooks: Vec<HookEntryV1>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookEntryV1 {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    /// PascalCase 触发器名。用 String 而非枚举：IDE 后续新增 trigger 时不应导致解析失败
    trigger: String,
    #[allow(dead_code)]
    matcher: Option<String>,
    /// 用 Value 而非枚举，理由同上；取值校验在 validate_v1 中按 type 分别处理
    action: Value,
    #[allow(dead_code)]
    timeout: Option<u64>,
    #[allow(dead_code)]
    enabled: Option<bool>,
    #[allow(dead_code)]
    confirm: Option<Value>,
}

pub struct HooksManager;

impl HooksManager {
    pub fn project_dir(project_dir: &str) -> PathBuf {
        PathBuf::from(project_dir).join(".kiro").join("hooks")
    }

    /// 用户级（全局）hooks 目录。
    /// Kiro IDE 1.0.182 引入 user-level global hooks；从 1.0.437 构建产物确认
    /// 其路径为 `~/.kiro/hooks`（与工作区级 `.kiro/hooks` 对称）。
    pub fn user_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("hooks"))
    }

    /// scope 为 "project" 时解析项目级目录，其余（"user"）解析用户级目录
    fn resolve_dir(scope: &str, project_dir: Option<&str>) -> Result<PathBuf, String> {
        match scope {
            "project" => {
                let pd = project_dir.ok_or("项目级操作需要提供项目目录")?;
                Ok(Self::project_dir(pd))
            }
            _ => Self::user_dir().ok_or_else(|| "无法获取用户目录".to_string()),
        }
    }

    /// 按文件内容自动判别 IDE 1.0 与 IDE 0.x 两种格式
    fn validate_hook_content_for_read(file_name: &str, content: &str) -> Result<(), String> {
        let value: Value = serde_json::from_str(content)
            .map_err(|e| format!("Hook 文件无效(invalid-data): {file_name}: {e}"))?;

        if value.get("hooks").and_then(Value::as_array).is_some() {
            return Self::validate_v1(file_name, &value);
        }

        if value.get("when").is_some() {
            return Self::validate_legacy(file_name, content);
        }

        Err(format!(
            "Hook 文件无效(invalid-data): {file_name}: 既非 IDE 1.0 格式（缺少 hooks 数组），也非 IDE 0.x 格式（缺少 when 字段）"
        ))
    }

    /// IDE 1.0：{version, hooks:[{name, trigger, action:{type,...}, ...}]}
    fn validate_v1(file_name: &str, value: &Value) -> Result<(), String> {
        let parsed: HookFileV1 = serde_json::from_value(value.clone())
            .map_err(|e| format!("Hook 文件无效(invalid-data): {file_name}: {e}"))?;

        for entry in parsed.hooks.iter() {
            if entry.name.trim().is_empty() {
                return Err(format!(
                    "Hook 文件无效(invalid-data): {file_name}: hooks[].name 不能为空"
                ));
            }

            if entry.trigger.trim().is_empty() {
                return Err(format!(
                    "Hook 文件无效(invalid-data): {file_name}: hooks[].trigger 不能为空"
                ));
            }

            let action_type = entry
                .action
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();

            match action_type {
                "command" => {
                    let command = entry
                        .action
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if command.trim().is_empty() {
                        return Err(format!(
                            "Hook 文件无效(invalid-data): {file_name}: action.command 不能为空"
                        ));
                    }
                }
                "agent" => {
                    let prompt = entry
                        .action
                        .get("prompt")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if prompt.trim().is_empty() {
                        return Err(format!(
                            "Hook 文件无效(invalid-data): {file_name}: action.prompt 不能为空"
                        ));
                    }
                }
                "" => {
                    return Err(format!(
                        "Hook 文件无效(invalid-data): {file_name}: action.type 不能为空"
                    ))
                }
                // 未知 action 类型放行：IDE 扩展新动作时不应让本工具读不了文件
                _ => {}
            }
        }

        Ok(())
    }

    /// IDE 0.x：{name, when:{type,...}, then:{type,...}}
    fn validate_legacy(file_name: &str, content: &str) -> Result<(), String> {
        let parsed: HookSchemaForRead = serde_json::from_str(content)
            .map_err(|e| format!("Hook 文件无效(invalid-data): {file_name}: {e}"))?;

        if parsed.name.trim().is_empty() {
            return Err(format!(
                "Hook 文件无效(invalid-data): {file_name}: name 不能为空"
            ));
        }

        match parsed.then {
            HookThen::AskAgent { prompt } => {
                if prompt.trim().is_empty() {
                    return Err(format!(
                        "Hook 文件无效(invalid-data): {file_name}: askAgent.prompt 不能为空"
                    ));
                }
            }
            HookThen::RunShellCommand { command, .. } => {
                if command.trim().is_empty() {
                    return Err(format!(
                        "Hook 文件无效(invalid-data): {file_name}: runShellCommand.command 不能为空"
                    ));
                }
            }
        }

        let _ = parsed.when.r#type;
        Ok(())
    }

    fn load_from_dir(dir: &PathBuf, scope: &str) -> Result<Vec<HookFile>, String> {
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut files = vec![];
        for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // IDE 1.0 起为 .json；保留 .kiro.hook 以兼容 IDE 0.x 遗留文件
            if !file_name.ends_with(".kiro.hook") && !file_name.ends_with(".json") {
                continue;
            }

            let content = fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {e}"))?;
            Self::validate_hook_content_for_read(&file_name, &content)?;

            let metadata = fs::metadata(&path).ok();
            let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
            let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                datetime.format("%Y/%m/%d %H:%M:%S").to_string()
            });

            files.push(HookFile {
                file_name,
                content,
                size,
                modified_at,
                scope: scope.to_string(),
            });
        }

        files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        Ok(files)
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
        // IDE 1.0 起为 .json；保留 .kiro.hook 以兼容 IDE 0.x 遗留文件
        if !file_name.ends_with(".json") && !file_name.ends_with(".kiro.hook") {
            return Err("文件名必须以 .json 结尾（IDE 0.x 遗留文件为 .kiro.hook）".to_string());
        }

        let path = Path::new(file_name);
        for comp in path.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("文件名非法".to_string());
            }
        }
        Ok(())
    }

    fn safe_hook_path(base_dir: &Path, file_name: &str) -> Result<PathBuf, String> {
        Self::validate_file_name(file_name)?;
        let candidate = base_dir.join(file_name);
        if !candidate.starts_with(base_dir) {
            return Err("非法路径".to_string());
        }
        Ok(candidate)
    }

    /// 同时扫描用户级与项目级目录；项目级仅在提供了项目目录时扫描
    pub fn load_all(project_dir: Option<&str>) -> Result<Vec<HookFile>, String> {
        let mut files = Vec::new();

        if let Some(dir) = Self::user_dir() {
            files.extend(Self::load_from_dir(&dir, "user")?);
        }

        if let Some(pd) = project_dir {
            files.extend(Self::load_from_dir(&Self::project_dir(pd), "project")?);
        }

        Ok(files)
    }

    pub fn load(
        file_name: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<HookFile, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let path = Self::safe_hook_path(&dir, file_name)?;
        if !path.exists() {
            return Err(format!("Hook 文件不存在: {file_name}"));
        }

        let content = fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {e}"))?;
        Self::validate_hook_content_for_read(file_name, &content)?;

        let metadata = fs::metadata(&path).ok();
        let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y/%m/%d %H:%M:%S").to_string()
        });

        Ok(HookFile {
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
    ) -> Result<(), String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        fs::create_dir_all(&dir).ok();
        let path = Self::safe_hook_path(&dir, file_name)?;
        fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))
    }

    pub fn delete(
        file_name: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<(), String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let path = Self::safe_hook_path(&dir, file_name)?;
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
    ) -> Result<HookFile, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        fs::create_dir_all(&dir).ok();
        let path = Self::safe_hook_path(&dir, file_name)?;
        if path.exists() {
            return Err(format!("文件已存在: {file_name}"));
        }
        fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
        Self::load(file_name, scope, project_dir)
    }
}
