// Kiro IDE 1.0 权限模型 (permissions.yaml) 读写
//
// 背景：Kiro IDE 1.0 把 0.x 的 Trusted Commands / Command Denylist 统一替换为
// 基于「能力(capability)」的权限规则文件。旧的 kiroAgent.trustedCommands 等键只在
// 首次启动时由 IDE 一次性迁移进 permissions.yaml，之后即被忽略；permissions.yaml
// 才是 1.0 下权限裁决的唯一真相源。本项目据此适配，直接读写该文件。
//
// 参考实现从本地安装的 IDE 构建产物 (extension.js) 抠出，规则 schema 如下：
//   rules:
//     - capability: shell            # read|write|shell|web|web_fetch|web_search|subagent|spec|context|mcp|@mcp|@powers|@builtin|@subagent|@subagent-explicit
//       effect: allow                # allow | deny | ask
//       match: ["git *"]             # 可选：匹配模式（命令 / 工具名）
//       exclude: ["git push *"]      # 可选：排除模式
//   policies:                        # 可选：引用预设策略 id
//     - "some-preset"
// 文件位置（与 IDE 的 MOi 解析逻辑一致）：
//   全局： ~/.kiro/settings/permissions.yaml  (回退 permissions.json)
//   项目： ~/.kiro/workspace-roots/<workspace-id>/permissions.yaml  (回退 permissions.json)
//
// 自 1.1.14 适配起同时支持**全局**与**项目**两级作用域。项目级的 workspace-id
// 算法由 IDE 构建产物逆向得出并经本机实测校验，见 `workspace_id_for`。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 权限作用域。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionScope {
    /// 用户级：`~/.kiro/settings/`
    Global,
    /// 项目级：`~/.kiro/workspace-roots/<workspace-id>/`
    /// 存的是项目真实路径，workspace-id 由 `resolve_workspace_id` 推导。
    Project { project_path: String },
}

impl PermissionScope {
    /// 前端传来的 scope 字符串 → 作用域。未知/空一律按全局处理（与历史行为一致）。
    pub fn from_id(id: Option<&str>, project_path: Option<String>) -> Self {
        match id.unwrap_or("global") {
            "project" => match project_path.filter(|p| !p.trim().is_empty()) {
                Some(p) => PermissionScope::Project { project_path: p },
                // 声明为 project 却没给路径 → 退化为全局，避免写出到错误位置。
                None => PermissionScope::Global,
            },
            _ => PermissionScope::Global,
        }
    }
}

/// 复刻 Kiro IDE 的 workspace-id 算法（1.1.14 逆向 + 本机实测）：
///
/// ```text
/// workspace_id = sha256( 路径转小写 且 分隔符统一为 '/' )[:16]
/// ```
///
/// 三个细节缺一不可，实测反例：
/// - 盘符**保留**（`d:` 不去掉）
/// - 大小写**必须**归一
/// - 分隔符**必须**是 `/`（用 `\` 算出来的对不上）
pub fn workspace_id_for(project_path: &str) -> String {
    let normalized = project_path.to_lowercase().replace('\\', "/");
    let digest = Sha256::digest(normalized.as_bytes());
    hex::encode(digest)[..16].to_string()
}

/// `~/.kiro/workspace-roots`
pub fn workspace_roots_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".kiro").join("workspace-roots"))
}

/// 一个已存在的 workspace-root（即 IDE 至少为该项目建过一次目录）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceRootInfo {
    /// 目录名，即 16 位 workspace-id
    pub id: String,
    /// 项目真实路径；来自 `<id>/.trust-migration.json` 的 `root` 字段。
    /// 迁移（1.0 时期）之后才加入的工作区没有该文件，故为 `None`。
    pub project_path: Option<String>,
    /// 该 workspace-root 下是否已存在非空的权限文件
    pub has_permissions: bool,
}

