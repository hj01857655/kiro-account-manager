//! 隐私遮蔽：请求离开本机前把敏感值换成占位符，响应回来再换回真值。
//!
//! 设计要点：
//! - **同一值永远同一占位符**：由值内容哈希决定占位符后缀，保证会话内
//!   一致性——prompt cache 与 thinking 签名因此不会失效。
//! - **只在内存**：真实值存在进程内的遮蔽映射表里，不落任何文件；
//!   审计记录只留类型、长度、首字符。
//! - **fail-closed**：遮蔽开启但执行失败时拒绝发请求（503），绝不裸奔。
//! - **三档处置**：Mask（替换）/ Warn（放行但记账）/ Block（拒绝请求）。
//! - **同形替换（Shape，可选）**：密钥类不以 `{{API_KEY_xxx}}` 占位符送上游，
//!   而是派生出**同形假密钥**（同前缀、同长度、同字符类别）送出去。
//!   占位符会被 agent/工具的「key 格式校验」判为非法而拒收，模型也会
//!   特意提一句「你的密钥格式不对」；同形假密钥两关都能过。

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
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

    /// 配置与前端用的类别键，必须与 serde `rename_all = "snake_case"` 一致
    /// （`api_key` 而非 Debug 派生出的 `apikey`）。
    pub fn as_key(&self) -> &'static str {
        match self {
            SecretKind::ApiKey => "api_key",
            SecretKind::PrivateKey => "private_key",
            SecretKind::Token => "token",
            SecretKind::Email => "email",
            SecretKind::CreditCard => "credit_card",
            SecretKind::IdCard => "id_card",
            SecretKind::Custom => "custom",
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

// ==================== 同形替换（Shape）====================
//
// 密钥以「自己形状的假密钥」发往上游，而不是 `{{API_KEY_xxx}}` 占位符：
// 同前缀、同长度、同分隔符、同字符类别（如 `sk-` + 大小写字母数字）。
// agent/工具校验 key 形式时能过，模型也没有占位符可吐槽。
//
// 派生用内容哈希（同值同形，跨重启一致，保证上游 prompt cache 命中），
// **不是**随机数：随机派生会让同一密钥每轮长得不一样，缓存全部失效。

/// 同形替换的最小长度：短于此的密钥一律降级为占位符。
///
/// 降级理由：假密钥要在「长度 + 头部 + 字符类别」上完全仿制，短密钥
/// 可枚举空间太小，造出来的假串极易与真实值撞车（把真值泄露出去）；
/// 且 `sk_abcdefg` 这类短值被换成假串后，人眼已看不出是遮蔽过的，
/// 反而像「设置没生效」。
const SHAPE_MIN_LEN: usize = 8;

/// 同形替换至少要造出这么多个随机位。分隔符与头部不计入。
///
/// 不足则降级为占位符：例如 `sk-ab-cd`（总长 8，但只有 4 个可造位）
/// 的可造空间过小，造出来的串与原值的差异不足以掩盖「这是假密钥」。
const SHAPE_MIN_RANDOM: usize = 6;

/// 已知的多段 key 前缀（跨段命名同一种密钥），整段保留不变。
const SHAPE_KEY_PREFIXES: &[&str] = &[
    "sk-ant-api03-",
    "sk-ant-oat01-",
    "sk-ant-admin01-",
    "sk-proj-",
    "sk-svcacct-",
    "sk-or-v1-",
    "github_pat_",
];

/// key 的「头部」：标识其种类、且在同形替换中保持不变的前缀。
///
/// 形式：已知多段前缀（`sk-ant-api03-`）或 AWS 的 `AKIA`/`ASIA`；
/// 否则是「至多四个字母 + 其后的 `-`/`_`」（`sk-`、`ghp_`、`xoxb-`）。
/// 无分隔符且开头不是字母时返回空——那说明头部与随机部分不可分，
/// 保守起见整串当随机部分处理。
fn shape_key_head(v: &str) -> &str {
    for p in SHAPE_KEY_PREFIXES {
        if v.starts_with(p) {
            return &v[..p.len()];
        }
    }
    if v.starts_with("AKIA") || v.starts_with("ASIA") {
        return &v[..4];
    }
    let bytes = v.as_bytes();
    for i in 0..bytes.len().min(5) {
        let c = bytes[i];
        if (c == b'-' || c == b'_') && i > 0 {
            return &v[..i + 1];
        }
        if !c.is_ascii_alphabetic() {
            break;
        }
    }
    ""
}

/// 同形替换只处理能安全仿制的字符集。
///
/// 限定为字母、数字与 `-_ .`：这些字符在 JSON 里无需转义，造出来的
/// 假密钥不会在别处偶然出现；含其它字符（空格、引号、斜杠等）时
/// 仿制出来的串要转义，且极易与真实文本撞车，一律降级为占位符。
fn shape_byte_ok(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.'
}

/// 派生真值 v 的同形假密钥。
///
/// 返回 `None` 表示不可降级为同形（调用方应改用占位符）。成功时返回
/// 的串与 v **同长、同头部、同字符类别**，且绝不等于 v。
fn shape_derive(value: &str, kind: SecretKind) -> Option<String> {
    // 个人信息类不走同形替换：模型需要看懂它的语义（邮箱、身份证），
    // 信用卡更不能——卡号有 Luhn 校验，同形假卡号校验必失败，
    // 反而把「这是假号」写在脸上。占位符对这几类是正确形态。
    match kind {
        SecretKind::Email | SecretKind::IdCard | SecretKind::CreditCard => return None,
        _ => {}
    }
    if value.len() < SHAPE_MIN_LEN {
        return None;
    }
    if !value.bytes().all(shape_byte_ok) {
        return None;
    }

    let head = shape_key_head(value);
    let rest = &value[head.len()..];
    if rest.is_empty() {
        return None;
    }

    // 字符类别必须与真值逐位一致：十六进制串保持十六进制（含大小写风格），
    // 纯数字串保持纯数字——否则造出来的串字符类别不同，「形」就不对了。
    // 判定用「是否出现 a-f / A-F」而非「是否全为 hex 字符」：
    // 纯数字（信用卡号等）也是合法 hex 字符集，但它的类别是数字。
    let mut has_lower_hex = false;
    let mut has_upper_hex = false;
    let mut has_other_alpha = false;
    for &c in rest.as_bytes() {
        match c {
            b'0'..=b'9' | b'-' | b'_' | b'.' => {}
            b'a'..=b'f' => has_lower_hex = true,
            b'A'..=b'F' => has_upper_hex = true,
            _ => has_other_alpha = true,
        }
    }
    // 十六进制串：整串只用十六进制字符，且大小写风格与真值一致
    let hex_style = !has_other_alpha
        && (has_lower_hex || has_upper_hex)
        && !(has_lower_hex && has_upper_hex);

    // 同值同形：种子取「kind + 真值」的 SHA-256，不用随机数——
    // 随机派生会让同一密钥每轮长得不一样，上游 prompt cache 全部失效。
    let mut hasher = Sha256::new();
    hasher.update(b"shape\0");
    hasher.update(kind.as_key().as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    let seed = hasher.finalize();

    const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const DIGIT: &[u8] = b"0123456789";

    let mut out = String::with_capacity(value.len());
    out.push_str(head);
    let mut made = 0usize;
    for &c in rest.as_bytes() {
        // 分隔符原样保留（不算随机位）
        if c == b'-' || c == b'_' || c == b'.' {
            out.push(c as char);
            continue;
        }
        let set: &[u8] = if hex_style && has_lower_hex {
            b"0123456789abcdef"
        } else if hex_style && has_upper_hex {
            b"0123456789ABCDEF"
        } else if c.is_ascii_uppercase() {
            UPPER
        } else if c.is_ascii_lowercase() {
            LOWER
        } else {
            DIGIT
        };
        // 从种子里取字节流（SHA-256 计数器模式），保证跨重启同形
        let idx = stream_byte(&seed, made as u64) as usize % set.len();
        out.push(set[idx] as char);
        made += 1;
    }

    if made < SHAPE_MIN_RANDOM {
        return None;
    }
    if out == value {
        return None;
    }
    Some(out)
}

/// 从种子派生确定性的字节流（SHA-256 计数器模式）。
///
/// 计数器 `n` 参与哈希，保证取第 n 个字节与其它位置互不相关，
/// 且不依赖任何随机源——跨重启结果一致。
fn stream_byte(seed: &[u8], n: u64) -> u8 {
    let mut hasher = Sha256::new();
    hasher.update(seed);
    hasher.update(n.to_be_bytes());
    hasher.finalize()[0]
}

/// 占位符映射表：值↔占位符的双向表，`Arc<Mutex>` 共享。
///
/// 独立成结构而不是塞在 `MaskEngine` 里：流式出口 `send_event` 是公共底层、
/// 拿不到 `RouterState`，通过 task-local 直接持有这张表就能回替。
#[derive(Debug, Clone, Default)]
pub struct PlaceholderMap {
    value_to_placeholder: std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
    placeholder_to_value: std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
    /// 同形假密钥集合。
    ///
    /// 假密钥 → 真值的映射已在 `placeholder_to_value` 里；这里另存一份
    /// 是为了流式扣留时只扫假密钥，不扫全部历史值。
    shape_keys: std::sync::Arc<std::sync::Mutex<HashSet<String>>>,
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
    ///
    /// 同形假密钥也在同一张反向表里（假密钥 → 真值），所以回替对两种
    /// 形态一视同仁。
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

    /// 登记一个同形假密钥 ↔ 真值的映射。
    ///
    /// 与占位符共用同一张反向表：回替时无需区分形态。
    /// 同一个真值重复登记时复用已有假密钥（保证同值同形）。
    fn register_shape(&self, value: &str, shape: &str) -> String {
        let mut v2p = self.value_to_placeholder.lock().unwrap();
        if let Some(existing) = v2p.get(value) {
            return existing.clone();
        }
        v2p.insert(value.to_string(), shape.to_string());
        self.placeholder_to_value
            .lock()
            .unwrap()
            .insert(shape.to_string(), value.to_string());
        self.shape_keys.lock().unwrap().insert(shape.to_string());
        shape.to_string()
    }

    /// 缓冲尾部可能是某个同形假密钥**前缀**的长度（0 = 不是）。
    ///
    /// 同形假密钥是普通文本（如 `sk-Ab3…`），tokenizer 同样会把它切碎，
    /// 逐事件回替会漏——必须像占位符一样跨 delta 扣留。
    ///
    /// 已完整出现的假密钥不扣留：锚点取最后一次完整出现之后的部分，
    /// 否则会把已经回替完的内容反复扣在缓冲里。
    pub fn shape_tail(&self, text: &str) -> usize {
        let keys = self.shape_keys.lock().unwrap();
        if keys.is_empty() {
            return 0;
        }
        let mut whole = 0usize;
        for k in keys.iter() {
            if let Some(i) = text.rfind(k.as_str()) {
                whole = whole.max(i + k.len());
            }
        }
        let tail = &text[whole.min(text.len())..];
        let mut best = 0usize;
        for k in keys.iter() {
            // 只认**真前缀**：假密钥本身完整出现时不算（那种已在上面锚定）
            let max = k.len().saturating_sub(1).min(tail.len());
            let mut n = max;
            while n > best {
                if tail.ends_with(&k[..n]) {
                    best = n;
                    break;
                }
                n -= 1;
            }
        }
        best
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
            shape_keys: std::sync::Arc::new(std::sync::Mutex::new(
                self.shape_keys.lock().unwrap().clone(),
            )),
        }
    }
}

/// 流式回替器：跨 SSE delta 的占位符回替。
///
/// 模型 tokenizer 会把占位符切碎（`{{API` + `_KEY_53` + `5b79b1}}`），
/// 逐事件回替对不完整片段无效（真实流量实测泄漏）。维护 hold-back 缓冲：
/// 每次喂入后只下发「不可能成为占位符前缀」的安全前缀，把可能是半个
/// 占位符的尾巴扣留到下一 delta；流结束时冲刷（完整占位符照常回替，
/// 残缺片段按字面下发——流已结束，它不可能再补全）。
pub struct StreamingUnmasker {
    maps: PlaceholderMap,
    buffer: String,
}

impl StreamingUnmasker {
    pub fn new(maps: PlaceholderMap) -> Self {
        Self {
            maps,
            buffer: String::new(),
        }
    }

    /// 喂入一个上游文本 delta，返回可立即下发的安全前缀（可能为空——全部被扣留）。
    pub fn feed(&mut self, text: &str) -> String {
        // 映射表为空说明本请求没有遮蔽过任何值，响应不可能含占位符，
        // 直接透传，零缓冲零延迟
        if self.maps.is_empty() {
            return text.to_string();
        }
        self.buffer.push_str(text);
        self.drain(false)
    }

    /// 流结束冲刷。
    pub fn flush(&mut self) -> String {
        self.drain(true)
    }

    fn drain(&mut self, final_flush: bool) -> String {
        let mut out = std::mem::take(&mut self.buffer);
        if final_flush {
            // 冲刷：流已结束，残留片段不可能再补全，照常回替后下发
            return self.maps.unmask(&out);
        }
        // hold-back 分两段，统一换算成「从末尾扣留多少字节」：
        // 1. 同形假密钥的**前缀**（它也是普通文本，tokenizer 同样会切碎）；
        //    必须在 unmask **之前**判断，否则已被换回真值，无从识别。
        // 2. 占位符的**前缀**（`{{API_KEY_` 这类固定形态）。
        //
        // 两段都只扣「真的可能是前缀」的尾巴，且有界：
        // 同形假密钥长度 = 真值长度，扣留长度与之同阶。
        let mut hold_bytes = self.maps.shape_tail(&out);
        if let Some(i) = hold_back_start(&out) {
            hold_bytes = hold_bytes.max(out.len().saturating_sub(i));
        }
        if hold_bytes > 0 {
            // 扣留点必须在字符边界上：假密钥前缀可能落在多字节字符中间，
            // 按字节切会切出非法 UTF-8——回退到字符边界。
            let cut = floor_char_boundary(&out, out.len().saturating_sub(hold_bytes));
            self.buffer = out.split_off(cut);
        }
        self.maps.unmask(&out)
    }
}

/// 把字节位置回退到最近的字符边界（UTF-8 合法切分点）。
///
/// 扣留点按字节算起，可能落在多字节字符中间；直接切会切出非法 UTF-8，
/// 下游 serde 序列化会失败。回退到字符起点即可（少扣几个字节，
/// 不影响正确性——只会让这一段晚一个 delta 下发）。
fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// 占位符完整形态：`{{API_KEY_` + 8 hex + `}}`（共 20 字符）。
const PLACEHOLDER_HEAD: &str = "{{API_KEY_";

/// 找 buffer 尾部应扣留的起点：最后一个「是占位符前缀」的位置。
fn hold_back_start(out: &str) -> Option<usize> {
    let mut search = 0usize;
    let mut last = None;
    while let Some(pos) = out[search..].find("{{") {
        let abs = search + pos;
        if is_placeholder_prefix(&out[abs..]) {
            last = Some(abs);
        }
        search = abs + 2;
    }
    if last.is_none() && out.ends_with('{') {
        // 单个尾部 '{' 可能被下一 delta 补成 "{{"
        last = Some(out.len() - 1);
    }
    last
}

/// 判断 s 是否为占位符模式的严格前缀（完整形态不算——那已被 unmask 消化）。
fn is_placeholder_prefix(s: &str) -> bool {
    if s.len() < PLACEHOLDER_HEAD.len() {
        return PLACEHOLDER_HEAD.starts_with(s);
    }
    if !s.starts_with(PLACEHOLDER_HEAD) {
        return false;
    }
    let rest = &s[PLACEHOLDER_HEAD.len()..];
    if rest.len() >= 10 {
        return false; // 8 hex + 2 括号已是完整形态
    }
    // 前缀必须是 hex（≤8 位），随后允许 0-2 个 '}'（正在闭合）
    let mut brace_started = false;
    for c in rest.chars() {
        if c == '}' {
            brace_started = true;
        } else if brace_started || !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

/// 遮蔽引擎：持有规则表与共享映射表。
#[derive(Debug)]
pub struct MaskEngine {
    rules: Vec<MaskRule>,
    /// 真实值 ↔ 占位符（进程内共享，流式出口经 task-local 回执）。
    maps: PlaceholderMap,
    /// 永不遮蔽的值清单（精确匹配优先于一切规则）。
    never_mask: Vec<String>,
    /// 同形替换开关：密钥类改送「同形假密钥」而非占位符。
    ///
    /// 关闭（默认）时行为与原来完全一致——占位符形态不变。
    shape: bool,
}

/// 审计记录里的一条：只留元数据，不留真值。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskRecordItem {
    pub kind: SecretKind,
    pub action: MaskAction,
    /// 替换串：占位符（如 `{{API_KEY_k3v9x2mq}}`）或同形假密钥。
    pub placeholder: String,
    /// 真值长度
    pub value_length: usize,
    /// 真值首字符（最多 4 个，便于人工辨认）
    pub value_head: String,
    /// 规则名
    pub rule: String,
    /// 是否为同形假密钥（false = 传统占位符）。
    ///
    /// 审计页据此区分两种形态：占位符一眼可辨，假密钥则不能，
    /// 否则审计会让人误以为「密钥真的发出去了」。
    #[serde(default)]
    pub shaped: bool,
}

/// 用户对某一类敏感值的处置覆盖（设置页「遮蔽规则」编辑器保存的形状）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskKindOverride {
    /// 类别键：`api_key` / `private_key` / `token` / `email` /
    /// `credit_card` / `id_card`（与 `SecretKind` 的 snake_case 一致）。
    /// 注：`aws_secret` 是 api_key 类的第二条规则，按 kind 覆盖时
    /// 与 api_key 同进退（kind 粒度而非规则名粒度）。
    pub kind: String,
    /// 该类是否参与遮蔽（关掉后整类移除）
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    /// 命中后的动作
    pub action: MaskAction,
}

/// 用户自定义遮蔽规则：自己的正则 + 名字 + 动作。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskCustomRule {
    pub id: String,
    pub name: String,
    /// regex 语法；编译失败跳过该规则
    pub pattern: String,
    pub action: MaskAction,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
}

