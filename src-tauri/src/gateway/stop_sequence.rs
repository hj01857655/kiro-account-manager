//! Stop 序列匹配：在响应侧（adapter 侧）实现 Anthropic 的 `stop_sequences`。
//!
//! Kiro 上游没有 stop 字段的对应物（`KiroPayload` 里没有），所以只能由网关
//! 自己截断——社区 kirocc 同样是 "Enforces stop_sequences and max_tokens
//! adapter-side"。
//!
//! 流式响应的 stop 序列可能跨 delta 边界（"EN" + "D" 分两次到达），所以匹配
//! 必须保留一段"待定尾巴"：已下发但可能构成 stop 前缀的部分要暂存，等下一个
//! delta 到达再决定是继续下发还是丢弃。
//!
//! 非流式响应是一次性拿到完整文本，直接找最早的 stop 序列并截断即可。

/// 流式 stop 序列匹配器。
///
/// 典型用法：
/// ```ignore
/// let mut matcher = StopSequenceMatcher::new(&["END"]);
/// // 每个 delta 到达时：
/// if let Some(out) = matcher.feed(chunk) {
///     // out 是可以安全下发的文本；matcher.stopped() 为 true 表示已命中
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct StopSequenceMatcher {
    /// 客户端给的 stop 序列（已过滤空串）
    sequences: Vec<String>,
    /// 长度等于 stop 序列中最大者的「待定尾巴」：已从输入中消费但还没下发的
    /// 文本（因为它可能是某个 stop 序列的前缀）。
    pending: String,
    /// 最长 stop 序列的长度（决定待定尾巴的最大长度）
    max_len: usize,
    /// 是否已命中 stop 序列
    stopped: bool,
    /// 命中的 stop 序列（首个命中者）
    matched: Option<String>,
}

