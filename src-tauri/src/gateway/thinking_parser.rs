#[derive(Debug, Clone, PartialEq)]
pub enum SegmentType {
    Thinking,
    Text,
}

#[derive(Debug, Clone)]
pub struct TextSegment {
    pub segment_type: SegmentType,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ParseState {
    Initial,
    InThinking,
    AfterThinking,
    Passthrough,
}

pub struct ThinkingParser {
    buffer: String,
    state: ParseState,
    thinking_extracted: bool,
}

impl ThinkingParser {
    const OPEN_TAG: &'static str = "<thinking>";
    const CLOSE_TAG: &'static str = "</thinking>";
    const QUOTE_CHARS: &'static [char] = &['`', '"', '\'', '「', '」', '『', '』'];

    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            state: ParseState::Initial,
            thinking_extracted: false,
        }
    }

    pub fn push_and_parse(&mut self, incoming: &str) -> Vec<TextSegment> {
        if incoming.is_empty() {
            return Vec::new();
        }

        self.buffer.push_str(incoming);
        let mut segments = Vec::new();

        loop {
            match self.state {
                ParseState::Initial => {
                    if let Some(should_continue) = self.handle_initial_state() {
                        if !should_continue {
                            break;
                        }
                        continue;
                    }
                    break;
                }
                ParseState::InThinking => {
                    if let Some(segment) = self.handle_in_thinking_state() {
                        if !segment.content.is_empty() {
                            segments.push(segment);
                        }
                        continue;
                    }
                    break;
                }
                ParseState::AfterThinking | ParseState::Passthrough => {
                    if !self.buffer.is_empty() {
                        segments.push(TextSegment {
                            segment_type: SegmentType::Text,
                            content: self.buffer.clone(),
                        });
                        self.buffer.clear();
                    }
                    break;
                }
            }
        }

        segments
    }

    pub fn flush(&mut self) -> Vec<TextSegment> {
        let mut segments = Vec::new();

        match self.state {
            ParseState::Initial | ParseState::AfterThinking | ParseState::Passthrough => {
                if !self.buffer.is_empty() {
                    segments.push(TextSegment {
                        segment_type: SegmentType::Text,
                        content: self.buffer.clone(),
                    });
                    self.buffer.clear();
                }
            }
            ParseState::InThinking => {
                if !self.buffer.is_empty() {
                    segments.push(TextSegment {
                        segment_type: SegmentType::Thinking,
                        content: self.buffer.clone(),
                    });
                    self.buffer.clear();
                }
            }
        }

        segments
    }

    #[cfg(test)]
    pub fn has_extracted_thinking(&self) -> bool {
        self.thinking_extracted
    }

    fn handle_initial_state(&mut self) -> Option<bool> {
        let stripped = self.buffer.trim_start();

        if stripped.len() < Self::OPEN_TAG.len() {
            if !stripped.is_empty() && Self::OPEN_TAG.starts_with(stripped) {
                return None;
            }
            if !stripped.is_empty() {
                self.state = ParseState::Passthrough;
                return Some(true);
            }
            return None;
        }

        if let Some(stripped) = stripped.strip_prefix(Self::OPEN_TAG) {
            self.buffer = stripped.to_string();
            self.state = ParseState::InThinking;
            return Some(true);
        }

        self.state = ParseState::Passthrough;
        Some(true)
    }

    fn handle_in_thinking_state(&mut self) -> Option<TextSegment> {
        let close_pos = self.find_real_close_tag();

        if close_pos.is_none() {
            let safe_len = self.buffer.len().saturating_sub(Self::CLOSE_TAG.len() - 1);
            if safe_len > 0 {
                let thinking_content = self.buffer[..safe_len].to_string();
                self.buffer = self.buffer[safe_len..].to_string();
                return Some(TextSegment {
                    segment_type: SegmentType::Thinking,
                    content: thinking_content,
                });
            }
            return None;
        }

        let close_pos = close_pos?;
        let thinking_content = self.buffer[..close_pos].to_string();
        let after_tag = &self.buffer[close_pos + Self::CLOSE_TAG.len()..];
        let after_tag = after_tag.trim_start_matches('\n');

        self.buffer = after_tag.to_string();
        self.state = ParseState::AfterThinking;
        self.thinking_extracted = true;

        Some(TextSegment {
            segment_type: SegmentType::Thinking,
            content: thinking_content,
        })
    }

    fn find_real_close_tag(&self) -> Option<usize> {
        let mut search_start = 0;

        loop {
            let pos = self.buffer[search_start..].find(Self::CLOSE_TAG)?;
            let pos = search_start + pos;

            if self.is_quoted_tag(pos) {
                search_start = pos + 1;
                continue;
            }

            let after_pos = pos + Self::CLOSE_TAG.len();
            if after_pos < self.buffer.len() {
                let next_char = self.buffer.chars().nth(after_pos);
                if matches!(next_char, Some('\n') | Some('\r')) {
                    return Some(pos);
                }
                if self.buffer.len() - after_pos > 10 {
                    search_start = pos + 1;
                    continue;
                }
                return Some(pos);
            }
            return Some(pos);
        }
    }

    fn is_quoted_tag(&self, tag_pos: usize) -> bool {
        if tag_pos == 0 {
            return false;
        }

        if let Some(prev_char) = self.buffer.chars().nth(tag_pos - 1) {
            if Self::QUOTE_CHARS.contains(&prev_char) {
                return true;
            }
        }

        let before_text = &self.buffer[..tag_pos];
        before_text.matches('`').count() % 2 == 1
    }
}

impl Default for ThinkingParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_thinking_block() {
        let mut parser = ThinkingParser::new();
        let segments =
            parser.push_and_parse("<thinking>Let me think...</thinking>\nHere is the answer.");

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].segment_type, SegmentType::Thinking);
        assert_eq!(segments[0].content, "Let me think...");
        assert_eq!(segments[1].segment_type, SegmentType::Text);
        assert_eq!(segments[1].content, "Here is the answer.");
    }

    #[test]
    fn keeps_plain_text_in_passthrough_mode() {
        let mut parser = ThinkingParser::new();
        let segments = parser.push_and_parse("Just a normal response.");

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].segment_type, SegmentType::Text);
        assert_eq!(segments[0].content, "Just a normal response.");
        assert!(!parser.has_extracted_thinking());
    }

    #[test]
    fn parses_incremental_thinking_content() {
        let mut parser = ThinkingParser::new();

        assert!(parser.push_and_parse("<think").is_empty());
        assert!(parser.push_and_parse("ing>Part 1").is_empty());

        let segments = parser.push_and_parse(" Part 2</thinking>\nText");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].segment_type, SegmentType::Thinking);
        assert_eq!(segments[0].content, "Part 1 Part 2");
        assert_eq!(segments[1].segment_type, SegmentType::Text);
        assert_eq!(segments[1].content, "Text");
    }
}
