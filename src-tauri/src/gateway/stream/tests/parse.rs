use super::*;

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
            name: None,
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
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            stop_reason: None,
            refusal: None,
        })
    );
}

#[test]
fn parse_kiro_event_full_reads_metadata_stop_reason() {
    // G-8：终帧携 stopReason 时不得丢失
    assert_eq!(
        parse_kiro_event_full(
            r#"{"metadataEvent":{"tokenUsage":{"uncachedInputTokens":10,"outputTokens":20},"stopReason":"MAX_TOKENS"}}"#
        ),
        Some(KiroEvent::Usage {
            input_tokens: 10,
            output_tokens: 20,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            stop_reason: Some("MAX_TOKENS".to_string()),
            refusal: None,
        })
    );
}

#[test]
fn parse_kiro_event_full_accepts_zero_token_stop_frame() {
    // 纯终止帧（无 tokenUsage 或全 0）：token 字段置 0，stop 语义仍要产出
    assert_eq!(
        parse_kiro_event_full(r#"{"metadataEvent":{"stopReason":"END_TURN"}}"#),
        Some(KiroEvent::Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            stop_reason: Some("END_TURN".to_string()),
            refusal: None,
        })
    );
}

#[test]
fn parse_kiro_event_full_reads_refusal_stop_details() {
    // G-9：stopDetails.refusal 的 explanation 优先于 category
    assert_eq!(
        parse_kiro_event_full(
            r#"{"metadataEvent":{"stopReason":"CONTENT_FILTERED","stopDetails":{"refusal":{"category":"policy","explanation":"blocked by filter"}}}}"#
        ),
        Some(KiroEvent::Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            stop_reason: Some("CONTENT_FILTERED".to_string()),
            refusal: Some("blocked by filter".to_string()),
        })
    );
    // 无 explanation 时退 category
    let event = parse_kiro_event_full(
        r#"{"metadataEvent":{"stopReason":"CONTENT_FILTERED","stopDetails":{"refusal":{"category":"policy"}}}}"#,
    );
    assert!(matches!(
        event,
        Some(KiroEvent::Usage { refusal: Some(text), .. }) if text == "policy"
    ));
}

#[test]
fn stop_reason_mapping_covers_known_enum() {
    // G-8b：取证自 ldi 枚举 + Ofd 终态
    assert_eq!(map_stop_reason_to_anthropic("END_TURN"), "end_turn");
    assert_eq!(map_stop_reason_to_anthropic("MAX_TOKENS"), "max_tokens");
    assert_eq!(map_stop_reason_to_anthropic("TOOL_USE"), "tool_use");
    assert_eq!(map_stop_reason_to_anthropic("CONTENT_FILTERED"), "refusal");
    assert_eq!(map_stop_reason_to_anthropic("PAUSE_TURN"), "pause_turn");
    assert_eq!(
        map_stop_reason_to_anthropic("MODEL_CONTEXT_WINDOW_EXCEEDED"),
        "max_tokens"
    );
    assert_eq!(
        map_stop_reason_to_anthropic("COMPACTION_REQUIRED_FOR_MODEL_HANDOFF"),
        "max_tokens"
    );
    assert_eq!(map_stop_reason_to_openai_finish("MAX_TOKENS"), "length");
    assert_eq!(map_stop_reason_to_openai_finish("TOOL_USE"), "tool_calls");
    assert_eq!(
        map_stop_reason_to_openai_finish("CONTENT_FILTERED"),
        "content_filter"
    );
    assert_eq!(
        map_stop_reason_to_responses_incomplete("MAX_TOKENS"),
        Some("max_output_tokens")
    );
    assert_eq!(
        map_stop_reason_to_responses_incomplete("CONTENT_FILTERED"),
        Some("content_filter")
    );
    assert_eq!(map_stop_reason_to_responses_incomplete("END_TURN"), None);
    assert_eq!(map_stop_reason_to_responses_incomplete("PAUSE_TURN"), None);
    assert!(is_kiro_stop_reason("UNKNOWN"));
    assert!(!is_kiro_stop_reason("BOGUS_REASON"));
}

#[test]
fn parse_kiro_event_full_reads_reasoning_content() {
    assert_eq!(
        parse_kiro_event_full(r#"{"reasoningContentEvent":{"text":"分析中"}}"#),
        Some(KiroEvent::Thinking("分析中".to_string()))
    );
}

// G-7：被屏蔽的思考是 base64 字符串，官方 Zfd 原样取用；text 优先于它
// （与官方「text/signature/redacted 三者全空才不产出」的顺序一致）。
#[test]
fn parse_kiro_event_full_reads_redacted_reasoning_content() {
    assert_eq!(
        parse_kiro_event_full(r#"{"reasoningContentEvent":{"redactedContent":"QUJD"}}"#),
        Some(KiroEvent::ThinkingRedacted("QUJD".to_string()))
    );
    assert_eq!(
        parse_kiro_event_full(
            r#"{"reasoningContentEvent":{"text":"t","redactedContent":"QUJD"}}"#
        ),
        Some(KiroEvent::Thinking("t".to_string()))
    );
    assert_eq!(
        parse_kiro_event_full(r#"{"reasoningContentEvent":{"redactedContent":""}}"#),
        None
    );
}

#[test]
fn parse_kiro_event_full_reads_metering_event() {
    assert_eq!(
        parse_kiro_event_full(
            r#"{"meteringEvent":{"unit":"credit","unitPlural":"credits","usage":0.3876425741791045}}"#
        ),
        Some(KiroEvent::Metering {
            unit: "credit".to_string(),
            unit_plural: "credits".to_string(),
            usage: 0.3876425741791045,
        })
    );
}

#[test]
fn parse_kiro_event_full_keeps_tool_name_with_wrapped_input() {
    assert_eq!(
        parse_kiro_event_full(
            r#"{"toolUseEvent":{"toolUseId":"tool_1","name":"server_health","input":"{}"}}"#
        ),
        Some(KiroEvent::ToolUseInputDelta {
            id: "tool_1".to_string(),
            name: Some("server_health".to_string()),
            input_delta: "{}".to_string(),
        })
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