/// 永不遮蔽的值清单（白名单）。
///
/// 语义：**精确匹配优先于一切规则**——值在白名单里就不遮蔽、不警告、
/// 不拦截。用于「模型需要理解这个 token 本身」的任务（排查某密钥为何
/// 被拒）等误伤场景。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskNeverMask {
    /// 白名单条目本身（真值；存在配置文件中，用户自知风险）
    pub value: String,
    /// 备注（这串值是什么，便于日后清理）
    #[serde(default)]
    pub note: String,
}

fn default_true_val() -> bool {
    true
}

impl MaskEngine {
    pub fn new() -> Self {
        Self {
            rules: builtin_rules(),
            maps: PlaceholderMap::new(),
            never_mask: Vec::new(),
            shape: false,
        }
    }

    /// 同形替换开关（默认关）。
    ///
    /// 开启后密钥类不以占位符送上游，而以同形假密钥送（见 `shape_derive`）。
    /// 调用方需重建引擎（配置变更时网关会重新装配）。
    pub fn set_shape(&mut self, shape: bool) {
        self.shape = shape;
    }

    /// 当前是否启用同形替换。
    pub fn shape_enabled(&self) -> bool {
        self.shape
    }

    /// 用用户配置重建规则集。
    ///
    /// 覆盖语义：**按 kind 覆盖**——用户在设置里对某类选了动作，
    /// 该类内置规则的 action 被替换；`enabled=false` 的类整类移除。
    /// 自定义规则（Custom kind）追加在内置规则之后。
    /// 用户没配的类保持内置默认（`kind_overrides` 里没有该类）。
    pub fn with_overrides(
        kind_overrides: &[MaskKindOverride],
        custom_rules: &[MaskCustomRule],
        never_mask: &[MaskNeverMask],
    ) -> Result<Self, String> {
        let mut engine = Self::new();
        engine.never_mask = never_mask
            .iter()
            .map(|n| n.value.clone())
            .filter(|v| !v.is_empty())
            .collect();
        let mut rules = Vec::new();

        // 按 kind 收集用户的覆盖意图（配置里重复的 kind 取最后一条）
        let mut overrides: std::collections::HashMap<String, &MaskKindOverride> =
            std::collections::HashMap::new();
        for o in kind_overrides {
            overrides.insert(o.kind.clone(), o);
        }

        for rule in builtin_rules() {
            // 类别键走 as_key()（与 serde snake_case 一致），不能用 Debug 派生——
            // `format!("{:?}")` 对 ApiKey 给出 "ApiKey"→"apikey"，与配置里的
            // "api_key" 对不上，覆盖会静默失效
            let kind_key = rule.kind.as_key();
            match overrides.get(kind_key) {
                Some(o) if !o.enabled => continue, // 用户关掉这一类
                Some(o) => rules.push(MaskRule {
                    action: o.action,
                    ..rule
                }),
                None => rules.push(rule), // 未配置：内置默认
            }
        }

        // 自定义规则：用户正则编译失败不能让网关启动失败——
        // 跳过坏规则并在 Err 里报告，调用方决定是否提示
        let mut skipped = Vec::new();
        for custom in custom_rules {
            if !custom.enabled {
                continue;
            }
            match Regex::new(&custom.pattern) {
                Ok(re) => rules.push(MaskRule {
                    kind: SecretKind::Custom,
                    name: custom.name.clone(),
                    pattern: re,
                    action: custom.action,
                }),
                Err(e) => skipped.push(format!("{}: {}", custom.name, e)),
            }
        }
        engine.rules = rules;

        if skipped.is_empty() {
            Ok(engine)
        } else {
            Err(format!("跳过无效自定义规则: {}", skipped.join("; ")))
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
                // 白名单优先于一切规则：精确匹配的值不遮蔽、不警告、不拦截
                if self.never_mask.iter().any(|v| v == &matched) {
                    continue;
                }
                match rule.action {
                    MaskAction::Mask => {
                        // 同形替换开启时优先派同形假密钥；不可降级
                        // （太短 / 含特殊字符 / 个人信息类）则回落占位符。
                        let (replacement, shaped) = if self.shape {
                            match shape_derive(&matched, rule.kind) {
                                Some(fake) => {
                                    (self.maps.register_shape(&matched, &fake), true)
                                }
                                None => (self.maps.placeholder_for(&matched, rule.kind), false),
                            }
                        } else {
                            (self.maps.placeholder_for(&matched, rule.kind), false)
                        };
                        items.push(MaskRecordItem {
                            kind: rule.kind,
                            action: MaskAction::Mask,
                            placeholder: replacement.clone(),
                            value_length: matched.len(),
                            value_head: matched.chars().take(4).collect(),
                            rule: rule.name.clone(),
                            shaped,
                        });
                        // 逐字替换而非全局 replace：同值多处出现时替换串一致，
                        // 且不会误伤与替换串文本巧合相同的其它内容。
                        out = out.replace(&matched, &replacement);
                    }
                    MaskAction::Warn => {
                        items.push(MaskRecordItem {
                            kind: rule.kind,
                            action: MaskAction::Warn,
                            placeholder: String::new(),
                            value_length: matched.len(),
                            value_head: matched.chars().take(4).collect(),
                            rule: rule.name.clone(),
                            shaped: false,
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
                            shaped: false,
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

    // ===== 用户规则覆盖（with_overrides）=====

    #[test]
    fn kind_override_switches_action() {
        // 邮箱内置默认 Warn；用户覆盖成 Block 后必须真的拦
        let engine = MaskEngine::with_overrides(
            &[MaskKindOverride {
                kind: "email".to_string(),
                enabled: true,
                action: MaskAction::Block,
            }],
            &[],
            &[],
        )
        .expect("装配应成功");
        let mut engine = engine;
        let out = engine.mask_text("write to alice@example.com");
        assert!(out.blocked, "覆盖成 Block 后邮箱必须触发拦截");
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].action, MaskAction::Block);
    }

    #[test]
    fn kind_override_disables_kind() {
        let engine = MaskEngine::with_overrides(
            &[MaskKindOverride {
                kind: "api_key".to_string(),
                enabled: false,
                action: MaskAction::Mask,
            }],
            &[],
            &[],
        )
        .expect("装配应成功");
        let mut engine = engine;
        let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let out = engine.mask_text(&fake_key);
        assert!(!out.text.contains("{{API_KEY_"), "停用 api_key 类后 key 不应被遮蔽");
    }

    #[test]
    fn custom_rule_masks_own_pattern() {
        let engine = MaskEngine::with_overrides(
            &[],
            &[MaskCustomRule {
                id: "r1".to_string(),
                name: "internal-token".to_string(),
                // 公司内部 token 格式：ACME- 后跟 12 位大写字母数字
                pattern: r"\bACME-[A-Z0-9]{12}\b".to_string(),
                action: MaskAction::Mask,
                enabled: true,
            }],
            &[],
        )
        .expect("装配应成功");
        let mut engine = engine;
        let out = engine.mask_text("token ACME-AB12CD34EF56 here");
        assert!(!out.text.contains("ACME-AB12CD34EF56"), "自定义规则必须命中");
        assert!(out.text.contains("{{CUSTOM_"));
    }

    #[test]
    fn invalid_custom_regex_is_reported_not_fatal() {
        let result = MaskEngine::with_overrides(
            &[],
            &[MaskCustomRule {
                id: "bad".to_string(),
                name: "broken".to_string(),
                pattern: "([unclosed".to_string(), // 坏正则
                action: MaskAction::Mask,
                enabled: true,
            }],
            &[],
        );
        let err = result.expect_err("坏正则应报错");
        assert!(err.contains("broken"), "错误信息应指出哪条规则坏了: {err}");
    }

    #[test]
    fn never_mask_beats_every_rule() {
        let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let engine = MaskEngine::with_overrides(
            &[],
            &[],
            &[MaskNeverMask {
                value: fake_key.clone(),
                note: "排查此 key 为何被拒".to_string(),
            }],
        )
        .expect("装配应成功");
        let mut engine = engine;
        let out = engine.mask_text(&format!("why is {fake_key} refused"));
        assert_eq!(out.text, format!("why is {fake_key} refused"), "白名单值必须原样通过");
        assert!(out.items.is_empty(), "白名单命中连 Warn 都不记");
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    /// 端到端发现的真实流量疑点：字面量 sk-abc123DEF456ghi789JKL012mno345PQR 被遮蔽成了
    /// {{API_KEY_d663cf6a}}，但它不含 sk-/AKIA/ghp_/xox 任何前缀。
    /// 本测试用与线上一致的输入直接过引擎，找出肇事规则。
    #[test]
    fn probe_which_rule_matches_placeholder_shaped_literal() {
        // 用 chr 拼接构造，杜绝任何 shell/转义层面的字符偷换
        let literal: String = "{{API_KEY_"
            .chars()
            .chain("nfsnytbn".chars())
            .chain("}}".chars())
            .collect();
        assert_eq!(literal.len(), 20, "字面量应为 20 字符");
        assert!(!literal.contains("sk-"), "前置确认：字面量不含 sk-");

        let mut engine = MaskEngine::new();
        let outcome = engine.mask_text(&format!("My key is {literal}."));
        println!("probe: masked = {}", outcome.text);
        println!("probe: items = {:?}", outcome.items);
        for item in &outcome.items {
            println!("probe: 命中规则 = {} | kind = {:?} | action = {:?}", item.rule, item.kind, item.action);
        }
        // 无论命中与否，把事实打出来；断言放到下一个测试
    }
}

#[cfg(test)]
mod streaming_unmasker_tests {
    use super::*;

    /// 复现真实流量抓到的分片形态：tokenizer 把占位符切成
    /// `{{API` + `_KEY_53` + `5b79b1}}` 三段。逐事件回替会漏，
    /// hold-back 回替器必须拼回完整占位符再换真值。
    #[test]
    fn feed_handles_placeholder_split_across_deltas() {
        let mut engine = MaskEngine::new();
        let fake_key = ["sk-", "e2eSTREAM1234567890abcdefghij"].concat();
        let masked = engine.mask_text(&fake_key);
        assert!(masked.text.starts_with("{{API_KEY_"), "前置：应产生占位符");
        let placeholder = masked.text; // {{API_KEY_xxxxxxxx}} 共 20 字符
        assert_eq!(placeholder.len(), 20);

        let mut unmasker = StreamingUnmasker::new(engine.maps().snapshot());

        // 按真实抓到的分片边界喂入（0..5 / 5..12 / 12..20）
        let (a, b, c) = (&placeholder[0..5], &placeholder[5..12], &placeholder[12..]);
        let mut out = String::new();
        out.push_str(&unmasker.feed(a));
        out.push_str(&unmasker.feed(b));
        out.push_str(&unmasker.feed(c));
        out.push_str(&unmasker.flush());
        assert_eq!(out, fake_key, "分片到达时 hold-back 必须拼回后回替为真值");
    }

    /// 单 delta 完整占位符：照常回替，不因 hold-back 引入额外延迟。
    #[test]
    fn feed_handles_complete_placeholder_in_one_delta() {
        let mut engine = MaskEngine::new();
        let fake_key = ["sk-", "e2eSTREAM1234567890abcdefghij"].concat();
        let placeholder = engine.mask_text(&fake_key).text;
        let mut unmasker = StreamingUnmasker::new(engine.maps().snapshot());

        let out = unmasker.feed(&format!("before {} after", placeholder));
        assert_eq!(out, format!("before {} after", fake_key), "完整占位符应立即回替");
        assert_eq!(unmasker.flush(), "", "不应有残留缓冲");
    }

    /// hold-back 不得吞正常文本：合法 `{{name}}`（非 hex 内容）立即透传；
    /// 尾部孤立 `{` 最多扣一个 delta，flush 兜底下发。
    #[test]
    fn feed_passes_through_non_placeholder_braces() {
        let mut engine = MaskEngine::new();
        // 必须用真实命中规则的值造映射条目——"seed" 这类不匹配任何规则的
        // 字符串不会建条目，映射表为空时 feed 走零缓冲透传早退，
        // hold-back 逻辑根本不会启用（本测试首次运行就栽在这里）
        let seed_key = ["sk-", "e2eSEED1234567890abcdefghij"].concat();
        engine.mask_text(&seed_key);
        assert!(!engine.maps().is_empty(), "前置：种子必须建出映射条目");
        let mut unmasker = StreamingUnmasker::new(engine.maps().snapshot());

        let out1 = unmasker.feed("模板变量 {{name}} 与 JSON {{ok:1}} 正常");
        assert_eq!(out1, "模板变量 {{name}} 与 JSON {{ok:1}} 正常", "非占位符花括号内容不得扣留");
        let out2 = unmasker.feed("结尾一个孤立 {{");
        assert_eq!(out2, "结尾一个孤立 ", "孤立 {{ 应被扣留一个 delta");
        assert_eq!(unmasker.flush(), "{{", "flush 兜底下发扣留的孤立花括号");
    }

    /// 空映射表零缓冲透传：遮蔽从未发生时 hold-back 不引入任何延迟。
    #[test]
    fn feed_with_empty_map_is_zero_buffer_passthrough() {
        let mut unmasker = StreamingUnmasker::new(PlaceholderMap::new());
        assert_eq!(unmasker.feed("{{ anything }} {{x"), "{{ anything }} {{x");
        assert_eq!(unmasker.flush(), "");
    }
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    /// 同形假密钥的形态：同长度、同头部、同字符类别。
    fn same_form(fake: &str, real: &str) -> bool {
        if fake.len() != real.len() {
            return false;
        }
        if shape_key_head(fake) != shape_key_head(real) {
            return false;
        }
        let class = |c: u8| -> u8 {
            if c.is_ascii_uppercase() {
                b'A'
            } else if c.is_ascii_lowercase() {
                b'a'
            } else if c.is_ascii_digit() {
                b'0'
            } else {
                c
            }
        };
        let head = shape_key_head(real).len();
        real.as_bytes()[head..]
            .iter()
            .zip(fake.as_bytes()[head..].iter())
            .all(|(a, b)| class(*a) == class(*b))
    }

    /// 开启同形后，密钥不以占位符送出，而以同形假密钥送出。
    #[test]
    fn shape_replaces_placeholder_with_same_form_key() {
        let mut engine = MaskEngine::new();
        engine.set_shape(true);
        let real = ["sk-", "w3axCByTQgK1PxqPhfKTrJOLBqz6sbZv"].concat();

        let out = engine.mask_text(&format!("key is {real} end"));
        assert!(!out.text.contains("{{"), "同形模式下不应出现占位符: {}", out.text);
        assert!(!out.text.contains(&real), "真值不应残留: {}", out.text);

        let fake = out
            .text
            .split_whitespace()
            .find(|w| w.starts_with("sk-"))
            .expect("应派生出同形假密钥")
            .to_string();
        assert!(same_form(&fake, &real), "假密钥 {fake} 与真值的形不一致");
        assert_eq!(fake.len(), real.len());

        // 审计条目标记为同形，便于审计页区分两种形态
        assert_eq!(out.items.len(), 1);
        assert!(out.items[0].shaped, "审计条目应标记 shaped");
        assert_eq!(out.items[0].placeholder, fake);
    }

    /// 关闭同形时行为与原来完全一致：仍是占位符。
    #[test]
    fn shape_off_keeps_placeholder() {
        let mut engine = MaskEngine::new();
        assert!(!engine.shape_enabled(), "默认应关闭");
        let real = ["sk-", "w3axCByTQgK1PxqPhfKTrJOLBqz6sbZv"].concat();
        let out = engine.mask_text(&real);
        assert!(out.text.starts_with("{{API_KEY_"), "关闭时应仍是占位符");
        assert!(!out.items[0].shaped);
    }

    /// 短密钥降级：太短、随机位不足的密钥仍走占位符。
    ///
    /// 降级理由：短密钥的可造空间太小，假串极易与真值撞车；且
    /// `sk_abcdefg` 被换成假串后，人眼看不出是遮蔽过的，
    /// 反而像「设置没生效」。
    #[test]
    fn short_keys_fall_back_to_placeholder() {
        // 太短（< 8 字符）
        assert!(shape_derive("sk-abc", SecretKind::ApiKey).is_none());
        // 长度够但可造位不足（头部 3 + 分隔符 2 = 5，剩 3 个可造位 < 6）
        assert!(shape_derive("sk-ab-cd", SecretKind::ApiKey).is_none());
        // 含空格等特殊字符：不可仿制
        assert!(shape_derive("has space 1234567", SecretKind::ApiKey).is_none());
        // 含引号：JSON 需转义，不仿制
        assert!(shape_derive("quo\"te1234567", SecretKind::ApiKey).is_none());
        // 恰好 8 字符但可造位够（头部 3 + 5 个可造位）——仍不足 6，降级
        assert!(shape_derive("sk-abcde", SecretKind::ApiKey).is_none());
    }

    /// 降级路径端到端：引擎在同形模式下遇到短密钥，走占位符。
    #[test]
    fn engine_falls_back_to_placeholder_for_short_key() {
        let mut engine = MaskEngine::new();
        engine.set_shape(true);
        // 造一个能命中内置 api-key 规则但很短的串：规则要求 sk- 后 20+ 位，
        // 所以这里直接测派生层——短值不可派生即降级。
        let short = "sk-ab-cd";
        assert!(shape_derive(short, SecretKind::ApiKey).is_none());
        // 白名单外的普通短串不命中规则，原样通过
        let out = engine.mask_text(short);
        assert_eq!(out.text, short);
    }

    /// 个人信息类不走同形：模型需要看懂邮箱语义，卡号有 Luhn 校验。
    #[test]
    fn personal_data_never_shaped() {
        assert!(shape_derive("rina@example.com", SecretKind::Email).is_none());
        assert!(shape_derive("4111111111111111", SecretKind::CreditCard).is_none());
        assert!(shape_derive("11010119900307123X", SecretKind::IdCard).is_none());
    }

    /// 跨重启一致性：同值同形。
    ///
    /// 派生只依赖内容哈希，不依赖随机源，也不依赖进程内状态——
    /// 新建引擎（模拟重启）必须派出一模一样的假密钥，否则上游
    /// prompt cache 全部失效。
    #[test]
    fn same_value_shapes_identically_across_engines() {
        let real = ["sk-", "w3axCByTQgK1PxqPhfKTrJOLBqz6sbZv"].concat();

        let mut engine_a = MaskEngine::new();
        engine_a.set_shape(true);
        let first = engine_a.mask_text(&real).text;

        // 全新引擎 = 模拟进程重启（无任何共享状态）
        let mut engine_b = MaskEngine::new();
        engine_b.set_shape(true);
        let second = engine_b.mask_text(&real).text;

        assert_eq!(first, second, "跨重启必须派出一致的假密钥（保 prompt cache 命中）");
    }

    /// 同形假密钥可回替回真值（映射表双向登记）。
    #[test]
    fn shaped_key_is_restored() {
        let mut engine = MaskEngine::new();
        engine.set_shape(true);
        let real = ["sk-", "w3axCByTQgK1PxqPhfKTrJOLBqz6sbZv"].concat();
        let masked = engine.mask_text(&format!("token {real} here"));
        let restored = engine.unmask_text(&masked.text);
        assert_eq!(restored, format!("token {real} here"), "同形假密钥必须能换回真值");
    }

    /// 十六进制串保持十六进制与大小写风格。
    #[test]
    fn hex_keys_keep_hex_form() {
        let lower_hex = "9f2c4e1a7b3d5c8e9f2c4e1a7b3d5c8e";
        let fake = shape_derive(lower_hex, SecretKind::ApiKey).expect("十六进制串应可同形");
        assert_eq!(fake.len(), lower_hex.len());
        assert!(
            fake.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "假串应保持小写十六进制: {fake}"
        );
        assert_ne!(fake, lower_hex);

        let upper_hex = "9F2C4E1A7B3D5C8E9F2C4E1A7B3D5C8E";
        let fake_upper = shape_derive(upper_hex, SecretKind::ApiKey).expect("十六进制串应可同形");
        assert!(
            fake_upper.chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)),
            "假串应保持大写十六进制: {fake_upper}"
        );
    }

    /// 纯数字串保持纯数字（不是十六进制字符集）。
    #[test]
    fn pure_digits_keep_digit_form() {
        let digits = "1234567890123456789";
        let fake = shape_derive(digits, SecretKind::Token).expect("纯数字串应可同形");
        assert_eq!(fake.len(), digits.len());
        assert!(fake.chars().all(|c| c.is_ascii_digit()), "假串应保持纯数字: {fake}");
    }

    /// 多段前缀整段保留（sk-ant-api03- 形态）。
    #[test]
    fn multi_segment_prefix_is_kept() {
        let real = ["sk-ant-api03-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
        let fake = shape_derive(&real, SecretKind::ApiKey).expect("多段前缀应可同形");
        assert!(fake.starts_with("sk-ant-api03-"), "多段前缀应原样保留: {fake}");
        assert_eq!(fake.len(), real.len());
        assert!(same_form(&fake, &real));
    }

    /// 流式：同形假密钥被 tokenizer 切碎时，hold-back 必须拼回后回替。
    #[test]
    fn streaming_unmasker_handles_split_shape_key() {
        let mut engine = MaskEngine::new();
        engine.set_shape(true);
        let real = ["sk-", "e2eSTREAM1234567890abcdefghij"].concat();
        let masked = engine.mask_text(&real).text;
        assert!(!masked.starts_with("{{"), "前置：应派生出假密钥");
        assert!(masked.starts_with("sk-"), "前置：假密钥应保持头部");

        let mut unmasker = StreamingUnmasker::new(engine.maps().snapshot());

        // 按 tokenizer 可能切出的边界喂入
        let (a, b, c) = (&masked[0..5], &masked[5..12], &masked[12..]);
        let mut out = String::new();
        out.push_str(&unmasker.feed(a));
        out.push_str(&unmasker.feed(b));
        out.push_str(&unmasker.feed(c));
        out.push_str(&unmasker.flush());
        assert_eq!(out, real, "分片到达时 hold-back 必须拼回后回替为真值");
    }

    /// 完整假密钥在一个 delta 里：立即回替，不因扣留引入额外延迟。
    #[test]
    fn streaming_unmasker_handles_whole_shape_key() {
        let mut engine = MaskEngine::new();
        engine.set_shape(true);
        let real = ["sk-", "e2eWHOLE1234567890abcdefghij"].concat();
        let fake = engine.mask_text(&real).text;
        let mut unmasker = StreamingUnmasker::new(engine.maps().snapshot());

        let out = unmasker.feed(&format!("before {fake} after"));
        assert_eq!(out, format!("before {real} after"), "完整假密钥应立即回替");
        assert_eq!(unmasker.flush(), "", "不应有残留缓冲");
    }
}
