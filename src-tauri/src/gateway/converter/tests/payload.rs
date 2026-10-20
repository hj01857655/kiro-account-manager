//! Kiro 上游 payload 构造（build_kiro_payload）的回归测试。

use super::*;

#[tokio::test]
async fn build_kiro_payload_moves_long_tool_docs_and_tool_results_into_context() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![
            NormalizedMessage {
                role: "system".to_string(),
                content: Some(json!("系统要求")),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
            NormalizedMessage {
                role: "assistant".to_string(),
                content: Some(json!("我先调用工具")),
                tool_calls: Some(vec![crate::gateway::models::ToolCall {
                    id: "call_1".to_string(),
                    call_type: "function".to_string(),
                    function: crate::gateway::models::ToolCallFunction {
                        name: "search_docs".to_string(),
                        arguments: "{\"q\":\"gateway\"}".to_string(),
                    },
                }]),
                tool_call_id: None,
                metadata: None,
            },
            NormalizedMessage {
                role: "tool".to_string(),
                content: Some(json!("命中结果")),
                tool_calls: None,
                tool_call_id: Some("call_1".to_string()),
                metadata: None,
            },
            NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!("继续总结")),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
        ],
        stream: true,
        max_tokens: Some(2048),
        temperature: Some(0.1),
        top_p: None,
        stop: Some(vec!["END".to_string()]),
        tools: Some(vec![Tool {
            tool_type: "function".to_string(),
            function: crate::gateway::models::ToolFunction {
                name: "search_docs".to_string(),
                description: Some("A".repeat(TOOL_DESCRIPTION_MAX_LENGTH + 32)),
                parameters: Some(json!({
                    "type": "object",
                    "properties": { "q": { "type": "string" } }
                })),
            },
            cache_control: None,
        }]),
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(
        &Client::new(),
        &request,
        Some("arn:aws:codewhisperer:::profile/test".to_string()),
        None,
    )
    .await
    .expect("payload should build");
    let current = &payload
        .conversation_state
        .current_message
        .user_input_message;

    assert!(current.content.contains("Tool Documentation"));
    assert_eq!(current.model_id, "claude-sonnet-4.5");
    assert_eq!(
        payload.profile_arn.as_deref(),
        Some("arn:aws:codewhisperer:::profile/test")
    );

    let history = payload
        .conversation_state
        .history
        .expect("history should exist");
    // sanitize_history 会补全结构：
    //   [0] 占位 User("Hello")（保证以 user 开头）
    //   [1] Assistant（携带 toolUses）
    //   [2] User（携带 toolResults）
    //   [3] 占位 Assistant("understood")（修复两个连续 user）
    assert_eq!(history.len(), 4);
    match &history[1] {
        HistoryItem::Assistant {
            assistant_response_message,
        } => {
            assert_eq!(
                assistant_response_message.tool_uses.as_ref().map(Vec::len),
                Some(1)
            );
        }
        other => panic!("unexpected history item: {other:?}"),
    }
    match &history[2] {
        HistoryItem::User { user_input_message } => {
            let context = user_input_message
                .user_input_message_context
                .as_ref()
                .expect("tool result context should exist");
            assert_eq!(context.tool_results.as_ref().map(Vec::len), Some(1));
        }
        other => panic!("unexpected history item: {other:?}"),
    }
}

#[tokio::test]
async fn build_kiro_payload_uses_cached_style_model_ids_for_claude_45() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("hello")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");

    assert_eq!(
        payload
            .conversation_state
            .current_message
            .user_input_message
            .model_id,
        "claude-sonnet-4.5"
    );
}

#[tokio::test]
async fn build_kiro_payload_uses_cached_style_model_ids_for_claude_46() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-6".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("hello")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");

    assert_eq!(
        payload
            .conversation_state
            .current_message
            .user_input_message
            .model_id,
        "claude-sonnet-4.6"
    );
}

#[tokio::test]
async fn build_kiro_payload_preserves_responses_tool_choice() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("hello")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: Some(vec![Tool {
            tool_type: "function".to_string(),
            function: crate::gateway::models::ToolFunction {
                name: "search_docs".to_string(),
                description: Some("搜索文档".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": { "q": { "type": "string" } }
                })),
            },
            cache_control: None,
        }]),
        tool_choice: Some(json!({ "type": "function", "name": "search_docs" })),
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");

    // Kiro API 实际请求中不包含 tool_choice 字段，
    // tool_choice 由网关消费但不传递给上游 —— 仅验证 tools 正常转发即可
    let context = payload
        .conversation_state
        .current_message
        .user_input_message
        .user_input_message_context
        .as_ref()
        .expect("tools context should exist");

    let kiro_tools = context.tools.as_ref().expect("tools should be present");
    assert_eq!(kiro_tools.len(), 1);
}

