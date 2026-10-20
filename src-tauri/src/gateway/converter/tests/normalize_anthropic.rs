//! Anthropic Messages 协议请求归一化的回归测试。

use super::*;

#[test]
fn normalize_anthropic_request_keeps_system_tools_and_tool_result() {
    let request = AnthropicMessagesRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![crate::gateway::models::AnthropicMessage {
            role: "user".to_string(),
            content: json!([
                {
                    "type": "tool_result",
                    "tool_use_id": "tool_1",
                    "content": "42",
                    "is_error": false
                },
                {
                    "type": "text",
                    "text": "继续"
                }
            ]),
        }],
        max_tokens: 4096,
        system: Some(json!([{ "type": "text", "text": "你是测试助手" }])),
        stream: false,
        temperature: Some(0.2),
        top_p: Some(0.8),
        stop_sequences: Some(vec!["STOP".to_string()]),
        tools: Some(vec![AnthropicTool {
            r#type: Some("custom".to_string()),
            name: "math".to_string(),
            description: Some("计算器".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": { "expr": { "type": "string" } },
                "required": ["expr"]
            }),
            cache_control: None,
        }]),
        tool_choice: Some(json!({"type":"auto"})),
        thinking: None,
        metadata: None,
        top_k: None,
    };

    let converted = normalize_anthropic_request(&request);
    assert_eq!(converted.messages.len(), 2);
    assert_eq!(converted.messages[0].role, "system");
    assert_eq!(converted.tools.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        converted.messages[1].tool_call_id.as_deref(),
        Some("tool_1")
    );
}

#[test]
fn normalize_anthropic_request_preserves_image_content() {
    // 测试：包含图片的 content 应该保留为数组，不应该被转换为字符串
    let request = AnthropicMessagesRequest {
        model: "claude-sonnet-4-5".to_string(),
        messages: vec![crate::gateway::models::AnthropicMessage {
            role: "user".to_string(),
            content: json!([
                {
                    "type": "text",
                    "text": "这是什么图片？"
                },
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
                    }
                }
            ]),
        }],
        max_tokens: 1024,
        system: None,
        stream: false,
        temperature: None,
        top_p: None,
        top_k: None,
        stop_sequences: None,
        tools: None,
        tool_choice: None,
        thinking: None,
        metadata: None,
    };

    let converted = normalize_anthropic_request(&request);

    // 验证 content 仍然是数组（而不是被转换成字符串）
    assert_eq!(converted.messages.len(), 1);
    let content = converted.messages[0]
        .content
        .as_ref()
        .expect("content should exist");

    // 关键断言：content 应该是 Array，不是 String
    assert!(
        content.is_array(),
        "content should be an array to preserve image data"
    );

    let content_array = content.as_array().expect("content should be array");
    assert_eq!(
        content_array.len(),
        2,
        "should have 2 items: text and image"
    );

    // 验证图片 block 仍然存在
    let image_block = &content_array[1];
    assert_eq!(
        image_block.get("type").and_then(Value::as_str),
        Some("image"),
        "image block should be preserved"
    );
    assert!(
        image_block.get("source").is_some(),
        "image source should be preserved"
    );
}

fn base_thinking_request(
    model: &str,
    thinking: Option<crate::gateway::models::Thinking>,
) -> AnthropicMessagesRequest {
    AnthropicMessagesRequest {
        model: model.to_string(),
        messages: vec![crate::gateway::models::AnthropicMessage {
            role: "user".to_string(),
            content: json!([{ "type": "text", "text": "hi" }]),
        }],
        max_tokens: 1024,
        system: None,
        stream: false,
        temperature: None,
        top_p: None,
        top_k: None,
        stop_sequences: None,
        tools: None,
        tool_choice: None,
        thinking,
        metadata: None,
    }
}

// 4.7+ 模型官方仅接受 adaptive thinking（docs.anthropic.com extended-thinking），
// 模型名含 -thinking 且客户端未显式传 thinking 时必须判为 adaptive。
#[test]
fn override_thinking_uses_adaptive_for_47_plus_models() {
    for model in [
        "claude-sonnet-5.5-thinking",
        "claude-opus-5.5-thinking",
        "claude-opus-5-thinking",
        "claude-fable-5-thinking",
        "claude-opus-4-7-thinking",
        "claude-sonnet-4.6-thinking",
    ] {
        let converted = normalize_anthropic_request(&base_thinking_request(model, None));
        let thinking = converted
            .thinking
            .as_ref()
            .unwrap_or_else(|| panic!("{model} should have thinking injected"));
        assert_eq!(thinking.thinking_type, "adaptive", "{model} should be adaptive");
    }
    // 4.5 代官方只有 extended 模式，保持 enabled
    let converted =
        normalize_anthropic_request(&base_thinking_request("claude-sonnet-4.5-thinking", None));
    assert_eq!(
        converted.thinking.as_ref().expect("4.5 should think").thinking_type,
        "enabled"
    );
}

// 客户端显式传了 thinking 配置（如 Claude Code 的 budget_tokens）时必须尊重：
// 后缀只作为无参客户端的兜底开关，不覆写显式配置。
#[test]
fn override_thinking_respects_explicit_client_config() {
    let explicit = crate::gateway::models::Thinking {
        thinking_type: "enabled".to_string(),
        budget_tokens: 12345,
    };
    let converted = normalize_anthropic_request(&base_thinking_request(
        "claude-sonnet-4.5-thinking",
        Some(explicit),
    ));
    let thinking = converted.thinking.as_ref().expect("explicit config kept");
    assert_eq!(thinking.budget_tokens, 12345);
}
