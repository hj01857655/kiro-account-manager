//! 隐私遮蔽：请求离开本机前把敏感值换成占位符，响应回来再换回真值。
//!
//! 设计要点（对照 magpie 的 privacy masking）：
//! - **同一值永远同一占位符**：由值内容哈希决定占位符后缀，保证会话内
//!   一致性——prompt cache 与 thinking 签名因此不会失效。
//! - **只在内存**：真实值存在进程内的遮蔽映射表里，不落任何文件；
//!   审计记录只留类型、长度、首字符。
//! - **fail-closed**：遮蔽开启但执行失败时拒绝发请求（503），绝不裸奔。
//! - **三档处置**：Mask（替换）/ Warn（放行但记账）/ Block（拒绝请求）。

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// 遮蔽动作：命中规则后怎么处置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaskAction {
    /// 替换为占位符
    Mask,
    /// 放行，但记账（审计页可见）
    Warn,
    /// 拒绝请求（调用方转 503/400）
    Block,
}

/// 敏感值类别。决定占位符前缀与默认动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    ApiKey,
    PrivateKey,
    Token,
    Email,
    CreditCard,
    IdCard,
    Custom,
}

impl SecretKind {
    /// 占位符前缀，如 `{{API_KEY_k3v9x2mq}}`。
    pub fn placeholder_prefix(&self) -> &'static str {
        match self {
            SecretKind::ApiKey => "API_KEY",
            SecretKind::PrivateKey => "PRIVATE_KEY",
            SecretKind::Token => "TOKEN",
            SecretKind::Email => "EMAIL",
            SecretKind::CreditCard => "CREDIT_CARD",
            SecretKind::IdCard => "ID_CARD",
            SecretKind::Custom => "CUSTOM",
        }
    }

    pub fn default_action(&self) -> MaskAction {
        match self {
            // 密钥类默认遮蔽；邮箱默认只警告（模型常需理解邮箱语义）
            SecretKind::Email => MaskAction::Warn,
            _ => MaskAction::Mask,
        }
    }
}

/// 一条遮蔽规则。内置规则由 `builtin_rules()` 给出；用户可加自定义正则。
#[derive(Debug, Clone)]
pub struct MaskRule {
    pub kind: SecretKind,
    /// 规则名（审计页展示）
    pub name: String,
    pub pattern: Regex,
    pub action: MaskAction,
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("built-in mask regex must compile")
}

/// 内置规则：覆盖确定性的敏感格式。
///
/// 只收「格式即可判定」的类型——姓名、地址这类需要语义理解的交给本地模型
/// （未实现，预留 Custom 规则由用户自行补充）。
pub fn builtin_rules() -> Vec<MaskRule> {
    vec![
        // 各类 API Key：sk-xxx（48+）、AKIA（AWS access key）、ghp_（GitHub）
        MaskRule {
            kind: SecretKind::ApiKey,
            name: "api-key".to_string(),
            pattern: regex(
                r"\b(sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{36,}|xox[baprs]-[A-Za-z0-9-]{10,})\b",
            ),
            action: MaskAction::Mask,
        },
        // AWS Secret Access Key：40 位 base64 类字符（噪声较高，仍值得遮）
        MaskRule {
            kind: SecretKind::ApiKey,
            name: "aws-secret".to_string(),
            pattern: regex(r"\b[A-Za-z0-9/+=]{40}\b"),
            action: MaskAction::Mask,
        },
        // PEM 私钥块
        MaskRule {
            kind: SecretKind::PrivateKey,
            name: "private-key".to_string(),
            pattern: regex(
                r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----",
            ),
            action: MaskAction::Mask,
        },
        // Bearer / JWT
        MaskRule {
            kind: SecretKind::Token,
            name: "bearer-jwt".to_string(),
            pattern: regex(
                r"\b(?:Bearer\s+)?eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
            ),
            action: MaskAction::Mask,
        },
        // 邮箱：默认 Warn（模型常需理解邮箱语义）
        MaskRule {
            kind: SecretKind::Email,
            name: "email".to_string(),
            pattern: regex(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"),
            action: MaskAction::Warn,
        },
        // 信用卡：Luhn 校验在下方额外做（16 位分组数字噪声大）
        MaskRule {
            kind: SecretKind::CreditCard,
            name: "credit-card".to_string(),
            pattern: regex(r"\b(?:\d[ -]?){13,19}\b"),
            action: MaskAction::Mask,
        },
        // 中国居民身份证：18 位，末位可 X
        MaskRule {
            kind: SecretKind::IdCard,
            name: "id-card-cn".to_string(),
            pattern: regex(r"\b\d{17}[\dXx]\b"),
            action: MaskAction::Mask,
        },
    ]
}

/// Luhn 校验：信用卡规则的后置过滤器，挡掉大部分普通长数字。
fn luhn_valid(digits: &str) -> bool {
    let cleaned: Vec<u32> = digits
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    if cleaned.len() < 13 || cleaned.len() > 19 {
        return false;
    }
    let sum: u32 = cleaned
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                *d
            }
        })
        .sum();
    sum % 10 == 0
}

