//! OpenAI Chat Completions 协议：请求归一化与消息 / 工具转换。

use super::*;

pub fn normalize_openai_chat_payload(payload: &Value) -> Result<NormalizedRequest, String> {
    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("claude-sonnet-4-5-20250929")
        .to_string();

    let messages = convert_openai_chat_messages(payload.get("messages"));
    if messages.is_empty() {
        return Err("chat.completions 请求缺少可转换的 messages".to_string());
    }

    let (tools, tool_name_map) = convert_openai_chat_tools(payload.get("tools"));
    Ok(build_normalized_request_from_payload(
        payload,
        model,
        messages,
        tools,
        tool_name_map,
    ))
}

pub fn normalize_openai_chat_request(request: &OpenAIChatRequest) -> Result<NormalizedRequest, String> {
    let mut messages = Vec::new();
    let mut pending_tool_results = Vec::new();

    for msg in &request.messages {
        match msg.role.as_str() {
            "system" => {
                let text = extract_text_content(msg.content.as_ref());
                if !text.is_empty() {
                    messages.push(NormalizedMessage {
                        role: "system".to_string(),
                        content: Some(Value::String(text)),
                        tool_calls: None,
                        tool_call_id: None,
                        metadata: None,
                    });
                }
            }
            "tool" => {
                let content = extract_text_content(msg.content.as_ref());
                let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
                pending_tool_results.push((tool_call_id, content));
            }
            "user" | "assistant" => {
                if !pending_tool_results.is_empty() {
                    messages.push(create_tool_results_message(&pending_tool_results));
                    pending_tool_results.clear();
                }

                let tool_calls = if msg.role == "assistant" {
                    msg.tool_calls.as_ref().map(|tcs| {
                        tcs.iter()
                            .map(|tc| ToolCall {
                                id: tc.id.clone(),
                                call_type: tc.call_type.clone(),
                                function: ToolCallFunction {
                                    name: tc.function.name.clone(),
                                    arguments: tc.function.arguments.to_string(),
                                },
                            })
                            .collect()
                    })
                } else {
                    None
                };

                messages.push(NormalizedMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    tool_calls,
                    tool_call_id: None,
                    metadata: None,
                });
            }
            // OpenAI 文档：developer 指令级消息 → 映射为 system
            "developer" => {
                let text = extract_text_content(msg.content.as_ref());
                if !text.is_empty() {
                    messages.push(NormalizedMessage {
                        role: "system".to_string(),
                        content: Some(Value::String(text)),
                        tool_calls: None,
                        tool_call_id: None,
                        metadata: None,
                    });
                }
            }
            // 旧版 OpenAI function 角色：按 tool 结果处理
            "function" => {
                let content = extract_text_content(msg.content.as_ref());
                let tool_call_id = msg
                    .tool_call_id
                    .clone()
                    .or_else(|| msg.name.clone())
                    .unwrap_or_default();
                pending_tool_results.push((tool_call_id, content));
            }
            other => {
                // 官方 Chat Completions 角色仅限 system/user/assistant/tool/developer（+遗留 function）
                return Err(format!(
                    "不支持的 chat message.role: \"{other}\"，官方取值: system|user|assistant|tool|developer"
                ));
            }
        }
    }

    if !pending_tool_results.is_empty() {
        messages.push(create_tool_results_message(&pending_tool_results));
    }

    let mut tool_name_map = std::collections::HashMap::new();
    let tools = request.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|t| {
                let original_name = t.function.name.clone();
                let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));

                if sanitized_name != original_name {
                    tool_name_map.insert(sanitized_name.clone(), original_name);
                }

                Tool {
                    tool_type: t.tool_type.clone(),
                    function: ToolFunction {
                        name: sanitized_name,
                        description: t.function.description.clone(),
                        parameters: t.function.parameters.clone(),
                    },
                    cache_control: None,
                }
            })
            .collect()
    });

    let include_usage = request
        .stream_options
        .as_ref()
        .map(|opts| opts.include_usage)
        .unwrap_or(false);

    // 处理 response_format：注入 system prompt 让模型按格式返回
    if let Some(response_format) = &request.response_format {
        let format_type = response_format.get("type").and_then(Value::as_str);
        match format_type {
            Some("json_object") => {
                let instr = "You must respond with a valid JSON object. Do not include any explanatory text outside the JSON.";
                if let Some(first) = messages.first_mut() {
                    if first.role == "system" {
                        if let Some(Value::String(text)) = &mut first.content {
                            text.push_str("\n\n");
                            text.push_str(instr);
                        }
                    } else {
                        messages.insert(0, NormalizedMessage {
                            role: "system".to_string(),
                            content: Some(Value::String(instr.to_string())),
                            tool_calls: None,
                            tool_call_id: None,
                            metadata: None,
                        });
                    }
                }
            }
            Some("json_schema") => {
                let name = response_format
                    .get("json_schema")
                    .and_then(|s| s.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("output");
                let instr = format!("You must respond with a valid JSON object conforming to the \"{}\" schema. Do not include any explanatory text outside the JSON.", name);
                if let Some(first) = messages.first_mut() {
                    if first.role == "system" {
                        if let Some(Value::String(text)) = &mut first.content {
                            text.push_str("\n\n");
                            text.push_str(&instr);
                        }
                    } else {
                        messages.insert(0, NormalizedMessage {
                            role: "system".to_string(),
                            content: Some(Value::String(instr)),
                            tool_calls: None,
                            tool_call_id: None,
                            metadata: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // 处理 parallel_tool_calls=false
    if request.parallel_tool_calls == Some(false) {
        let instr = "Call only one tool at a time. Wait for the result before calling the next tool.";
        if let Some(first) = messages.first_mut() {
            if first.role == "system" {
                if let Some(Value::String(text)) = &mut first.content {
                    text.push_str("\n\n");
                    text.push_str(instr);
                }
            } else {
                messages.insert(0, NormalizedMessage {
                    role: "system".to_string(),
                    content: Some(Value::String(instr.to_string())),
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: None,
                });
            }
        }
    }

    // 处理 n > 1：Kiro 只支持返回 1 个，打日志提示
    if let Some(n) = request.n {
        if n > 1 {
            log::warn!("[OpenAI Chat] 请求 n={}，但 Kiro 上游仅支持返回 1 个 choice", n);
        }
    }

    Ok(NormalizedRequest {
        model: request.model.clone(),
        messages,
        stream: request.stream,
        max_tokens: request.max_tokens.or(request.max_completion_tokens),
        temperature: request.temperature,
        top_p: request.top_p,
        stop: request.stop.clone(),
        tools,
        tool_choice: request.tool_choice.clone(),
        previous_response_id: None,
        thinking: None,
        include_usage,
        tool_name_map,
        // OpenAI Chat 协议无 Anthropic 式服务端工具
        server_tool_names: Vec::new(),
    })
}

pub fn convert_openai_chat_messages(messages: Option<&Value>) -> Vec<NormalizedMessage> {
    let Some(Value::Array(items)) = messages else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            // 官方 Chat Completions roles: system | user | assistant | tool | developer
            // 另兼容遗留 function（按 tool 结果处理）
            let raw_role = item.get("role").and_then(Value::as_str)?;
            let (role, tool_call_id) = match raw_role {
                "system" | "user" | "assistant" | "tool" => {
                    let tool_call_id = item
                        .get("tool_call_id")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    (raw_role.to_string(), tool_call_id)
                }
                // OpenAI developer 指令级消息 → 内部 system
                "developer" => ("system".to_string(), None),
                // 旧版 function 角色 → tool，name 作 tool_call_id 回退
                "function" => {
                    let tool_call_id = item
                        .get("tool_call_id")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("name").and_then(Value::as_str))
                        .map(str::to_string);
                    ("tool".to_string(), tool_call_id)
                }
                other => {
                    log::warn!(
                        "[协议映射] 不支持的 chat message.role=\"{other}\"，已跳过。官方取值: system|user|assistant|tool|developer"
                    );
                    return None;
                }
            };

            let tool_calls = if role == "assistant" {
                item.get("tool_calls")
                    .and_then(Value::as_array)
                    .map(|calls| {
                        calls
                            .iter()
                            .filter_map(|call| {
                                Some(ToolCall {
                                    id: call.get("id").and_then(Value::as_str)?.to_string(),
                                    call_type: call
                                        .get("type")
                                        .and_then(Value::as_str)
                                        .unwrap_or("function")
                                        .to_string(),
                                    function: ToolCallFunction {
                                        name: call
                                            .get("function")?
                                            .get("name")
                                            .and_then(Value::as_str)?
                                            .to_string(),
                                        arguments: call
                                            .get("function")?
                                            .get("arguments")
                                            .and_then(Value::as_str)
                                            .unwrap_or("{}")
                                            .to_string(),
                                    },
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|calls| !calls.is_empty())
            } else {
                None
            };

            let content = item.get("content").map(convert_openai_chat_content);
            Some(NormalizedMessage {
                role,
                content,
                tool_calls,
                tool_call_id,
                metadata: None,
            })
        })
        .collect()
}

pub fn convert_openai_chat_content(content: &Value) -> Value {
    match content {
        Value::String(text) => Value::String(text.clone()),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| {
                    if item.get("type").and_then(Value::as_str) == Some("text") {
                        json!({
                            "type": "input_text",
                            "text": item.get("text").and_then(Value::as_str).unwrap_or_default()
                        })
                    } else {
                        item.clone()
                    }
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

pub fn convert_openai_chat_tools(
    tools: Option<&Value>,
) -> (Option<Vec<Tool>>, std::collections::HashMap<String, String>) {
    convert_responses_tools(tools)
}