/// 读取 `<workspace-root>/.trust-migration.json` 里的 `root` 字段。
fn read_workspace_root_field(dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join(".trust-migration.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value
        .get("root")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// 列出机器上所有已知 workspace-root。目录不存在 / 读失败都返回空列表。
pub fn list_workspace_roots() -> Vec<WorkspaceRootInfo> {
    let Some(dir) = workspace_roots_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<WorkspaceRootInfo> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let id = path.file_name()?.to_str()?.to_string();
            let has_permissions = file_has_content(&path.join("permissions.yaml"))
                || file_has_content(&path.join("permissions.json"));
            Some(WorkspaceRootInfo {
                id,
                project_path: read_workspace_root_field(&path),
                has_permissions,
            })
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// 求某个项目路径对应的 workspace-id。
///
/// 先在已知 workspace-root 里按 `.trust-migration.json` 记录的 `root` 精确匹配
/// （大小写与分隔符不敏感）；匹配不到再按算法现算。这样即使 IDE 的哈希规则
/// 将来微调，只要目录里留有 root 记录就仍然能对上。
pub fn resolve_workspace_id(project_path: &str) -> String {
    let needle = project_path.to_lowercase().replace('\\', "/");
    for info in list_workspace_roots() {
        if let Some(root) = &info.project_path {
            if root.to_lowercase().replace('\\', "/") == needle {
                return info.id;
            }
        }
    }
    workspace_id_for(project_path)
}

/// 单条权限规则。
///
/// 注意：`match` 是 Rust 关键字，故字段命名为 `match_patterns` 并以
/// `#[serde(rename = "match")]` 映射到 YAML/JSON 中的 `match` 键。
/// capability / effect 一律用 `String` 而非枚举——与 hooks 的 trigger 同理：
/// IDE 会自行校验未知值（未知 capability 仅产生 warning 并跳过该规则，
/// 不会整体解析失败），用枚举反而会在 IDE 新增能力时导致本项目解析报错。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PermissionRule {
    pub capability: String,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_patterns: Option<Vec<String>>,
    pub effect: String,
    #[serde(rename = "exclude", skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
}

/// 完整权限策略（对应 permissions.yaml / permissions.json 的顶层结构）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionPolicy {
    #[serde(default)]
    pub rules: Vec<PermissionRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policies: Option<Vec<String>>,
}

/// 1.1.14 实测确认存在的能力（前端下拉的**优先候选**）。
///
/// 依据：`dist/extension.js` 中 `capability:"…"` 字面量的频次统计：
///   shell 24 · fs_write 8 · fs_read 5 · mcp 3 · subagent 2 · web_fetch 2 ·
///   filesystem 2 · power 1 · context 1 · all 1 · web_search 1
///
/// ⚠️ 关键更正：1.1.14 中**不存在** `read` / `write`，真实名字是 `fs_read` / `fs_write`。
/// 选错时 IDE 只会 warning 并**静默跳过该规则**（不报错），用户完全无感知——
/// 所以这两个名字必须放在列表里，且排在前面。
pub const VERIFIED_CAPABILITIES: &[&str] = &[
    "shell",
    "fs_read",
    "fs_write",
    "mcp",
    "subagent",
    "web_fetch",
    "web_search",
    "context",
    "skill",
    "power",
    "diagnostics",
    "filesystem",
    "all",
];

/// 旧版（1.0.x）遗留的能力名，1.1.14 中**未实测到**。
///
/// 保留用于兼容仍在使用旧版 IDE 的用户；若当前 IDE 不认识，规则会被跳过。
/// 前端展示时应排在 `VERIFIED_CAPABILITIES` 之后，避免用户误选。
pub const LEGACY_CAPABILITIES: &[&str] = &[
    "read",
    "write",
    "web",
    "spec",
    "@mcp",
    "@powers",
    "@builtin",
    "@subagent",
    "@subagent-explicit",
];

/// 全局权限文件所在目录：`~/.kiro/settings`
fn permissions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".kiro").join("settings"))
}

/// 作用域 → 权限文件所在目录。
fn scope_dir(scope: &PermissionScope) -> Option<PathBuf> {
    match scope {
        PermissionScope::Global => permissions_dir(),
        PermissionScope::Project { project_path } => {
            // 优先用 IDE 已建好的目录（id 由 .trust-migration.json 反查），
            // 没有记录时退回算法现算出的 id（写入时会自动创建该目录）。
            let id = resolve_workspace_id(project_path);
            workspace_roots_dir().map(|dir| dir.join(id))
        }
    }
}

/// 在给定目录内解析权限文件路径，复刻 IDE 的 MOi 逻辑：
/// 优先用有内容的 permissions.yaml；否则用有内容的 permissions.json；
/// 两者皆空/不存在时返回 permissions.yaml（用于新建）。
fn permissions_file_path_in(dir: &Path) -> PathBuf {
    let yaml = dir.join("permissions.yaml");
    let json = dir.join("permissions.json");

    if file_has_content(&yaml) {
        return yaml;
    }
    if file_has_content(&json) {
        return json;
    }
    yaml
}

