//! 工具（function calling）转换：工具名规范化、JSON Schema 清洗与长描述外移。

use super::*;

/// 规范化工具名称为 Kiro API 接受的格式
///
/// Kiro API 要求工具名称必须是纯 camelCase 格式（不能包含下划线或横杠）
/// 将分隔符（_、-、多下划线命名空间前缀）转换为 camelCase 边界
pub fn sanitize_tool_name(name: &str) -> String {
    // 按下划线和横杠分割
    let parts: Vec<&str> = name
        .split(|c| c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return "tool".to_string();
    }

    // 构建 camelCase：第一部分小写开头，其余部分首字母大写
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // 第一部分：首字母小写
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_lowercase().to_string());
                result.push_str(chars.as_str());
            }
        } else {
            // 其余部分：首字母大写
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_uppercase().to_string());
                result.push_str(chars.as_str());
            }
        }
    }

    if result.is_empty() {
        "tool".to_string()
    } else {
        result
    }
}

/// 判定 Anthropic 工具是否由 **服务端** 执行（因此必须从下发清单剥离）。
///
/// 判别依据是官方 tool-reference 的 Execution 列，不是 `type != "custom"`：
/// 官方另有 client tool，它们同样带日期化 `type`，但 schema 由 Anthropic 定义、
/// 执行在客户端——剥掉会让模型看不到工具，客户端功能直接失效。
///
/// - Server：`web_search_*` / `web_fetch_*` / `code_execution_*` / `advisor_*` /
///   `tool_search_tool_*` / `mcp_toolset`
/// - Client：`memory_*` / `bash_*` / `text_editor_*` / `computer_*` /
///   `computer_toolset_*` / `browser_toolset_*`
///
/// `type` 为空或 `custom` 是客户端自定义工具，不剥离。未知 `type` 按客户端工具
/// 保留：官方会持续新增版本，剥错（功能消失）比留着（上游多一个普通工具）更糟。
pub fn is_server_executed_tool(tool_type: Option<&str>) -> bool {
    const SERVER_PREFIXES: &[&str] = &[
        "web_search",
        "web_fetch",
        "code_execution",
        "advisor",
        "tool_search_tool",
        "mcp_toolset",
    ];
    let Some(tool_type) = tool_type.map(str::trim).filter(|t| !t.is_empty()) else {
        return false;
    };
    if tool_type == "custom" {
        return false;
    }
    SERVER_PREFIXES.iter().any(|prefix| {
        tool_type == *prefix
            || tool_type
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('_'))
    })
}

/// 缩短工具名称以符合 Kiro API 的 64 字符限制
///
/// 对于 MCP 工具（格式：mcp__server__tool），尝试缩短为 mcp__tool
/// 其他工具直接截断到 64 字符
pub fn shorten_tool_name(name: &str) -> String {
    if name.len() <= 64 {
        return name.to_string();
    }

    // MCP 工具：mcp__server__tool -> mcp__tool
    if name.starts_with("mcp__") {
        if let Some(last_idx) = name.rfind("__") {
            if last_idx > 5 {
                let shortened = format!("mcp__{}", &name[last_idx + 2..]);
                if shortened.len() <= 64 {
                    return shortened;
                }
            }
        }
    }

    // 直接截断到 64 字符
    name.chars().take(64).collect()
}

/// 转换 Anthropic 工具定义，返回 (Tool, Option<(sanitized_name, original_name)>)
pub fn convert_anthropic_tool(
    tool: &crate::gateway::models::AnthropicTool,
) -> (Tool, Option<(String, String)>) {
    let sanitized = shorten_tool_name(&sanitize_tool_name(&tool.name));

    // 截断超长描述（和 Kiro-Go 保持一致）
    let description = tool.description.as_ref().map(|desc| {
        if desc.len() > TOOL_DESCRIPTION_MAX_LENGTH {
            // 按字节上限截断，但回退到最近的字符边界，避免在多字节字符（中文/emoji）中间切断导致 panic
            let mut end = TOOL_DESCRIPTION_MAX_LENGTH;
            while end > 0 && !desc.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &desc[..end])
        } else {
            desc.clone()
        }
    });

    let converted_tool = Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: sanitized.clone(),
            description,
            parameters: Some(normalize_json_schema(tool.input_schema.clone())),
        },
        cache_control: tool.cache_control.clone(),
    };

    // 如果工具名被修改，记录映射关系
    let mapping = if sanitized != tool.name {
        Some((sanitized, tool.name.clone()))
    } else {
        None
    };

    (converted_tool, mapping)
}

