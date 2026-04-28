
#[derive(Debug, Clone, PartialEq)]
pub enum KiroEvent {
    Text(String),
    Thinking(String),
    ToolUseStart {
        id: String,
        name: String,
    },
    ToolUseInputDelta {
        id: String,
        input_delta: String,
    },
    ToolUseStop {
        id: String,
    },
    Usage {
        input_tokens: i32,
        output_tokens: i32,
    },
    ContextUsage {
        percentage: f32,
    },
    Citation {
        text: Option<String>,
        link: String,
        target: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedCitation {
    pub text: Option<String>,
    pub link: String,
    pub target: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AggregatedKiroResponse {
    pub text: String,
    pub thinking: String,
    pub tool_calls: Vec<(String, String, String)>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub context_usage_percentage: Option<f32>,
    pub citations: Vec<AggregatedCitation>,
}

pub fn extract_json(source: &str) -> Option<String> {
    if !source.starts_with('{') {
        return None;
    }

    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (index, ch) in source.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if !in_string {
            if ch == '{' {
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    return Some(source[..=index].to_string());
                }
            }
        }
    }

    None
}

pub fn parse_kiro_event_full(json_str: &str) -> Option<KiroEvent> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

    if let Some(usage) = value.get("usage").and_then(|item| item.as_object()) {
        let input_tokens = usage
            .get("inputTokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|item| item.as_i64())
            .unwrap_or(0) as i32;
        let output_tokens = usage
            .get("outputTokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|item| item.as_i64())
            .unwrap_or(0) as i32;
        if input_tokens > 0 || output_tokens > 0 {
            return Some(KiroEvent::Usage {
                input_tokens,
                output_tokens,
            });
        }
    }

    if let Some(percentage) = value
        .get("contextUsagePercentage")
        .and_then(|item| item.as_f64())
    {
        return Some(KiroEvent::ContextUsage {
            percentage: percentage as f32,
        });
    }

    if let Some(text) = parse_reasoning_text(&value) {
        if !text.is_empty() {
            return Some(KiroEvent::Thinking(text));
        }
    }

    if let Some(citation) = parse_citation_event(&value) {
        return Some(KiroEvent::Citation {
            text: citation.text,
            link: citation.link,
            target: citation.target,
        });
    }

    if let Some(tool_use_id) = value.get("toolUseId").and_then(|item| item.as_str()) {
        let name = value
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();

        if value.get("stop").and_then(|item| item.as_bool()) == Some(true) {
            return Some(KiroEvent::ToolUseStop {
                id: tool_use_id.to_string(),
            });
        }

        if let Some(input) = value.get("input") {
            let input_delta = if let Some(text) = input.as_str() {
                text.to_string()
            } else if input.is_object() || input.is_array() {
                serde_json::to_string(input).unwrap_or_default()
            } else {
                String::new()
            };
            if !input_delta.is_empty() {
                return Some(KiroEvent::ToolUseInputDelta {
                    id: tool_use_id.to_string(),
                    input_delta,
                });
            }
        }

        if !name.is_empty() {
            return Some(KiroEvent::ToolUseStart {
                id: tool_use_id.to_string(),
                name,
            });
        }
    }

    if let Some(tool) = value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("toolUses"))
        .and_then(|item| item.as_array())
        .and_then(|items| items.first())
    {
        let id = tool
            .get("toolUseId")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let name = tool
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();

        if !name.is_empty() {
            return Some(KiroEvent::ToolUseStart { id, name });
        }
    }

    parse_text_content(&value).map(KiroEvent::Text)
}

pub fn deduplicate_tool_calls(
    tool_calls: Vec<(String, String, String)>,
) -> Vec<(String, String, String)> {
    use std::collections::HashMap;

    if tool_calls.is_empty() {
        return tool_calls;
    }

    let mut by_id: HashMap<String, (String, String, String)> = HashMap::new();
    let mut order = Vec::new();

    for (id, name, args) in tool_calls {
        if !by_id.contains_key(&id) {
            order.push(id.clone());
        }
        by_id.insert(id.clone(), (id, name, args));
    }

    order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect()
}

pub fn aggregate_kiro_response(raw: &str) -> AggregatedKiroResponse {
    let mut aggregated = AggregatedKiroResponse::default();
    let mut remaining = raw;
    let mut tool_accumulators: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();

    while let Some(start) = remaining.find('{') {
        remaining = &remaining[start..];
        let Some(json_str) = extract_json(remaining) else {
            break;
        };
        let json_len = json_str.len();

        if let Some(event) = parse_kiro_event_full(&json_str) {
            match event {
                KiroEvent::Text(text) => aggregated.text.push_str(&text),
                KiroEvent::Thinking(text) => aggregated.thinking.push_str(&text),
                KiroEvent::ToolUseStart { id, name } => {
                    tool_accumulators.entry(id).or_insert((name, String::new()));
                }
                KiroEvent::ToolUseInputDelta { id, input_delta } => {
                    if let Some((_, current_input)) = tool_accumulators.get_mut(&id) {
                        current_input.push_str(&input_delta);
                    } else {
                        tool_accumulators.insert(id, (String::new(), input_delta));
                    }
                }
                KiroEvent::ToolUseStop { id } => {
                    if let Some((name, input)) = tool_accumulators.remove(&id) {
                        aggregated.tool_calls.push((id, name, input));
                    }
                }
                KiroEvent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    aggregated.input_tokens = input_tokens;
                    aggregated.output_tokens = output_tokens;
                }
                KiroEvent::ContextUsage { percentage } => {
                    aggregated.context_usage_percentage = Some(percentage);
                }
                KiroEvent::Citation { text, link, target } => {
                    aggregated
                        .citations
                        .push(AggregatedCitation { text, link, target });
                }
            }
        }

        remaining = &remaining[json_len..];
    }

