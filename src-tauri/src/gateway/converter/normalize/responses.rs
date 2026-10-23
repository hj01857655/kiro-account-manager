//! OpenAI Responses 协议：请求归一化与 input items 解析。

use super::*;

pub fn normalize_openai_responses_request(payload: &Value) -> Result<NormalizedRequest, String> {
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

    let (tools, tool_name_map, deferred_tool_names) =
        convert_responses_tools(payload.get("tools"));
    Ok(build_normalized_request_from_payload(
        payload,
        model,
        messages,
        tools,
        tool_name_map,
        deferred_tool_names,
    ))
}

pub fn convert_responses_input(input: &Value) -> Vec<NormalizedMessage> {
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

pub fn convert_responses_input_items(items: &[Value]) -> Vec<NormalizedMessage> {
    let mut messages = Vec::new();
    let mut pending_user_items = Vec::new();

    for item in items {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();

        // EasyInputMessage / message 项：官方 role = system|developer|user|assistant
        if let Some(raw_role) = item.get("role").and_then(Value::as_str) {
            flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
            let role = match raw_role {
                "system" | "user" | "assistant" => raw_role.to_string(),
                // OpenAI Responses: developer 指令级 → 内部 system
                "developer" => "system".to_string(),
                other => {
                    log::warn!(
                        "[协议映射] Responses 不支持的 message.role=\"{other}\"，已跳过。官方取值: system|developer|user|assistant"
                    );
                    continue;
                }
            };
            messages.push(NormalizedMessage {
                role: role.clone(),
                content: responses_message_content(item),
                tool_calls: None,
                tool_call_id: None,
                metadata: extract_responses_message_metadata(item, &role),
            });
            continue;
        }

        match item_type {
            "message" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                // type=message 但缺 role 时官方默认按 user 处理
                let role = "user".to_string();
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
            "input_text" | "output_text" | "input_image" | "image_url" | "image"
            | "input_file" | "file" => {
                pending_user_items.push(item.clone());
            }
            // OpenAI Responses 文档 reasoning item → Kiro reasoningContent
            "reasoning" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                let mut metadata = Map::new();
                if let Some(reasoning) = extract_responses_reasoning_item(item)
                    .or_else(|| extract_reasoning_content(Some(item)))
                    .or_else(|| extract_reasoning_content(item.get("content")))
                {
                    metadata.insert("reasoningContent".to_string(), reasoning);
                }
                messages.push(NormalizedMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: if metadata.is_empty() {
                        None
                    } else {
                        Some(Value::Object(metadata))
                    },
                });
            }
            // 官方工具类 item：按 function_call / function_call_output 语义映射到 Kiro tools
            "custom_tool_call" | "web_search_call" | "file_search_call" | "code_interpreter_call"
            | "computer_call" | "local_shell_call" | "image_generation_call" | "mcp_call" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                push_responses_tool_call_message(&mut messages, item, item_type);
            }
            "custom_tool_call_output" | "computer_call_output" | "local_shell_call_output"
            | "mcp_approval_response" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                push_responses_tool_output_message(&mut messages, item);
            }
            // 会话引用：依赖 previous_response_id 恢复，input 内 item_reference 本身无正文可转
            "item_reference" => {
                log::info!(
                    "[协议映射] Responses item_reference 跳过（依赖 previous_response_id 会话恢复）"
                );
            }
            // 服务端审批/列表类：无对等 Kiro 语义，记录后跳过（不伪造 user 文本）
            "mcp_list_tools" | "mcp_approval_request" => {
                log::warn!(
                    "[协议映射] Responses 官方类型 \"{item_type}\" 暂无 Kiro 对等语义，已跳过"
                );
            }
            // compaction 非 Responses 核心 input 类型；若客户端带入则保留标记（非 /compact 实现）
            "compaction" => {
                flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
                messages.push(NormalizedMessage {
                    role: "system".to_string(),
                    content: Some(item.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: Some(json!({ "is_compaction": true })),
                });
            }
            other => {
                // 不在官方已知表内的 type：明确告警并跳过，禁止塞成假 user 文本
                log::warn!(
                    "[协议映射] 非官方/未映射的 Responses input type=\"{other}\"，已跳过。\
                     已映射: message|function_call|function_call_output|reasoning|\
                     custom_tool_call(|_output)|web_search_call|file_search_call|\
                     code_interpreter_call|computer_call(|_output)|local_shell_call(|_output)|\
                     image_generation_call|mcp_call|item_reference|input_text|output_text|image*"
                );
            }
        }
    }

    flush_pending_responses_user_items(&mut messages, &mut pending_user_items);
    messages
}