#[tokio::test]
async fn build_kiro_payload_includes_tools_when_current_message_has_tool_results() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!([{
                "type": "tool_result",
                "tool_use_id": "toolu_123",
                "content": "result text"
            }])),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: Some(vec![Tool {
            tool_type: "function".to_string(),
            function: crate::gateway::models::ToolFunction {
                name: "search_docs".to_string(),
                description: Some("搜索文档".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": { "q": { "type": "string" } }
                })),
            },
            cache_control: None,
        }]),
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");

    let context = payload
        .conversation_state
        .current_message
        .user_input_message
        .user_input_message_context
        .as_ref()
        .expect("tool_results context should exist");

    assert!(
        payload
            .conversation_state
            .current_message
            .user_input_message
            .content
            .is_empty(),
        "tool result continuation content must be empty"
    );
    assert_eq!(
        context.tools.as_ref().map(Vec::len),
        Some(1),
        "Kiro IDE includes tools with current toolResults"
    );
    assert_eq!(context.tool_results.as_ref().map(Vec::len), Some(1));
}

#[tokio::test]
async fn build_kiro_payload_orders_current_tool_results_like_previous_tool_uses() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![
            NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!("search twice")),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
            NormalizedMessage {
                role: "assistant".to_string(),
                content: None,
                tool_calls: Some(vec![
                    ToolCall {
                        id: "tooluse_a".to_string(),
                        call_type: "function".to_string(),
                        function: ToolCallFunction {
                            name: "searchPathnamesOnly".to_string(),
                            arguments: "{}".to_string(),
                        },
                    },
                    ToolCall {
                        id: "tooluse_b".to_string(),
                        call_type: "function".to_string(),
                        function: ToolCallFunction {
                            name: "runTerminalCmd".to_string(),
                            arguments: "{}".to_string(),
                        },
                    },
                ]),
                tool_call_id: None,
                metadata: None,
            },
            NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!([
                    {
                        "type": "tool_result",
                        "tool_use_id": "tooluse_b",
                        "content": "second result"
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tooluse_a",
                        "content": "first result"
                    }
                ])),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
        ],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");

    let context = payload
        .conversation_state
        .current_message
        .user_input_message
        .user_input_message_context
        .as_ref()
        .expect("tool_results context should exist");

    let ids: Vec<_> = context
        .tool_results
        .as_ref()
        .expect("tool_results should exist")
        .iter()
        .map(|result| result.tool_use_id.as_str())
        .collect();

    assert_eq!(ids, vec!["tooluse_a", "tooluse_b"]);
    assert!(
        payload
            .conversation_state
            .current_message
            .user_input_message
            .content
            .is_empty(),
        "tool result continuation content must be empty"
    );
}

#[tokio::test]
async fn build_kiro_payload_reuses_previous_response_id_as_conversation_id() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("继续")),
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
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");

    assert_eq!(payload.conversation_state.conversation_id, "resp_prev_123");
}

#[tokio::test]
async fn build_kiro_payload_rejects_unknown_tool_choice_function() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("hello")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: Some(vec![Tool {
            tool_type: "function".to_string(),
            function: crate::gateway::models::ToolFunction {
                name: "search_docs".to_string(),
                description: Some("搜索文档".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": { "q": { "type": "string" } }
                })),
            },
            cache_control: None,
        }]),
        tool_choice: Some(json!({ "type": "function", "name": "missing_tool" })),
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let error = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect_err("unknown tool choice should fail");

    assert!(error.contains("tool_choice 指定的工具不存在"));
}