// 各作用域的路径统一由 `scope_dir()` + `permissions_file_path_in()` 组合得到。

fn file_has_content(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|c| !c.trim().is_empty())
        .unwrap_or(false)
}

/// 读取指定作用域的权限策略。文件不存在 / 解析失败均安全回退为空策略。
///
/// 全局作用域下若 `permissions.yaml` 尚不存在，会尝试一次性迁移 0.x 的
/// `kiroAgent.*` 旧键（见 `migrate_legacy_kiro_agent_permissions`）；
/// 项目作用域没有历史包袱，文件不存在即为空策略。
pub fn read_permissions_scoped(scope: &PermissionScope) -> PermissionPolicy {
    let Some(dir) = scope_dir(scope) else {
        return PermissionPolicy::default();
    };
    let path = permissions_file_path_in(&dir);

    if !path.exists() {
        return match scope {
            PermissionScope::Global => migrate_legacy_kiro_agent_permissions(),
            PermissionScope::Project { .. } => PermissionPolicy::default(),
        };
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return PermissionPolicy::default(),
    };
    if content.trim().is_empty() {
        return PermissionPolicy::default();
    }

    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_yaml::from_str(&content).unwrap_or_default()
    }
}

// 说明：这里不再保留 `read_permissions()` 全局包装——命令层已统一走
// `read_permissions_scoped(&PermissionScope::Global)`，留着只会触发 dead_code 警告。
// （`write_permissions()` 不同：它被 `migrate_legacy_kiro_agent_permissions` 调用，仍需保留。）

