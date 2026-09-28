// Workflows 管理（读取/编辑 <project>/.kiro/workflows/*.workflow.{json,yaml,yml}
// 以及 ~/.kiro/workflows/*）
//
// 逆向依据（Kiro 1.1.14，见 docs/Kiro 1.1.14/未覆盖功能与权限预设.md）：
// - 文件：`.workflow.json` / `.workflow.yaml` / `.workflow.yml`（产物中分别 5/6 次）
// - 四个合法根目录（优先级）：工作区 `.kiro/workflows/` > `~/.kiro/workflows/` >
//   `~/.kiro/cloud-cache/<scope>/config/workflows/`（云端只读）> bundled（`bundled://<name>`）
// - 另有 `generated://<id>`（workflow-creator 生成，一次性，运行即消费）
// - Schema：name + inputs（模板变量）+ steps[] 节点；可设 workflow 级默认 modelId/effortLevel
// - catalog 总长上限 12e3 字符
//
// 本管理端只覆盖「用户级 + 项目级」两类可写 workflow 文件（与 Steering/Specs 同级），
// bundled/generated/cloud 属运行时/只读来源，不在此编辑。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowFile {
    pub file_name: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<String>,
    /// "user" 或 "project"
    pub scope: String,
}

pub struct WorkflowManager;

/// 合法的工作流文件扩展名。
const WORKFLOW_EXTS: &[&str] = &["workflow.json", "workflow.yaml", "workflow.yml"];

// ===== 写入侧轻量 schema 校验（对齐 Kiro 1.1.70 bundle 系统提示词 "WORKFLOW SCHEMA" 披露）=====
// 运行前服务端按 schema 校验；未知 modelId 过校验但运行时必失败（无静默降级）。
// 管理端做同构校验把错误提前到编辑时刻；只报确定性错误，不猜 modelId。

const WORKFLOW_NODE_TYPES: &[&str] = &["step", "repeat", "sequence", "parallel", "watch"];

/// 校验 workflow 内容，返回错误列表（空 = 通过）。
/// JSON / YAML 均先转 serde_json::Value 再走同一套节点校验。
pub fn diagnose_workflow_content(content: &str) -> Vec<String> {
    let mut issues = Vec::new();

    let trimmed = content.trim();
    if trimmed.is_empty() {
        // 空文件是 create_workflow 的合法初始态，交由运行时处理
        return issues;
    }

    let value: serde_json::Value = if trimmed.starts_with('{') {
        match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                issues.push(format!("JSON 解析失败: {e}"));
                return issues;
            }
        }
    } else {
        match serde_yaml::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                issues.push(format!("YAML 解析失败: {e}"));
                return issues;
            }
        }
    };

    let Some(obj) = value.as_object() else {
        issues.push("根节点必须是对象（name/inputs/steps）".to_string());
        return issues;
    };

    let name = obj
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if name.trim().is_empty() {
        issues.push("缺少 name 字段（workflow 必须有名字）".to_string());
    }

    if let Some(effort) = obj.get("effortLevel").and_then(serde_json::Value::as_str) {
        if !matches!(
            effort,
            "low" | "medium" | "high" | "xhigh" | "max" | "auto"
        ) {
            issues.push(format!(
                "workflow 级 effortLevel \"{effort}\" 非常规取值（常见: low|medium|high|xhigh|max），运行时若不支持会回落模型默认"
            ));
        }
    }

    let Some(steps) = obj.get("steps").and_then(serde_json::Value::as_array) else {
        issues.push("缺少 steps 数组（workflow 必须至少有一个节点）".to_string());
        return issues;
    };
    if steps.is_empty() {
        issues.push("steps 数组为空（workflow 必须至少有一个节点）".to_string());
    }

    let mut seen_ids = std::collections::HashSet::new();
    for (index, node) in steps.iter().enumerate() {
        diagnose_workflow_node(node, &format!("steps[{index}]"), &mut seen_ids, &mut issues);
    }

    issues
}

