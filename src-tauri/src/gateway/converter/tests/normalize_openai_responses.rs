//! OpenAI Responses 协议请求归一化的回归测试。

use super::*;

#[test]
fn normalize_openai_responses_request_preserves_message_content_items() {
    let payload = json!({
        "model": "claude-sonnet-4",
        "stream": true,
        "input": [
            {
                "role": "user",
                "content": [
                    { "type": "input_text", "text": "第一段" },
                    { "type": "input_text", "text": "第二段" },
                    { "type": "input_image", "image_url": "data:image/png;base64,aGVsbG8=" }
                ]
            }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses payload should convert");
    assert!(converted.stream);
    assert_eq!(converted.messages.len(), 1);
    assert_eq!(converted.messages[0].role, "user");
    assert_eq!(
        converted.messages[0].content,
        Some(json!([
            { "type": "input_text", "text": "第一段" },
            { "type": "input_text", "text": "第二段" },
            { "type": "input_image", "image_url": "data:image/png;base64,aGVsbG8=" }
        ]))
    );
}

#[test]
fn normalize_openai_responses_request_defaults_to_claude_sonnet_45() {
    let payload = json!({
        "input": [
            {
                "role": "user",
                "content": "hello"
            }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses payload should convert");
    assert_eq!(converted.model, "claude-sonnet-4-5-20250929");
}

#[test]
fn normalize_openai_responses_request_keeps_tools_tool_choice_and_function_call_items() {
    let payload = json!({
        "model": "claude-sonnet-4",
        "tool_choice": { "type": "function", "name": "search_docs" },
        "tools": [
            {
                "type": "function",
                "name": "search_docs",
                "description": "搜索文档",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "q": { "type": "string" }
                    },
                    "required": ["q"]
                }
            }
        ],
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    { "type": "input_text", "text": "先检索 gateway" }
                ]
            },
            {
                "type": "function_call",
                "call_id": "call_1",
                "name": "search_docs",
                "arguments": "{\"q\":\"gateway\"}"
            },
            {
                "type": "function_call_output",
                "call_id": "call_1",
                "output": "命中结果"
            }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses payload should convert");

    assert_eq!(
        converted.tool_choice,
        Some(json!({ "type": "function", "name": "search_docs" }))
    );
    assert_eq!(converted.tools.as_ref().map(Vec::len), Some(1));
    // 工具名会被规范化为 camelCase（Kiro API 要求）
    assert_eq!(
        converted
            .tools
            .as_ref()
            .and_then(|items| items.first())
            .map(|tool| tool.function.name.as_str()),
        Some("searchDocs")
    );
    assert_eq!(converted.messages.len(), 3);
    assert_eq!(converted.messages[0].role, "user");
    assert_eq!(
        converted.messages[0].content,
        Some(json!([
            { "type": "input_text", "text": "先检索 gateway" }
        ]))
    );
    // 工具调用名在归一化阶段保持原样（构造上游 payload 时才做 camelCase 转换）
    assert_eq!(
        converted.messages[1]
            .tool_calls
            .as_ref()
            .and_then(|items| items.first())
            .map(|call| call.function.name.as_str()),
        Some("search_docs")
    );
    assert_eq!(converted.messages[2].role, "tool");
    assert_eq!(
        converted.messages[2].tool_call_id.as_deref(),
        Some("call_1")
    );
    assert_eq!(converted.messages[2].content, Some(json!("命中结果")));
}

#[test]
fn normalize_openai_responses_maps_developer_and_reasoning_and_skips_unknown_type() {
    let payload = json!({
        "model": "claude-sonnet-4",
        "input": [
            { "role": "developer", "content": "系统约束" },
            {
                "type": "reasoning",
                "summary": [{ "type": "summary_text", "text": "思考过程" }],
                "content": [{ "type": "reasoning_text", "text": "思考过程" }]
            },
            {
                "type": "function_call",
                "call_id": "call_2",
                "name": "lookup",
                "arguments": "{}"
            },
            {
                "type": "function_call_output",
                "call_id": "call_2",
                "output": "ok"
            },
            { "type": "totally_unknown_item", "foo": 1 }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses should convert");
    assert_eq!(converted.messages[0].role, "system");
    assert_eq!(converted.messages[0].content, Some(json!("系统约束")));
    assert_eq!(converted.messages[1].role, "assistant");
    assert!(converted.messages[1].metadata.is_some());
    assert_eq!(
        converted.messages[2]
            .tool_calls
            .as_ref()
            .and_then(|c| c.first())
            .map(|c| c.function.name.as_str()),
        Some("lookup")
    );
    assert_eq!(converted.messages[3].role, "tool");
    assert_eq!(converted.messages[3].tool_call_id.as_deref(), Some("call_2"));
    // unknown type 被跳过，不伪造 user 文本
    assert_eq!(converted.messages.len(), 4);
}

#[test]
fn normalize_openai_responses_request_preserves_assistant_message_metadata() {
    let payload = json!({
        "model": "claude-sonnet-4",
        "input": [
            {
                "type": "message",
                "role": "assistant",
                "id": "msg_history_1",
                "cachePoint": { "type": "default" },
                "content": [
                    { "type": "output_text", "text": "历史回答" },
                    { "type": "reasoning", "summary": "内部推理" }
                ]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{ "type": "input_text", "text": "继续" }]
            }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses payload should convert");

    assert_eq!(converted.messages.len(), 2);
    assert_eq!(converted.messages[0].role, "assistant");
    assert_eq!(
        converted.messages[0].metadata,
        Some(json!({
            "messageId": "msg_history_1",
            "cachePoint": { "type": "default" },
            "reasoningContent": {
                "reasoningText": {
                    "text": "内部推理"
                }
            }
        }))
    );
}

#[test]
fn normalize_openai_responses_request_preserves_compaction_items() {
    // 测试：Responses API 的 compaction item 应该被保留
    let payload = json!({
        "model": "gpt-5",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": "Hello"
            },
            {
                "type": "message",
                "role": "assistant",
                "content": "Hi there!"
            },
            {
                "type": "compaction",
                "data": "encrypted_compaction_data_here"
            },
            {
                "type": "message",
                "role": "user",
                "content": "Continue"
            }
        ]
    });

    let normalized =
        normalize_openai_responses_request(&payload).expect("should normalize successfully");

    // 验证消息数量：user + assistant + compaction + user = 4
    assert_eq!(normalized.messages.len(), 4, "should have 4 messages");

    // 验证 compaction item 被保留为 system 消息
    assert_eq!(
        normalized.messages[2].role, "system",
        "compaction should be system role"
    );
    assert!(
        normalized.messages[2]
            .metadata
            .as_ref()
            .and_then(|m| m.get("is_compaction"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        "compaction should have is_compaction metadata"
    );

    // 验证 compaction 内容被原样保留
    let compaction_content = normalized.messages[2].content.as_ref().unwrap();
    assert_eq!(
        compaction_content.get("type").and_then(|v| v.as_str()),
        Some("compaction"),
        "compaction type should be preserved"
    );
    assert_eq!(
        compaction_content.get("data").and_then(|v| v.as_str()),
        Some("encrypted_compaction_data_here"),
        "compaction data should be preserved"
    );
}

/// R2：入站 reasoning item 的 `encrypted_content` 是官方 C6i JSON 信封，
/// 按官方 `efd` 语义解包还原 Kiro reasoningContent（含签名回写）。
#[test]
fn normalize_openai_responses_reasoning_item_decodes_official_envelope() {
    let text_envelope = json!({
        "reasoningText": { "text": "思考", "signature": "sig" }
    })
    .to_string();
    let redacted_envelope = json!({ "redactedContent": "QUJDREVG" }).to_string();

    let payload = json!({
        "model": "claude-sonnet-4",
        "input": [
            {
                "type": "reasoning",
                "summary": [],
                "encrypted_content": text_envelope
            },
            {
                "type": "reasoning",
                "summary": [],
                "encrypted_content": redacted_envelope
            }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses should convert");
    assert_eq!(converted.messages[0].role, "assistant");
    assert_eq!(
        converted.messages[0].metadata,
        Some(json!({
            "reasoningContent": { "reasoningText": { "text": "思考", "signature": "sig" } }
        }))
    );
    assert_eq!(
        converted.messages[1].metadata,
        Some(json!({
            "reasoningContent": { "redactedContent": "QUJDREVG" }
        }))
    );
}

/// 无信封时退回文档字段：content[].reasoning_text 优先，其次 summary[].summary_text。
#[test]
fn normalize_openai_responses_reasoning_item_falls_back_to_summary_text() {
    let payload = json!({
        "model": "claude-sonnet-4",
        "input": [
            {
                "type": "reasoning",
                "summary": [{ "type": "summary_text", "text": "摘要" }]
            }
        ]
    });

    let converted =
        normalize_openai_responses_request(&payload).expect("responses should convert");
    assert_eq!(
        converted.messages[0].metadata,
        Some(json!({
            "reasoningContent": { "reasoningText": { "text": "摘要" } }
        }))
    );
}