/// 一次性迁移 0.x 的 `kiroAgent.*` 旧键到 1.0 的 `permissions.yaml` 规则。
///
/// 仅作为兜底：Kiro IDE 在 1.0 首次启动时会自己做同样的迁移（并写入
/// `~/.kiro/.trust-migration.json`）。本项目在用户尚未启动过 IDE 1.0、却已用本应用
/// 配置过旧版 Trusted Commands 的情况下，保证这些配置不丢。迁移是幂等的——
/// 只要 `permissions.yaml` 已存在就直接返回默认空策略，绝不覆盖。
fn migrate_legacy_kiro_agent_permissions() -> PermissionPolicy {
    let Some(path) = legacy_kiro_user_settings_path() else {
        return PermissionPolicy::default();
    };
    if !path.exists() {
        return PermissionPolicy::default();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return PermissionPolicy::default(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return PermissionPolicy::default(),
    };
    let Some(kiro) = json.get("kiroAgent").and_then(|v| v.as_object()) else {
        return PermissionPolicy::default();
    };

    let rules = build_rules_from_legacy_kiro_agent(kiro);
    if rules.is_empty() {
        return PermissionPolicy::default();
    }

    let policy = PermissionPolicy {
        rules,
        policies: None,
    };

    // 仅当目标文件确实不存在时才写入（避免与 IDE 的迁移竞态覆盖用户配置）。
    if let Some(dir) = permissions_dir() {
        let target = dir.join("permissions.yaml");
        if !target.exists() {
            let _ = write_permissions(&policy);
        }
    }
    policy
}

/// 把旧版 `kiroAgent` 配置对象翻译成 1.0 的权限规则。纯函数，便于单测。
///
/// 映射规则（对齐 IDE 1.0 的迁移语义）：
/// - `trustedCommands == "all"` 或 `autoApproveAgentCommands == true` → `shell allow`（match: "*"）
/// - `commandDenylist: string[]` → `shell deny`（match: 各模式）
/// - `trustedTools: string[]` → 每个工具一条 `<tool> allow`
/// `trustedCommands` 为 `"none"`/`"common"` 时不产生 allow 规则（common 的固定命令集
/// 由 IDE 内部维护，本项目无法精确还原，交由用户在面板里手动配置）。
fn build_rules_from_legacy_kiro_agent(kiro: &serde_json::Map<String, serde_json::Value>) -> Vec<PermissionRule> {
    let mut rules: Vec<PermissionRule> = Vec::new();

    let trusted_commands = kiro
        .get("trustedCommands")
        .and_then(|v| v.as_str());
    let auto_approve = kiro
        .get("autoApproveAgentCommands")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if trusted_commands == Some("all") || auto_approve {
        rules.push(PermissionRule {
            capability: "shell".to_string(),
            match_patterns: Some(vec!["*".to_string()]),
            effect: "allow".to_string(),
            exclude: None,
        });
    }

    if let Some(denylist) = kiro.get("commandDenylist").and_then(|v| v.as_array()) {
        let patterns: Vec<String> = denylist
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect();
        if !patterns.is_empty() {
            rules.push(PermissionRule {
                capability: "shell".to_string(),
                match_patterns: Some(patterns),
                effect: "deny".to_string(),
                exclude: None,
            });
        }
    }

    if let Some(tools) = kiro.get("trustedTools").and_then(|v| v.as_array()) {
        for tool in tools.iter().filter_map(|i| i.as_str()) {
            rules.push(PermissionRule {
                capability: tool.to_string(),
                match_patterns: None,
                effect: "allow".to_string(),
                exclude: None,
            });
        }
    }

    rules
}

/// 旧版 Kiro IDE 用户设置路径（与 `commands::kiro_settings_cmd::get_kiro_settings_path`
/// 保持一致的副本——`permissions` 模块不能反向依赖 `commands` 以避免循环引用，故此处
/// 复制一份。若修改此处，请同步修改 kiro_settings_cmd 中的同名函数）。
/// 0.x 的 `kiroAgent.*` 旧键就存放在该 `settings.json` 中。
fn legacy_kiro_user_settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata)
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|home| {
            home.join(".config")
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// 写入指定作用域的权限策略（始终写 permissions.yaml，与 IDE 的 canonical 形式一致）。
///
/// 项目作用域的目录不存在时会自动创建——IDE 也是按需创建 `workspace-roots/<id>/` 的。
pub fn write_permissions_scoped(
    scope: &PermissionScope,
    policy: &PermissionPolicy,
) -> Result<(), String> {
    let dir = scope_dir(scope).ok_or_else(|| match scope {
        PermissionScope::Global => "无法定位 ~/.kiro/settings 目录（HOME 未设置？）".to_string(),
        PermissionScope::Project { .. } => {
            "无法定位 ~/.kiro/workspace-roots 目录（HOME 未设置？）".to_string()
        }
    })?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建权限目录失败: {e}"))?;

    let path = dir.join("permissions.yaml");
    // 没有规则时序列化为 `rules: []`，保证文件可被 IDE 正确解析。
    let yaml =
        serde_yaml::to_string(policy).map_err(|e| format!("序列化 permissions 失败: {e}"))?;
    std::fs::write(&path, yaml).map_err(|e| format!("写入 permissions.yaml 失败: {e}"))?;
    Ok(())
}

/// 写入全局权限策略（保留历史签名，等价于 `write_permissions_scoped(Global, …)`）。
pub fn write_permissions(policy: &PermissionPolicy) -> Result<(), String> {
    write_permissions_scoped(&PermissionScope::Global, policy)
}

/// 返回已知的能力名列表，供前端构建下拉候选。
/// 顺序为「实测确认」在前、「旧版遗留」在后，前端直接按序展示即可。
pub fn known_capabilities() -> Vec<String> {
    VERIFIED_CAPABILITIES
        .iter()
        .chain(LEGACY_CAPABILITIES.iter())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kiro(map: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        map.as_object().unwrap().clone()
    }

    #[test]
    fn legacy_all_trusted_commands_maps_to_shell_allow() {
        let kiro = kiro(serde_json::json!({
            "trustedCommands": "all",
        }));
        let rules = build_rules_from_legacy_kiro_agent(&kiro);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].capability, "shell");
        assert_eq!(rules[0].effect, "allow");
        assert_eq!(rules[0].match_patterns.as_deref(), Some(&["*".to_string()][..]));
    }

    #[test]
    fn legacy_auto_approve_maps_to_shell_allow() {
        let kiro = kiro(serde_json::json!({
            "autoApproveAgentCommands": true,
        }));
        let rules = build_rules_from_legacy_kiro_agent(&kiro);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].capability, "shell");
        assert_eq!(rules[0].effect, "allow");
    }

    #[test]
    fn legacy_none_and_common_produce_no_allow_rule() {
        // "none" 与 "common" 不应产生 shell allow 规则（common 固定命令集不还原）。
        let none = kiro(serde_json::json!({ "trustedCommands": "none" }));
        assert!(build_rules_from_legacy_kiro_agent(&none).is_empty());

        let common = kiro(serde_json::json!({ "trustedCommands": "common" }));
        assert!(build_rules_from_legacy_kiro_agent(&common).is_empty());
    }

    #[test]
    fn legacy_command_denylist_maps_to_shell_deny() {
        let kiro = kiro(serde_json::json!({
            "commandDenylist": ["rm -rf *", "git push --force"],
        }));
        let rules = build_rules_from_legacy_kiro_agent(&kiro);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].capability, "shell");
        assert_eq!(rules[0].effect, "deny");
        assert_eq!(
            rules[0].match_patterns.as_deref(),
            Some(&["rm -rf *".to_string(), "git push --force".to_string()][..])
        );
    }

    #[test]
    fn legacy_trusted_tools_map_to_capability_allow() {
        let kiro = kiro(serde_json::json!({
            "trustedTools": ["webFetch", "remote_web_search"],
        }));
        let rules = build_rules_from_legacy_kiro_agent(&kiro);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].capability, "webFetch");
        assert_eq!(rules[0].effect, "allow");
        assert_eq!(rules[1].capability, "remote_web_search");
        assert_eq!(rules[1].effect, "allow");
    }

    #[test]
    fn legacy_combined_produces_all_expected_rules() {
        let kiro = kiro(serde_json::json!({
            "trustedCommands": "all",
            "commandDenylist": ["rm *"],
            "trustedTools": ["webFetch"],
        }));
        let rules = build_rules_from_legacy_kiro_agent(&kiro);
        assert_eq!(rules.len(), 3);
        assert!(rules.iter().any(|r| r.capability == "shell" && r.effect == "allow"));
        assert!(rules.iter().any(|r| r.capability == "shell" && r.effect == "deny"));
        assert!(rules.iter().any(|r| r.capability == "webFetch" && r.effect == "allow"));
    }

    #[test]
    fn known_capabilities_contains_core_and_specific() {
        let caps = known_capabilities();
        assert!(caps.contains(&"shell".to_string()));
        assert!(caps.contains(&"web_fetch".to_string()));
        assert!(caps.contains(&"mcp".to_string()));
        assert!(caps.contains(&"@subagent".to_string()));
    }

    // ---- 项目级作用域（workspace-roots）----

    /// 固定向量：路径 → workspace-id。算法一旦被改坏，这条会先红。
    ///
    /// 这里刻意用**通用路径**而不是本机真实路径——本仓库是公开的，
    /// 不把本地目录结构提交上去。真实机器的校验记录放在 `docs/Kiro 1.1.14/`
    /// （`docs/` 已被 .gitignore 忽略）。
    #[test]
    fn workspace_id_matches_known_vector() {
        assert_eq!(workspace_id_for("D:/projects/demo-app"), "f777371bf8d890b2");
    }

    #[test]
    fn workspace_id_is_16_hex_chars() {
        let id = workspace_id_for("D:/any/where");
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// 大小写与分隔符必须归一：三种写法要算出同一个 id。
    #[test]
    fn workspace_id_normalizes_case_and_separator() {
        let backslash = workspace_id_for(r"E:\Foo\Bar");
        let forward = workspace_id_for("e:/foo/bar");
        let upper = workspace_id_for("E:/FOO/BAR");
        assert_eq!(backslash, forward, "反斜杠与正斜杠必须等价");
        assert_eq!(forward, upper, "大小写必须归一");
    }

    /// 反过来：大小写不同但已归一的路径相同 → id 相同；
    /// 只有分隔符差异而不做归一则会算错（由上面那条的 replace 保证）。
    #[test]
    fn workspace_id_keeps_drive_letter() {
        // 去掉盘符会算出完全不同的值，确认我们没有做这个多余动作
        assert_ne!(workspace_id_for("D:/x"), workspace_id_for("/x"));
    }

    #[test]
    fn project_scope_requires_non_empty_path() {
        // 声明 project 却没给路径 → 退化为全局，避免写到错误的目录
        assert_eq!(
            PermissionScope::from_id(Some("project"), None),
            PermissionScope::Global
        );
        assert_eq!(
            PermissionScope::from_id(Some("project"), Some("   ".to_string())),
            PermissionScope::Global
        );
        // 未指定 / 未知 scope → 全局（与历史行为一致）
        assert_eq!(PermissionScope::from_id(None, None), PermissionScope::Global);
        assert_eq!(
            PermissionScope::from_id(Some("bogus"), None),
            PermissionScope::Global
        );
    }

    #[test]
    fn project_scope_carries_project_path() {
        match PermissionScope::from_id(Some("project"), Some("D:/code/x".to_string())) {
            PermissionScope::Project { project_path } => assert_eq!(project_path, "D:/code/x"),
            other => panic!("期望 Project 作用域，实际得到: {other:?}"),
        }
    }
}
