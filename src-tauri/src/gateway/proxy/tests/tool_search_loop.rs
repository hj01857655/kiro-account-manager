//! Tool Search 代答环的回合构造逻辑（非流式与流式共用）。
//!
//! 缓冲/重发本身依赖真实上游 HTTP，单测覆盖不到；这里锁住两侧共用的
//! 可测部分：从聚合结果挑出 tool_search 调用、生成 assistant(tool_use) +
//! user(tool_result) 两条消息，以及名字/ID 的保真约束。

use super::*;
use crate::gateway::proxy::upstream::build_tool_search_turn;

fn deferred_tools() -> Vec<Tool> {
    use crate::gateway::models::ToolFunction;
    vec![Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: "Read".to_string(),
            description: Some("Read a file from the local filesystem".to_string()),
            parameters: Some(json!({
                "type": "object",
                "properties": {"file_path": {"type": "string"}}
            })),
        },
        cache_control: None,
    }]
}

fn aggregated_with(calls: Vec<(&str, &str, &str)>) -> stream::AggregatedKiroResponse {
    let mut aggregated = stream::AggregatedKiroResponse::default();
    aggregated.tool_calls = calls
        .into_iter()
        .map(|(id, name, args)| (id.to_string(), name.to_string(), args.to_string()))
        .collect();
    aggregated
}

#[test]
fn no_tool_search_call_ends_the_loop() {
    let aggregated = aggregated_with(vec![("call_1", "Read", r#"{"file_path":"a.txt"}"#)]);
    assert!(
        build_tool_search_turn(&aggregated, &deferred_tools()).is_none(),
        "普通工具调用不应触发代答环"
    );
}

#[test]
fn tool_search_call_produces_paired_messages() {
    let aggregated = aggregated_with(vec![(
        "call_ts",
        "tool_search",
        r#"{"query":"read file"}"#,
    )]);

    let (messages, answer) =
        build_tool_search_turn(&aggregated, &deferred_tools()).expect("should intercept");
    assert_eq!(messages.len(), 2, "assistant + user 两条");

    // assistant: 保留模型实际调用的名字与 id
    let assistant = &messages[0];
    assert_eq!(assistant.role, "assistant");
    let calls = assistant.tool_calls.as_ref().expect("tool_calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call_ts");
    assert_eq!(calls[0].function.name, "tool_search");

    // user: tool_use_id 必须与 assistant 的 id 一致（上游强校验）
    let user = &messages[1];
    assert_eq!(user.role, "user");
    assert_eq!(user.tool_call_id.as_deref(), Some("call_ts"));
    let content = user.content.as_ref().expect("content");
    assert_eq!(
        content[0].get("tool_use_id").and_then(Value::as_str),
        Some("call_ts")
    );

    // 回灌内容携带被拆下工具的完整定义
    assert!(answer.contains("Read"), "answer: {answer}");
    assert!(answer.contains("input_schema"), "answer: {answer}");
}

/// 模型按官方别名调用时也要拦截，且 assistant 历史保留别名（保真）。
#[test]
fn official_alias_call_is_intercepted_and_preserved() {
    let aggregated = aggregated_with(vec![(
        "call_alias",
        "tool_search_tool_regex_20251119",
        r#"{"pattern":"file"}"#,
    )]);

    let (messages, answer) =
        build_tool_search_turn(&aggregated, &deferred_tools()).expect("alias must be intercepted");
    let calls = messages[0].tool_calls.as_ref().expect("tool_calls");
    assert_eq!(
        calls[0].function.name, "tool_search_tool_regex_20251119",
        "assistant 消息应保留模型实际调用的名字"
    );
    assert!(answer.contains("Read"), "pattern 参数也要能检索: {answer}");
}

/// 多个 tool_search 调用时只处理第一个，其余留给下一轮环。
#[test]
fn only_first_tool_search_call_is_taken_per_round() {
    let aggregated = aggregated_with(vec![
        ("call_a", "tool_search", r#"{"query":"read"}"#),
        ("call_b", "tool_search", r#"{"query":"read"}"#),
    ]);

    let (messages, _) = build_tool_search_turn(&aggregated, &deferred_tools()).expect("intercept");
    let calls = messages[0].tool_calls.as_ref().expect("tool_calls");
    assert_eq!(calls[0].id, "call_a", "每轮只取第一个");
}
