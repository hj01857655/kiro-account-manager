use crate::gateway::models::{
    AnthropicMessagesRequest, ConversationState, CurrentMessage, HistoryAssistantMessage,
    HistoryItem, HistoryUserMessage, ImageBlock, ImageSource, KiroInputSchema,
    KiroPayload, KiroTool, KiroToolResult, KiroToolResultContent, KiroToolSpec, KiroToolUse,
    ModelInfo, NormalizedMessage, NormalizedRequest, OpenAIChatRequest,
    Tool, ToolCall, ToolCallFunction, ToolFunction, UserInputMessage,
    UserInputMessageContext, WebSearchToolOptions,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::Client;
use serde_json::{json, Map, Value};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};
use tokio::net::lookup_host;
use uuid::Uuid;

pub const TOOL_DESCRIPTION_MAX_LENGTH: usize = 1024;
const WEB_SEARCH_TOOL_NAME: &str = "web_search";
const WEB_SEARCH_TOOL_DESCRIPTION: &str =
    "Search the web for current information and return relevant results.";
const MAX_IMAGE_SOURCE_BYTES: usize = 5 * 1024 * 1024;
const MAX_IMAGE_REDIRECTS: usize = 3;
const IMAGE_FETCH_TIMEOUT_SECONDS: u64 = 15;

pub fn normalize_anthropic_request(request: &AnthropicMessagesRequest) -> NormalizedRequest {
    let mut messages = Vec::new();

    if let Some(system) = &request.system {
        let system_text = extract_text_blocks(system, &["text"]);
        if !system_text.is_empty() {
            messages.push(NormalizedMessage {
                role: "system".to_string(),
                content: Some(Value::String(system_text)),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            });
        }
    }

    for message in &request.messages {
        messages.push(NormalizedMessage {
            role: message.role.clone(),
            content: Some(convert_anthropic_content(&message.content)),
            tool_calls: extract_anthropic_tool_calls(&message.content),
            tool_call_id: extract_anthropic_tool_result_id(&message.content),
            metadata: extract_anthropic_message_metadata(message),
        });
    }

    let tools = request
        .tools
        .as_ref()
        .map(|tools| tools.iter().map(convert_anthropic_tool).collect());

    NormalizedRequest {
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
    }
}

pub fn normalize_responses_request(payload: &Value) -> Result<NormalizedRequest, String> {
    if payload.get("messages").is_some() && payload.get("input").is_none() {
        return normalize_openai_chat_payload(payload);
    }

    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("claude-sonnet-4-5-20250929")
        .to_string();

    let mut messages = Vec::new();

    if let Some(instructions) = payload.get("instructions") {
        let text = extract_text_blocks(instructions, &["text", "input_text", "output_text"]);
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

    if let Some(input) = payload.get("input") {
        messages.extend(convert_responses_input(input));
    }

    if messages.is_empty() {
        return Err("Responses 请求缺少可转换的 input".to_string());
    }

    Ok(build_normalized_request_from_payload(
        payload,
        model,
        messages,
        convert_responses_tools(payload.get("tools")),
    ))
}

fn normalize_openai_chat_payload(payload: &Value) -> Result<NormalizedRequest, String> {
    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("claude-sonnet-4-5-20250929")
        .to_string();

    let messages = convert_openai_chat_messages(payload.get("messages"));
    if messages.is_empty() {
        return Err("chat.completions 请求缺少可转换的 messages".to_string());
    }

    Ok(build_normalized_request_from_payload(
        payload,
        model,
        messages,
        convert_openai_chat_tools(payload.get("tools")),
    ))
}

pub fn normalize_openai_chat_request(request: &OpenAIChatRequest) -> NormalizedRequest {
    let mut messages = Vec::new();
    let mut pending_tool_results = Vec::new();

    for msg in &request.messages {
        match msg.role.as_str() {
            "system" => {
                let text = extract_text_content(Some(&msg.content));
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
                let content = extract_text_content(Some(&msg.content));
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
                    content: Some(msg.content.clone()),
                    tool_calls,
                    tool_call_id: None,
                    metadata: None,
                });
            }
            _ => {}
        }
    }

    if !pending_tool_results.is_empty() {
        messages.push(create_tool_results_message(&pending_tool_results));
    }

    let tools = request.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|t| Tool {
                tool_type: t.tool_type.clone(),
                function: t.function.clone(),
                web_search: None,
            })
            .collect()
    });

    NormalizedRequest {
        model: request.model.clone(),
        messages,
        stream: request.stream,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        top_p: request.top_p,
        stop: request.stop.clone(),
        tools,
        tool_choice: request.tool_choice.clone(),
        previous_response_id: None,
    }
}

fn create_tool_results_message(tool_results: &[(String, String)]) -> NormalizedMessage {
    let mut content_array = Vec::new();
    for (tool_call_id, content) in tool_results {
        content_array.push(json!({
            "type": "tool_result",
            "tool_use_id": tool_call_id,
            "content": content
        }));
    }

    NormalizedMessage {
        role: "user".to_string(),
        content: Some(Value::Array(content_array)),
        tool_calls: None,
        tool_call_id: None,
        metadata: None,
    }
}

fn build_normalized_request_from_payload(
    payload: &Value,
    model: String,
    messages: Vec<NormalizedMessage>,
    tools: Option<Vec<Tool>>,
) -> NormalizedRequest {
    NormalizedRequest {
        model,
        messages,
        stream: payload
            .get("stream")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        max_tokens: payload
            .get("max_output_tokens")
            .or_else(|| payload.get("max_tokens"))
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        temperature: payload
            .get("temperature")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        top_p: payload
            .get("top_p")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        stop: payload.get("stop").and_then(|value| match value {
            Value::String(item) => Some(vec![item.to_string()]),
            Value::Array(items) => Some(
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            ),
            _ => None,
        }),
        tools,
        tool_choice: payload.get("tool_choice").cloned(),
        previous_response_id: payload
            .get("previous_response_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    }
}

fn convert_openai_chat_messages(messages: Option<&Value>) -> Vec<NormalizedMessage> {
    let Some(Value::Array(items)) = messages else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let role = item.get("role").and_then(Value::as_str)?.to_string();
            let tool_calls = item
                .get("tool_calls")
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
                .filter(|calls| !calls.is_empty());

            let content = item.get("content").map(convert_openai_chat_content);
            Some(NormalizedMessage {
                role,
                content,
                tool_calls,
                tool_call_id: item
                    .get("tool_call_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                metadata: None,
            })
        })
        .collect()
}

fn convert_openai_chat_content(content: &Value) -> Value {
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

fn convert_openai_chat_tools(tools: Option<&Value>) -> Option<Vec<Tool>> {
    convert_responses_tools(tools)
}

fn convert_anthropic_tool(tool: &crate::gateway::models::AnthropicTool) -> Tool {
    let tool_type = tool.r#type.as_deref().unwrap_or("function");
    if is_web_search_tool_type(tool_type) {
        return convert_web_search_tool(
            tool_type,
            &tool.name,
            tool.description.clone(),
            tool.max_uses,
            tool.allowed_domains.clone(),
            tool.blocked_domains.clone(),
            tool.user_location.clone(),
        );
    }

    Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: Some(normalize_json_schema(tool.input_schema.clone())),
        },
        web_search: None,
    }
}