pub fn extract_tool_uses(message: &NormalizedMessage) -> Option<Vec<KiroToolUse>> {
    let tool_calls = message.tool_calls.as_ref()?;
    let tool_uses: Vec<KiroToolUse> = tool_calls
        .iter()
        .map(|tool_call| KiroToolUse {
            // 与 toolSpecification 保持一致：历史里的 tool_use 名字也需 sanitize，否则 Kiro 拒绝（名字不匹配）
            name: shorten_tool_name(&sanitize_tool_name(&tool_call.function.name)),
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

pub fn normalize_tool_choice(
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

    // 官方取值：
    // - Anthropic: auto | any | tool | none
    // - OpenAI Chat/Responses: auto | none | required | function
    // 内部统一为 auto | none | required | function(name)
    match choice_type {
        "auto" => Ok(Some(json!({ "type": "auto" }))),
        "none" => Ok(Some(json!({ "type": "none" }))),
        // Anthropic any ≡ OpenAI required
        "any" | "required" => {
            if tools.as_ref().is_none_or(|items| items.is_empty()) {
                return Err("tool_choice=required/any 时必须同时提供 tools".to_string());
            }
            Ok(Some(json!({ "type": "required" })))
        }
        // Anthropic tool ≡ OpenAI function
        "function" | "tool" => {
            let name = choice
                .get("name")
                .or_else(|| choice.pointer("/function/name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    "tool_choice 指定具体工具时 name 不能为空（Anthropic: type=tool+name；OpenAI: type=function+function.name）"
                        .to_string()
                })?;

            // 归一化后的 tools 已经过 sanitize（web_search → webSearch），
            // 客户端传的 name 是原始名：两侧都按 sanitize 规则比对，否则
            // "指定的工具不存在" 误报（Claude Code WebSearch 子请求实测踩中）。
            let wanted = sanitize_tool_name(name);
            let tool_exists = tools
                .as_ref()
                .map(|items| {
                    items
                        .iter()
                        .any(|tool| tool.function.name == name || tool.function.name == wanted)
                })
                .unwrap_or(false);
            if !tool_exists {
                return Err(format!("tool_choice 指定的工具不存在: {name}"));
            }

            Ok(Some(json!({
                "type": "function",
                "name": name
            })))
        }
        other => Err(format!(
            "不支持的 tool_choice.type: \"{other}\"，官方取值: auto|none|any|required|tool|function"
        )),
    }
}

pub fn convert_tools(tools: &Option<Vec<Tool>>) -> Option<Vec<KiroTool>> {
    tools.as_ref().map(|items| {
        let mut result = Vec::new();

        // 插入所有工具定义
        for tool in items {
            result.push(KiroTool::ToolSpecification {
                tool_specification: KiroToolSpec {
                    name: tool.function.name.clone(),
                    description: tool_description(tool),
                    input_schema: KiroInputSchema {
                        json: tool_input_schema(tool),
                    },
                },
            });
        }

        // 注意：不要在 tools 数组末尾添加 cachePoint
        // Kiro API 会拒绝这种格式，导致 "Improperly formed request" 错误
        // Prompt Caching 应该通过其他方式触发（如消息的 cache_point metadata）

        result
    })
}

pub fn tool_description(tool: &Tool) -> String {
    tool.function.description.clone().unwrap_or_default()
}

pub fn tool_input_schema(tool: &Tool) -> Value {
    normalize_json_schema(
        tool.function
            .parameters
            .clone()
            .unwrap_or_else(|| json!({})),
    )
}

pub fn normalize_json_schema(value: Value) -> Value {
    let mut schema = match value {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    // 清理 schema（删除 required: null 和空数组，递归清理嵌套结构）
    clean_schema(&mut schema);

    // 确保顶层有 type: "object"
    if !schema.contains_key("type") {
        schema.insert("type".to_string(), Value::String("object".to_string()));
    }

    Value::Object(schema)
}

/// 递归清理 JSON Schema，删除无效的 required 字段
/// Kiro API 会拒绝 required: null 或空的 required: []
pub fn clean_schema(schema: &mut Map<String, Value>) {
    // 修复 required 字段：必须是非空数组或不存在
    if let Some(required) = schema.get("required") {
        let should_remove = match required {
            Value::Null => true,
            Value::Array(arr) if arr.is_empty() => true,
            _ => false,
        };
        if should_remove {
            schema.remove("required");
        }
    }

    // 递归清理 properties
    if let Some(Value::Object(properties)) = schema.get_mut("properties") {
        for value in properties.values_mut() {
            if let Value::Object(sub_schema) = value {
                clean_schema(sub_schema);
            }
        }
    }

    // 递归清理 items
    if let Some(Value::Object(items)) = schema.get_mut("items") {
        clean_schema(items);
    }

    // 递归清理 additionalProperties
    if let Some(Value::Object(additional)) = schema.get_mut("additionalProperties") {
        clean_schema(additional);
    }

    // 递归清理 allOf, oneOf, anyOf
    for key in &["allOf", "oneOf", "anyOf"] {
        if let Some(Value::Array(schemas)) = schema.get_mut(*key) {
            for item in schemas {
                if let Value::Object(sub_schema) = item {
                    clean_schema(sub_schema);
                }
            }
        }
    }
}

pub fn process_tools_with_long_descriptions(
    tools: &Option<Vec<Tool>>,
    max_len: u32,
) -> (Option<Vec<Tool>>, Option<String>) {
    let Some(tools) = tools else {
        return (None, None);
    };

    // 0 = 不限制：描述原样随工具送上游，不外移到系统提示
    if max_len == 0 {
        return (Some(tools.clone()), None);
    }

    let mut processed = Vec::new();
    let mut long_docs = Vec::new();

    for tool in tools {
        let description = tool.function.description.clone().unwrap_or_default();
        if description.len() > max_len as usize {
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
                cache_control: tool.cache_control.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 官方 tool-reference 的 Execution 列：Server 侧必须剥离，Client 侧必须保留。
    #[test]
    fn server_tools_are_detected_and_client_tools_are_not() {
        // Server：Kiro 上游无执行环境，必须剥
        for ty in [
            "web_search_20250305",
            "web_search_20260209",
            "web_search_20260318",
            "web_fetch_20250910",
            "web_fetch_20260318",
            "code_execution_20260120",
            "code_execution_20260521",
            "advisor_20260301",
            "tool_search_tool_regex_20251119",
            "tool_search_tool_bm25",
            "mcp_toolset",
        ] {
            assert!(is_server_executed_tool(Some(ty)), "{ty} should be server");
        }

        // Client：执行在客户端，剥了功能就没了
        for ty in [
            "memory_20250818",
            "bash_20250124",
            "text_editor_20250728",
            "text_editor_20250124",
            "computer_20250124",
            "computer_20251124",
            "computer_toolset_20260801",
            "browser_toolset_20260801",
            "custom",
        ] {
            assert!(!is_server_executed_tool(Some(ty)), "{ty} should be client");
        }

        // 空 / 缺省 / 未知：保留（剥错的代价更大）
        assert!(!is_server_executed_tool(None));
        assert!(!is_server_executed_tool(Some("")));
        assert!(!is_server_executed_tool(Some("some_future_tool_20990101")));
    }

    /// 前缀判定不能跨词误伤：`web_searches` 不是 web_search 的版本。
    #[test]
    fn server_prefix_requires_underscore_boundary() {
        assert!(!is_server_executed_tool(Some("web_searches")));
        assert!(is_server_executed_tool(Some("web_search")));
    }

    /// 官方新增的工具属性必须能反序列化，不能因为多一个字段就让整个请求 400。
    #[test]
    fn new_tool_properties_are_parsed() {
        let tool: crate::gateway::models::AnthropicTool = serde_json::from_value(serde_json::json!({
            "type": "computer_toolset_20260801",
            "configs": { "screenshot": { "enabled": true } },
            "allowed_callers": ["direct"]
        }))
        .expect("toolset entry without name must parse");
        assert!(tool.name.is_empty(), "toolset 条目官方规定无 name");
        assert_eq!(tool.allowed_callers.as_deref(), Some(&["direct".to_string()][..]));
        assert!(tool.configs.is_some());

        // 普通工具不受影响
        let plain: crate::gateway::models::AnthropicTool = serde_json::from_value(
            serde_json::json!({"name": "get_weather", "input_schema": {"type": "object"}}),
        )
        .expect("plain tool must parse");
        assert_eq!(plain.name, "get_weather");
        assert!(plain.allowed_callers.is_none());
    }
}