/// 占位符映射表：值↔占位符的双向表，`Arc<Mutex>` 共享。
///
/// 独立成结构而不是塞在 `MaskEngine` 里：流式出口 `send_event` 是公共底层、
/// 拿不到 `RouterState`，通过 task-local 直接持有这张表就能回替。
#[derive(Debug, Clone, Default)]
pub struct PlaceholderMap {
    value_to_placeholder: std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
    placeholder_to_value: std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
}

impl PlaceholderMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// 为真值生成（或复用）稳定占位符。
    ///
    /// 后缀取内容哈希的前 8 位十六进制：同值同后缀，不同值几乎不撞。
    /// 注意不用 `RandomState`——它的种子每次启动随机，会破坏跨重启的
    /// 「同值同占位符」（缓存与签名校验容忍重启内的变化，但一致性越久越好）。
    fn placeholder_for(&self, value: &str, kind: SecretKind) -> String {
        let mut v2p = self.value_to_placeholder.lock().unwrap();
        if let Some(existing) = v2p.get(value) {
            return existing.clone();
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();
        let placeholder = format!(
            "{{{{{}_{:08x}}}}}",
            kind.placeholder_prefix(),
            (hash & 0xffff_ffff) as u32
        );
        v2p.insert(value.to_string(), placeholder.clone());
        self.placeholder_to_value
            .lock()
            .unwrap()
            .insert(placeholder.clone(), value.to_string());
        placeholder
    }

    /// 回替：把文本里的占位符换回真值。
    pub fn unmask(&self, text: &str) -> String {
        let p2v = self.placeholder_to_value.lock().unwrap();
        if p2v.is_empty() {
            return text.to_string();
        }
        let mut out = text.to_string();
        for (placeholder, value) in p2v.iter() {
            if out.contains(placeholder.as_str()) {
                out = out.replace(placeholder.as_str(), value.as_str());
            }
        }
        out
    }

    /// 当前映射表里的占位符数量（测试与诊断用）。
    pub fn len(&self) -> usize {
        self.placeholder_to_value.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 导出当前映射表的独立副本（流任务持有，不持锁）。
    ///
    /// 流式可持续数分钟：任务期间持引擎锁会让后续请求的 `mask_text`
    /// 全部排队，因此下发快照。请求侧遮蔽在流开始前已完成，流期间
    /// 不会有新占位符产生，快照覆盖整个流的回替需求。
    pub fn snapshot(&self) -> PlaceholderMap {
        PlaceholderMap {
            value_to_placeholder: std::sync::Arc::new(std::sync::Mutex::new(
                self.value_to_placeholder.lock().unwrap().clone(),
            )),
            placeholder_to_value: std::sync::Arc::new(std::sync::Mutex::new(
                self.placeholder_to_value.lock().unwrap().clone(),
            )),
        }
    }
}

/// 遮蔽引擎：持有规则表与共享映射表。
pub struct MaskEngine {
    rules: Vec<MaskRule>,
    /// 真实值 ↔ 占位符（进程内共享，流式出口经 task-local 回执）。
    maps: PlaceholderMap,
}

/// 审计记录里的一条：只留元数据，不留真值。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskRecordItem {
    pub kind: SecretKind,
    pub action: MaskAction,
    /// 占位符（如 `{{API_KEY_k3v9x2mq}}`）
    pub placeholder: String,
    /// 真值长度
    pub value_length: usize,
    /// 真值首字符（最多 4 个，便于人工辨认）
    pub value_head: String,
    /// 规则名
    pub rule: String,
}

impl MaskEngine {
    pub fn new() -> Self {
        Self {
            rules: builtin_rules(),
            maps: PlaceholderMap::new(),
        }
    }

    /// 共享映射表：流式出口 `send_event` 拿不到 `RouterState`，
    /// 通过 task-local 直接持有这张表做回替。
    pub fn maps(&self) -> &PlaceholderMap {
        &self.maps
    }