impl StopSequenceMatcher {
    pub fn new<S: AsRef<str>>(sequences: &[S]) -> Self {
        let sequences: Vec<String> = sequences
            .iter()
            .map(|value| value.as_ref().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        let max_len = sequences.iter().map(|value| value.len()).max().unwrap_or(0);
        Self {
            sequences,
            pending: String::new(),
            max_len,
            stopped: false,
            matched: None,
        }
    }

    /// 是否配置了 stop 序列
    pub fn is_enabled(&self) -> bool {
        !self.sequences.is_empty()
    }

    /// 是否已命中 stop 序列
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// 命中的 stop 序列（未命中为 None）
    pub fn matched(&self) -> Option<&str> {
        self.matched.as_deref()
    }

    /// 喂入一段增量文本，返回可以安全下发的文本。
    ///
    /// 返回 `None` 表示已命中 stop 序列（后续文本一律丢弃）。
    pub fn feed(&mut self, chunk: &str) -> Option<String> {
        if self.stopped {
            return None;
        }
        if !self.is_enabled() {
            return Some(chunk.to_string());
        }
        if chunk.is_empty() {
            return Some(String::new());
        }

        // 待定尾巴 + 新文本一起扫描
        let mut buffer = std::mem::take(&mut self.pending);
        buffer.push_str(chunk);

        // 找最早的 stop 序列命中位置
        let mut hit: Option<(usize, &str)> = None;
        for sequence in &self.sequences {
            if let Some(index) = buffer.find(sequence.as_str()) {
                let better = match hit {
                    Some((best_index, _)) => index < best_index,
                    None => true,
                };
                if better {
                    hit = Some((index, sequence.as_str()));
                }
            }
        }

        if let Some((index, sequence)) = hit {
            self.stopped = true;
            self.matched = Some(sequence.to_string());
            // 命中位置之前的内容可以下发（不含 stop 序列本身）
            let output = buffer[..index].to_string();
            self.pending.clear();
            return Some(output);
        }

        // 未命中：保留可能是 stop 前缀的尾巴，其余下发
        let keep = self.tail_length_to_keep(&buffer);
        let split = buffer.len() - keep;
        let output = buffer[..split].to_string();
        self.pending = buffer[split..].to_string();
        Some(output)
    }

    /// 收尾：把待定尾巴全部下发（没有命中 stop 时用）
    pub fn flush(&mut self) -> Option<String> {
        if self.stopped {
            return None;
        }
        Some(std::mem::take(&mut self.pending))
    }

    /// 计算应当保留多少字符作为待定尾巴。
    ///
    /// 只保留「确实是某个 stop 序列的真前缀」的后缀——保留多了会让正常文本
    /// 被无谓地延迟下发（stop 序列长度为 N 时最多压后 N-1 个字符）。
    fn tail_length_to_keep(&self, buffer: &str) -> usize {
        if self.max_len <= 1 {
            return 0;
        }

        let mut best = 0usize;
        for sequence in &self.sequences {
            // 后缀长度 k 需满足：k < sequence.len()（否则已构成完整 stop，
            // 那在上面就命中了），且 buffer 的末尾 k 个字符等于 sequence 的前 k 个。
            // k 按字符计数，切分必须落在字符边界上（多字节序列不能按字节切）。
            let chars: Vec<char> = sequence.chars().collect();
            let mut k = chars.len().saturating_sub(1);
            while k > best {
                let prefix: String = chars[..k].iter().collect();
                if buffer.ends_with(&prefix) {
                    best = k;
                    break;
                }
                k -= 1;
            }
        }

        if best == 0 {
            return 0;
        }
        // 转换为字节长度（k 是字符数，需按字符边界切）
        let char_count = buffer.chars().count();
        if char_count <= best {
            return char_count;
        }
        buffer
            .char_indices()
            .nth(char_count - best)
            .map(|(index, _)| buffer.len() - index)
            .unwrap_or(0)
    }
}

/// 在完整文本中找最早的 stop 序列并截断。
///
/// 返回 `(截断后的文本, 命中的 stop 序列)`。未配置 stop 序列或未命中时，
/// 第二个元素为 `None`。
pub fn truncate_at_stop_sequence<S: AsRef<str>>(
    text: &str,
    sequences: &[S],
) -> (String, Option<String>) {
    let sequences: Vec<&str> = sequences
        .iter()
        .map(|value| value.as_ref())
        .filter(|value| !value.is_empty())
        .collect();
    if sequences.is_empty() || text.is_empty() {
        return (text.to_string(), None);
    }

    let mut hit: Option<(usize, &str)> = None;
    for sequence in &sequences {
        if let Some(index) = text.find(sequence) {
            let better = match hit {
                Some((best_index, _)) => index < best_index,
                None => true,
            };
            if better {
                hit = Some((index, sequence));
            }
        }
    }

    match hit {
        Some((index, sequence)) => (text[..index].to_string(), Some(sequence.to_string())),
        None => (text.to_string(), None),
    }
}

/// 提取文本中的可见部分：剥离内联的 `<thinking>...</thinking>` 块。
///
/// stop 序列只应匹配用户可见的文本——thinking 块是模型的内部推理，
/// 客户端给的 stop 序列不应在推理内容里生效。
pub fn visible_text(text: &str) -> String {
    use super::thinking_parser::{SegmentType, ThinkingParser};

    let mut parser = ThinkingParser::new();
    let mut visible = String::new();
    for segment in parser.push_and_parse(text) {
        if segment.segment_type == SegmentType::Text {
            visible.push_str(&segment.content);
        }
    }
    for segment in parser.flush() {
        if segment.segment_type == SegmentType::Text {
            visible.push_str(&segment.content);
        }
    }
    visible
}

#[cfg(test)]
mod tests {
    use super::*;

    // 命中即截断：stop 序列本身不下发
    #[test]
    fn matcher_truncates_at_stop_sequence() {
        let mut matcher = StopSequenceMatcher::new(&["END"]);
        assert_eq!(matcher.feed("hello END world").as_deref(), Some("hello "));
        assert!(matcher.stopped());
        assert_eq!(matcher.matched(), Some("END"));
        // 命中后后续文本一律丢弃
        assert_eq!(matcher.feed(" more text"), None);
    }

