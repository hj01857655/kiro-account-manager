//! 对话历史组装：history 结构补全、相邻消息合并与交替修复。

use super::*;

/// 修复 history 使其符合 Kiro API 的 7 条验证规则
/// 参考 Kiro IDE 源码中的 v10 函数，按顺序执行修复步骤：
/// 1. 确保以 user 开始
/// 2. 过滤空 user 消息
/// 3. 补充缺失的 toolResults
/// 4. 修复交替（插入占位消息）
/// 5. 确保以 user 结束
pub fn sanitize_history(mut items: Vec<HistoryItem>) -> Vec<HistoryItem> {
    if items.is_empty() {
        return items;
    }

    // 步骤 1：确保以 user 开始
    if !matches!(items.first(), Some(HistoryItem::User { .. })) {
        items.insert(
            0,
            HistoryItem::User {
                user_input_message: HistoryUserMessage {
                    content: "Hello".to_string(),
                    model_id: String::new(),
                    origin: "AI_EDITOR".to_string(),
                    images: None,
                    documents: None,
                    user_input_message_context: None,
                },
            },
        );
    }

    // 步骤 2：过滤空 user 消息（保留第一个 user 和有 content/toolResults 的 user）
    let first_user_idx = items
        .iter()
        .position(|item| matches!(item, HistoryItem::User { .. }));
    items = items
        .into_iter()
        .enumerate()
        .filter(|(idx, item)| {
            match item {
                HistoryItem::User { user_input_message } => {
                    // 保留第一个 user
                    if Some(*idx) == first_user_idx {
                        return true;
                    }
                    // 保留有 content 的 user
                    if !user_input_message.content.trim().is_empty() {
                        return true;
                    }
                    // 保留有 toolResults 的 user
                    if let Some(ctx) = &user_input_message.user_input_message_context {
                        if let Some(results) = &ctx.tool_results {
                            if !results.is_empty() {
                                return true;
                            }
                        }
                    }
                    false
                }
                _ => true,
            }
        })
        .map(|(_, item)| item)
        .collect();

    // 步骤 3：补充缺失的 toolResults
    // 如果 assistant 有 toolUses 但下一条 user 没有对应 toolResults，插入错误占位
    let mut patched: Vec<HistoryItem> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        patched.push(item.clone());

        if let HistoryItem::Assistant {
            assistant_response_message,
        } = item
        {
            if let Some(tool_uses) = &assistant_response_message.tool_uses {
                if !tool_uses.is_empty() {
                    // 检查下一条是否是带 toolResults 的 user
                    let next = items.get(idx + 1);
                    let next_has_results = match next {
                        Some(HistoryItem::User { user_input_message }) => user_input_message
                            .user_input_message_context
                            .as_ref()
                            .and_then(|ctx| ctx.tool_results.as_ref())
                            .map(|r| !r.is_empty())
                            .unwrap_or(false),
                        _ => false,
                    };

                    if !next_has_results {
                        // 插入错误占位的 toolResults
                        let error_results: Vec<KiroToolResult> = tool_uses
                            .iter()
                            .map(|tu| KiroToolResult {
                                tool_use_id: tu.tool_use_id.clone(),
                                content: vec![KiroToolResultContent::Text {
                                    text: "Tool execution failed".to_string(),
                                }],
                                status: "error".to_string(),
                            })
                            .collect();

                        patched.push(HistoryItem::User {
                            user_input_message: HistoryUserMessage {
                                content: String::new(),
                                model_id: String::new(),
                                origin: "AI_EDITOR".to_string(),
                                images: None,
                                documents: None,
                                user_input_message_context: Some(UserInputMessageContext {
                                    additional_context: None,
                                    app_studio_context: None,
                                    console_state: None,
                                    diagnostic: None,
                                    editor_state: None,
                                    env_state: None,
                                    git_state: None,
                                    shell_state: None,
                                    tool_results: Some(error_results),
                                    tools: None,
                                    user_settings: None,
                                }),
                            },
                        });
                    }
                }
            }
        }
    }
    items = patched;

    // 步骤 4：修复交替（两个连续 user 之间插入 assistant，两个连续 assistant 之间插入 user）
    let mut alternated: Vec<HistoryItem> = Vec::new();
    for item in items {
        if let Some(last) = alternated.last() {
            let both_user = matches!(last, HistoryItem::User { .. })
                && matches!(&item, HistoryItem::User { .. });
            let both_assistant = matches!(last, HistoryItem::Assistant { .. })
                && matches!(&item, HistoryItem::Assistant { .. });

            if both_user {
                // 插入占位 assistant
                alternated.push(HistoryItem::Assistant {
                    assistant_response_message: history_assistant_message_from_response_content(
                        "understood",
                        &[],
                    ),
                });
            } else if both_assistant {
                // 插入占位 user
                alternated.push(HistoryItem::User {
                    user_input_message: HistoryUserMessage {
                        content: "Continue".to_string(),
                        model_id: String::new(),
                        origin: "AI_EDITOR".to_string(),
                        images: None,
                        documents: None,
                        user_input_message_context: None,
                    },
                });
            }
        }
        alternated.push(item);
    }
    items = alternated;

    items
}