pub fn flush_pending_responses_user_items(
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

/// OpenAI Responses 官方 reasoning item → Kiro reasoningContent 结构。
/// `encrypted_content` 是官方 `C6i` JSON 信封（`{"redactedContent":<b64>}` 或
/// `{"reasoningText":{text,signature?}}`），按官方 `efd` 语义解包：信封存在时
/// 它就是全部内容；无信封时退回文档字段 content[].reasoning_text，其次
/// summary[].summary_text。
pub fn extract_responses_reasoning_item(item: &Value) -> Option<Value> {
    let encrypted = item
        .get("encrypted_content")
        .or_else(|| item.get("encryptedContent"));
    if let Some(reasoning) = parse_responses_encrypted_envelope(encrypted) {
        return Some(reasoning);
    }

    let mut texts = Vec::new();

    if let Some(content) = item.get("content") {
        match content {
            Value::Array(parts) => {
                for part in parts {
                    let part_type = part.get("type").and_then(Value::as_str).unwrap_or_default();
                    if matches!(part_type, "reasoning_text" | "summary_text" | "text") {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                texts.push(text.to_string());
                            }
                        }
                    }
                }
            }
            Value::String(text) if !text.is_empty() => texts.push(text.clone()),
            _ => {}
        }
    }

    // content 缺失时才看 summary（两者并存时 content 是思考正文，summary 只是摘要）
    if texts.is_empty() {
        if let Some(Value::Array(parts)) = item.get("summary") {
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        texts.push(text.to_string());
                    }
                }
            }
        }
    }

    // summary / content 都没有时，兜底 text 字段
    if texts.is_empty() {
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                texts.push(text.to_string());
            }
        }
    }

    if texts.is_empty() {
        return None;
    }

    meaningful_optional_value(Some(json!({
        "reasoningText": { "text": texts.join("\n") }
    })))
}

/// 官方 `efd` 的信封解析：解不出或形状不符返回 None（官方同样不落 reasoning）。
fn parse_responses_encrypted_envelope(encrypted: Option<&Value>) -> Option<Value> {
    let encoded = encrypted?.as_str()?;
    let parsed: Value = serde_json::from_str(encoded).ok()?;
    let obj = parsed.as_object()?;

    if let Some(redacted) = obj
        .get("redactedContent")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return Some(json!({ "redactedContent": redacted }));
    }

    let reasoning_text = obj.get("reasoningText")?.as_object()?;
    let text = reasoning_text.get("text").and_then(Value::as_str)?;
    let mut envelope = json!({ "reasoningText": { "text": text } });
    if let Some(signature) = reasoning_text.get("signature").and_then(Value::as_str) {
        envelope["reasoningText"]["signature"] = json!(signature);
    }
    Some(envelope)
}

/// Responses 官方工具调用 item → assistant.tool_calls（function 语义）
pub fn push_responses_tool_call_message(
    messages: &mut Vec<NormalizedMessage>,
    item: &Value,
    item_type: &str,
) {
    let name = item
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(item_type)
        .to_string();
    let arguments = item
        .get("arguments")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            serde_json::to_string(
                item.get("arguments")
                    .or_else(|| item.get("action"))
                    .or_else(|| item.get("input"))
                    .unwrap_or(&json!({})),
            )
            .unwrap_or_else(|_| "{}".to_string())
        });
    messages.push(NormalizedMessage {
        role: "assistant".to_string(),
        content: None,
        tool_calls: Some(vec![ToolCall {
            id: item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction { name, arguments },
        }]),
        tool_call_id: None,
        metadata: None,
    });
}

