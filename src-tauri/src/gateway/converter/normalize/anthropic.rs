//! Anthropic Messages 协议：请求归一化与内容块解析。

use super::*;

pub fn normalize_anthropic_request(request: &AnthropicMessagesRequest) -> NormalizedRequest {
    let mut messages = Vec::new();

    // 处理 system prompt，提取 cache_control
    if let Some(system) = &request.system {
        let (system_text, system_cache_point) = extract_text_and_cache_control(system);
        if !system_text.is_empty() {
            let mut metadata = None;
            if let Some(cache_point) = system_cache_point {
                metadata = Some(json!({"cache_point": cache_point}));
            }
            messages.push(NormalizedMessage {
                role: "system".to_string(),
                content: Some(Value::String(system_text)),
                tool_calls: None,
                tool_call_id: None,
                metadata,
            });
        }
    }

    // 处理消息，提取每条消息中的 cache_control
    for message in &request.messages {
        let cache_point = extract_cache_control_from_content(&message.content);
        let mut metadata = extract_anthropic_message_metadata(message);

        // 如果消息内容中有 cache_control，添加到 metadata
        if let Some(cp) = cache_point {
            let mut meta_obj = metadata.unwrap_or_else(|| json!({}));
            if let Some(obj) = meta_obj.as_object_mut() {
                obj.insert("cache_point".to_string(), cp);
            }
            metadata = Some(meta_obj);
        }

        messages.push(NormalizedMessage {
            role: message.role.clone(),
            content: Some(convert_anthropic_content(&message.content)),
            tool_calls: extract_anthropic_tool_calls(&message.content),
            tool_call_id: extract_anthropic_tool_result_id(&message.content),
            metadata,
        });
    }

    let mut tool_name_map = std::collections::HashMap::new();
    // Anthropic 服务端工具（type 非 "custom" 的具名类型，如 web_search_20250305）：
    // 记录 sanitized 名单，payload 构建时剥离（Kiro 上游无服务端执行环境）。
    // 保留在 request.tools 中是为了 tool_choice 校验与 WebSearch 子请求检测。
    let mut server_tool_names = Vec::new();
    let tools = request.tools.as_ref().map(|tools| {
        tools
            .iter()
            .filter_map(|tool| {
                let is_server_tool = tool
                    .r#type
                    .as_deref()
                    .map(|t| !t.is_empty() && t != "custom")
                    .unwrap_or(false);
                if is_server_tool {
                    server_tool_names.push(crate::gateway::converter::sanitize_tool_name(
                        &tool.name,
                    ));
                    return None;
                }
                let (converted_tool, mapping) = convert_anthropic_tool(tool);
                if let Some((sanitized, original)) = mapping {
                    tool_name_map.insert(sanitized, original);
                }
                Some(converted_tool)
            })
            .collect()
    });

    let mut normalized = NormalizedRequest {
        model: request.model.clone(),
        messages,
        stream: request.stream,
        max_tokens: Some(request.max_tokens),
        temperature: request.temperature,
        top_p: request.top_p,
        stop: request.stop_sequences.clone(),
        tools,
        tool_choice: request.tool_choice.clone(),
        previous_response_id: None,
        thinking: request.thinking.clone(),
        include_usage: false,
        tool_name_map,
        server_tool_names,
    };

    // 检测模型名是否包含 "thinking" 后缀，若包含则自动启用 thinking
    override_thinking_from_model_name(&mut normalized);

    normalized
}

pub fn convert_anthropic_content(content: &Value) -> Value {
    match content {
        Value::String(text) => Value::String(text.clone()),
        Value::Array(items) => {
            // 检查是否包含 tool_result
            let has_tool_result = items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some("tool_result"));
            if has_tool_result {
                return content.clone();
            }

            // 检查是否包含图片（必须保留原始数组，extract_images 需要从中提取）
            let has_image = items.iter().any(|item| {
                let t = item.get("type").and_then(Value::as_str).unwrap_or_default();
                t == "image" || t == "image_url" || t == "input_image"
            });
            if has_image {
                return content.clone();
            }

            // 只有纯文本内容才转换为字符串
            let text = extract_text_blocks(content, &["text"]);
            if text.is_empty() {
                content.clone()
            } else {
                Value::String(text)
            }
        }
        other => other.clone(),
    }
}

pub fn extract_anthropic_message_metadata(
    message: &crate::gateway::models::AnthropicMessage,
) -> Option<Value> {
    if message.role != "assistant" {
        return None;
    }

    let mut metadata = Map::new();
    if let Some(reasoning) = extract_reasoning_content(Some(&message.content)) {
        metadata.insert("reasoningContent".to_string(), reasoning);
    }

    if metadata.is_empty() {
        None
    } else {
        Some(Value::Object(metadata))
    }
}

pub fn extract_anthropic_tool_calls(content: &Value) -> Option<Vec<ToolCall>> {
    let Value::Array(items) = content else {
        return None;
    };

    let tool_calls: Vec<ToolCall> = items
        .iter()
        .filter_map(|item| {
            let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
            if item_type != "tool_use" {
                return None;
            }

            Some(ToolCall {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                call_type: "function".to_string(),
                function: ToolCallFunction {
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    arguments: serde_json::to_string(
                        &item.get("input").cloned().unwrap_or_else(|| json!({})),
                    )
                    .unwrap_or_else(|_| "{}".to_string()),
                },
            })
        })
        .collect();

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

pub fn extract_anthropic_tool_result_id(content: &Value) -> Option<String> {
    let Value::Array(items) = content else {
        return None;
    };

    items.iter().find_map(|item| {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
        if item_type == "tool_result" {
            item.get("tool_use_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        } else {
            None
        }
    })
}
