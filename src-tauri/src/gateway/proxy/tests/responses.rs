//! 下游响应体与流式事件构造的回归测试。

use super::*;

#[test]
fn build_responses_response_emits_kiro_citation_annotations() {
    let aggregated = stream::AggregatedKiroResponse {
        text: "Hello Rust".to_string(),
        thinking: String::new(),
        thinking_signature: None,
        tool_calls: Vec::new(),
        input_tokens: 3,
        output_tokens: 5,
        context_usage_percentage: None,
        citations: vec![stream::AggregatedCitation {
            text: Some("Rust".to_string()),
            link: "https://example.com/rust".to_string(),
            target: json!({ "range": { "start": 6, "end": 10 } }),
        }],
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        metering_usage: None,
        stop_reason: None,
        refusal: None,
    };

    let response = build_responses_response_with_ids(
        "gpt-5.4",
        &aggregated,
        "resp_test",
        "msg_test",
        123,
        None,
    );

    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["type"],
        "url_citation"
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["start_index"],
        6
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["end_index"],
        10
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["url"],
        "https://example.com/rust"
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["citationText"],
        "Rust"
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["citationLink"],
        "https://example.com/rust"
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["target"]["range"]["start"],
        6
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["target"]["range"]["end"],
        10
    );
    assert!(response["output"][0]["content"][0]["annotations"][0]["title"].is_null());
}

#[test]
fn build_responses_response_omits_guessed_range_for_location_citations() {
    let aggregated = stream::AggregatedKiroResponse {
        text: "Hello Rust".to_string(),
        thinking: String::new(),
        thinking_signature: None,
        tool_calls: Vec::new(),
        input_tokens: 3,
        output_tokens: 5,
        context_usage_percentage: None,
        citations: vec![stream::AggregatedCitation {
            text: Some("Rust".to_string()),
            link: "https://example.com/rust".to_string(),
            target: json!({ "location": 6 }),
        }],
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        metering_usage: None,
        stop_reason: None,
        refusal: None,
    };

    let response = build_responses_response_with_ids(
        "gpt-4.1",
        &aggregated,
        "resp_test",
        "msg_test",
        123,
        None,
    );

    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["type"],
        "url_citation"
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["citationText"],
        "Rust"
    );
    assert_eq!(
        response["output"][0]["content"][0]["annotations"][0]["target"]["location"],
        6
    );
    assert!(response["output"][0]["content"][0]["annotations"][0]["start_index"].is_null());
    assert!(response["output"][0]["content"][0]["annotations"][0]["end_index"].is_null());
    assert!(response["output"][0]["content"][0]["annotations"][0]["title"].is_null());
}

#[test]
fn build_anthropic_response_maps_kiro_citations_into_sdk_shape() {
    let aggregated = stream::AggregatedKiroResponse {
        text: "Hello Rust".to_string(),
        thinking: String::new(),
        thinking_signature: None,
        tool_calls: Vec::new(),
        input_tokens: 3,
        output_tokens: 5,
        context_usage_percentage: None,
        citations: vec![stream::AggregatedCitation {
            text: Some("Rust".to_string()),
            link: "https://example.com/rust".to_string(),
            target: json!({ "range": { "start": 6, "end": 10 } }),
        }],
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        metering_usage: None,
        stop_reason: None,
        refusal: None,
    };

    let response = build_anthropic_response("claude-sonnet-4-5", &aggregated);

    assert_eq!(response["content"][0]["type"], "text");
    assert_eq!(
        response["content"][0]["citations"][0]["type"],
        "char_location"
    );
    assert_eq!(
        response["content"][0]["citations"][0]["start_char_index"],
        6
    );
    assert_eq!(response["content"][0]["citations"][0]["end_char_index"], 10);
    assert_eq!(response["content"][0]["citations"][0]["cited_text"], "Rust");
    assert_eq!(
        response["content"][0]["citations"][0]["document_title"],
        "https://example.com/rust"
    );
    assert!(response["content"][0]["citations"][0]["file_id"].is_null());
}

#[test]
fn build_stream_responses_completed_event_keeps_citations_and_tool_calls() {
    let aggregated = stream::AggregatedKiroResponse {
        text: "Hello Rust".to_string(),
        thinking: String::new(),
        thinking_signature: None,
        tool_calls: vec![(
            "call_1".to_string(),
            "search_docs".to_string(),
            "{\"q\":\"rust\"}".to_string(),
        )],
        input_tokens: 3,
        output_tokens: 5,
        context_usage_percentage: None,
        citations: vec![stream::AggregatedCitation {
            text: Some("Rust".to_string()),
            link: "https://example.com/rust".to_string(),
            target: json!({ "range": { "start": 6, "end": 10 } }),
        }],
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        metering_usage: None,
        stop_reason: None,
        refusal: None,
    };

    let event = build_stream_responses_completed_event(
        "gpt-4.1",
        &aggregated,
        "resp_test",
        "msg_test",
        123,
        None,
    );

    assert_eq!(event["type"], "response.completed");
    assert_eq!(event["response"]["output_text"], "Hello Rust");
    assert_eq!(
        event["response"]["output"][0]["content"][0]["annotations"][0]["citationText"],
        "Rust"
    );
    assert!(event["response"]["output"][0]["content"][0]["annotations"][0]["title"].is_null());
    assert_eq!(
        event["response"]["output"][0]["content"][1]["type"],
        "function_call"
    );
    assert_eq!(
        event["response"]["output"][0]["content"][1]["call_id"],
        "call_1"
    );
}

