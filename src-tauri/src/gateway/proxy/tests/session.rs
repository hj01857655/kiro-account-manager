//! Responses 会话复用的回归测试。

use super::*;

#[tokio::test]
async fn restore_responses_session_messages_replays_previous_assistant_turn() {
    let state = proxy_test_state();
    {
        let mut sessions = state.responses_sessions.lock().await;
        sessions.insert(
            "resp_prev_123".to_string(),
            ResponsesSessionEntry {
                response_id: "resp_prev_123".to_string(),
                previous_response_id: None,
                request_messages: vec![NormalizedMessage {
                    role: "user".to_string(),
                    content: Some(json!("第一问")),
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: None,
                }],
                response_text: "第一答".to_string(),
                tool_calls: vec![(
                    "call_1".to_string(),
                    "search_docs".to_string(),
                    "{\"q\":\"gateway\"}".to_string(),
                )],
                request_tools: None,
                request_tool_choice: None,
                updated_at: Instant::now(),
            },
        );
    }

    let request = NormalizedRequest {
        model: "claude-sonnet-4-5".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("第二问")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: Some("resp_prev_123".to_string()),
        thinking: None,
        effort: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
        deferred_tool_names: Vec::new(),
        deferred_tools: Vec::new(),
    };

    let merged = restore_responses_session_messages(&state, &request).await;

    assert_eq!(merged.len(), 3);
    assert_eq!(merged[0].role, "user");
    assert_eq!(merged[1].role, "assistant");
    assert_eq!(merged[2].role, "user");
    assert_eq!(merged[1].content, Some(json!("第一答")));
    assert_eq!(
        merged[1]
            .tool_calls
            .as_ref()
            .and_then(|items| items.first())
            .map(|call| call.function.name.as_str()),
        Some("search_docs")
    );
}

#[test]
fn prune_sessions_drops_expired_entries() {
    let mut sessions: HashMap<String, ResponsesSessionEntry> = HashMap::new();

    let entry = ResponsesSessionEntry {
        response_id: String::new(),
        previous_response_id: None,
        request_messages: Vec::new(),
        request_tools: None,
        request_tool_choice: None,
        response_text: String::new(),
        tool_calls: Vec::new(),
        updated_at: Instant::now(),
    };

    // 过期条目：TTL 1 小时，这里造一个 2 小时前的
    sessions.insert(
        "stale".to_string(),
        ResponsesSessionEntry {
            updated_at: Instant::now() - Duration::from_secs(2 * 60 * 60),
            ..entry.clone()
        },
    );
    // 新鲜条目：5 分钟前的算新鲜
    sessions.insert(
        "fresh".to_string(),
        ResponsesSessionEntry {
            updated_at: Instant::now() - Duration::from_secs(5 * 60),
            ..entry.clone()
        },
    );

    prune_sessions(&mut sessions);

    assert!(sessions.contains_key("fresh"), "新鲜会话应保留");
    assert!(!sessions.contains_key("stale"), "过期会话应淘汰");
}

#[test]
fn prune_sessions_enforces_entry_cap() {
    let mut sessions: HashMap<String, ResponsesSessionEntry> = HashMap::new();

    // 造 250 个会话（上限 200）
    for i in 0..250 {
        sessions.insert(
            format!("s{i}"),
            ResponsesSessionEntry {
                response_id: format!("s{i}"),
                previous_response_id: None,
                request_messages: Vec::new(),
                request_tools: None,
                request_tool_choice: None,
                response_text: String::new(),
                tool_calls: Vec::new(),
                updated_at: Instant::now(),
            },
        );
    }

    prune_sessions(&mut sessions);

    assert_eq!(sessions.len(), 200, "应裁到条数上限");
}
