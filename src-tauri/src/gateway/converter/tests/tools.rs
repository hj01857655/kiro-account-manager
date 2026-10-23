//! 工具定义转换与 tool_choice 归一化的回归测试。

use super::*;

/// 回归测试：工具描述超长且截断点落在多字节字符（中文）中间时不得 panic。
/// 旧实现 `&desc[..TOOL_DESCRIPTION_MAX_LENGTH]` 按字节切，会在
/// `byte index ... is not a char boundary` 处 panic（叠加 panic=abort 直接崩进程）。
#[test]
fn convert_anthropic_tool_truncates_multibyte_description_without_panic() {
    // '中' = 3 字节。3413 个 '中' = 10239 字节 > 10237(TOOL_DESCRIPTION_MAX_LENGTH)，
    // 且字节边界只落在 3 的倍数上，10237 不是边界 → 旧代码会 panic。
    let multibyte_desc = "中".repeat(3413);
    assert!(multibyte_desc.len() > TOOL_DESCRIPTION_MAX_LENGTH);
    assert!(
        !multibyte_desc.is_char_boundary(TOOL_DESCRIPTION_MAX_LENGTH),
        "测试前提：截断点必须落在多字节字符中间，否则测不到该 bug"
    );

    let tool = AnthropicTool {
        r#type: Some("custom".to_string()),
        name: "长描述工具".to_string(),
        description: Some(multibyte_desc),
        input_schema: json!({ "type": "object" }),
        cache_control: None,
        defer_loading: None,
        allowed_callers: None,
        configs: None,
    };

    // 不 panic 即通过；同时校验截断后的描述本身是合法 UTF-8（隐含于 String 类型）
    let (converted, _mapping) = convert_anthropic_tool(&tool);
    let desc = converted.function.description.expect("描述应存在");
    assert!(desc.ends_with("..."), "超长描述应被截断并加省略号");
    // 截断主体（去掉结尾的 "..."）必须落在字符边界，长度不超过字节上限
    let body_len = desc.len() - "...".len();
    assert!(body_len <= TOOL_DESCRIPTION_MAX_LENGTH);
}

#[test]
fn normalize_tool_choice_maps_anthropic_any_and_tool() {
    let tools = Some(vec![Tool {
        tool_type: "function".to_string(),
        function: crate::gateway::models::ToolFunction {
            name: "search_docs".to_string(),
            description: Some("搜索".to_string()),
            parameters: Some(json!({"type": "object"})),
        },
        cache_control: None,
    }]);

    assert_eq!(
        normalize_tool_choice(&Some(json!({ "type": "any" })), &tools).unwrap(),
        Some(json!({ "type": "required" }))
    );
    assert_eq!(
        normalize_tool_choice(
            &Some(json!({ "type": "tool", "name": "search_docs" })),
            &tools
        )
        .unwrap(),
        Some(json!({ "type": "function", "name": "search_docs" }))
    );
    assert_eq!(
        normalize_tool_choice(&Some(json!({ "type": "required" })), &tools).unwrap(),
        Some(json!({ "type": "required" }))
    );
    let err = normalize_tool_choice(&Some(json!({ "type": "mystery" })), &tools)
        .expect_err("unknown type should fail");
    assert!(err.contains("不支持的 tool_choice.type"));
}