fn convert_web_search_tool(
    tool_type: &str,
    _name: &str,
    description: Option<String>,
    max_uses: Option<i32>,
    allowed_domains: Option<Vec<String>>,
    blocked_domains: Option<Vec<String>>,
    user_location: Option<Value>,
) -> Tool {
    Tool {
        tool_type: tool_type.to_string(),
        function: ToolFunction {
            name: WEB_SEARCH_TOOL_NAME.to_string(),
            description: Some(
                description.unwrap_or_else(|| WEB_SEARCH_TOOL_DESCRIPTION.to_string()),
            ),
            parameters: Some(web_search_input_schema()),
        },
        web_search: Some(WebSearchToolOptions {
            max_uses,
            allowed_domains,
            blocked_domains,
            user_location,
        }),
    }
}

fn is_web_search_tool_type(tool_type: &str) -> bool {
    tool_type.starts_with("web_search_") || tool_type == "remote_web_search"
}

fn web_search_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query"
            }
        },
        "required": ["query"]
    })
}

fn string_array_from_values(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

pub fn get_internal_model_id(external_model: &str) -> Result<String, String> {
    let normalized_model = normalize_external_model_alias(external_model);
    let model_id = match normalized_model.as_str() {
        "claude-opus-4-6-20260205" => "claude-opus-4.6",
        "claude-opus-4-6" | "claude-opus-4.6" | "opus-4-6" => "claude-opus-4.6",
        "claude-opus-4-5" | "claude-opus-4-5-20251101" | "claude-opus-4.5" | "opus" => {
            "claude-opus-4.5"
        }
        "claude-haiku-4-5" | "claude-haiku-4-5-20251001" | "claude-haiku-4.5" | "haiku" => {
            "claude-haiku-4.5"
        }
        "claude-sonnet-4-6-20260217" => "claude-sonnet-4.6",
        "claude-sonnet-4-6" | "claude-sonnet-4.6" | "sonnet-4-6" => "claude-sonnet-4.6",
        "claude-sonnet-4-5"
        | "claude-sonnet-4-5-20250929"
        | "claude-sonnet-4.5"
        | "claude-sonnet-latest"
        | "sonnet" => "claude-sonnet-4.5",
        "claude-sonnet-4" | "claude-sonnet-4-20250514" => "claude-sonnet-4",
        "claude-3-7-sonnet-20250219" | "claude-3.7-sonnet" => "claude-3-7-sonnet-20250219",
        "claude-3-5-sonnet-20241022" | "claude-3-5-sonnet-latest" | "claude-3.5-sonnet" => {
            "claude-3-5-sonnet-20241022"
        }
        "auto" | "default" => "auto",
        other if other.starts_with("claude-opus-4-6-") => "claude-opus-4.6",
        other if other.starts_with("claude-sonnet-4-6-") => "claude-sonnet-4.6",
        other => other,
    };

    Ok(model_id.to_string())
}

fn normalize_external_model_alias(external_model: &str) -> String {
    external_model.trim().to_ascii_lowercase()
}

pub async fn build_kiro_payload(
    client: &Client,
    request: &NormalizedRequest,
    profile_arn: Option<String>,
) -> Result<KiroPayload, String> {
    let model_id = get_internal_model_id(&request.model)?;
    let conversation_id = request
        .previous_response_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let (processed_tools, tool_docs) = process_tools_with_long_descriptions(&request.tools);
    let tool_docs_for_current = tool_docs.clone();

    let mut system_prompt = String::new();
    let mut other_messages = Vec::new();

    for message in &request.messages {
        if message.role == "system" {
            let text = extract_text_content(message.content.as_ref());
            if !text.is_empty() {
                if !system_prompt.is_empty() {
                    system_prompt.push_str("\n\n");
                }
                system_prompt.push_str(&text);
            }
        } else {
            other_messages.push(message);
        }
    }

    if let Some(tool_docs) = tool_docs {
        if !system_prompt.is_empty() {
            system_prompt.push_str("\n\n");
        }
        system_prompt.push_str(&tool_docs);
    }

    if other_messages.is_empty() {
        return Err("没有可发送的消息".to_string());
    }

    let merged_messages = merge_adjacent_messages(&other_messages);
    let first_user_index = merged_messages
        .iter()
        .position(|message| matches!(message.role.as_str(), "user" | "tool"));

    let history = if merged_messages.len() > 1 {
        let mut history_items = Vec::new();

        for (index, message) in merged_messages[..merged_messages.len() - 1]
            .iter()
            .enumerate()
        {
            match message.role.as_str() {
                "assistant" => history_items.push(HistoryItem::Assistant {
                    assistant_response_message: build_history_assistant_message(message),
                }),
                "user" => {
                    let mut content = extract_text_content(message.content.as_ref());
                    if Some(index) == first_user_index && !system_prompt.is_empty() {
                        content = join_with_double_newline(&system_prompt, &content);
                    }
                    let images = extract_images(client, message.content.as_ref()).await;
                    history_items.push(HistoryItem::User {
                        user_input_message: HistoryUserMessage {
                            content,
                            model_id: model_id.clone(),
                            origin: "AI_EDITOR".to_string(),
                            images: images_option(images),
                            user_input_message_context: build_user_context(
                                None,
                                None,
                                extract_tool_results(message.content.as_ref()),
                            ),
                        },
                    });
                }
                "tool" => {
                    history_items.push(HistoryItem::User {
                        user_input_message: HistoryUserMessage {
                            content: if Some(index) == first_user_index && !system_prompt.is_empty()
                            {
                                system_prompt.clone()
                            } else {
                                String::new()
                            },
                            model_id: model_id.clone(),
                            origin: "AI_EDITOR".to_string(),
                            images: None,
                            user_input_message_context: build_user_context(
                                None,
                                None,
                                extract_tool_results_from_tool_message(message),
                            ),
                        },
                    });
                }
                _ => {}
            }
        }

        if history_items.is_empty() {
            None
        } else {
            Some(history_items)
        }
    } else {
        None
    };

    let current_message = merged_messages
        .last()
        .ok_or_else(|| "没有当前消息".to_string())?;
    let mut current_content = extract_text_content(current_message.content.as_ref());
    if history.is_none() && !system_prompt.is_empty() {
        current_content = join_with_double_newline(&system_prompt, &current_content);
    }
    if let Some(tool_docs) = tool_docs_for_current {
        current_content = join_with_double_newline(&tool_docs, &current_content);
    }
    if current_message.role == "assistant" || current_content.is_empty() {
        current_content = if current_content.is_empty() {
            "Continue".to_string()
        } else {
            current_content
        };
    }

    let current_tool_results = match current_message.role.as_str() {
        "tool" => extract_tool_results_from_tool_message(current_message),
        _ => extract_tool_results(current_message.content.as_ref()),
    };
    let current_images = extract_images(client, current_message.content.as_ref()).await;

    Ok(KiroPayload {
        conversation_state: ConversationState {
            chat_trigger_type: "MANUAL".to_string(),
            conversation_id: conversation_id.clone(),
            agent_continuation_id: None,
            agent_task_type: None,
            current_message: CurrentMessage {
                user_input_message: UserInputMessage {
                    content: current_content,
                    model_id,
                    origin: "AI_EDITOR".to_string(),
                    images: images_option(current_images),
                    user_input_message_context: build_user_context(
                        convert_tools(&processed_tools),
                        normalize_tool_choice(&request.tool_choice, &processed_tools)?,
                        current_tool_results,
                    ),
                },
            },
            history,
            customization_arn: None,
            workspace_id: None,
        },
        profile_arn,
    })
}

pub fn get_available_models() -> Vec<ModelInfo> {
    [
        "claude-opus-4-6",
        "claude-opus-4-6-20260205",
        "claude-opus-4-5",
        "claude-opus-4-5-20251101",
        "claude-haiku-4-5",
        "claude-haiku-4-5-20251001",
        "claude-sonnet-4-6",
        "claude-sonnet-4-6-20260217",
        "claude-sonnet-4-5",
        "claude-sonnet-4-5-20250929",
        "claude-sonnet-4",
        "claude-sonnet-4-20250514",
        "claude-3-7-sonnet-20250219",
    ]
    .into_iter()
    .map(|id| ModelInfo {
        id: id.to_string(),
        object: "model".to_string(),
        created: 1_700_000_000,
        owned_by: "anthropic".to_string(),
    })
    .collect()
}

fn convert_anthropic_content(content: &Value) -> Value {
    match content {
        Value::String(text) => Value::String(text.clone()),
        Value::Array(items) => {
            let has_tool_result = items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some("tool_result"));
            if has_tool_result {
                return content.clone();
            }

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

fn convert_responses_input(input: &Value) -> Vec<NormalizedMessage> {
    match input {
        Value::String(text) => vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(Value::String(text.clone())),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        Value::Array(items) => convert_responses_input_items(items),
        _ => Vec::new(),
    }
}

fn convert_responses_input_items(items: &[Value]) -> Vec<NormalizedMessage> {
    let mut messages = Vec::new();
    let mut pending_user_items = Vec::new();

    for item in items {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();

        if let Some(role) = item.get("role").and_then(Value::as_str) {
            flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
            messages.push(NormalizedMessage {
                role: role.to_string(),
                content: responses_message_content(item),
                tool_calls: None,
                tool_call_id: None,
                metadata: extract_responses_message_metadata(item, role),
            });
            continue;
        }

        match item_type {
            "message" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                let role = item
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("user")
                    .to_string();
                messages.push(NormalizedMessage {
                    role: role.clone(),
                    content: responses_message_content(item),
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: extract_responses_message_metadata(item, &role),
                });
            }
            "function_call" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                messages.push(NormalizedMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: Some(vec![ToolCall {
                        id: item
                            .get("call_id")
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
                            arguments: item
                                .get("arguments")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                                .unwrap_or_else(|| {
                                    serde_json::to_string(
                                        &item
                                            .get("arguments")
                                            .cloned()
                                            .unwrap_or_else(|| json!({})),
                                    )
                                    .unwrap_or_else(|_| "{}".to_string())
                                }),
                        },
                    }]),
                    tool_call_id: None,
                    metadata: None,
                });
            }
            "function_call_output" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                messages.push(NormalizedMessage {
                    role: "tool".to_string(),
                    content: responses_tool_output_content(item.get("output")),
                    tool_calls: None,
                    tool_call_id: item
                        .get("call_id")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    metadata: None,
                });
            }
            "input_text" | "output_text" | "input_image" | "image_url" | "image" => {
                pending_user_items.push(item.clone());
            }
            _ => {}
        }
    }

    flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
    messages
}