    aggregated.tool_calls = deduplicate_tool_calls(aggregated.tool_calls);
    aggregated
}

fn parse_text_content(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.get("content").and_then(|item| item.as_str()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("delta")
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("contentBlockDelta")
        .and_then(|item| item.get("delta"))
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("content"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    None
}

fn parse_reasoning_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value
        .get("reasoningContentEvent")
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("delta")
        .and_then(|item| item.get("thinking"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("contentBlockDelta")
        .and_then(|item| item.get("delta"))
        .and_then(|item| item.get("thinking"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    None
}

fn parse_citation_event(value: &serde_json::Value) -> Option<AggregatedCitation> {
    let target = value.get("target")?.clone();
    let link = value
        .get("citationLink")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())?
        .to_string();
    let text = value
        .get("citationText")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string);
    ensure_citation_target_supported(&target)?;

    Some(AggregatedCitation { text, link, target })
}

fn ensure_citation_target_supported(target: &serde_json::Value) -> Option<()> {
    if let Some(range) = target.get("range") {
        let start_index = range.get("start").and_then(|item| item.as_u64())? as usize;
        let end_index = range.get("end").and_then(|item| item.as_u64())? as usize;
        if end_index < start_index {
            return None;
        }
        return Some(());
    }

    if target
        .get("location")
        .and_then(|item| item.as_u64())
        .is_some()
    {
        return Some(());
    }

    None
}

use crate::gateway::models::{
    OpenAIChatChunk, OpenAIChatChunkChoice, OpenAIChatDelta, OpenAIChatResponse,
    OpenAIChatChoice, OpenAIChatResponseMessage, OpenAIResponseToolCall, OpenAIChatUsage,
    OpenAIToolCallFunction, OpenAIUsage,
};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub fn build_openai_chunk(
    completion_id: &str,
    created: i64,
    model: &str,
    delta: OpenAIChatDelta,
    finish_reason: Option<String>,
    usage: Option<OpenAIChatUsage>,
) -> OpenAIChatChunk {
    OpenAIChatChunk {
        id: completion_id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created,
        model: model.to_string(),
        choices: vec![OpenAIChatChunkChoice {
            index: 0,
            delta,
            finish_reason,
        }],
        usage,
    }
}

pub fn build_openai_response(
    model: &str,
    aggregated: &AggregatedKiroResponse,
) -> OpenAIChatResponse {
    let completion_id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut message = OpenAIChatResponseMessage {
        role: "assistant".to_string(),
        content: if aggregated.text.is_empty() {
            None
        } else {
            Some(aggregated.text.clone())
        },
        tool_calls: None,
    };

    let finish_reason = if !aggregated.tool_calls.is_empty() {
        let tool_calls: Vec<OpenAIResponseToolCall> = aggregated
            .tool_calls
            .iter()
            .map(|(id, name, arguments)| OpenAIResponseToolCall {
                id: id.clone(),
                call_type: "function".to_string(),
                function: OpenAIToolCallFunction {
                    name: name.clone(),
                    arguments: arguments.clone(),
                },
            })
            .collect();
        message.tool_calls = Some(tool_calls);
        "tool_calls".to_string()
    } else {
        "stop".to_string()
    };

    OpenAIChatResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created,
        model: model.to_string(),
        choices: vec![OpenAIChatChoice {
            index: 0,
            message,
            finish_reason: Some(finish_reason),
        }],
        usage: OpenAIUsage {
            input_tokens: aggregated.input_tokens,
            output_tokens: aggregated.output_tokens,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_supports_nested_and_escaped_quotes() {
        let raw = r#"{"content":"a { brace } and \"quoted\"","nested":{"k":"v"}}"#;
        let parsed = extract_json(raw).expect("json should be extracted");
        assert_eq!(parsed, raw);
    }

    #[test]
    fn parse_kiro_event_full_reads_text_tool_and_usage_events() {
        assert_eq!(
            parse_kiro_event_full(r#"{"assistantResponseEvent":{"content":"hello"}}"#),
            Some(KiroEvent::Text("hello".to_string()))
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"toolUseId":"tool_1","name":"search_docs"}"#),
            Some(KiroEvent::ToolUseStart {
                id: "tool_1".to_string(),
                name: "search_docs".to_string(),
            })
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"toolUseId":"tool_1","input":{"q":"gateway"}}"#),
            Some(KiroEvent::ToolUseInputDelta {
                id: "tool_1".to_string(),
                input_delta: "{\"q\":\"gateway\"}".to_string(),
            })
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"toolUseId":"tool_1","stop":true}"#),
            Some(KiroEvent::ToolUseStop {
                id: "tool_1".to_string(),
            })
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"usage":{"inputTokens":12,"outputTokens":34}}"#),
            Some(KiroEvent::Usage {
                input_tokens: 12,
                output_tokens: 34,
            })
        );
    }

    #[test]
    fn parse_kiro_event_full_reads_reasoning_content() {
        assert_eq!(
            parse_kiro_event_full(r#"{"reasoningContentEvent":{"text":"分析中"}}"#),
            Some(KiroEvent::Thinking("分析中".to_string()))
        );
    }

    #[test]
    fn parse_kiro_event_full_reads_citation_events() {
        assert_eq!(
            parse_kiro_event_full(
                r#"{"target":{"range":{"start":2,"end":5}},"citationText":"Rust","citationLink":"https://example.com/rust"}"#
            ),
            Some(KiroEvent::Citation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: serde_json::json!({ "range": { "start": 2, "end": 5 } }),
            })
        );
        assert_eq!(
            parse_kiro_event_full(
                r#"{"target":{"location":6},"citationText":"Rust","citationLink":"https://example.com/location"}"#
            ),
            Some(KiroEvent::Citation {
                text: Some("Rust".to_string()),
                link: "https://example.com/location".to_string(),
                target: serde_json::json!({ "location": 6 }),
            })
        );
    }

    #[test]
    fn ensure_citation_target_supported_accepts_location_without_guessing_range() {
        assert_eq!(
            ensure_citation_target_supported(&serde_json::json!({ "location": 6 })),
            Some(())
        );
    }

    #[test]
    fn aggregate_kiro_response_collects_citations() {
        let raw = concat!(
            r#"{"assistantResponseEvent":{"content":"Hello Rust"}}"#,
            r#"{"target":{"range":{"start":6,"end":10}},"citationText":"Rust","citationLink":"https://example.com/rust"}"#
        );

        let aggregated = aggregate_kiro_response(raw);

        assert_eq!(aggregated.text, "Hello Rust");
        assert_eq!(
            aggregated.citations,
            vec![AggregatedCitation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: serde_json::json!({ "range": { "start": 6, "end": 10 } }),
            }]
        );
    }

    #[test]
    fn deduplicate_tool_calls_keeps_latest_args_per_id() {
        let input = vec![
            ("tool_1".to_string(), "search".to_string(), "{}".to_string()),
            (
                "tool_1".to_string(),
                "search".to_string(),
                "{\"q\":\"gateway\"}".to_string(),
            ),
            (
                "tool_2".to_string(),
                "open".to_string(),
                "{\"path\":\"README.md\"}".to_string(),
            ),
        ];

        let deduped = deduplicate_tool_calls(input);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].0, "tool_1");
        assert_eq!(deduped[0].2, "{\"q\":\"gateway\"}");
    }
}