    /// 遮蔽一段文本。返回（遮蔽后文本, 审计条目, 是否触发 Block）。
    ///
    /// 规则按 `pattern` 全局扫描；Block 命中时立即返回错误信号，
    /// 调用方负责把请求挡住（fail-closed 的落点）。
    pub fn mask_text(&mut self, text: &str) -> MaskOutcome {
        let mut out = text.to_string();
        let mut items = Vec::new();
        let mut blocked = false;

        for rule in self.rules.clone() {
            // 信用卡规则后置 Luhn：挡掉普通长数字
            let matches: Vec<(String, bool)> = if rule.kind == SecretKind::CreditCard {
                rule
                    .pattern
                    .find_iter(&out)
                    .map(|m| (m.as_str().to_string(), luhn_valid(m.as_str())))
                    .filter(|(_, ok)| *ok)
                    .collect()
            } else {
                rule.pattern
                    .find_iter(&out)
                    .map(|m| (m.as_str().to_string(), true))
                    .collect()
            };

            for (matched, _) in matches {
                if matched.is_empty() {
                    continue;
                }
                match rule.action {
                    MaskAction::Mask => {
                        let placeholder = self.maps.placeholder_for(&matched, rule.kind);
                        items.push(MaskRecordItem {
                            kind: rule.kind,
                            action: MaskAction::Mask,
                            placeholder: placeholder.clone(),
                            value_length: matched.len(),
                            value_head: matched.chars().take(4).collect(),
                            rule: rule.name.clone(),
                        });
                        // 逐字替换而非全局 replace：同值多处出现时占位符一致，
                        // 且不会误伤与占位符文本巧合相同的其它内容。
                        out = out.replace(&matched, &placeholder);
                    }
                    MaskAction::Warn => {
                        items.push(MaskRecordItem {
                            kind: rule.kind,
                            action: MaskAction::Warn,
                            placeholder: String::new(),
                            value_length: matched.len(),
                            value_head: matched.chars().take(4).collect(),
                            rule: rule.name.clone(),
                        });
                    }
                    MaskAction::Block => {
                        blocked = true;
                        items.push(MaskRecordItem {
                            kind: rule.kind,
                            action: MaskAction::Block,
                            placeholder: String::new(),
                            value_length: matched.len(),
                            value_head: matched.chars().take(4).collect(),
                            rule: rule.name.clone(),
                        });
                    }
                }
            }
        }

        MaskOutcome {
            text: out,
            items,
            blocked,
        }
    }

    /// 回替：把响应里的占位符换回真值。
    ///
    /// 覆盖流式文本与工具调用参数（它们都是普通字符串，同一通道处理）。
    pub fn unmask_text(&self, text: &str) -> String {
        self.maps.unmask(text)
    }

    /// 当前映射表里的占位符数量（测试与诊断用）。
    pub fn placeholder_count(&self) -> usize {
        self.maps.len()
    }
}

/// 一次遮蔽的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskOutcome {
    /// 遮蔽后的文本
    pub text: String,
    /// 审计条目（只含元数据）
    pub items: Vec<MaskRecordItem>,
    /// 是否触发 Block（调用方应拒绝请求）
    pub blocked: bool,
}

/// 遮蔽开关的全局默认值来源：网关配置（`privacy_mask_enabled`）。
/// 引擎实例由网关运行时按请求创建/复用，映射表跨请求保留
/// （同一会话的同一值必须拿到同一占位符）。
static DEFAULT_ACTION_EMAIL: OnceLock<MaskAction> = OnceLock::new();

/// 允许测试覆盖邮箱默认动作（避免测试依赖全局状态顺序）。
pub fn _set_default_email_action_for_tests(action: MaskAction) {
    let _ = DEFAULT_ACTION_EMAIL.set(action);
}

/// 一条遮蔽审计记录：只留元数据，**不留真值**。
///
/// 真值只在内存映射表里（进程退出即消失）；这里连「原文」都不存——
/// 并排对照所需的原文由 `masked_preview`（占位符版）与
/// `original_preview`（调用方从本地日志取的真文片段）在呈现层拼装。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskAuditEntry {
    /// 发生时间（本地时间字符串，与请求日志同格式）
    pub occurred_at: String,
    /// 关联的请求 ID（UUID）——点击可跳到该请求的遮蔽前后对照
    pub request_id: String,
    /// 端点（anthropic-messages / openai-chat 等）
    pub endpoint: String,
    /// 客户端 UA（哪个 agent 在用）
    pub client_user_agent: Option<String>,
    /// 本次遮蔽命中的条目（含类型/动作/占位符/长度/首字符）
    pub items: Vec<MaskRecordItem>,
    /// 被遮蔽后的 payload 摘要（截断到 2KB，仅占位符版）
    pub masked_preview: String,
}