fn flush_pending_responses_user_items(
    messages: &mut Vec<NormalizedMessage>,
    pending_user_items: &mut Vec<Value>,
) {
    if pending_user_items.is_empty() {
        return;
    }

    messages.push(NormalizedMessage {
        role: "user".to_string(),
        content: Some(Value::Array(std::mem::take(pending_user_items))),
        tool_calls: None,
        tool_call_id: None,
        metadata: None,
    });
}

fn responses_message_content(item: &Value) -> Option<Value> {
    item.get("content")
        .cloned()
        .or_else(|| item.get("text").cloned())
}

fn responses_tool_output_content(output: Option<&Value>) -> Option<Value> {
    match output {
        None => None,
        Some(Value::String(text)) => Some(Value::String(text.clone())),
        Some(other) => Some(Value::String(other.to_string())),
    }
}

fn extract_responses_message_metadata(item: &Value, role: &str) -> Option<Value> {
    if role != "assistant" {
        return None;
    }

    let mut metadata = Map::new();
    for key in [
        "reasoningContent",
        "references",
        "supplementaryWebLinks",
        "followupPrompt",
        "cachePoint",
    ] {
        if let Some(value) = meaningful_optional_value(item.get(key).cloned()) {
            metadata.insert(key.to_string(), value);
        }
    }

    if let Some(message_id) = item
        .get("messageId")
        .or_else(|| item.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        metadata.insert(
            "messageId".to_string(),
            Value::String(message_id.to_string()),
        );
    }

    if !metadata.contains_key("reasoningContent") {
        if let Some(reasoning) = extract_reasoning_content(item.get("content")) {
            metadata.insert("reasoningContent".to_string(), reasoning);
        }
    }

    if metadata.is_empty() {
        None
    } else {
        Some(Value::Object(metadata))
    }
}

fn extract_anthropic_message_metadata(
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

fn convert_responses_tools(tools: Option<&Value>) -> Option<Vec<Tool>> {
    let items = tools?.as_array()?;
    let converted: Vec<Tool> = items.iter().filter_map(convert_responses_tool).collect();

    if converted.is_empty() {
        None
    } else {
        Some(converted)
    }
}

fn convert_responses_tool(item: &Value) -> Option<Tool> {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();

    if is_web_search_tool_type(item_type) {
        return Some(convert_web_search_tool(
            item_type,
            item.get("name")
                .and_then(Value::as_str)
                .unwrap_or(WEB_SEARCH_TOOL_NAME),
            item.get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            item.get("max_uses")
                .and_then(Value::as_i64)
                .map(|value| value as i32),
            item.get("allowed_domains")
                .and_then(Value::as_array)
                .map(|values| string_array_from_values(values)),
            item.get("blocked_domains")
                .and_then(Value::as_array)
                .map(|values| string_array_from_values(values)),
            item.get("user_location").cloned(),
        ));
    }

    if item.get("function").is_some() {
        let mut tool = serde_json::from_value::<Tool>(item.clone()).ok()?;
        tool.web_search = None;
        return Some(tool);
    }

    if item_type != "function" {
        return None;
    }

    Some(Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: item.get("name").and_then(Value::as_str)?.to_string(),
            description: item
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            parameters: item.get("parameters").cloned(),
        },
        web_search: None,
    })
}