fn diagnose_workflow_node(
    node: &serde_json::Value,
    path: &str,
    seen_ids: &mut std::collections::HashSet<String>,
    issues: &mut Vec<String>,
) {
    let Some(obj) = node.as_object() else {
        issues.push(format!("{path}: 节点必须是对象"));
        return;
    };

    let node_type = obj
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !WORKFLOW_NODE_TYPES.contains(&node_type) {
        issues.push(format!(
            "{path}: 未知节点类型 \"{}\"（合法: {}）",
            if node_type.is_empty() { "<空>" } else { node_type },
            WORKFLOW_NODE_TYPES.join("|")
        ));
        return;
    }

    let node_id = obj
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    if node_id.trim().is_empty() {
        issues.push(format!("{path}: 缺少 id（节点必须有唯一 id）"));
    } else if !seen_ids.insert(node_id.clone()) {
        issues.push(format!("{path}: 节点 id \"{node_id}\" 重复"));
    }

    match node_type {
        "step" => {
            for field in ["agent", "prompt"] {
                if obj
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
                {
                    issues.push(format!("{path}: step 节点缺少必填字段 {field}"));
                }
            }
        }
        "repeat" => {
            if obj.get("steps").and_then(serde_json::Value::as_array).is_none() {
                issues.push(format!("{path}: repeat 节点缺少 steps 子节点数组"));
            }
            match obj.get("maxIterations").and_then(serde_json::Value::as_i64) {
                Some(n) if (1..=1000).contains(&n) => {}
                Some(n) => issues.push(format!(
                    "{path}: maxIterations={n} 超出范围（1-1000）"
                )),
                None => issues.push(format!(
                    "{path}: repeat 节点缺少 maxIterations（1-1000）"
                )),
            }
            let on_max = obj
                .get("onMaxIterations")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !matches!(on_max, "abort" | "continue" | "pause") {
                issues.push(format!(
                    "{path}: onMaxIterations 必须是 abort|continue|pause（当前: \"{}\"）",
                    if on_max.is_empty() { "<空>" } else { on_max }
                ));
            }
            let has_stop_condition = obj.get("stopCondition").is_some_and(|v| !v.is_null());
            let has_stop_when = obj
                .get("stopWhen")
                .and_then(serde_json::Value::as_str)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            match (has_stop_condition, has_stop_when) {
                (false, false) => issues.push(format!(
                    "{path}: repeat 节点必须提供 stopCondition 或 stopWhen 之一"
                )),
                (true, true) => issues.push(format!(
                    "{path}: stopCondition 与 stopWhen 只能二选一"
                )),
                _ => {}
            }
        }
        "sequence" => {
            if obj.get("steps").and_then(serde_json::Value::as_array).is_none() {
                issues.push(format!("{path}: sequence 节点缺少 steps 子节点数组"));
            }
        }
        "parallel" => {
            let has_branches = obj
                .get("branches")
                .and_then(serde_json::Value::as_array)
                .is_some();
            if !has_branches {
                issues.push(format!("{path}: parallel 节点缺少 branches 数组"));
            }
            let join = obj
                .get("joinPolicy")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !matches!(join, "all" | "allSettled" | "any") {
                issues.push(format!(
                    "{path}: joinPolicy 必须是 all|allSettled|any（当前: \"{}\"）",
                    if join.is_empty() { "<空>" } else { join }
                ));
            }
        }
        "watch" => {
            if obj
                .get("handler")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
            {
                issues.push(format!("{path}: watch 节点缺少必填字段 handler"));
            }
        }
        _ => {}
    }

    // 递归子节点
    for child_key in ["steps", "branches"] {
        if let Some(children) = obj.get(child_key).and_then(serde_json::Value::as_array) {
            for (index, child) in children.iter().enumerate() {
                diagnose_workflow_node(child, &format!("{path}.{child_key}[{index}]"), seen_ids, issues);
            }
        }
    }
}