#[test]
fn build_stream_responses_done_events_use_expected_shape() {
    let function_done = build_stream_responses_function_call_arguments_done_event(
        "resp_test",
        "call_1",
        "{\"q\":\"rust\"}",
    );
    let text_done = build_stream_responses_output_text_done_event("resp_test", "Hello Rust");
    let reasoning_done = build_stream_responses_reasoning_done_event("resp_test", "Think");

    assert_eq!(
        function_done["type"],
        "response.function_call_arguments.done"
    );
    assert_eq!(function_done["response_id"], "resp_test");
    assert_eq!(function_done["call_id"], "call_1");
    assert_eq!(function_done["arguments"], "{\"q\":\"rust\"}");

    assert_eq!(text_done["type"], "response.output_text.done");
    assert_eq!(text_done["response_id"], "resp_test");
    assert_eq!(text_done["text"], "Hello Rust");

    assert_eq!(reasoning_done["type"], "response.reasoning.done");
    assert_eq!(reasoning_done["response_id"], "resp_test");
    assert_eq!(reasoning_done["text"], "Think");
}

/// 构造只改 stop_reason/refusal 的聚合体基线
fn aggregated_with_stop(
    stop_reason: Option<&str>,
    refusal: Option<&str>,
) -> stream::AggregatedKiroResponse {
    stream::AggregatedKiroResponse {
        text: "partial output".to_string(),
        thinking: String::new(),
        thinking_signature: None,
        tool_calls: Vec::new(),
        input_tokens: 3,
        output_tokens: 5,
        context_usage_percentage: None,
        citations: Vec::new(),
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        metering_usage: None,
        stop_reason: stop_reason.map(str::to_string),
        refusal: refusal.map(str::to_string),
    }
}

#[test]
fn anthropic_stop_reason_prefers_upstream_max_tokens() {
    // G-8：上游 MAX_TOKENS 不得被启发式报成 end_turn
    let aggregated = aggregated_with_stop(Some("MAX_TOKENS"), None);
    let response =
        build_anthropic_response_with_stop("claude-sonnet-4.5", &aggregated, None);
    assert_eq!(response["stop_reason"], "max_tokens");
}

#[test]
fn anthropic_stop_sequence_hit_wins_over_upstream() {
    let aggregated = aggregated_with_stop(Some("MAX_TOKENS"), None);
    let response = build_anthropic_response_with_stop(
        "claude-sonnet-4.5",
        &aggregated,
        Some("###STOP"),
    );
    assert_eq!(response["stop_reason"], "stop_sequence");
    assert_eq!(response["stop_sequence"], "###STOP");
}

#[test]
fn responses_terminal_reports_incomplete_on_max_tokens() {
    // G-8b：截断终态报 incomplete + incomplete_details
    let aggregated = aggregated_with_stop(Some("MAX_TOKENS"), None);
    let event = build_stream_responses_completed_event(
        "claude-sonnet-4.5",
        &aggregated,
        "resp_test",
        "msg_test",
        123,
        None,
    );
    assert_eq!(event["type"], "response.incomplete");
    assert_eq!(event["response"]["status"], "incomplete");
    assert_eq!(
        event["response"]["incomplete_details"]["reason"],
        "max_output_tokens"
    );
}

#[test]
fn responses_terminal_content_filter_reason() {
    let aggregated = aggregated_with_stop(Some("CONTENT_FILTERED"), Some("blocked"));
    let event = build_stream_responses_completed_event(
        "claude-sonnet-4.5",
        &aggregated,
        "resp_test",
        "msg_test",
        123,
        None,
    );
    assert_eq!(event["type"], "response.incomplete");
    assert_eq!(
        event["response"]["incomplete_details"]["reason"],
        "content_filter"
    );
}

#[test]
fn responses_terminal_end_turn_stays_completed() {
    let aggregated = aggregated_with_stop(Some("END_TURN"), None);
    let event = build_stream_responses_completed_event(
        "claude-sonnet-4.5",
        &aggregated,
        "resp_test",
        "msg_test",
        123,
        None,
    );
    assert_eq!(event["type"], "response.completed");
    assert!(event["response"]["incomplete_details"].is_null());
}

#[test]
fn openai_response_fills_refusal_and_content_filter_finish() {
    // G-9 + G-8b：CONTENT_FILTERED → finish_reason=content_filter 且 refusal 回填
    let aggregated = aggregated_with_stop(Some("CONTENT_FILTERED"), Some("blocked by filter"));
    let response = stream::build_openai_response("gpt-5.6", &aggregated);
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("content_filter"));
    assert_eq!(
        response.choices[0].message.refusal.as_deref(),
        Some("blocked by filter")
    );
}

#[test]
fn openai_response_without_upstream_stop_keeps_heuristic() {
    // 无上游 stopReason：有工具→tool_calls，否则 stop（启发式兑底保留）
    let mut aggregated = aggregated_with_stop(None, None);
    aggregated.tool_calls = vec![("call_1".into(), "search".into(), "{}".into())];
    let response = stream::build_openai_response("gpt-5.6", &aggregated);
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("tool_calls"));

    let plain = aggregated_with_stop(None, None);
    let response = stream::build_openai_response("gpt-5.6", &plain);
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("stop"));
}