#[tokio::test]
async fn build_kiro_payload_preserves_assistant_message_metadata() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![
            NormalizedMessage {
                role: "assistant".to_string(),
                content: Some(json!([
                    { "type": "output_text", "text": "历史回答" },
                    { "type": "reasoning", "summary": "内部推理" }
                ])),
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    call_type: "function".to_string(),
                    function: ToolCallFunction {
                        name: "search_docs".to_string(),
                        arguments: "{\"q\":\"gateway\"}".to_string(),
                    },
                }]),
                tool_call_id: None,
                metadata: Some(json!({
                    "reasoningContent": {
                        "reasoningText": {
                            "text": "内部推理",
                            "signature": "sig_1"
                        }
                    },
                    "references": [
                        {
                            "licenseName": "MIT",
                            "repository": "repo",
                            "url": "https://example.com/ref"
                        }
                    ],
                    "supplementaryWebLinks": [
                        {
                            "url": "https://example.com",
                            "title": "example",
                            "snippet": "snippet"
                        }
                    ],
                    "followupPrompt": {
                        "content": "继续",
                        "userIntent": "SHOW_EXAMPLES"
                    },
                    "messageId": "msg_123",
                    "cachePoint": {
                        "type": "default"
                    }
                })),
            },
            NormalizedMessage {
                role: "user".to_string(),
                content: Some(Value::String("继续".to_string())),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            },
        ],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");
    let history = payload
        .conversation_state
        .history
        .expect("history should exist");

    // history[0] 是 sanitize_history 补的占位 User("Hello")，assistant 消息在 [1]
    match &history[1] {
        HistoryItem::Assistant {
            assistant_response_message,
        } => {
            assert_eq!(assistant_response_message.content, "历史回答");
            assert_eq!(
                assistant_response_message.reasoning_content,
                Some(json!({
                    "reasoningText": {
                        "text": "内部推理",
                        "signature": "sig_1"
                    }
                }))
            );
            assert_eq!(
                assistant_response_message.references,
                Some(json!([
                    {
                        "licenseName": "MIT",
                        "repository": "repo",
                        "url": "https://example.com/ref"
                    }
                ]))
            );
            assert_eq!(
                assistant_response_message.supplementary_web_links,
                Some(json!([
                    {
                        "url": "https://example.com",
                        "title": "example",
                        "snippet": "snippet"
                    }
                ]))
            );
            assert_eq!(
                assistant_response_message.followup_prompt,
                Some(json!({
                    "content": "继续",
                    "userIntent": "SHOW_EXAMPLES"
                }))
            );
            assert_eq!(
                assistant_response_message.message_id.as_deref(),
                Some("msg_123")
            );
            assert_eq!(
                assistant_response_message.cache_point,
                Some(json!({ "type": "default" }))
            );
        }
        other => panic!("unexpected history item: {other:?}"),
    }
}

fn minimal_request_with_thinking(model: &str, thinking: Option<crate::gateway::models::Thinking>) -> NormalizedRequest {
    NormalizedRequest {
        model: model.to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!("1+1=?")),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    }
}

// 官方 thinking 通道白名单：thinking 启用且模型支持时才带
// additionalModelRequestFields；4.5 代际 / GPT-5.6 / 开源 / auto 必须为 None
// （社区实测：盲传会被上游 400 或整体跳过）。
#[tokio::test]
async fn build_kiro_payload_adds_additional_fields_only_for_supported_models() {
    let thinking = Some(crate::gateway::models::Thinking {
        thinking_type: "adaptive".to_string(),
        budget_tokens: 20000,
        effort: None,
    });

    for model in [
        "claude-sonnet-4.6",
        "claude-opus-4.6",
        "claude-opus-5.5",
        "claude-sonnet-5",
        "claude-fable-5",
    ] {
        let payload = build_kiro_payload(
            &Client::new(),
            &minimal_request_with_thinking(model, thinking.clone()),
            None,
            None,
        )
        .await
        .expect("payload should build");
        let fields = payload
            .additional_model_request_fields
            .unwrap_or_else(|| panic!("{model} should carry additionalModelRequestFields"));
        assert_eq!(
            fields["thinking"]["type"], "adaptive",
            "{model} thinking type should be adaptive"
        );
    }

    for model in [
        "claude-sonnet-4.5",
        "claude-opus-4.5",
        "claude-haiku-4.5",
        "gpt-5.6-sol",
        "deepseek-3.2",
        "auto",
    ] {
        let payload = build_kiro_payload(
            &Client::new(),
            &minimal_request_with_thinking(model, thinking.clone()),
            None,
            None,
        )
        .await
        .expect("payload should build");
        assert!(
            payload.additional_model_request_fields.is_none(),
            "{model} must NOT carry additionalModelRequestFields"
        );
    }
}