fn extract_anthropic_tool_calls(content: &Value) -> Option<Vec<ToolCall>> {
    let Value::Array(items) = content else {
        return None;
    };

    let tool_calls: Vec<ToolCall> = items
        .iter()
        .filter_map(|item| {
            let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
            if item_type != "tool_use" && item_type != "server_tool_use" {
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

fn extract_anthropic_tool_result_id(content: &Value) -> Option<String> {
    let Value::Array(items) = content else {
        return None;
    };

    items.iter().find_map(|item| {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
        if item_type == "tool_result" || item_type == "web_search_tool_result" {
            item.get("tool_use_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        } else {
            None
        }
    })
}




fn merge_adjacent_messages(messages: &[&NormalizedMessage]) -> Vec<NormalizedMessage> {
    let mut merged: Vec<NormalizedMessage> = Vec::new();

    for message in messages {
        if let Some(last) = merged.last_mut() {
            if last.role == message.role && last.role != "tool" {
                let existing = extract_text_content(last.content.as_ref());
                let incoming = extract_text_content(message.content.as_ref());
                last.content = Some(Value::String(join_with_newline(&existing, &incoming)));

                match (&mut last.tool_calls, &message.tool_calls) {
                    (Some(existing_calls), Some(next_calls)) => {
                        existing_calls.extend(next_calls.clone())
                    }
                    (None, Some(next_calls)) => last.tool_calls = Some(next_calls.clone()),
                    _ => {}
                }
                if last.tool_call_id.is_none() {
                    last.tool_call_id = message.tool_call_id.clone();
                }
                continue;
            }
        }
        merged.push((*message).clone());
    }

    merged
}

fn build_user_context(
    tools: Option<Vec<KiroTool>>,
    tool_choice: Option<Value>,
    tool_results: Vec<KiroToolResult>,
) -> Option<UserInputMessageContext> {
    if tools.is_none() && tool_choice.is_none() && tool_results.is_empty() {
        return None;
    }

    Some(UserInputMessageContext {
        tools,
        tool_choice,
        tool_results: if tool_results.is_empty() {
            None
        } else {
            Some(tool_results)
        },
    })
}

fn images_option(images: Vec<ImageBlock>) -> Option<Vec<ImageBlock>> {
    if images.is_empty() {
        None
    } else {
        Some(images)
    }
}

fn extract_text_content(content: Option<&Value>) -> String {
    match content {
        None => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(value @ Value::Array(_)) => {
            extract_text_blocks(value, &["text", "input_text", "output_text"])
        }
        Some(other) => other.to_string(),
    }
}

#[allow(dead_code)]
pub fn normalized_user_message_from_text(text: &str) -> NormalizedMessage {
    NormalizedMessage {
        role: "user".to_string(),
        content: Some(Value::String(text.to_string())),
        tool_calls: None,
        tool_call_id: None,
        metadata: None,
    }
}

#[allow(dead_code)]
pub fn normalized_tool_message_from_output(tool_call_id: &str, output: &str) -> NormalizedMessage {
    NormalizedMessage {
        role: "tool".to_string(),
        content: Some(Value::String(output.to_string())),
        tool_calls: None,
        tool_call_id: Some(tool_call_id.to_string()),
        metadata: None,
    }
}

#[allow(dead_code)]
pub fn history_assistant_message_from_response_content(
    content: &str,
    tool_calls: &[(String, String, String)],
) -> HistoryAssistantMessage {
    let tool_uses = if tool_calls.is_empty() {
        None
    } else {
        Some(
            tool_calls
                .iter()
                .map(|(id, name, arguments)| KiroToolUse {
                    name: name.clone(),
                    input: serde_json::from_str(arguments).unwrap_or_else(|_| json!({})),
                    tool_use_id: id.clone(),
                })
                .collect(),
        )
    };

    HistoryAssistantMessage {
        content: if content.trim().is_empty() {
            "I understand.".to_string()
        } else {
            content.to_string()
        },
        tool_uses,
        reasoning_content: None,
        references: None,
        supplementary_web_links: None,
        followup_prompt: None,
        message_id: None,
        cache_point: None,
    }
}

fn build_history_assistant_message(message: &NormalizedMessage) -> HistoryAssistantMessage {
    HistoryAssistantMessage {
        content: extract_text_content(message.content.as_ref()),
        tool_uses: extract_tool_uses(message),
        reasoning_content: assistant_metadata_value(message, "reasoningContent")
            .or_else(|| extract_reasoning_content(message.content.as_ref()))
            .and_then(|value| meaningful_optional_value(Some(value))),
        references: assistant_metadata_value(message, "references")
            .and_then(|value| meaningful_optional_value(Some(value))),
        supplementary_web_links: assistant_metadata_value(message, "supplementaryWebLinks")
            .and_then(|value| meaningful_optional_value(Some(value))),
        followup_prompt: assistant_metadata_value(message, "followupPrompt")
            .and_then(|value| meaningful_optional_value(Some(value))),
        message_id: assistant_metadata_value(message, "messageId")
            .and_then(|value| value.as_str().map(str::to_string))
            .filter(|value| !value.trim().is_empty()),
        cache_point: assistant_metadata_value(message, "cachePoint")
            .and_then(|value| meaningful_optional_value(Some(value))),
    }
}

fn assistant_metadata_value(message: &NormalizedMessage, key: &str) -> Option<Value> {
    message
        .metadata
        .as_ref()
        .and_then(|value| value.get(key).cloned())
        .or_else(|| {
            message
                .content
                .as_ref()
                .and_then(|value| value.get(key).cloned())
        })
}

fn extract_reasoning_content(content: Option<&Value>) -> Option<Value> {
    let content = content?;

    if let Some(existing) = content.get("reasoningContent") {
        return meaningful_optional_value(Some(existing.clone()));
    }

    let content_items = content.get("content").unwrap_or(content);
    let Value::Array(items) = content_items else {
        return None;
    };

    let mut texts = Vec::new();
    let mut signature: Option<Value> = None;
    let mut redacted_content: Option<Value> = None;

    for item in items {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
        if item_type != "reasoning" && item_type != "thinking" {
            continue;
        }

        if let Some(text) = item
            .get("summary")
            .map(|value| extract_text_content(Some(value)))
        {
            if !text.is_empty() {
                texts.push(text);
            }
        } else if let Some(text) = item.get("thinking").and_then(Value::as_str) {
            if !text.is_empty() {
                texts.push(text.to_string());
            }
        } else if let Some(text) = item.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                texts.push(text.to_string());
            }
        }

        if signature.is_none() {
            signature = item.get("signature").cloned();
        }
        if redacted_content.is_none() {
            redacted_content = item.get("redactedContent").cloned();
        }
    }

    if texts.is_empty() && signature.is_none() && redacted_content.is_none() {
        return None;
    }

    let mut reasoning_text = Map::new();
    let merged_text = texts.join("\n");
    if !merged_text.is_empty() {
        reasoning_text.insert("text".to_string(), Value::String(merged_text));
    }
    if let Some(signature) = signature {
        reasoning_text.insert("signature".to_string(), signature);
    }

    let mut reasoning = Map::new();
    if !reasoning_text.is_empty() {
        reasoning.insert("reasoningText".to_string(), Value::Object(reasoning_text));
    }
    if let Some(redacted_content) = redacted_content {
        reasoning.insert("redactedContent".to_string(), redacted_content);
    }

    meaningful_optional_value(Some(Value::Object(reasoning)))
}

fn meaningful_optional_value(value: Option<Value>) -> Option<Value> {
    match value {
        Some(Value::Null) => None,
        Some(Value::String(text)) if text.trim().is_empty() => None,
        Some(Value::Array(items)) if items.is_empty() => None,
        Some(Value::Object(map)) if map.is_empty() => None,
        other => other,
    }
}

fn extract_text_blocks(value: &Value, text_types: &[&str]) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
                if text_types.contains(&item_type) {
                    item.get("text").and_then(Value::as_str).map(str::to_string)
                } else if item_type == "image" {
                    Some("[Image]".to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => map
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn extract_tool_results(content: Option<&Value>) -> Vec<KiroToolResult> {
    let Some(Value::Array(items)) = content else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
            if item_type != "tool_result" && item_type != "web_search_tool_result" {
                return None;
            }

            let tool_use_id = item
                .get("tool_use_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let content_text = match item.get("content") {
                Some(Value::String(text)) => text.clone(),
                Some(Value::Array(array)) if item_type == "web_search_tool_result" => {
                    Value::Array(array.clone()).to_string()
                }
                Some(Value::Array(array)) => {
                    extract_text_blocks(&Value::Array(array.clone()), &["text", "output_text"])
                }
                Some(other) => other.to_string(),
                None => String::new(),
            };
            let status = if item
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "error"
            } else {
                "success"
            };

            Some(KiroToolResult {
                content: vec![KiroToolResultContent { text: content_text }],
                status: status.to_string(),
                tool_use_id,
            })
        })
        .collect()
}

fn extract_tool_results_from_tool_message(message: &NormalizedMessage) -> Vec<KiroToolResult> {
    vec![KiroToolResult {
        content: vec![KiroToolResultContent {
            text: extract_text_content(message.content.as_ref()),
        }],
        status: "success".to_string(),
        tool_use_id: message.tool_call_id.clone().unwrap_or_default(),
    }]
}

async fn extract_images(client: &Client, content: Option<&Value>) -> Vec<ImageBlock> {
    let Some(Value::Array(items)) = content else {
        return Vec::new();
    };

    let mut images = Vec::new();
    for item in items {
        if let Some(image) = extract_image_block(client, item).await {
            images.push(image);
        }
    }
    images
}

async fn extract_image_block(client: &Client, item: &Value) -> Option<ImageBlock> {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
    match item_type {
        "image" => {
            let source = item.get("source")?;
            let bytes = source
                .get("data")
                .and_then(Value::as_str)
                .map(str::to_string)?;
            if encoded_image_exceeds_limit(&bytes) {
                return None;
            }
            let media_type = source
                .get("media_type")
                .and_then(Value::as_str)
                .unwrap_or("image/png");
            Some(ImageBlock {
                format: media_type_to_format(media_type)?,
                source: ImageSource {
                    source_type: "base64".to_string(),
                    data: bytes,
                },
            })
        }
        "image_url" => {
            let url = item
                .get("image_url")
                .and_then(|value| value.get("url").or(Some(value)))
                .and_then(Value::as_str)?;
            let (format, bytes) = resolve_image_source(client, url).await?;
            Some(ImageBlock {
                format,
                source: ImageSource {
                    source_type: "base64".to_string(),
                    data: bytes,
                },
            })
        }
        "input_image" => {
            let url = item
                .get("image_url")
                .and_then(Value::as_str)
                .or_else(|| item.get("url").and_then(Value::as_str))?;
            let (format, bytes) = resolve_image_source(client, url).await?;
            Some(ImageBlock {
                format,
                source: ImageSource {
                    source_type: "base64".to_string(),
                    data: bytes,
                },
            })
        }
        _ => None,
    }
}

fn media_type_to_format(media_type: &str) -> Option<String> {
    match media_type.trim().to_ascii_lowercase().as_str() {
        "image/png" | "png" => Some("png".to_string()),
        "image/jpeg" | "image/jpg" | "jpeg" | "jpg" => Some("jpeg".to_string()),
        "image/gif" | "gif" => Some("gif".to_string()),
        "image/webp" | "webp" => Some("webp".to_string()),
        _ => None,
    }
}

fn parse_data_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("data:")?;
    let (meta, bytes) = rest.split_once(',')?;
    let media_type = meta.split(';').next().unwrap_or_default();
    if encoded_image_exceeds_limit(bytes) {
        return None;
    }
    Some((media_type_to_format(media_type)?, bytes.to_string()))
}

async fn resolve_image_source(client: &Client, url: &str) -> Option<(String, String)> {
    let _ = client;
    if let Some(parsed) = parse_data_url(url) {
        return Some(parsed);
    }

    let image_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(IMAGE_FETCH_TIMEOUT_SECONDS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;
    let mut current_url = validate_remote_image_url(url).await?;

    for _ in 0..=MAX_IMAGE_REDIRECTS {
        let response = image_client.get(current_url.clone()).send().await.ok()?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)?
                .to_str()
                .ok()?;
            let next_url = current_url.join(location).ok()?;
            current_url = validate_remote_image_url(next_url.as_str()).await?;
            continue;
        }
        if !response.status().is_success() {
            return None;
        }

        if response
            .content_length()
            .map(|length| length > MAX_IMAGE_SOURCE_BYTES as u64)
            .unwrap_or(false)
        {
            return None;
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let final_url = response.url().clone();
        let bytes = response.bytes().await.ok()?;
        if bytes.len() > MAX_IMAGE_SOURCE_BYTES {
            return None;
        }
        let format = content_type
            .as_deref()
            .and_then(|value| value.split(';').next())
            .and_then(media_type_to_format)
            .or_else(|| infer_image_format_from_url(final_url.as_str()))?;

        return Some((format, STANDARD.encode(bytes)));
    }

    None
}

fn infer_image_format_from_url(url: &str) -> Option<String> {
    let path = reqwest::Url::parse(url).ok()?.path().to_ascii_lowercase();
    if path.ends_with(".png") {
        Some("png".to_string())
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Some("jpeg".to_string())
    } else if path.ends_with(".gif") {
        Some("gif".to_string())
    } else if path.ends_with(".webp") {
        Some("webp".to_string())
    } else {
        None
    }
}

async fn validate_remote_image_url(url: &str) -> Option<reqwest::Url> {
    let parsed = reqwest::Url::parse(url).ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return None,
    }

    let host = parsed.host_str()?;
    if host.eq_ignore_ascii_case("localhost") {
        return None;
    }

    let port = parsed.port_or_known_default()?;
    let mut resolved_any = false;
    for address in lookup_host((host, port)).await.ok()? {
        resolved_any = true;
        if is_restricted_remote_ip(address.ip()) {
            return None;
        }
    }

    if !resolved_any {
        return None;
    }

    Some(parsed)
}