/// Responses 官方工具输出 item → role=tool
pub fn push_responses_tool_output_message(messages: &mut Vec<NormalizedMessage>, item: &Value) {
    messages.push(NormalizedMessage {
        role: "tool".to_string(),
        content: responses_tool_output_content(
            item.get("output")
                .or_else(|| item.get("result"))
                .or_else(|| item.get("content")),
        ),
        tool_calls: None,
        tool_call_id: item
            .get("call_id")
            .or_else(|| item.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        metadata: None,
    });
}

pub fn responses_message_content(item: &Value) -> Option<Value> {
    item.get("content")
        .cloned()
        .or_else(|| item.get("text").cloned())
}

pub fn responses_tool_output_content(output: Option<&Value>) -> Option<Value> {
    match output {
        None => None,
        Some(Value::String(text)) => Some(Value::String(text.clone())),
        Some(other) => Some(Value::String(other.to_string())),
    }
}

pub fn extract_responses_message_metadata(item: &Value, role: &str) -> Option<Value> {
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

pub fn convert_responses_tools(
    tools: Option<&Value>,
) -> (
    Option<Vec<Tool>>,
    std::collections::HashMap<String, String>,
    Vec<String>,
) {
    let mut tool_name_map = std::collections::HashMap::new();
    // Tool Search 规范：defer_loading 工具的 sanitized 名单。三种工具形状
    // （嵌套 function / MCP / 扁平 function）统一从原始 item 顶层读取，
    // 语义对齐 Anthropic 侧（见 normalize/anthropic.rs 与 proxy/tool_search.rs）。
    let mut deferred_tool_names = Vec::new();

    let Some(items) = tools.and_then(Value::as_array) else {
        return (None, tool_name_map, deferred_tool_names);
    };

    let converted: Vec<Tool> = items
        .iter()
        .flat_map(|item| {
            // `namespace` 是官方「把一组 function/custom 工具包在共享命名空间下」
            // 的形状：`{ description, name, tools, type: "namespace" }`。
            // 整项跳过会让命名空间里的所有工具消失，必须把成员逐个展开。
            if item.get("type").and_then(Value::as_str) == Some("namespace") {
                return expand_namespace_tool(item, &mut tool_name_map, &mut deferred_tool_names);
            }
            let Some((tool, mapping)) = convert_responses_tool(item) else {
                return Vec::new();
            };
            if let Some((sanitized, original)) = mapping {
                tool_name_map.insert(sanitized, original);
            }
            if item.get("defer_loading") == Some(&Value::Bool(true)) {
                // tool.function.name 已是 sanitize 后的名字，与请求侧拆分的
                // 消费约定一致（proxy/tool_search.rs）
                deferred_tool_names.push(tool.function.name.clone());
            }
            vec![tool]
        })
        .collect();

    if converted.is_empty() {
        (None, tool_name_map, deferred_tool_names)
    } else {
        (Some(converted), tool_name_map, deferred_tool_names)
    }
}

/// 展开官方 `Namespace object { description, name, tools, type }`。
///
/// 命名空间是「把一组 function/custom 工具包在共享名前缀下」的声明容器，
/// 本身不是可调用工具——成员才是。展开后每个成员都变成普通工具；成员的
/// `defer_loading` 逐个收集（官方允许成员各自延迟加载）。
///
/// 成员名前缀：官方说命名空间名「用于 tool calls」（例如 `crm`），但没说成员
/// 调用名是否要拼接。为了不让两个命名空间的同名成员撞车（且模型要能区分），
/// 这里统一拼成 `namespace__member`，与 MCP 的 `mcp__server__tool` 习惯一致。
fn expand_namespace_tool(
    item: &Value,
    tool_name_map: &mut std::collections::HashMap<String, String>,
    deferred_tool_names: &mut Vec<String>,
) -> Vec<Tool> {
    let namespace = item
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("namespace");
    let namespace_desc = item.get("description").and_then(Value::as_str);

    let Some(members) = item.get("tools").and_then(Value::as_array) else {
        log::warn!("[Responses] namespace `{namespace}` 缺少 tools 数组，已跳过");
        return Vec::new();
    };

    let mut out = Vec::with_capacity(members.len());
    for member in members {
        let Some((mut tool, _)) = convert_responses_tool(member) else {
            log::warn!(
                "[Responses] namespace `{namespace}` 中的成员无法转换（type={:?}），已跳过",
                member.get("type")
            );
            continue;
        };
        // 响应侧要把工具名还原成客户端声明的原名（不带命名空间前缀），
        // 否则客户端认不出自己声明的工具。
        let raw_member_name = member
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let qualified = shorten_tool_name(&sanitize_tool_name(&format!(
            "{namespace}__{raw_member_name}"
        )));
        if qualified != raw_member_name && !raw_member_name.is_empty() {
            tool_name_map.insert(qualified.clone(), raw_member_name);
        }
        // 命名空间描述与成员描述合并：上游只看到普通工具，丢掉了命名空间的
        // 分组语义，保留一层上下文能帮模型判断何时用哪个成员。
        tool.function.description = match (namespace_desc, tool.function.description) {
            (Some(ns), Some(desc)) => Some(format!("[{namespace}] {ns}\n{desc}")),
            (Some(ns), None) => Some(format!("[{namespace}] {ns}")),
            (None, Some(desc)) => Some(format!("[{namespace}] {desc}")),
            (None, None) => None,
        };
        tool.function.name = qualified.clone();

        // defer_loading 从成员自身读取（官方定义在成员层，不在 namespace 层）
        if member.get("defer_loading") == Some(&Value::Bool(true)) {
            deferred_tool_names.push(qualified);
        }
        out.push(tool);
    }

    if out.is_empty() {
        log::warn!("[Responses] namespace `{namespace}` 无可用成员，已跳过");
    }
    out
}

pub fn convert_responses_tool(item: &Value) -> Option<(Tool, Option<(String, String)>)> {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();

    if item.get("function").is_some() {
        let mut tool: Tool = serde_json::from_value(item.clone()).ok()?;
        let original_name = tool.function.name.clone();
        let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));
        tool.function.name = sanitized_name.clone();

        let mapping = if sanitized_name != original_name {
            Some((sanitized_name, original_name))
        } else {
            None
        };

        return Some((tool, mapping));
    }

    // 修复：MCP 工具缺少 type 字段导致之前被跳过
    // MCP 格式：{ "name": "...", "description": "...", "inputSchema": {...} }
    // 转换为 OpenAI 格式：{ "type": "function", "function": { "name": "...", "parameters": {...} } }
    if item_type.is_empty() && item.get("name").is_some() {
        let original_name = item.get("name").and_then(Value::as_str)?.to_string();
        let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));
        let description = item
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);

        // 从 inputSchema 或 parameters 中提取参数定义
        // MCP 工具的 inputSchema 本身就是 JSON Schema，不需要访问 .json 字段
        let parameters = item
            .get("inputSchema")
            .cloned()
            .or_else(|| item.get("parameters").cloned());

        let mapping = if sanitized_name != original_name {
            Some((sanitized_name.clone(), original_name))
        } else {
            None
        };

        return Some((
            Tool {
                tool_type: "function".to_string(),
                function: ToolFunction {
                    name: sanitized_name,
                    description,
                    parameters,
                },
                cache_control: None,
            },
            mapping,
        ));
    }

    // OpenAI `custom` 工具（free-form 文本输入/输出）：官方形状
    // `{ "type": "custom", "name": "...", "description": "...", "format": {...} }`。
    // 没有 `parameters`——上游 Kiro 只认 JSON Schema 工具，因此用单字段对象
    // 包住自由文本输入，让模型仍能发起调用（内容走 `input` 字符串）。
    // 直接丢弃会让客户端声明的 custom 工具彻底消失。
    if item_type == "custom" {
        let original_name = item.get("name").and_then(Value::as_str)?.to_string();
        let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));
        let description = item
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);
        let mapping = if sanitized_name != original_name {
            Some((sanitized_name.clone(), original_name))
        } else {
            None
        };
        return Some((
            Tool {
                tool_type: "function".to_string(),
                function: ToolFunction {
                    name: sanitized_name,
                    description,
                    parameters: Some(json!({
                        "type": "object",
                        "properties": {
                            "input": {
                                "type": "string",
                                "description": "Free-form text input for this custom tool."
                            }
                        },
                        "required": ["input"]
                    })),
                },
                cache_control: None,
            },
            mapping,
        ));
    }

    if item_type != "function" {
        // 官方 hosted tools（web_search / file_search / code_interpreter / mcp /
        // image_generation 等）与未来的新工具类型都落到这里。上游 Kiro 只认
        // function 形状，无法转发；但静默丢弃会让客户端以为工具已声明，
        // 因此至少留一条可诊断的日志（带名字，便于定位是哪个工具）。
        log::warn!(
            "[Responses] 工具 `{}` 的类型 `{item_type}` 不受支持（上游只认 function），已跳过",
            item.get("name").and_then(Value::as_str).unwrap_or("<unnamed>")
        );
        return None;
    }

    let original_name = item.get("name").and_then(Value::as_str)?.to_string();
    let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));

    let mapping = if sanitized_name != original_name {
        Some((sanitized_name.clone(), original_name))
    } else {
        None
    };

    Some((
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: sanitized_name,
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                parameters: item.get("parameters").cloned(),
            },
            cache_control: None,
        },
        mapping,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// OpenAI free-form custom tool：无 parameters，不能静默丢弃
    #[test]
    fn custom_tool_is_not_dropped() {
        let item = serde_json::json!({
            "type": "custom",
            "name": "my_custom",
            "description": "free form tool"
        });
        let (tool, _) = convert_responses_tool(&item).expect("custom tool must be converted");
        assert_eq!(tool.function.name, "myCustom");
        let params = tool.function.parameters.unwrap();
        assert_eq!(params["type"], "object");
        assert!(params["properties"].get("input").is_some());
    }

    /// 官方 `Namespace object { description, name, tools, type }`：整项跳过会
    /// 让命名空间里的所有工具消失，必须展开成逐个成员。
    #[test]
    fn namespace_tool_expands_members() {
        let tools = serde_json::json!([
            {
                "type": "namespace",
                "name": "crm",
                "description": "Customer relationship tools",
                "tools": [
                    {
                        "type": "function",
                        "name": "find_customer",
                        "description": "Find a customer",
                        "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}
                    },
                    {
                        "type": "custom",
                        "name": "draft_email",
                        "description": "Draft an email",
                        "defer_loading": true
                    }
                ]
            }
        ]);

        let (converted, map, deferred) = convert_responses_tools(Some(&tools));
        let list = converted.expect("namespace must expand");
        assert_eq!(list.len(), 2, "两个成员都应展开");

        let names: Vec<&str> = list.iter().map(|t| t.function.name.as_str()).collect();
        // sanitize_tool_name 会把 `crm__find_customer` 归一成 camelCase
        assert!(names.iter().any(|n| n.to_lowercase().contains("find")), "actual: {names:?}");
        assert!(names.iter().any(|n| n.to_lowercase().contains("draft")), "actual: {names:?}");

        // 命名空间描述要保留在成员描述里（上游没有分组概念）
        assert!(list
            .iter()
            .all(|t| t.function.description.as_deref().unwrap_or("").contains("crm")));

        // 成员的 defer_loading 逐个收集，且用展开后的限定名
        assert_eq!(deferred.len(), 1, "只有 draft_email 标了 defer_loading");
        assert!(deferred[0].to_lowercase().contains("draft"));

        // 限定名要能从 sanitize 后的名字映射回原名
        assert!(map.values().any(|v| v == "draft_email"));
    }

    /// 命名空间缺 tools 数组时不能 panic，也不应产生工具。
    #[test]
    fn namespace_without_tools_is_skipped() {
        let tools = serde_json::json!([{"type": "namespace", "name": "empty"}]);
        let (converted, _, deferred) = convert_responses_tools(Some(&tools));
        assert!(converted.is_none());
        assert!(deferred.is_empty());
    }

    /// hosted tools（web_search / file_search / mcp 等）上游无法执行，
    /// 必须跳过（不能当 function 下发），且不影响同一数组里的 function 工具。
    #[test]
    fn hosted_tools_are_skipped_without_breaking_siblings() {
        let tools = serde_json::json!([
            {"type": "web_search"},
            {"type": "file_search", "vector_store_ids": ["vs_1"]},
            {"type": "code_interpreter", "container": {"type": "auto"}},
            {
                "type": "function",
                "name": "get_weather",
                "parameters": {"type": "object"}
            }
        ]);

        let (converted, _, deferred) = convert_responses_tools(Some(&tools));
        let list = converted.expect("function 工具应保留");
        assert_eq!(list.len(), 1, "三个 hosted tool 都被跳过，只剩 function");
        assert_eq!(list[0].function.name, "getWeather");
        assert!(deferred.is_empty());
    }

    /// namespace 层自身的 defer_loading 不参与（官方定义在成员层）。
    /// 若将来官方改成允许 namespace 层，这条会提醒改实现。
    #[test]
    fn namespace_level_defer_loading_is_not_consumed() {
        let tools = serde_json::json!([{
            "type": "namespace",
            "name": "crm",
            "defer_loading": true,
            "tools": [{
                "type": "function",
                "name": "find_customer",
                "parameters": {"type": "object"}
            }]
        }]);

        let (converted, _, deferred) = convert_responses_tools(Some(&tools));
        let list = converted.expect("成员仍应展开");
        assert_eq!(list.len(), 1);
        assert!(
            deferred.is_empty(),
            "namespace 层 defer_loading 不消费，只认成员层"
        );
    }
}