    // 跨 delta 边界：stop 序列分两次到达也要能命中
    #[test]
    fn matcher_handles_stop_sequence_split_across_deltas() {
        let mut matcher = StopSequenceMatcher::new(&["END"]);
        // "EN" 是 "END" 的前缀，必须暂存不下发
        assert_eq!(matcher.feed("say EN").as_deref(), Some("say "));
        // "D" 到达后构成完整 stop → 截断
        assert_eq!(matcher.feed("D rest").as_deref(), Some(""));
        assert!(matcher.stopped());
        assert_eq!(matcher.matched(), Some("END"));
    }

    // 未命中时全部内容都要下发（待定尾巴在 flush 时补上）
    #[test]
    fn matcher_passes_through_when_no_stop_hit() {
        let mut matcher = StopSequenceMatcher::new(&["END"]);
        assert_eq!(matcher.feed("hello").as_deref(), Some("hello"));
        assert_eq!(matcher.feed(" world").as_deref(), Some(" world"));
        assert!(!matcher.stopped());
        assert_eq!(matcher.flush().as_deref(), Some(""));
    }

    // 尾部是 stop 前缀但最终没构成 stop → flush 时必须下发（不能吞字）
    #[test]
    fn matcher_flushes_pending_tail_when_stop_not_hit() {
        let mut matcher = StopSequenceMatcher::new(&["END"]);
        // "EN" 是前缀，被暂存
        assert_eq!(matcher.feed("trailing EN").as_deref(), Some("trailing "));
        assert_eq!(matcher.flush().as_deref(), Some("EN"));
    }

    // 未配置 stop 序列时原样透传
    #[test]
    fn matcher_without_sequences_passes_through() {
        let mut matcher = StopSequenceMatcher::new(&[] as &[&str]);
        assert!(!matcher.is_enabled());
        assert_eq!(matcher.feed("anything").as_deref(), Some("anything"));
    }

    // 多个 stop 序列：取最早命中的
    #[test]
    fn matcher_picks_earliest_of_multiple_sequences() {
        let mut matcher = StopSequenceMatcher::new(&["STOP", "END"]);
        assert_eq!(matcher.feed("a END b STOP c").as_deref(), Some("a "));
        assert_eq!(matcher.matched(), Some("END"));
    }

    // 多字节字符不能被切断
    #[test]
    fn matcher_does_not_split_multibyte_characters() {
        let mut matcher = StopSequenceMatcher::new(&["结束标记"]);
        // "结" 是 "结束标记" 的前缀，暂存
        let out = matcher.feed("正文结").expect("should emit");
        assert!(out.contains("正文"));
        // 尾巴必须完整（不能是半个字符）
        let flushed = matcher.flush().expect("should flush");
        assert!(flushed.ends_with('结') || flushed.is_empty());
    }

    // 非流式截断：取最早的 stop 序列
    #[test]
    fn truncate_returns_earliest_hit_and_matched_sequence() {
        let (text, hit) = truncate_at_stop_sequence("hello END world", &["END"]);
        assert_eq!(text, "hello ");
        assert_eq!(hit.as_deref(), Some("END"));

        let (text, hit) = truncate_at_stop_sequence("a STOP b END", &["END", "STOP"]);
        assert_eq!(text, "a ");
        assert_eq!(hit.as_deref(), Some("STOP"));

        let (text, hit) = truncate_at_stop_sequence("no stop here", &["END"]);
        assert_eq!(text, "no stop here");
        assert!(hit.is_none());
    }

    // 空 stop 序列被忽略（不能把整段文本截成空）
    #[test]
    fn truncate_ignores_empty_sequences() {
        let (text, hit) = truncate_at_stop_sequence("hello", &[""]);
        assert_eq!(text, "hello");
        assert!(hit.is_none());
    }

    // thinking 块不参与 stop 匹配：stop 序列出现在推理内容里不应命中
    #[test]
    fn visible_text_excludes_thinking_blocks() {
        let raw = "<thinking>reasoning with END inside</thinking>\nvisible part";
        let visible = visible_text(raw);
        assert!(!visible.contains("END"));
        assert!(visible.contains("visible part"));

        // thinking 里的 stop 不应导致截断
        let (text, hit) = truncate_at_stop_sequence(&visible, &["END"]);
        assert!(hit.is_none());
        assert!(text.contains("visible part"));
    }
}
