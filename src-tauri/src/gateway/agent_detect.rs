//! 客户端识别：从 User-Agent 推断来源 agent（claude-code / codex / cline / kiro…）。
//!
//! 规则表按各 agent 公开的 UA 惯例维护，大小写不敏感；按「特征越具体越先判」
//! 的顺序匹配，命中即返回规范 id。识别不出返回 None（日志保留原始 UA 兜底）。
//! 客户端也可以用 `x-agent-name` 请求头显式声明，优先于 UA 推断。

/// UA 特征表：按序匹配，特征越具体的越靠前（子串大小写不敏感）。
const PATTERNS: &[(&str, &str)] = &[
    // CLI 类：自带可识别标识
    ("claude-cli", "claude-code"),
    ("claude-code", "claude-code"),
    ("codex_cli", "codex"),
    ("codex-cli", "codex"),
    ("gemini-cli", "gemini-cli"),
    ("qwen-code", "qwen-code"),
    ("opencode", "opencode"),
    ("kiro-cli", "kiro"),
    ("kiro", "kiro"),
    ("cline", "cline"),
    ("roo-code", "roo-code"),
    ("continuedev", "continue"),
    ("droid", "droid"),
    ("goose", "goose"),
    ("aider", "aider"),
    ("crush", "crush"),
    ("kode", "kode"),
    // IDE / 编辑器类
    ("cursor", "cursor"),
    ("windsurf", "windsurf"),
    ("zed", "zed"),
    ("qoder", "qoder"),
    ("trae", "trae"),
    ("vscode", "vscode"),
    ("visual-studio-code", "vscode"),
    ("jetbrains", "jetbrains"),
    ("intellij", "jetbrains"),
    // 通用工具类（能发请求但不是 agent）
    ("postman", "postman"),
    ("insomnia", "insomnia"),
    ("curl", "curl"),
    ("python-requests", "python"),
    ("python-httpx", "python"),
    ("okhttp", "java"),
    ("node", "node"),
    ("undici", "node"),
];

/// 从 User-Agent 识别 agent，返回规范 id；无法识别返回 None。
pub fn detect(user_agent: &str) -> Option<String> {
    let ua = user_agent.to_ascii_lowercase();
    if ua.trim().is_empty() {
        return None;
    }
    PATTERNS
        .iter()
        .find(|(needle, _)| ua.contains(needle))
        .map(|(_, id)| id.to_string())
}

/// 提取显式声明头（`x-agent-name`）：非空才生效，优先于 UA 推断。
pub fn from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-agent-name")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// 统一入口：显式头 > UA 推断。
pub fn resolve(headers: &axum::http::HeaderMap, user_agent: Option<&str>) -> Option<String> {
    from_headers(headers).or_else(|| user_agent.and_then(detect))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_agents() {
        assert_eq!(
            detect("claude-cli/2.0.14 (external, cli)").as_deref(),
            Some("claude-code")
        );
        assert_eq!(detect("codex_cli_rs/0.42.0").as_deref(), Some("codex"));
        assert_eq!(detect("Cline/3.18.0").as_deref(), Some("cline"));
        assert_eq!(detect("kiro-cli/1.4.5").as_deref(), Some("kiro"));
        assert_eq!(detect("opencode/1.0").as_deref(), Some("opencode"));
        assert_eq!(detect("gemini-cli/0.8.0").as_deref(), Some("gemini-cli"));
    }

    #[test]
    fn specific_patterns_win_over_generic() {
        // kiro-cli 含 "kiro"，两条都能命中，先命中的应是更具体的 kiro-cli 行
        assert_eq!(detect("kiro-cli/1.0").as_deref(), Some("kiro"));
        // vscode 内嵌的 claude-code 优先按 claude-code 归
        assert_eq!(
            detect("claude-cli/1.0 vscode/1.95").as_deref(),
            Some("claude-code")
        );
    }

    #[test]
    fn unknown_and_empty_return_none() {
        assert!(detect("Mozilla/5.0 (Windows NT 10.0)").is_none());
        assert!(detect("").is_none());
        assert!(detect("   ").is_none());
    }

    #[test]
    fn explicit_header_overrides_ua() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-agent-name", "my-pipeline".parse().unwrap());
        assert_eq!(
            resolve(&headers, Some("claude-cli/1.0")).as_deref(),
            Some("my-pipeline")
        );
        // 空头回退 UA
        let empty = axum::http::HeaderMap::new();
        assert_eq!(
            resolve(&empty, Some("Cline/3.0")).as_deref(),
            Some("cline")
        );
        assert!(resolve(&empty, None).is_none());
    }
}