impl WorkflowManager {
    /// 用户级 workflows 目录：`~/.kiro/workflows`
    fn user_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("workflows"))
    }

    /// 项目级 workflows 目录：`<project>/.kiro/workflows`
    fn project_dir(project_dir: &str) -> PathBuf {
        PathBuf::from(project_dir).join(".kiro").join("workflows")
    }

    fn resolve_root(scope: &str, project_dir: Option<&str>) -> Result<PathBuf, String> {
        match scope {
            "project" => {
                let pd = project_dir.ok_or("项目级操作需要提供项目目录")?;
                Ok(Self::project_dir(pd))
            }
            _ => Self::user_dir().ok_or_else(|| "无法获取用户目录".to_string()),
        }
    }

    /// 校验文件名：必须以某个合法扩展名结尾，且整体是单个普通路径组件（防穿越）。
    fn sanitize_file_name(file_name: &str) -> Result<String, String> {
        let trimmed = file_name.trim();
        if trimmed.is_empty() {
            return Err("文件名不能为空".to_string());
        }
        if !WORKFLOW_EXTS.iter().any(|e| trimmed.ends_with(e)) {
            return Err("工作流文件必须以 .workflow.json / .workflow.yaml / .workflow.yml 结尾".to_string());
        }
        let path = Path::new(trimmed);
        let mut components = path.components();
        let only_normal =
            matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
        if !only_normal {
            return Err("文件名不合法".to_string());
        }
        Ok(trimmed.to_string())
    }

    /// 列出某一级别下的所有 workflow 文件。
    pub fn list_workflows(scope: &str, project_dir: Option<&str>) -> Result<Vec<WorkflowFile>, String> {
        let root = Self::resolve_root(scope, project_dir)?;
        if !root.exists() {
            return Ok(vec![]);
        }
        let mut files = vec![];
        for entry in fs::read_dir(&root).map_err(|e| format!("读取 workflows 目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if path.is_file() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !WORKFLOW_EXTS.iter().any(|e| name.ends_with(e)) {
                    continue;
                }
                let metadata = fs::metadata(&path).ok();
                let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
                let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
                    let datetime: chrono::DateTime<chrono::Local> = t.into();
                    datetime.format("%Y/%m/%d %H:%M:%S").to_string()
                });
                let content = fs::read_to_string(&path).unwrap_or_default();
                files.push(WorkflowFile {
                    file_name: name,
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

    /// 读取单个 workflow 文件（不存在则报错，便于前端区分「未选中」与「文件缺失」）。
    pub fn read_workflow(
        scope: &str,
        project_dir: Option<&str>,
        file_name: &str,
    ) -> Result<WorkflowFile, String> {
        let file_name = Self::sanitize_file_name(file_name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        let path = root.join(&file_name);
        if !path.exists() {
            return Err(format!("工作流文件不存在: {file_name}"));
        }
        let metadata = fs::metadata(&path).map_err(|e| format!("读取元数据失败: {e}"))?;
        let size = std::fs::Metadata::len(&metadata);
        let modified_at = metadata.modified().ok().map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y/%m/%d %H:%M:%S").to_string()
        });
        let content = fs::read_to_string(&path).map_err(|e| format!("读取 {file_name} 失败: {e}"))?;
        Ok(WorkflowFile {
            file_name,
            content,
            size,
            modified_at,
            scope: scope.to_string(),
        })
    }

    /// 写入单个 workflow 文件（不存在则创建，父目录自动创建）。
    pub fn write_workflow(
        scope: &str,
        project_dir: Option<&str>,
        file_name: &str,
        content: &str,
    ) -> Result<Vec<String>, String> {
        let file_name = Self::sanitize_file_name(file_name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        fs::create_dir_all(&root).map_err(|e| format!("创建 workflows 目录失败: {e}"))?;
        let path = root.join(&file_name);
        fs::write(&path, content).map_err(|e| format!("写入 {file_name} 失败: {e}"))?;
        // 保存不阻塞草稿，但把运行时必失败的 schema 问题带回前端展示
        Ok(diagnose_workflow_content(content))
    }

    /// 新建一个空 workflow 文件（已存在则报错，避免覆盖已有内容）。
    pub fn create_workflow(
        scope: &str,
        project_dir: Option<&str>,
        file_name: &str,
    ) -> Result<(), String> {
        let file_name = Self::sanitize_file_name(file_name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        fs::create_dir_all(&root).map_err(|e| format!("创建 workflows 目录失败: {e}"))?;
        let path = root.join(&file_name);
        if path.exists() {
            return Err(format!("工作流文件已存在: {file_name}"));
        }
        fs::write(&path, "").map_err(|e| format!("创建 {file_name} 失败: {e}"))
    }

    /// 删除单个 workflow 文件。
    pub fn delete_workflow(
        scope: &str,
        project_dir: Option<&str>,
        file_name: &str,
    ) -> Result<(), String> {
        let file_name = Self::sanitize_file_name(file_name)?;
        let root = Self::resolve_root(scope, project_dir)?;
        let path = root.join(&file_name);
        if !path.exists() {
            return Ok(());
        }
        fs::remove_file(&path).map_err(|e| format!("删除 {file_name} 失败: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::diagnose_workflow_content;

    #[test]
    fn valid_workflow_passes() {
        let wf = r#"{
            "name": "ralph",
            "inputs": {"goal": "prompt"},
            "steps": [{
                "type": "repeat", "id": "loop", "maxIterations": 10,
                "onMaxIterations": "abort",
                "stopCondition": {"containsText": "DONE"},
                "steps": [
                    {"type": "step", "id": "s1", "agent": "coder", "prompt": "work {{goal}}"}
                ]
            }]
        }"#;
        assert!(diagnose_workflow_content(wf).is_empty());
    }

    #[test]
    fn invalid_workflows_produce_specific_errors() {
        // repeat: 缺 stopCondition/stopWhen、maxIterations 越界、onMaxIterations 非法
        let wf = r#"{"name":"x","steps":[{"type":"repeat","id":"l","maxIterations":5000,"onMaxIterations":"maybe","steps":[]}]}"#;
        let issues = diagnose_workflow_content(wf);
        assert!(issues.iter().any(|i| i.contains("stopCondition")));
        assert!(issues.iter().any(|i| i.contains("1-1000")));
        assert!(issues.iter().any(|i| i.contains("abort|continue|pause")));

        // parallel: 缺 joinPolicy；step: 缺 agent/prompt；未知类型；重复 id
        let wf = r#"{"name":"x","steps":[
            {"type":"parallel","id":"p1","branches":[]},
            {"type":"step","id":"s","prompt":"x"},
            {"type":"dance","id":"d"},
            {"type":"step","id":"s","agent":"a","prompt":"p"}
        ]}"#;
        let issues = diagnose_workflow_content(wf);
        assert!(issues.iter().any(|i| i.contains("joinPolicy")));
        assert!(issues.iter().any(|i| i.contains("agent")));
        assert!(issues.iter().any(|i| i.contains("未知节点类型")));
        assert!(issues.iter().any(|i| i.contains("重复")));

        // 顶层：缺 name / 空 steps
        let issues = diagnose_workflow_content(r#"{"steps":[]}"#);
        assert!(issues.iter().any(|i| i.contains("name")));
        assert!(issues.iter().any(|i| i.contains("steps 数组为空")));

        // 坏 JSON
        assert!(diagnose_workflow_content("{oops").iter().any(|i| i.contains("JSON 解析失败")));
    }

    #[test]
    fn yaml_workflows_are_diagnosed_too() {
        let wf = "name: y\nsteps:\n  - type: step\n    id: a\n    agent: coder\n    prompt: hi\n";
        assert!(diagnose_workflow_content(wf).is_empty());
        let bad = "name: y\nsteps:\n  - type: step\n    id: a\n    prompt: hi\n";
        assert!(diagnose_workflow_content(bad).iter().any(|i| i.contains("agent")));
    }

    #[test]
    fn empty_content_is_allowed_as_initial_state() {
        assert!(diagnose_workflow_content("").is_empty());
        assert!(diagnose_workflow_content("   \n").is_empty());
    }
}
