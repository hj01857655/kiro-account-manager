//! Prompt 构成估算：把请求体拆成 7 类（system/tools/memory/files/results/chat）
//! 并估算 tokens，上游计费 tokens 回来后整体等比校准。
//!
//! 估算规则：ASCII 约 4 字符/token，CJK 约 1 字符/token；图片按固定值计。
//! 分类语义：
//! - system：body["system"]（agent 自带的系统提示）
//! - tools：body["tools"] 工具定义
//! - memory：消息文本里以 `<` 或 `# AGENTS.md` 开头的注入内容（指令文件/reminder）
//! - files：图片块（内容不计，按固定值）
//! - results：tool_result 块
//! - chat：其余文本（对话与模型输出历史）

use serde::{Deserialize, Serialize};

/// 一类构成。kind 取值：system / tools / memory / files / results / chat / other
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PromptPart {
    pub kind: String,
    pub tokens: i64,
}

/// 一次请求的 prompt 构成（校准后 tokens 为上游计费口径）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PromptInfo {
    /// 估算（counted=false）或校准后（counted=true）的 prompt 总 tokens
    pub tokens: i64,
    /// 上游是否报了 usage（true = parts 已按计费 tokens 校准）
    pub counted: bool,
    pub parts: Vec<PromptPart>,
}

/// 估算 tokens：ASCII 约 4 字符/token，CJK（>0x2E80）约 1 字符/token。
fn estimate_tokens(text: &str) -> i64 {
    let mut cjk = 0usize;
    for ch in text.chars() {
        if (ch as u32) >= 0x2E80 {
            cjk += 1;
        }
    }
    let ascii = text.chars().count() - cjk;
    (ascii as i64) / 4 + (cjk as i64)
}

/// 图片块按固定 1600 tokens 估。
const IMAGE_TOKENS: i64 = 1600;

/// 文本块判 memory：以 `<`（XML 注入）或 `# AGENTS.md` 开头的注入内容。
fn is_memory_text(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with('<') || t.starts_with("# AGENTS.md") || t.starts_with("# CLAUDE.md")
}

/// 从 content 字段（字符串或 block 数组）累加 tokens 到对应分类。
fn count_content(content: &serde_json::Value, parts: &mut PromptPartsRaw) {
    match content {
        serde_json::Value::String(s) => parts.add_text(s),
        serde_json::Value::Array(blocks) => {
            for b in blocks {
                match b.get("type").and_then(|t| t.as_str()) {
                    Some("image") => parts.files += IMAGE_TOKENS,
                    Some("tool_result") => {
                        parts.results += estimate_tokens(&b["content"].to_string())
                    }
                    Some("text") => {
                        if let Some(s) = b.get("text").and_then(|t| t.as_str()) {
                            parts.add_text(s);
                        }
                    }
                    _ => parts.chat += estimate_tokens(&b.to_string()),
                }
            }
        }
        other => parts.chat += estimate_tokens(&other.to_string()),
    }
}

#[derive(Default)]
struct PromptPartsRaw {
    system: i64,
    tools: i64,
    memory: i64,
    files: i64,
    results: i64,
    chat: i64,
}

impl PromptPartsRaw {
    fn add_text(&mut self, s: &str) {
        if is_memory_text(s) {
            self.memory += estimate_tokens(s);
        } else {
            self.chat += estimate_tokens(s);
        }
    }

    fn total(&self) -> i64 {
        self.system + self.tools + self.memory + self.files + self.results + self.chat
    }

    fn into_parts(self) -> Vec<PromptPart> {
        // 展示顺序：构成从固定起点到对话增量；tokens 为 0 的类省略
        let mut out = Vec::new();
        for (kind, tokens) in [
            ("system", self.system),
            ("tools", self.tools),
            ("memory", self.memory),
            ("files", self.files),
            ("results", self.results),
            ("chat", self.chat),
        ] {
            if tokens > 0 {
                out.push(PromptPart { kind: kind.to_string(), tokens });
            }
        }
        if out.is_empty() {
            out.push(PromptPart { kind: "other".to_string(), tokens: 0 });
        }
        out
    }
}