// thinking 未启用时不带该字段（非 thinking 请求不受影响）。
#[tokio::test]
async fn build_kiro_payload_omits_additional_fields_without_thinking() {
    let payload = build_kiro_payload(
        &Client::new(),
        &minimal_request_with_thinking("claude-sonnet-4.6", None),
        None,
        None,
    )
    .await
    .expect("payload should build");
    assert!(payload.additional_model_request_fields.is_none());
}

// effort 档位原生透传（kiro.rs 实测规则）：客户端 effort 进 output_config.effort；
// 4.6 代不接受 xhigh 降级 high；5.x/fable 保留 xhigh；无 effort 不带 output_config。
#[tokio::test]
async fn build_kiro_payload_passes_effort_into_output_config() {
    let client = Client::new();
    async fn build(
        client: &Client,
        model: &str,
        effort: Option<&str>,
    ) -> serde_json::Value {
        let request = minimal_request_with_thinking(
            model,
            Some(crate::gateway::models::Thinking {
                thinking_type: "adaptive".to_string(),
                budget_tokens: 20000,
                effort: effort.map(str::to_string),
            }),
        );
        build_kiro_payload(client, &request, None, None)
            .await
            .expect("payload should build")
            .additional_model_request_fields
            .expect("fields should exist")
    }

    // sonnet-4.6 + xhigh → 降级 high（4.6 代不接受 xhigh）
    let fields = build(&client, "claude-sonnet-4.6", Some("xhigh"))
        .await;
    assert_eq!(fields["output_config"]["effort"], "high");

    // sonnet-5.5 + xhigh → 保留
    let fields = build(&client, "claude-sonnet-5.5", Some("xhigh"))
        .await;
    assert_eq!(fields["output_config"]["effort"], "xhigh");

    // opus-5.5 + high → 原样
    let fields = build(&client, "claude-opus-5.5", Some("high"))
        .await;
    assert_eq!(fields["output_config"]["effort"], "high");

    // 未知档位 → 回退 high
    let fields = build(&client, "claude-sonnet-5.5", Some("turbo"))
        .await;
    assert_eq!(fields["output_config"]["effort"], "high");

    // 无 effort → 只有 thinking 子字段，不带 output_config
    let fields = build(&client, "claude-sonnet-5.5", None)
        .await;
    assert!(fields.get("output_config").is_none());
    assert_eq!(fields["thinking"]["type"], "adaptive");
}

// 客户端 budget_tokens 必须传导到 <max_thinking_length> 标签（clamp 到
// [1024, 24576]，kiro.rs 实测上限），否则「尊重显式配置」是空话。
#[tokio::test]
async fn build_kiro_payload_clamps_budget_tokens_into_thinking_tag() {
    let make_thinking = |budget| {
        Some(crate::gateway::models::Thinking {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget,
            effort: None,
        })
    };

    // 范围内：原样映射
    let payload = build_kiro_payload(
        &Client::new(),
        &minimal_request_with_thinking("claude-sonnet-4.5", make_thinking(10_000)),
        None,
        None,
    )
    .await
    .expect("payload should build");
    let content = &payload.conversation_state.current_message.user_input_message.content;
    assert!(
        content.contains("<max_thinking_length>10000</max_thinking_length>"),
        "budget 10000 should be mapped as-is, got: {content}"
    );

    // 超上限：夹紧到 24576（不能回落到更大的 200000，客户端限制意图必须保留）
    let payload = build_kiro_payload(
        &Client::new(),
        &minimal_request_with_thinking("claude-sonnet-4.5", make_thinking(60_000)),
        None,
        None,
    )
    .await
    .expect("payload should build");
    let content = &payload.conversation_state.current_message.user_input_message.content;
    assert!(
        content.contains("<max_thinking_length>24576</max_thinking_length>"),
        "budget 60000 should clamp to 24576, got: {content}"
    );

    // 低于官方最小值：夹紧到 1024
    let payload = build_kiro_payload(
        &Client::new(),
        &minimal_request_with_thinking("claude-sonnet-4.5", make_thinking(100)),
        None,
        None,
    )
    .await
    .expect("payload should build");
    let content = &payload.conversation_state.current_message.user_input_message.content;
    assert!(
        content.contains("<max_thinking_length>1024</max_thinking_length>"),
        "budget 100 should clamp to 1024, got: {content}"
    );
}
