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
    // Anthropic 服务端工具（在 Anthropic 基础设施上执行，Kiro 上游无执行环境）：
    // 记录 sanitized 名单，payload 构建时剥离。保留在 request.tools 中是为了
    // tool_choice 校验与 WebSearch 子请求检测。
    //
    // 判别必须按官方执行方，不能"type != custom 即 server"一刀切：官方另有
    // **client tool**（schema 由 Anthropic 定义、执行在客户端），它们同样带
    // 日期化 type，却必须原样下发，否则模型看不到工具、客户端功能失效。
    // 参见 tool-reference：Server = web_search / web_fetch / code_execution /
    // advisor / tool_search_tool / mcp_toolset；Client = memory / bash /
    // text_editor / computer_toolset / browser_toolset。
    let mut server_tool_names = Vec::new();
    // Tool Search 规范：defer_loading 工具的 sanitized 名单。工具本体保留在
    // tools 里，由 upstream 挂载点统一拆分（含 tool_choice 保护），见
    // proxy/tool_search.rs。
    let mut deferred_tool_names = Vec::new();
    // client toolset 展开出的成员工具，追加在末尾（不扰动客户端已有工具的
    // 缓存顺序）。展开后模型才能看到 `screenshot`/`navigate` 等成员并调用。
    let mut toolset_members: Vec<Tool> = Vec::new();
    let tools = request.tools.as_ref().map(|tools| {
        tools
            .iter()
            .filter_map(|tool| {
                if is_server_executed_tool(tool.r#type.as_deref()) {
                    server_tool_names.push(crate::gateway::converter::sanitize_tool_name(
                        &tool.name,
                    ));
                    return None;
                }
                // client toolset（computer/browser）：官方定义为「一个无名条目，
                // 声明一组固定成员」，上游只认普通工具定义——必须展开成逐个
                // 成员下发，否则模型看不到任何成员，功能在网关后直接失效。
                if tool.name.is_empty() {
                    let Some(tool_type) = tool.r#type.as_deref() else {
                        log::warn!("[Anthropic] 无名且无 type 的工具条目，已跳过");
                        return None;
                    };
                    match crate::gateway::converter::expand_toolset(
                        tool_type,
                        tool.configs.as_ref(),
                    ) {
                        Some(members) => {
                            log::info!(
                                "[Anthropic] toolset `{tool_type}` 展开为 {} 个成员工具",
                                members.len()
                            );
                            for (name, description, parameters) in members {
                                toolset_members.push(Tool {
                                    tool_type: "function".to_string(),
                                    function: ToolFunction {
                                        name: name.to_string(),
                                        description: Some(description.to_string()),
                                        parameters: Some(parameters),
                                    },
                                    cache_control: tool.cache_control.clone(),
                                });
                            }
                        }
                        None => {
                            // 全部成员被 configs 禁用，或未知 toolset 类型。
                            log::warn!(
                                "[Anthropic] toolset `{tool_type}` 无可展开成员（全部禁用或未知类型），已跳过"
                            );
                        }
                    }
                    return None;
                }
                // 只允许代码执行沙箱调用的工具：上游无该环境，模型永远调不到。
                if let Some(callers) = tool.allowed_callers.as_ref() {
                    let has_direct = callers.iter().any(|c| c == "direct");
                    if !has_direct && !callers.is_empty() {
                        log::warn!(
                            "[Anthropic] 工具 `{}` 的 allowed_callers={:?} 不含 direct：\
                             上游 Kiro 无代码执行沙箱，该工具实际不可调用",
                            tool.name,
                            callers
                        );
                    }
                }
                if tool.defer_loading == Some(true) {
                    deferred_tool_names.push(crate::gateway::converter::sanitize_tool_name(
                        &tool.name,
                    ));
                }
                let (converted_tool, mapping) = convert_anthropic_tool(tool);
                if let Some((sanitized, original)) = mapping {
                    tool_name_map.insert(sanitized, original);
                }
                Some(converted_tool)
            })
            .collect()
    });

    // toolset 成员追加在末尾：它们与客户端已有工具同名时以客户端定义为准
    // （客户端自建的同名工具可能参数不同，保留先出现的那个）。
    let tools: Option<Vec<Tool>> = tools.map(|mut list: Vec<Tool>| {
        for member in toolset_members {
            if !list.iter().any(|t| t.function.name == member.function.name) {
                list.push(member);
            }
        }
        list
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
        // Anthropic 官方 effort 字段是顶层的 output_config.effort
        // （Kiro 官方文档的 Claude 示例全部用 output_config.effort），
        // 与 thinking 正交——thinking 未启用时 effort 依然生效。
        effort: request.output_config.as_ref().and_then(|config| {
            config
                .get("effort")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        }),
        include_usage: false,
        tool_name_map,
        server_tool_names,
        deferred_tool_names,
        deferred_tools: Vec::new(),
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

            // 检查是否包含图片或文档（必须保留原始数组，extract_images/extract_documents 需要从中提取）
            let has_image = items.iter().any(|item| {
                let t = item.get("type").and_then(Value::as_str).unwrap_or_default();
                t == "image" || t == "image_url" || t == "input_image"
            });
            let has_document = items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some("document"));
            if has_image || has_document {
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