/// 从原始请求体估算 prompt 构成。body 不是合法 JSON / 结构异常时返回 None。
pub fn estimate(body: &str) -> Option<PromptInfo> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let mut raw = PromptPartsRaw::default();

    // system：字符串或 block 数组
    match v.get("system") {
        Some(serde_json::Value::String(s)) => raw.system += estimate_tokens(s),
        Some(serde_json::Value::Array(blocks)) => {
            for b in blocks {
                if let Some(s) = b.get("text").and_then(|t| t.as_str()) {
                    raw.system += estimate_tokens(s);
                } else {
                    raw.system += estimate_tokens(&b.to_string());
                }
            }
        }
        _ => {}
    }

    // tools：定义体积
    if let Some(tools) = v.get("tools").and_then(|t| t.as_array()) {
        raw.tools += estimate_tokens(&serde_json::to_string(tools).unwrap_or_default());
    }

    // messages：逐条按块分类
    if let Some(messages) = v.get("messages").and_then(|m| m.as_array()) {
        for m in messages {
            if let Some(content) = m.get("content") {
                count_content(content, &mut raw);
            }
        }
    }

    Some(PromptInfo {
        tokens: raw.total(),
        counted: false,
        parts: raw.into_parts(),
    })
}

/// 校准：上游计费 tokens（input + cacheRead + cacheWrite，即 prompt 侧总量）
/// 回来后整体等比缩放，余数归入最大 part。counted 置 true。
pub fn calibrate(info: &mut PromptInfo, counted_total: i64) {
    info.counted = true;
    info.tokens = counted_total;
    let est_total: i64 = info.parts.iter().map(|p| p.tokens).sum();
    if est_total <= 0 || counted_total <= 0 {
        return;
    }
    let mut allocated = 0i64;
    let mut largest = 0usize;
    let mut largest_tokens = 0i64;
    for (i, p) in info.parts.iter_mut().enumerate() {
        p.tokens = p.tokens * counted_total / est_total;
        allocated += p.tokens;
        if p.tokens > largest_tokens {
            largest_tokens = p.tokens;
            largest = i;
        }
    }
    // 余数（舍入损失）归入最大 part，保证 sum == counted_total
    let remainder = counted_total - allocated;
    if remainder != 0 {
        info.parts[largest].tokens += remainder;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_ascii_vs_cjk() {
        // 40 个 ASCII 字符 ≈ 10 tokens；9 个 CJK 字符 = 9 tokens
        assert_eq!(estimate_tokens("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), 10);
        assert_eq!(estimate_tokens("你好世界上下文示例"), 9);
        assert_eq!(estimate_tokens("abc你好"), 2); // 3 ascii/4→0, 2 cjk=2
    }

    #[test]
    fn classifies_parts_by_block_type() {
        let body = r#"{
            "system": "You are a helpful assistant with a fairly long system prompt here",
            "tools": [{"name": "read_file", "description": "reads a file from the workspace"}],
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "<system-reminder>do the thing</system-reminder>"},
                    {"type": "text", "text": "请帮我读一下这个文件"},
                    {"type": "image", "source": {}},
                    {"type": "tool_result", "content": "plain ascii tool output content here!"}
                ]}
            ]
        }"#;
        let info = estimate(body).unwrap();
        let kind = |k: &str| info.parts.iter().find(|p| p.kind == k).map(|p| p.tokens).unwrap_or(0);
        assert!(kind("system") > 0);
        assert!(kind("tools") > 0);
        assert!(kind("memory") > 0); // <system-reminder> 前缀 → memory
        assert!(kind("chat") > 0); // CJK 文本
        assert_eq!(kind("files"), IMAGE_TOKENS);
        assert!(kind("results") > 0);
        assert!(!info.counted);
    }

    #[test]
    fn calibrate_scales_and_preserves_total() {
        let body = r#"{
            "system": "system prompt words here",
            "messages": [{"role": "user", "content": "hello world this is a plain message"}]
        }"#;
        let mut info = estimate(body).unwrap();
        let est_total: i64 = info.parts.iter().map(|p| p.tokens).sum();
        assert_eq!(info.tokens, est_total);
        calibrate(&mut info, 10_000);
        assert!(info.counted);
        assert_eq!(info.tokens, 10_000);
        let sum: i64 = info.parts.iter().map(|p| p.tokens).sum();
        assert_eq!(sum, 10_000, "余数归入最大 part 后总和必须等于计费总量");
        // 等比：system 占比与估算占比接近（±1%）
        let est_sys = estimate(body).unwrap().parts[0].tokens as f64 / est_total as f64;
        let got_sys = info.parts[0].tokens as f64 / 10_000.0;
        assert!((est_sys - got_sys).abs() < 0.01);
    }

    #[test]
    fn invalid_body_returns_none() {
        assert!(estimate("not json at all").is_none());
    }
}
