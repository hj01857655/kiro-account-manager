use super::*;

#[test]
fn aggregate_kiro_response_from_payloads_keeps_name_when_input_arrives_first() {
    let payloads = vec![
        r#"{"toolUseEvent":{"toolUseId":"tool_1","name":"server_health","input":"{}"}}"#
            .to_string(),
        r#"{"toolUseEvent":{"toolUseId":"tool_1","stop":true}}"#.to_string(),
    ];

    let aggregated = aggregate_kiro_response_from_payloads(&payloads);

    assert_eq!(
        aggregated.tool_calls,
        vec![(
            "tool_1".to_string(),
            "server_health".to_string(),
            "{}".to_string()
        )]
    );
}

#[test]
fn aggregate_kiro_response_from_payloads_concatenates_string_input_deltas() {
    let payloads = vec![
        r#"{"toolUseEvent":{"toolUseId":"tool_1","name":"todo_write","input":"{\"todos\":[{\"content\":\""}}"#.to_string(),
        r#"{"toolUseEvent":{"toolUseId":"tool_1","input":"plan"}}"#.to_string(),
        r#"{"toolUseEvent":{"toolUseId":"tool_1","input":"a\",\"status\":\"pending\"}]}"}}"#.to_string(),
        r#"{"toolUseEvent":{"toolUseId":"tool_1","stop":true}}"#.to_string(),
    ];

    let aggregated = aggregate_kiro_response_from_payloads(&payloads);

    assert_eq!(aggregated.tool_calls.len(), 1);
    assert_eq!(aggregated.tool_calls[0].0, "tool_1");
    assert_eq!(aggregated.tool_calls[0].1, "todo_write");
    assert_eq!(
        aggregated.tool_calls[0].2,
        r#"{"todos":[{"content":"plana","status":"pending"}]}"#
    );
}

#[test]
fn aggregate_kiro_response_from_payloads_collects_citations() {
    let payloads = vec![
        r#"{"assistantResponseEvent":{"content":"Hello Rust"}}"#.to_string(),
        r#"{"target":{"range":{"start":6,"end":10}},"citationText":"Rust","citationLink":"https://example.com/rust"}"#.to_string(),
    ];

    let aggregated = aggregate_kiro_response_from_payloads(&payloads);

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

#[test]
fn aggregate_keeps_usage_when_terminal_stop_frame_carries_no_tokens() {
    // G-8：上游先给 tokenUsage、再给一个只带 stopReason 的终帧；
    // 终帧不得把已累计的 usage 清零
    let payloads = vec![
        r#"{"metadataEvent":{"tokenUsage":{"uncachedInputTokens":10,"outputTokens":20},"stopReason":"END_TURN"}}"#.to_string(),
        r#"{"metadataEvent":{"stopReason":"MAX_TOKENS"}}"#.to_string(),
    ];

    let aggregated = aggregate_kiro_response_from_payloads(&payloads);

    assert_eq!(aggregated.input_tokens, 10);
    assert_eq!(aggregated.output_tokens, 20);
    assert_eq!(aggregated.stop_reason.as_deref(), Some("MAX_TOKENS"));
}

#[test]
fn aggregate_collects_refusal_from_stop_details() {
    // G-9：stopDetails.refusal 在 CONTENT_FILTERED 时随 stopReason 一同聚合
    let payloads = vec![
        r#"{"metadataEvent":{"tokenUsage":{"uncachedInputTokens":3,"outputTokens":0},"stopReason":"CONTENT_FILTERED","stopDetails":{"refusal":{"category":"policy","explanation":"flagged"}}}}"#
            .to_string(),
    ];

    let aggregated = aggregate_kiro_response_from_payloads(&payloads);

    assert_eq!(aggregated.stop_reason.as_deref(), Some("CONTENT_FILTERED"));
    assert_eq!(aggregated.refusal.as_deref(), Some("flagged"));
}

#[test]
fn aggregate_maps_model_context_window_exceeded_to_max_tokens() {
    // G-8b：窗口超限与压缩交接都归入 max_tokens（映射手法在此验证）
    for reason in ["MODEL_CONTEXT_WINDOW_EXCEEDED", "COMPACTION_REQUIRED_FOR_MODEL_HANDOFF"] {
        assert_eq!(
            map_stop_reason_to_anthropic(reason),
            "max_tokens",
            "{reason}"
        );
        assert_eq!(
            map_stop_reason_to_responses_incomplete(reason),
            Some("max_output_tokens"),
            "{reason}"
        );
    }
    // PAUSE_TURN 是 Anthropic 自有枚举，原值透传不降为 end_turn
    assert_eq!(map_stop_reason_to_anthropic("PAUSE_TURN"), "pause_turn");
}