fn encoded_image_exceeds_limit(encoded: &str) -> bool {
    encoded.len() > max_base64_len_for_bytes(MAX_IMAGE_SOURCE_BYTES)
}

fn max_base64_len_for_bytes(max_bytes: usize) -> usize {
    max_bytes.div_ceil(3) * 4
}

fn is_restricted_remote_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => {
            addr.is_private()
                || addr.is_loopback()
                || addr.is_link_local()
                || addr.is_broadcast()
                || addr.is_documentation()
                || addr.is_unspecified()
                || addr.is_multicast()
                || is_ipv4_shared(addr)
                || is_ipv4_reserved(addr)
        }
        IpAddr::V6(addr) => {
            addr.is_loopback()
                || addr.is_unspecified()
                || addr.is_multicast()
                || addr.is_unique_local()
                || addr.is_unicast_link_local()
                || is_ipv6_documentation(addr)
        }
    }
}

fn is_ipv4_shared(addr: Ipv4Addr) -> bool {
    let octets = addr.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_ipv4_reserved(addr: Ipv4Addr) -> bool {
    let octets = addr.octets();
    octets[0] >= 240
}

fn is_ipv6_documentation(addr: Ipv6Addr) -> bool {
    let segments = addr.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

fn extract_tool_uses(message: &NormalizedMessage) -> Option<Vec<KiroToolUse>> {
    let tool_calls = message.tool_calls.as_ref()?;
    let tool_uses: Vec<KiroToolUse> = tool_calls
        .iter()
        .map(|tool_call| KiroToolUse {
            name: tool_call.function.name.clone(),
            input: serde_json::from_str(&tool_call.function.arguments)
                .unwrap_or_else(|_| json!({})),
            tool_use_id: tool_call.id.clone(),
        })
        .collect();

    if tool_uses.is_empty() {
        None
    } else {
        Some(tool_uses)
    }
}

fn normalize_tool_choice(
    tool_choice: &Option<Value>,
    tools: &Option<Vec<Tool>>,
) -> Result<Option<Value>, String> {
    let Some(choice) = tool_choice.as_ref() else {
        return Ok(None);
    };

    let choice_type = match choice {
        Value::String(raw) => raw.trim(),
        Value::Object(_) => choice
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| "tool_choice.type 无效".to_string())?,
        _ => return Err("tool_choice 格式无效".to_string()),
    };

    match choice_type {
        "auto" => Ok(Some(json!({ "type": "auto" }))),
        "none" => Ok(Some(json!({ "type": "none" }))),
        "required" => {
            if tools.as_ref().is_none_or(|items| items.is_empty()) {
                return Err("tool_choice=required 时必须同时提供 tools".to_string());
            }
            Ok(Some(json!({ "type": "required" })))
        }
        "function" => {
            let name = choice
                .get("name")
                .or_else(|| choice.pointer("/function/name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "tool_choice.function.name 不能为空".to_string())?;

            let tool_exists = tools
                .as_ref()
                .map(|items| items.iter().any(|tool| tool.function.name == name))
                .unwrap_or(false);
            if !tool_exists {
                return Err(format!("tool_choice 指定的工具不存在: {name}"));
            }

            Ok(Some(json!({
                "type": "function",
                "name": name
            })))
        }
        other => Err(format!("暂不支持的 tool_choice.type: {other}")),
    }
}


fn convert_tools(tools: &Option<Vec<Tool>>) -> Option<Vec<KiroTool>> {
    tools.as_ref().map(|items| {
        items
            .iter()
            .map(|tool| KiroTool {
                tool_specification: KiroToolSpec {
                    name: tool.function.name.clone(),
                    description: tool_description(tool),
                    input_schema: KiroInputSchema {
                        json: tool_input_schema(tool),
                    },
                },
            })
            .collect()
    })
}

fn tool_description(tool: &Tool) -> String {
    if is_web_search_tool_type(&tool.tool_type) {
        let mut parts = vec![tool
            .function
            .description
            .clone()
            .unwrap_or_else(|| WEB_SEARCH_TOOL_DESCRIPTION.to_string())];
        if let Some(options) = &tool.web_search {
            if let Some(domains) = options
                .allowed_domains
                .as_ref()
                .filter(|items| !items.is_empty())
            {
                parts.push(format!("Only return results from: {}", domains.join(", ")));
            }
            if let Some(domains) = options
                .blocked_domains
                .as_ref()
                .filter(|items| !items.is_empty())
            {
                parts.push(format!("Exclude results from: {}", domains.join(", ")));
            }
        }
        return parts.join(" ");
    }

    tool.function.description.clone().unwrap_or_default()
}

fn tool_input_schema(tool: &Tool) -> Value {
    if is_web_search_tool_type(&tool.tool_type) {
        return web_search_input_schema();
    }

    normalize_json_schema(
        tool.function
            .parameters
            .clone()
            .unwrap_or_else(|| json!({})),
    )
}

fn normalize_json_schema(value: Value) -> Value {
    let mut schema = match value {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    if !schema.contains_key("type") {
        schema.insert("type".to_string(), Value::String("object".to_string()));
    }
    if !matches!(schema.get("properties"), Some(Value::Object(_))) {
        schema.insert("properties".to_string(), Value::Object(Map::new()));
    }
    if !matches!(schema.get("required"), Some(Value::Array(_))) {
        schema.insert("required".to_string(), Value::Array(Vec::new()));
    }

    Value::Object(schema)
}

fn process_tools_with_long_descriptions(
    tools: &Option<Vec<Tool>>,
) -> (Option<Vec<Tool>>, Option<String>) {
    let Some(tools) = tools else {
        return (None, None);
    };

    let mut processed = Vec::new();
    let mut long_docs = Vec::new();

    for tool in tools {
        let description = tool.function.description.clone().unwrap_or_default();
        if description.len() > TOOL_DESCRIPTION_MAX_LENGTH {
            long_docs.push(format!(
                "## Tool: {}\n\n{}",
                tool.function.name, description
            ));
            processed.push(Tool {
                tool_type: tool.tool_type.clone(),
                function: ToolFunction {
                    name: tool.function.name.clone(),
                    description: Some(format!(
                        "[Full documentation in system prompt under '## Tool: {}']",
                        tool.function.name
                    )),
                    parameters: tool.function.parameters.clone(),
                },
                web_search: tool.web_search.clone(),
            });
        } else {
            processed.push(tool.clone());
        }
    }

    let docs = if long_docs.is_empty() {
        None
    } else {
        Some(format!(
            "# Tool Documentation\n\n{}",
            long_docs.join("\n\n")
        ))
    };

    (Some(processed), docs)
}

fn join_with_newline(left: &str, right: &str) -> String {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => String::new(),
        (true, false) => right.to_string(),
        (false, true) => left.to_string(),
        (false, false) => format!("{left}\n{right}"),
    }
}

fn join_with_double_newline(left: &str, right: &str) -> String {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => String::new(),
        (true, false) => right.to_string(),
        (false, true) => left.to_string(),
        (false, false) => format!("{left}\n\n{right}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::models::AnthropicTool;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

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
                max_uses: None,
                allowed_domains: None,
                blocked_domains: None,
                user_location: None,
            }]),
            tool_choice: Some(json!({"type":"auto"})),
            metadata: None,
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
    fn normalize_anthropic_request_supports_versioned_web_search_tools() {
        let request = AnthropicMessagesRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![crate::gateway::models::AnthropicMessage {
                role: "user".to_string(),
                content: json!([{ "type": "text", "text": "查一下今天的 Rust 发布" }]),
            }],
            max_tokens: 2048,
            system: None,
            stream: false,
            temperature: None,
            top_p: None,
            stop_sequences: None,
            tools: Some(vec![AnthropicTool {
                // Use a documented versioned name as a compatibility sample. This test only
                // proves `web_search_*` matching, not that Kiro emits this exact version.
                r#type: Some("web_search_20260209".to_string()),
                name: "web_search".to_string(),
                description: None,
                input_schema: Value::Null,
                max_uses: Some(3),
                allowed_domains: Some(vec!["blog.rust-lang.org".to_string()]),
                blocked_domains: Some(vec!["example.com".to_string()]),
                user_location: Some(json!({ "type": "approximate", "city": "Singapore" })),
            }]),
            tool_choice: Some(json!({ "type": "auto" })),
            metadata: None,
        };

        let converted = normalize_anthropic_request(&request);
        let tool = converted
            .tools
            .as_ref()
            .and_then(|items| items.first())
            .expect("web search tool should exist");

        assert_eq!(tool.tool_type, "web_search_20260209");
        assert_eq!(tool.function.name, "web_search");
        assert_eq!(tool.function.parameters, Some(web_search_input_schema()));
        assert_eq!(
            tool.web_search
                .as_ref()
                .and_then(|options| options.max_uses),
            Some(3)
        );
        assert_eq!(
            tool.web_search
                .as_ref()
                .and_then(|options| options.allowed_domains.as_ref())
                .and_then(|items| items.first())
                .map(String::as_str),
            Some("blog.rust-lang.org")
        );
    }

    #[test]
    fn normalize_anthropic_request_understands_server_tool_use_history() {
        let request = AnthropicMessagesRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![
                crate::gateway::models::AnthropicMessage {
                    role: "assistant".to_string(),
                    content: json!([{
                        "type": "server_tool_use",
                        "id": "srv_1",
                        "name": "web_search",
                        "input": { "query": "Rust 1.90 release" }
                    }]),
                },
                crate::gateway::models::AnthropicMessage {
                    role: "user".to_string(),
                    content: json!([{
                        "type": "web_search_tool_result",
                        "tool_use_id": "srv_1",
                        "content": [{
                            "type": "web_search_result",
                            "title": "Rust Blog",
                            "url": "https://blog.rust-lang.org"
                        }]
                    }]),
                },
            ],
            max_tokens: 2048,
            system: None,
            stream: false,
            temperature: None,
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            metadata: None,
        };

        let converted = normalize_anthropic_request(&request);

        assert_eq!(
            converted.messages[0]
                .tool_calls
                .as_ref()
                .and_then(|items| items.first())
                .map(|call| call.function.name.as_str()),
            Some("web_search")
        );
        assert_eq!(converted.messages[1].tool_call_id.as_deref(), Some("srv_1"));
        assert_eq!(
            converted.messages[1].content,
            Some(json!([{
                "type": "web_search_tool_result",
                "tool_use_id": "srv_1",
                "content": [{
                    "type": "web_search_result",
                    "title": "Rust Blog",
                    "url": "https://blog.rust-lang.org"
                }]
            }]))
        );
    }

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
                web_search: None,
            }]),
            tool_choice: None,
            previous_response_id: None,
        };

        let payload = build_kiro_payload(
            &Client::new(),
            &request,
            Some("arn:aws:codewhisperer:::profile/test".to_string()),
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
        assert_eq!(history.len(), 2);
        match &history[0] {
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
        match &history[1] {
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
    async fn build_kiro_payload_keeps_multiple_tool_messages_separate() {
        let request = NormalizedRequest {
            model: "claude-sonnet-4-5-20250929".to_string(),
            messages: vec![
                NormalizedMessage {
                    role: "assistant".to_string(),
                    content: Some(json!("我会调用工具")),
                    tool_calls: Some(vec![
                        crate::gateway::models::ToolCall {
                            id: "call_1".to_string(),
                            call_type: "function".to_string(),
                            function: crate::gateway::models::ToolCallFunction {
                                name: "Read".to_string(),
                                arguments: "{\"path\":\"a\"}".to_string(),
                            },
                        },
                        crate::gateway::models::ToolCall {
                            id: "call_2".to_string(),
                            call_type: "function".to_string(),
                            function: crate::gateway::models::ToolCallFunction {
                                name: "Ls".to_string(),
                                arguments: "{\"path\":\".\"}".to_string(),
                            },
                        },
                    ]),
                    tool_call_id: None,
                    metadata: None,
                },
                NormalizedMessage {
                    role: "tool".to_string(),
                    content: Some(json!("{\"ok\":1}")),
                    tool_calls: None,
                    tool_call_id: Some("call_1".to_string()),
                    metadata: None,
                },
                NormalizedMessage {
                    role: "tool".to_string(),
                    content: Some(json!("{\"ok\":2}")),
                    tool_calls: None,
                    tool_call_id: Some("call_2".to_string()),
                    metadata: None,
                },
            ],
            stream: true,
            max_tokens: Some(2048),
            temperature: None,
            top_p: None,
            stop: None,
            tools: None,
            tool_choice: None,
            previous_response_id: Some("resp_prev_test".to_string()),
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect("payload should build");

        let history = payload
            .conversation_state
            .history
            .as_ref()
            .expect("history should exist");

        let history_tool_results = history
            .iter()
            .filter_map(|item| match item {
                HistoryItem::User { user_input_message } => user_input_message
                    .user_input_message_context
                    .as_ref()
                    .and_then(|context| context.tool_results.as_ref())
                    .map(Vec::len),
                _ => None,
            })
            .sum::<usize>();

        let current_tool_results = payload
            .conversation_state
            .current_message
            .user_input_message
            .user_input_message_context
            .as_ref()
            .and_then(|context| context.tool_results.as_ref())
            .map(Vec::len)
            .unwrap_or(0);

        assert_eq!(history_tool_results + current_tool_results, 2);
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
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
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
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
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

    #[test]
    fn normalize_responses_request_preserves_message_content_items() {
        let payload = json!({
            "model": "claude-3-7-sonnet-20250219",
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
            normalize_responses_request(&payload).expect("responses payload should convert");
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
    fn extract_text_content_reads_text_from_content_array_without_unwrap() {
        let content = json!([
            { "type": "input_text", "text": "第一段" },
            { "type": "output_text", "text": "第二段" },
            { "type": "input_image", "image_url": "data:image/png;base64,aGVsbG8=" }
        ]);

        assert_eq!(extract_text_content(Some(&content)), "第一段\n第二段");
    }

    #[test]
    fn normalize_responses_request_defaults_to_claude_sonnet_45() {
        let payload = json!({
            "input": [
                {
                    "role": "user",
                    "content": "hello"
                }
            ]
        });

        let converted =
            normalize_responses_request(&payload).expect("responses payload should convert");
        assert_eq!(converted.model, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn normalize_responses_request_keeps_tools_tool_choice_and_function_call_items() {
        let payload = json!({
            "model": "claude-3-7-sonnet-20250219",
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
            normalize_responses_request(&payload).expect("responses payload should convert");

        assert_eq!(
            converted.tool_choice,
            Some(json!({ "type": "function", "name": "search_docs" }))
        );
        assert_eq!(converted.tools.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            converted
                .tools
                .as_ref()
                .and_then(|items| items.first())
                .map(|tool| tool.function.name.as_str()),
            Some("search_docs")
        );
        assert_eq!(converted.messages.len(), 3);
        assert_eq!(converted.messages[0].role, "user");
        assert_eq!(
            converted.messages[0].content,
            Some(json!([
                { "type": "input_text", "text": "先检索 gateway" }
            ]))
        );
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
    fn normalize_responses_request_preserves_assistant_message_metadata() {
        let payload = json!({
            "model": "claude-3-7-sonnet-20250219",
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
            normalize_responses_request(&payload).expect("responses payload should convert");

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
                web_search: None,
            }]),
            tool_choice: Some(json!({ "type": "function", "name": "search_docs" })),
            previous_response_id: None,
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect("payload should build");

        assert_eq!(
            payload
                .conversation_state
                .current_message
                .user_input_message
                .user_input_message_context
                .as_ref()
                .and_then(|context| context.tool_choice.as_ref())
                .cloned(),
            Some(json!({ "type": "function", "name": "search_docs" }))
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
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
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
                web_search: None,
            }]),
            tool_choice: Some(json!({ "type": "function", "name": "missing_tool" })),
            previous_response_id: None,
        };

        let error = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect_err("unknown tool choice should fail");

        assert!(error.contains("tool_choice 指定的工具不存在"));
    }

    #[tokio::test]
    async fn build_kiro_payload_extracts_base64_images() {
        let request = NormalizedRequest {
            model: "claude-sonnet-4-5-20250929".to_string(),
            messages: vec![NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!([
                    {
                        "type": "text",
                        "text": "看图回答"
                    },
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": "aGVsbG8="
                        }
                    }
                ])),
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
            previous_response_id: None,
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect("payload should build");
        let current = &payload
            .conversation_state
            .current_message
            .user_input_message;

        assert_eq!(current.images.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            current
                .images
                .as_ref()
                .and_then(|images| images.first())
                .map(|image| image.format.as_str()),
            Some("png")
        );
    }

    #[tokio::test]
    async fn build_kiro_payload_rejects_private_remote_images() {
        let expected_bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        listener
            .set_nonblocking(true)
            .expect("listener should set nonblocking");
        let address = format!(
            "http://{}",
            listener.local_addr().expect("local addr should resolve")
        );

        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buffer = [0u8; 1024];
                        let _ = stream.read(&mut buffer);
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            expected_bytes.len()
                        );
                        stream
                            .write_all(response.as_bytes())
                            .expect("headers should write");
                        stream
                            .write_all(&expected_bytes)
                            .expect("body should write");
                        return true;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            return false;
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => return false,
                }
            }
        });

        let request = NormalizedRequest {
            model: "claude-sonnet-4-5-20250929".to_string(),
            messages: vec![NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!([
                    {
                        "type": "text",
                        "text": "看图回答"
                    },
                    {
                        "type": "input_image",
                        "image_url": format!("{address}/sample.png")
                    }
                ])),
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
            previous_response_id: None,
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect("payload should build");
        assert!(
            !handle.join().expect("server thread should finish"),
            "client should not fetch private image"
        );
        let current = &payload
            .conversation_state
            .current_message
            .user_input_message;

        assert!(current.images.as_ref().map(Vec::is_empty).unwrap_or(true));
    }

    #[tokio::test]
    async fn build_kiro_payload_rejects_oversized_data_url_images() {
        let oversized = STANDARD.encode(vec![0u8; 6 * 1024 * 1024]);
        let request = NormalizedRequest {
            model: "claude-sonnet-4-5-20250929".to_string(),
            messages: vec![NormalizedMessage {
                role: "user".to_string(),
                content: Some(json!([
                    {
                        "type": "text",
                        "text": "看图回答"
                    },
                    {
                        "type": "input_image",
                        "image_url": format!("data:image/png;base64,{oversized}")
                    }
                ])),
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
            previous_response_id: None,
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect("payload should build");
        let current = &payload
            .conversation_state
            .current_message
            .user_input_message;

        assert!(current.images.as_ref().map(Vec::is_empty).unwrap_or(true));
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
        };

        let payload = build_kiro_payload(&Client::new(), &request, None)
            .await
            .expect("payload should build");
        let history = payload
            .conversation_state
            .history
            .expect("history should exist");

        match &history[0] {
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

    #[test]
    fn get_internal_model_id_normalizes_versioned_public_model_names() {
        assert_eq!(
            get_internal_model_id("claude-sonnet-4-5-20250929")
                .expect("versioned sonnet 4.5 should map"),
            "claude-sonnet-4.5"
        );
        assert_eq!(
            get_internal_model_id("claude-sonnet-4-6").expect("sonnet 4.6 alias should map"),
            "claude-sonnet-4.6"
        );
        assert_eq!(
            get_internal_model_id("claude-sonnet-4-6-20260217")
                .expect("versioned sonnet 4.6 should map"),
            "claude-sonnet-4.6"
        );
        assert_eq!(
            get_internal_model_id("claude-opus-4-6").expect("opus 4.6 alias should map"),
            "claude-opus-4.6"
        );
        assert_eq!(
            get_internal_model_id("claude-opus-4-6-20260205")
                .expect("versioned opus 4.6 should map"),
            "claude-opus-4.6"
        );
        assert_eq!(
            get_internal_model_id("claude-haiku-4-5-20251001")
                .expect("versioned haiku 4.5 should map"),
            "claude-haiku-4.5"
        );
        assert_eq!(
            get_internal_model_id("claude-sonnet-latest")
                .expect("latest sonnet alias should default to 4.5"),
            "claude-sonnet-4.5"
        );
        assert_eq!(
            get_internal_model_id("sonnet").expect("plain sonnet alias should default to 4.5"),
            "claude-sonnet-4.5"
        );
    }

    #[test]
    fn get_available_models_includes_claude_46_official_ids() {
        let model_ids: Vec<_> = get_available_models()
            .into_iter()
            .map(|model| model.id)
            .collect();

        assert!(model_ids.iter().any(|id| id == "claude-opus-4-6"));
        assert!(model_ids.iter().any(|id| id == "claude-opus-4-6-20260205"));
        assert!(model_ids.iter().any(|id| id == "claude-sonnet-4-6"));
        assert!(model_ids
            .iter()
            .any(|id| id == "claude-sonnet-4-6-20260217"));
    }
}