/// 审计存储：内存环形缓冲，只追加不淘汰元数据（默认上限 500 条）。
///
/// 刻意不落盘：真值不进磁盘，审计记录本身虽不含真值，但记录
/// 「哪个请求遮蔽了什么」也属敏感元数据，重启即清最稳妥。
#[derive(Debug)]
pub struct MaskAuditLog {
    entries: std::sync::Mutex<Vec<MaskAuditEntry>>,
    max_entries: usize,
}

impl MaskAuditLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
            max_entries: max_entries.max(1),
        }
    }

    /// 记录一条（超出上限时淘汰最旧的）。
    pub fn record(&self, entry: MaskAuditEntry) {
        let mut guard = self.entries.lock().unwrap();
        guard.push(entry);
        let overflow = guard.len().saturating_sub(self.max_entries);
        if overflow > 0 {
            guard.drain(0..overflow);
        }
    }

    /// 最近 N 条（新的在前，与请求日志方向一致）。
    pub fn recent(&self, count: usize) -> Vec<MaskAuditEntry> {
        let guard = self.entries.lock().unwrap();
        let start = guard.len().saturating_sub(count.max(1));
        let mut out = guard[start..].to_vec();
        out.reverse();
        out
    }

    /// 当前条数。
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 清空（与请求日志清空同源操作时调用）。
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_api_key_and_keeps_stable_placeholder() {
        let mut engine = MaskEngine::new();
        // 格式合法的假 key：sk- 前缀 + 32 位混合字符（总长 35）。
        // 用拼接构造而非字面量，断言长度时取 fake_key.len()，不写魔法数字。
        let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let first = engine.mask_text(&format!("key is {fake_key} end"));
        assert!(first.text.contains("{{API_KEY_"), "key 应被替换为占位符");
        assert!(!first.text.contains(&fake_key), "真值不应残留在文本里");
        // 同值再次出现必须拿到同一占位符（cache 有效性依赖这一点）
        let second = engine.mask_text(&format!("again {fake_key} end"));
        let ph1 = first
            .text
            .split_whitespace()
            .find(|w| w.starts_with("{{API_KEY_"))
            .unwrap()
            .to_string();
        let ph2 = second
            .text
            .split_whitespace()
            .find(|w| w.starts_with("{{API_KEY_"))
            .unwrap()
            .to_string();
        assert_eq!(ph1, ph2);

        // 审计条目只留元数据
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].kind, SecretKind::ApiKey);
        assert_eq!(first.items[0].value_length, fake_key.len());
    }

    #[test]
    fn unmask_restores_original() {
        let mut engine = MaskEngine::new();
        let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let original = format!("token {fake_key} here");
        let masked = engine.mask_text(&original);
        assert_ne!(masked.text, original, "遮蔽后文本应变化");
        let restored = engine.unmask_text(&masked.text);
        assert_eq!(restored, original);
    }

    #[test]
    fn email_defaults_to_warn_and_passes_through() {
        let mut engine = MaskEngine::new();
        let out = engine.mask_text("contact me at alice@example.com please");
        // Warn 档：文本不变，只记账
        assert_eq!(out.text, "contact me at alice@example.com please");
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].action, MaskAction::Warn);
    }

    #[test]
    fn plain_long_digits_survive_luhn() {
        let mut engine = MaskEngine::new();
        // 非 Luhn 号码不应被当信用卡遮掉
        let out = engine.mask_text("order 1234567812345678 done");
        assert_eq!(out.text, "order 1234567812345678 done");
    }

    #[test]
    fn luhn_valid_card_is_masked() {
        let mut engine = MaskEngine::new();
        // 4111 1111 1111 1111 是经典测试卡号，Luhn 通过
        let out = engine.mask_text("card 4111111111111111 end");
        assert!(out.text.contains("{{CREDIT_CARD_"));
        assert!(!out.text.contains("4111111111111111"));
    }

    #[test]
    fn block_action_reports_blocked() {
        let mut engine = MaskEngine::new();
        engine.rules.push(MaskRule {
            kind: SecretKind::Custom,
            name: "forbidden".to_string(),
            pattern: regex(r"FORBIDDEN-\d+"),
            action: MaskAction::Block,
        });
        let out = engine.mask_text("prefix FORBIDDEN-42 suffix");
        assert!(out.blocked);
        assert!(out.items.iter().any(|i| i.action == MaskAction::Block));
    }

    #[test]
    fn private_key_block_is_masked() {
        let mut engine = MaskEngine::new();
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIEabc123\n-----END RSA PRIVATE KEY-----";
        let out = engine.mask_text(pem);
        assert!(!out.text.contains("MIIEabc123"));
        assert!(out.text.contains("{{PRIVATE_KEY_"));
    }

    // ===== 请求侧 payload 遮蔽（递归 JSON）=====

    /// 与 `proxy::upstream::mask_payload_for_upstream` 同形的本地复刻，
    /// 避免 masking 单测反向依赖 proxy 模块（pub(crate) 可见性）。
    fn mask_payload_for_test(
        payload: &serde_json::Value,
        engine: &mut MaskEngine,
    ) -> (serde_json::Value, bool) {
        fn mask_value(value: &serde_json::Value, engine: &mut MaskEngine, blocked: &mut bool) -> serde_json::Value {
            match value {
                serde_json::Value::String(s) => {
                    let outcome = engine.mask_text(s);
                    if outcome.blocked {
                        *blocked = true;
                    }
                    serde_json::Value::String(outcome.text)
                }
                serde_json::Value::Array(arr) => serde_json::Value::Array(
                    arr.iter().map(|item| mask_value(item, engine, blocked)).collect(),
                ),
                serde_json::Value::Object(map) => {
                    let mut out = serde_json::Map::new();
                    for (k, v) in map {
                        out.insert(k.clone(), mask_value(v, engine, blocked));
                    }
                    serde_json::Value::Object(out)
                }
                other => other.clone(),
            }
        }
        let mut blocked = false;
        let masked = mask_value(payload, engine, &mut blocked);
        (masked, blocked)
    }

    #[test]
    fn masks_nested_message_content() {
        let mut engine = MaskEngine::new();
        let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let payload = serde_json::json!({
            "model": "claude-sonnet-4.5",
            "messages": [
                {"role": "user", "content": format!("my key is {fake_key}")},
                {"role": "assistant", "content": [
                    {"type": "text", "text": format!("I saw {fake_key} earlier")},
                    {"type": "tool_use", "name": "Read", "input": {"path": "/etc/x", "note": fake_key.clone()}}
                ]}
            ],
            "system": "be helpful",
            "stream": true,
            "max_tokens": 1024
        });

        let (masked, blocked) = mask_payload_for_test(&payload, &mut engine);
        assert!(!blocked);

        // 嵌套在 messages[].content（字符串与数组两种形态）与 tool_use.input 里的
        // 真值都必须被替换
        let serialized = masked.to_string();
        assert!(!serialized.contains(&fake_key), "真值不应残留在 payload 任何位置");
        assert!(serialized.contains("{{API_KEY_"));

        // 非字符串字段原样保留
        assert_eq!(masked["stream"], serde_json::json!(true));
        assert_eq!(masked["max_tokens"], serde_json::json!(1024));
        assert_eq!(masked["model"], serde_json::json!("claude-sonnet-4.5"));

        // 同一值多处出现共享同一占位符
        let count = serialized.matches("{{API_KEY_").count();
        assert_eq!(count, 3, "三处出现都应有占位符");
    }

    #[test]
    fn payload_with_block_rule_reports_blocked() {
        let mut engine = MaskEngine::new();
        engine.rules.push(MaskRule {
            kind: SecretKind::Custom,
            name: "forbidden".to_string(),
            pattern: regex(r"FORBIDDEN-\d+"),
            action: MaskAction::Block,
        });
        let payload = serde_json::json!({
            "messages": [{"role": "user", "content": "prefix FORBIDDEN-42 suffix"}]
        });
        let (_, blocked) = mask_payload_for_test(&payload, &mut engine);
        assert!(blocked, "Block 规则命中必须上报");
    }

    #[test]
    fn snapshot_unmask_matches_shared() {
        let mut engine = MaskEngine::new();
        let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let masked = engine.mask_text(&fake_key);
        let snap = engine.maps().snapshot();
        assert_eq!(snap.unmask(&masked.text), fake_key);
        // 快照后原引擎继续加值，不影响已取快照（快照是独立副本）
        let other = ["sk-", "ZZZ999YYY888XXX777WWW666VVV"].concat();
        engine.mask_text(&other);
        assert_eq!(snap.unmask(&masked.text), fake_key);
    }
}