pub fn merge_adjacent_messages(messages: &[&NormalizedMessage]) -> Vec<NormalizedMessage> {
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

pub fn normalized_message_has_tool_results(message: &NormalizedMessage) -> bool {
    if message.role == "tool" || message.tool_call_id.is_some() {
        return true;
    }

    message
        .content
        .as_ref()
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some("tool_result"))
        })
        .unwrap_or(false)
}

pub fn build_user_context(
    tools: Option<Vec<KiroTool>>,
    tool_results: Vec<KiroToolResult>,
) -> Option<UserInputMessageContext> {
    if tools.is_none() && tool_results.is_empty() {
        return None;
    }

    Some(UserInputMessageContext {
        additional_context: None,
        app_studio_context: None,
        console_state: None,
        diagnostic: None,
        editor_state: None,
        env_state: None,
        git_state: None,
        shell_state: None,
        tool_results: if tool_results.is_empty() {
            None
        } else {
            Some(tool_results)
        },
        tools,
        user_settings: None,
    })
}

pub fn order_tool_results_like_previous_tool_uses(
    tool_results: &mut Vec<KiroToolResult>,
    history: &Option<Vec<HistoryItem>>,
) {
    if tool_results.len() < 2 {
        return;
    }

    let Some(tool_use_ids) = history.as_ref().and_then(|items| {
        items.iter().rev().find_map(|item| match item {
            HistoryItem::Assistant {
                assistant_response_message,
            } => assistant_response_message
                .tool_uses
                .as_ref()
                .filter(|tool_uses| !tool_uses.is_empty())
                .map(|tool_uses| {
                    tool_uses
                        .iter()
                        .map(|tool_use| tool_use.tool_use_id.clone())
                        .collect::<Vec<_>>()
                }),
            _ => None,
        })
    }) else {
        return;
    };

    let mut remaining = std::mem::take(tool_results);
    let mut ordered = Vec::with_capacity(remaining.len());
    for tool_use_id in tool_use_ids {
        if let Some(index) = remaining
            .iter()
            .position(|result| result.tool_use_id == tool_use_id)
        {
            ordered.push(remaining.remove(index));
        }
    }
    ordered.extend(remaining);
    *tool_results = ordered;
}

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

pub fn build_history_assistant_message(message: &NormalizedMessage) -> HistoryAssistantMessage {
    let content = extract_text_content(message.content.as_ref());
    let tool_uses = extract_tool_uses(message);
    // Kiro API 要求 assistant content 非空
    let content = if content.trim().is_empty() {
        if tool_uses.is_some() {
            " ".to_string() // 有 toolUses 时用空格占位
        } else {
            "I understand.".to_string()
        }
    } else {
        content
    };
    HistoryAssistantMessage {
        content,
        tool_uses,
        reasoning_content: assistant_metadata_value(message, "reasoningContent")
            .or_else(|| extract_reasoning_content(message.content.as_ref()))
            .and_then(|value| meaningful_optional_value(Some(value)))
            .map(|mut rc| {
                // 清理空 signature（Kiro API 不接受空字符串的 signature）
                if let Some(rt) = rc.get_mut("reasoningText") {
                    if let Some(sig) = rt.get("signature") {
                        if sig.as_str().map(|s| s.is_empty()).unwrap_or(false) {
                            rt.as_object_mut().map(|m| m.remove("signature"));
                        }
                    }
                }
                rc
            }),
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
