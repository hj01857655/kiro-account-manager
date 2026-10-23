// Specs 管理（读取/编辑 ~/.kiro/specs/<name>/{requirements,design,tasks}.md
// 以及 <project>/.kiro/specs/<name>/...）
//
// 协议依据（Kiro 1.1.14，见 docs/Kiro 1.1.14/未覆盖功能与权限预设.md）：
// - 目录常量 `.kiro/specs`，每个 spec 一个子目录 `<name>/`
// - 子目录内固定三个约定命名的 Markdown：requirements.md(112 次) / design.md(109) / tasks.md(94)
// - 无 JSON schema，纯 Markdown，故实现门槛低
// - 与 Steering 同属「用户级 + 项目级」两级存储

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// 一个 spec 内的三个固定文件之一。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecFile {
    /// 文件种类：`requirements` / `design` / `tasks`
    pub file_kind: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<String>,
    /// 该文件在磁盘上是否存在（新建 spec 时三个文件都还不存在）
    pub exists: bool,
    /// "user" 或 "project"
    pub scope: String,
}

/// 一个 spec 的整体视图：名称 + 三个文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecInfo {
    pub name: String,
    pub scope: String,
    pub requirements: SpecFile,
    pub design: SpecFile,
    pub tasks: SpecFile,
}

/// 列表项：仅名称与级别，供前端左栏展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecSummary {
    pub name: String,
    pub scope: String,
}

pub struct SpecManager;

/// 三个固定文件名（与 Kiro 1.1.14 产物里的常量命名逐一对应）。
const SPEC_FILES: &[(&str, &str)] = &[
    ("requirements", "requirements.md"),
    ("design", "design.md"),
    ("tasks", "tasks.md"),
];

impl SpecManager {
    /// 用户级 specs 根目录：`~/.kiro/specs`
    fn user_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("specs"))
    }

    /// 项目级 specs 根目录：`<project>/.kiro/specs`
    fn project_dir(project_dir: &str) -> PathBuf {
        PathBuf::from(project_dir).join(".kiro").join("specs")
    }

    /// 根据 scope 解析 specs 根目录。
    fn resolve_root(scope: &str, project_dir: Option<&str>) -> Result<PathBuf, String> {
        match scope {
            "project" => {
                let pd = project_dir.ok_or("项目级操作需要提供项目目录")?;
                Ok(Self::project_dir(pd))
            }
            _ => Self::user_dir().ok_or_else(|| "无法获取用户目录".to_string()),
        }
    }

    /// 校验 spec 名称：只能是单个普通路径组件，禁止 `/`、`.`、`..` 等，防穿越。
    fn sanitize_spec_name(name: &str) -> Result<String, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("spec 名称不能为空".to_string());
        }
        let path = Path::new(trimmed);
        let mut components = path.components();
        let only_normal =
            matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
        if !only_normal {
            return Err("spec 名称只能包含字母、数字、连字符或下划线".to_string());
        }
        Ok(trimmed.to_string())
    }

    fn sanitize_file_kind(file_kind: &str) -> Result<&'static str, String> {
        match SPEC_FILES.iter().find(|(k, _)| *k == file_kind) {
            Some((_, fname)) => Ok(fname),
            None => Err(format!("非法的 spec 文件种类: {file_kind}")),
        }
    }

    fn read_one_file(path: &Path, scope: &str) -> SpecFile {
        let file_kind = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if !path.exists() {
            return SpecFile {
                file_kind,
                content: String::new(),
                size: 0,
                modified_at: None,
                exists: false,
                scope: scope.to_string(),
            };
        }
        let metadata = fs::metadata(path).ok();
        let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y/%m/%d %H:%M:%S").to_string()
        });
        let content = fs::read_to_string(path).unwrap_or_default();
        SpecFile {
            file_kind,
            content,
            size,
            modified_at,
            exists: true,
            scope: scope.to_string(),
        }
    }

    /// 列出某一级别下的所有 spec（返回每个 spec 目录名）。
    pub fn list_specs(scope: &str, project_dir: Option<&str>) -> Result<Vec<SpecSummary>, String> {
        let root = Self::resolve_root(scope, project_dir)?;
        if !root.exists() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for entry in fs::read_dir(&root).map_err(|e| format!("读取 specs 目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                out.push(SpecSummary {
                    name,
                    scope: scope.to_string(),
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// 读取一个 spec 的三个文件。spec 目录不存在时返回三个 exists=false 的文件（便于前端新建）。
    pub fn read_spec(
        scope: &str,
        project_dir: Option<&str>,
        name: &str,
    ) -> Result<SpecInfo, String> {
        let name = Self::sanitize_spec_name(name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        let spec_dir = root.join(&name);

        let make = |kind: &str, fname: &str| -> SpecFile {
            if spec_dir.exists() {
                Self::read_one_file(&spec_dir.join(fname), scope)
            } else {
                SpecFile {
                    file_kind: kind.to_string(),
                    content: String::new(),
                    size: 0,
                    modified_at: None,
                    exists: false,
                    scope: scope.to_string(),
                }
            }
        };

        Ok(SpecInfo {
            name,
            scope: scope.to_string(),
            requirements: make("requirements", "requirements.md"),
            design: make("design", "design.md"),
            tasks: make("tasks", "tasks.md"),
        })
    }

    /// 写入一个 spec 的某个文件（不存在则创建，父目录自动创建）。
    pub fn write_spec_file(
        scope: &str,
        project_dir: Option<&str>,
        name: &str,
        file_kind: &str,
        content: &str,
    ) -> Result<(), String> {
        let name = Self::sanitize_spec_name(name)?;
        let fname = Self::sanitize_file_kind(file_kind)?;
        let root = Self::resolve_root(scope, project_dir)?;
        let spec_dir = root.join(&name);
        fs::create_dir_all(&spec_dir).map_err(|e| format!("创建 spec 目录失败: {e}"))?;
        let path = spec_dir.join(fname);
        fs::write(&path, content).map_err(|e| format!("写入 {fname} 失败: {e}"))
    }

    /// 新建一个空 spec 目录（若已存在则忽略）。
    pub fn create_spec(scope: &str, project_dir: Option<&str>, name: &str) -> Result<(), String> {
        let name = Self::sanitize_spec_name(name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        let spec_dir = root.join(&name);
        fs::create_dir_all(&spec_dir).map_err(|e| format!("创建 spec 目录失败: {e}"))
    }

    /// 删除整个 spec 目录（递归）。
    pub fn delete_spec(scope: &str, project_dir: Option<&str>, name: &str) -> Result<(), String> {
        let name = Self::sanitize_spec_name(name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        let spec_dir = root.join(&name);
        if !spec_dir.exists() {
            return Ok(());
        }
        fs::remove_dir_all(&spec_dir).map_err(|e| format!("删除 spec 失败: {e}"))
    }
}
