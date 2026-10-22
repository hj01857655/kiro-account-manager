//! Kiro 上游 payload 构建：把 `NormalizedRequest` 组装成 `KiroPayload`。

use super::*;

/// thinking 启用但客户端未给 effort 时的默认档位。
///
/// 社区做法一致：kirocc v0.4.0 回落 medium，OmniRoute 回落 high。
/// 取 high——Claude 多数模型的官方默认档就是 high（Opus 5.5 / Haiku 5.5 默认
/// medium），语义上更接近「不改变客户端可见行为，只保证思考意图送达后端」。
pub const DEFAULT_THINKING_EFFORT_LEVEL: &str = "high";

pub async fn build_kiro_payload(
    client: &Client,
    request: &NormalizedRequest,
    profile_arn: Option<String>,
    available_models: Option<&[Value]>,
) -> Result<KiroPayload, String> {
    // 校验 tool_choice（如果指定了 function，则必须在 tools 列表中存在）
    // 虽然 Kiro 上游请求不包含 tool_choice 字段，但网关层仍需做入参校验，
    // 避免客户端传入无效的工具名却静默成功。
    normalize_tool_choice(&request.tool_choice, &request.tools)?;

    // 裁剪策略：基于 Kiro API 的 7 条 history 验证规则
    // 1. STARTS_WITH_USER_MESSAGE - 必须以 user 开始
    // 2. ENDS_WITH_USER_MESSAGE - 必须以 user 结束
    // 3. ALTERNATING_MESSAGES - user/assistant 严格交替
    // 4. TOOL_USES_AND_RESULTS - assistant 有 toolUses → 下一条 user 必须有 toolResults
    // 5. TOOL_RESULTS_AND_NO_USES - user 有 toolResults → 前一条 assistant 必须有 toolUses
    // 6. TOOL_RESULTS_ORPHAN_IDS - toolResults 的 ID 必须匹配 assistant 的 toolUseId
    // 7. NON_EMPTY_USER_MESSAGE - user 消息必须有 content 或 toolResults
    const MAX_HISTORY_MESSAGES: usize = 30;
    const KEEP_RECENT_MESSAGES: usize = 20;

    let mut request = request.clone();

    // 分离 system 消息和对话消息（user/assistant/tool）
    let mut system_messages: Vec<NormalizedMessage> = Vec::new();
    let mut conversation_messages: Vec<NormalizedMessage> = Vec::new();

    for msg in request.messages.iter() {
        if msg.role == "system" {
            system_messages.push(msg.clone());
        } else {
            conversation_messages.push(msg.clone());
        }
    }

    // 只对对话消息进行裁剪
    if conversation_messages.len() > MAX_HISTORY_MESSAGES {
        log::warn!(
            "[网关] 对话消息数量 {} 超过限制 {}，开始裁剪",
            conversation_messages.len(),
            MAX_HISTORY_MESSAGES
        );

        // 策略：从后往前收集"完整轮次"
        // 一个完整轮次 = user + assistant（可能带 toolUses）+ user（带 toolResults）+ ...
        // 确保不切断 toolUse/toolResult 配对
        let total = conversation_messages.len();
        let mut keep_from_index = total; // 从这个索引开始保留

        // 从最后一条消息往前扫描，收集完整轮次
        let mut kept_count = 0;
        let mut idx = total;

        while idx > 0 && kept_count < KEEP_RECENT_MESSAGES {
            idx -= 1;
            let msg = &conversation_messages[idx];

            // 如果是 user/tool 消息，直接计入
            if msg.role == "user" || msg.role == "tool" {
                kept_count += 1;
                keep_from_index = idx;

                // 检查这个 user 消息是否有 toolResults
                let has_tool_results = msg
                    .content
                    .as_ref()
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter().any(|item| {
                            item.get("type").and_then(|t| t.as_str()) == Some("tool_result")
                        })
                    })
                    .unwrap_or(false)
                    || msg.tool_call_id.is_some();

                // 如果有 toolResults，必须保留前面的 assistant（带 toolUses）
                if has_tool_results && idx > 0 {
                    let prev = &conversation_messages[idx - 1];
                    if prev.role == "assistant" {
                        idx -= 1;
                        kept_count += 1;
                        keep_from_index = idx;
                    }
                }
            } else if msg.role == "assistant" {
                kept_count += 1;
                keep_from_index = idx;

                // 如果 assistant 有 tool_calls，必须保留后面的 user（带 toolResults）
                // 但因为我们是从后往前扫描，后面的已经被保留了，所以只需确保
                // 前面有 user 消息（规则 3：交替）
                // 继续往前找 user
            }
        }

        // 确保裁剪边界不切断工具调用链：
        // - 如果保留片段从 toolResults 开始，必须把前一个 assistant(toolUses) 一起保留；
        // - 如果保留片段从 assistant 开始，优先把它前面的 user 一起保留，而不是向后跳过。
        while keep_from_index > 0 {
            let first = &conversation_messages[keep_from_index];
            if normalized_message_has_tool_results(first) {
                keep_from_index -= 1;
                continue;
            }
            if first.role == "assistant" {
                keep_from_index -= 1;
                continue;
            }
            break;
        }

        // 如果已经无法再向前扩展，才向后寻找一个普通 user 开头；
        // 注意不能从带 toolResults 的 user 开始，否则会变成孤儿 toolResults。
        while keep_from_index < total
            && (conversation_messages[keep_from_index].role != "user"
                || normalized_message_has_tool_results(&conversation_messages[keep_from_index]))
        {
            keep_from_index += 1;
        }

        // 确保最后一条是 user（规则 2）
        let mut end_index = total;
        while end_index > keep_from_index && conversation_messages[end_index - 1].role != "user" {
            end_index -= 1;
        }

        if keep_from_index >= end_index {
            // 极端情况：裁剪后没有有效消息，只保留最后一条 user
            if let Some(last_user_idx) =
                conversation_messages.iter().rposition(|m| m.role == "user")
            {
                conversation_messages = vec![conversation_messages[last_user_idx].clone()];
            } else {
                return Err("No user message found in conversation".into());
            }
        } else {
            conversation_messages = conversation_messages[keep_from_index..end_index].to_vec();
        }

        let history_len_after_trim = conversation_messages.len().saturating_sub(1);
        let first_role_after_trim = conversation_messages
            .first()
            .map(|message| message.role.as_str())
            .unwrap_or("none");
        let last_role_after_trim = conversation_messages
            .last()
            .map(|message| message.role.as_str())
            .unwrap_or("none");
        let current_has_tool_results_after_trim = conversation_messages
            .last()
            .map(normalized_message_has_tool_results)
            .unwrap_or(false);
        let first_has_tool_results_after_trim = conversation_messages
            .first()
            .map(normalized_message_has_tool_results)
            .unwrap_or(false);
        log::info!(
            "[网关] 裁剪完成：{} → {} 条对话消息 | history={} | first={}{} | current={}{}",
            total,
            conversation_messages.len(),
            history_len_after_trim,
            first_role_after_trim,
            if first_has_tool_results_after_trim {
                "(toolResults)"
            } else {
                ""
            },
            last_role_after_trim,
            if current_has_tool_results_after_trim {
                "(toolResults)"
            } else {
                ""
            }
        );
    }

    // 合并回去：system 消息在前，对话消息在后
    request.messages = system_messages;
    request.messages.extend(conversation_messages);

    // 验证最终消息格式
    if request.messages.is_empty() {
        log::error!("[网关] 合并后消息为空");
        return Err("No messages after merging".into());
    }

    log::info!(
        "[网关] 消息格式验证通过：总计 {} 条消息（system: {}, 对话: {}）",
        request.messages.len(),
        request
            .messages
            .iter()
            .filter(|m| m.role == "system")
            .count(),
        request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .count()
    );

    let model_id = if let Some(models) = available_models {
        get_internal_model_id_with_fallback(&request.model, &extract_model_ids(models))?
    } else {
        get_internal_model_id(&request.model)?
    };

    // 如果模型被降级，需要根据降级后的模型重新调整 thinking 配置
    // 避免出现不兼容的组合（例如：sonnet-4.5 + adaptive thinking）
    if request.thinking.is_some() {
        // 客户端显式禁用 thinking 时不做重建——disabled 对任何模型都兼容，
        // 重建会把客户端的「关闭」意图反转成「开启」。
        if request
            .thinking
            .as_ref()
            .map(|thinking| thinking.is_disabled())
            .unwrap_or(false)
        {
            log::info!("[网关] 客户端显式禁用 thinking，模型降级后保持禁用");
        } else {
            let original_model = get_internal_model_id(&request.model).unwrap_or_default();
            if model_id != original_model {
                log::info!(
                    "[网关] 模型从 {} 降级到 {}，重新调整 thinking 配置",
                    original_model,
                    model_id
                );

                // 判断降级后的模型是否支持 Adaptive Thinking
                // （与 normalize/mod.rs 的 override_thinking_from_model_name 同一规则：
                //   官方 4.7+ 仅接受 adaptive；"sonnet-5" 同时覆盖 5 与 5.5）
                let supports_adaptive = model_id.contains("fable")
                    || (model_id.contains("opus")
                        && (model_id.contains("4.7")
                            || model_id.contains("4-7")
                            || model_id.contains("opus-5")))
                    || (model_id.contains("sonnet")
                        && (model_id.contains("4.6")
                            || model_id.contains("4-6")
                            || model_id.contains("sonnet-5")));

                let thinking_type = if supports_adaptive {
                    "adaptive"
                } else {
                    "enabled"
                };

                use crate::gateway::models::Thinking;
                request.thinking = Some(Thinking {
                    thinking_type: thinking_type.to_string(),
                    budget_tokens: 20000,
                });
            }
        }
    }

    let conversation_id = request
        .previous_response_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let agent_continuation_id = conversation_id.clone();
    let (processed_tools, tool_docs) = process_tools_with_long_descriptions(&request.tools);
    let tool_docs_for_current = tool_docs.clone();

    let mut system_prompt = String::new();
    let mut other_messages = Vec::new();

    for message in &request.messages {
        if message.role == "system" {
            let mut text = extract_text_content(message.content.as_ref());
            if !text.is_empty() {
                // 清洗 system prompt：移除 Claude Code 和 Kiro IDE 注入的内容
                text = clean_system_prompt(&text);

                if !text.is_empty() {
                    if !system_prompt.is_empty() {
                        system_prompt.push_str("\n\n");
                    }
                    system_prompt.push_str(&text);
                }
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

    // Thinking 模式：在 system prompt 前注入 thinking 标签
    // Kiro API 通过 system prompt 中的 <thinking_mode> 标签启用思考
    //
    // 客户端显式传 thinking.type="disabled" 时必须真的关掉——官方 schema 里
    // 4.6+ 的 thinking.type 接受 adaptive / disabled，客户端要关就不能又开。
    if let Some(thinking) = &request.thinking {
        if thinking.is_disabled() {
            log::info!("[网关] 客户端显式禁用 thinking，跳过 thinking 标签注入");
        } else {
            // 客户端显式传了 budget_tokens（如 Claude Code）时映射到 max_thinking_length，
            // 让「尊重显式配置」名副其实；clamp 到 [1024, 24576]：
            //   - 下限 1024 是 Anthropic 官方 budget 最小值；
            //   - 上限 24576 是社区实测的 Kiro 标签值上限（kiro.rs：budget_tokens 限制在 24576 以内）。
            // 越界值夹紧而非丢弃——客户端限制思考预算的意图必须被保留。
            let max_thinking_length = thinking.budget_tokens.clamp(1024, 24_576);
            let thinking_prompt = format!(
                "<thinking_mode>enabled</thinking_mode>\n<max_thinking_length>{}</max_thinking_length>",
                max_thinking_length
            );
            system_prompt = if system_prompt.is_empty() {
                thinking_prompt
            } else {
                format!("{}\n\n{}", thinking_prompt, system_prompt)
            };
        }
    }

    // 用边界标记包裹整个系统提示（包括 thinking 标签）
    if !system_prompt.is_empty() {
        system_prompt = format!(
            "--- SYSTEM PROMPT ---\n{}\n--- END SYSTEM PROMPT ---",
            system_prompt
        );
    }

    if other_messages.is_empty() {
        return Err("没有可发送的消息".to_string());
    }

    let merged_messages = merge_adjacent_messages(&other_messages);

    let first_user_index = merged_messages
        .iter()
        .position(|message| matches!(message.role.as_str(), "user" | "tool"));

    let (history, sanitized_current) = if merged_messages.len() > 1 {
        let mut history_items = Vec::new();

        for (index, message) in merged_messages[..merged_messages.len() - 1]
            .iter()
            .enumerate()
        {
            match message.role.as_str() {
                "assistant" => {
                    let assistant_msg = build_history_assistant_message(message);

                    history_items.push(HistoryItem::Assistant {
                        assistant_response_message: assistant_msg,
                    });
                }
                "user" => {
                    let mut content = extract_text_content(message.content.as_ref());

                    // Prompt Caching 策略 1：缓存系统提示
                    // 在第一条用户消息中添加系统提示，并标记缓存点
                    let should_add_cache_point = Some(index) == first_user_index
                        && !system_prompt.is_empty()
                        && processed_tools.is_some(); // 只有在有工具定义时才缓存系统提示

                    if Some(index) == first_user_index && !system_prompt.is_empty() {
                        content = join_with_double_newline(&system_prompt, &content);
                    }

                    let images = extract_images(client, message.content.as_ref()).await;
                    let tool_results = extract_tool_results(message.content.as_ref());
                    let user_context = build_user_context(None, tool_results.clone());
                    let documents = extract_documents(message.content.as_ref());
                    ensure_documents_within_limit(&documents)?;

                    // 规则 7：user 消息必须有 content 或 toolResults
                    if content.trim().is_empty() && tool_results.is_empty() {
                        content = "Continue".to_string();
                    }

                    // 如果需要缓存系统提示，在用户上下文中添加缓存点
                    if should_add_cache_point {
                        if let Some(ref _ctx) = user_context {
                            // 注意：缓存点应该添加在系统提示之后，工具定义之前
                            // 但由于 Kiro API 的限制，我们只能在消息级别添加缓存点
                        }
                    }

                    history_items.push(HistoryItem::User {
                        user_input_message: HistoryUserMessage {
                            content,
                            model_id: model_id.clone(),
                            origin: "AI_EDITOR".to_string(),
                            images: images_option(images),
                            documents: documents_option(documents),
                            user_input_message_context: user_context,
                        },
                    });
                }
                "tool" => {
                    let tool_results = extract_tool_results_from_tool_message(message);
                    // 始终传递工具定义,让 AI 能够调用工具
                    // 只有当工具列表真的为空时,才会返回 None
                    let tools_for_context = convert_tools(&processed_tools);

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
                            documents: None,
                            user_input_message_context: build_user_context(
                                tools_for_context,
                                tool_results,
                            ),
                        },
                    });
                }
                other => {
                    // 归一化后理论上只有 system/user/assistant/tool；此处仅防御日志
                    log::warn!(
                        "[协议映射] history 遇到未处理角色 \"{other}\"，已跳过（应在 normalize 阶段映射）"
                    );
                }
            }
        }

        // 把 currentMessage 也加入 history_items 一起 sanitize（参考项目做法）
        let current_msg = &merged_messages[merged_messages.len() - 1];
        let current_tool_results_for_history = match current_msg.role.as_str() {
            "tool" => extract_tool_results_from_tool_message(current_msg),
            _ => extract_tool_results(current_msg.content.as_ref()),
        };
        let current_content_for_history = extract_text_content(current_msg.content.as_ref());
        let current_documents_for_history = extract_documents(current_msg.content.as_ref());
        ensure_documents_within_limit(&current_documents_for_history)?;
        history_items.push(HistoryItem::User {
            user_input_message: HistoryUserMessage {
                content: if current_content_for_history.trim().is_empty()
                    && current_tool_results_for_history.is_empty()
                {
                    "Continue".to_string()
                } else {
                    current_content_for_history
                },
                model_id: model_id.clone(),
                origin: "AI_EDITOR".to_string(),
                images: None,
                documents: documents_option(current_documents_for_history),
                user_input_message_context: if current_tool_results_for_history.is_empty() {
                    None
                } else {
                    // 如果消息包含 toolResults，必须同时包含 tools 定义（Kiro API 要求）
                    Some(UserInputMessageContext {
                        additional_context: None,
                        app_studio_context: None,
                        console_state: None,
                        diagnostic: None,
                        editor_state: None,
                        env_state: None,
                        git_state: None,
                        shell_state: None,
                        tool_results: Some(current_tool_results_for_history),
                        tools: Some(convert_tools(&processed_tools).unwrap_or_else(|| vec![])),
                        user_settings: None,
                    })
                },
            },
        });

        // sanitize 所有消息（包括 currentMessage）
        let all_sanitized = sanitize_history(history_items);

        // 分割：最后一条作为 currentMessage 的数据源，其余作为 history
        if all_sanitized.len() <= 1 {
            (None, all_sanitized.into_iter().last())
        } else {
            let mut history_part: Vec<HistoryItem> =
                all_sanitized[..all_sanitized.len() - 1].to_vec();
            // 剥掉 history 中签名为空/缺失的 reasoningContent
            // Kiro API 后端会校验 reasoningContent 的 SHA-256 签名：
            //   - opus-4.7 原生 thinking 会产生有效签名 → 保留可让模型记得上一轮思考
            //   - 其他模型靠 <thinking_mode> 提示词强制思考时签名为空 → 必须剥掉，否则 400 THINKING_SIGNATURE_INVALID
            for item in &mut history_part {
                if let HistoryItem::Assistant {
                    assistant_response_message,
                } = item
                {
                    if has_empty_thinking_signature(&assistant_response_message.reasoning_content) {
                        assistant_response_message.reasoning_content = None;
                    }
                }
            }
            let current_part = all_sanitized.into_iter().last();
            (Some(history_part), current_part)
        }
    } else {
        (None, None)
    };

    // 从 sanitized currentMessage item 中提取 content 和 toolResults
    let current_message = merged_messages
        .last()
        .ok_or_else(|| "没有当前消息".to_string())?;

    let mut current_content =
        if let Some(HistoryItem::User { user_input_message }) = &sanitized_current {
            user_input_message.content.clone()
        } else {
            extract_text_content(current_message.content.as_ref())
        };

    if history.is_none() && !system_prompt.is_empty() {
        current_content = join_with_double_newline(&system_prompt, &current_content);
    }
    if let Some(tool_docs) = tool_docs_for_current {
        current_content = join_with_double_newline(&tool_docs, &current_content);
    }
    if current_content.trim().is_empty() {
        current_content = "Continue".to_string();
    }

    // toolResults 从 sanitized item 中获取（如果有的话）
    let mut current_tool_results =
        if let Some(HistoryItem::User { user_input_message }) = &sanitized_current {
            user_input_message
                .user_input_message_context
                .as_ref()
                .and_then(|ctx| ctx.tool_results.clone())
                .unwrap_or_default()
        } else {
            match current_message.role.as_str() {
                "tool" => extract_tool_results_from_tool_message(current_message),
                _ => extract_tool_results(current_message.content.as_ref()),
            }
        };
    order_tool_results_like_previous_tool_uses(&mut current_tool_results, &history);

    // 始终传递工具定义给 currentMessage,让 AI 能够调用工具
    // 只有当工具列表真的为空时,才会返回 None
    let tools_for_current = convert_tools(&processed_tools);

    // 最终保护:如果 content 和 toolResults 都为空,设置默认 content
    if current_content.trim().is_empty() && current_tool_results.is_empty() {
        current_content = "Continue".to_string();
    }
    // 如果有 toolResults，content 必须为空（Kiro API 要求）
    // 同时检查原始消息中是否有 tool_result 内容
    let original_has_tool_results = match current_message.content.as_ref() {
        Some(Value::Array(arr)) => arr
            .iter()
            .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("tool_result")),
        _ => false,
    } || current_message.tool_call_id.is_some();

    // Kiro 的 tool result continuation 要求 content 为空，结果只放在 toolResults。
    if !current_tool_results.is_empty() || original_has_tool_results {
        current_content.clear();
    }
    let current_images = extract_images(client, current_message.content.as_ref()).await;
    let current_documents = if let Some(HistoryItem::User { user_input_message }) = &sanitized_current {
        user_input_message.documents.clone().unwrap_or_default()
    } else {
        extract_documents(current_message.content.as_ref())
    };
    ensure_documents_within_limit(&current_documents)?;

    // 官方 thinking 通道：带顶层 additionalModelRequestFields，上游返回原生
    // reasoningContentEvent（带签名）。
    //
    // 档位与字段位置均由上游 `ListAvailableModels` 的
    // additionalModelRequestFieldsSchema 决定（权威来源，模型换代无需改代码）：
    // - 有 schema → 该模型支持 AMRF；
    // - schema 里 `output_config.effort` → Claude 系，走 output_config（需 thinking 启用）；
    // - schema 里 `reasoning.effort` → GPT-5.6 系，走 reasoning（不需 thinking 启用，
    //   且不能带 Claude 的 output_config/thinking 信封，上游会当未知字段拒收）。
    // 上游没声明（无 schema / 未取到清单）→ 退回官方文档静态表兜底。
    let effort_profile = available_models.map(ModelEffortProfile::from_models);
    let supports_amrf = effort_profile
        .as_ref()
        .map(|profile| profile.supports_amrf(&model_id))
        .unwrap_or_else(|| supports_additional_model_request_fields(&model_id));

    // effort 字段位置：上游声明优先，否则按官方表兜底
    let field = effort_profile
        .as_ref()
        .and_then(|profile| profile.effort_field(&model_id))
        .or_else(|| fallback_effort_field(&model_id))
        .unwrap_or(EffortField::OutputConfig);

    let thinking_enabled = request
        .thinking
        .as_ref()
        .map(|thinking| !thinking.is_disabled())
        .unwrap_or(false);

    let thinking_disabled = request
        .thinking
        .as_ref()
        .map(|thinking| thinking.is_disabled())
        .unwrap_or(false);

    let requested_effort = request
        .effort
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    // thinking 已启用但客户端没给 effort 时补一个默认档位，把「我要思考」的意图
    // 送到后端——否则 adaptive 语义下由模型自行决定是否思考，客户端显式启用
    // thinking 的诉求可能落空。
    // 社区做法一致：kirocc v0.4.0 回落 medium，OmniRoute 回落 high。
    // 这里取 high——Claude 多数模型官方默认档就是 high（Opus 5.5 / Haiku 5.5
    // 默认 medium），语义上更接近「不改客户端行为，只保证意图送达」。
    // 只补 Claude 系：GPT-5.6 的 reasoning.effort 默认本来就是 high，官方明确
    // 「省略 effort ≡ 显式填该模型默认档」，补过去是冗余字段，不如不发。
    // 最终仍走上游枚举 clamp：模型若不允许该档会被夹到最近的有效档。
    let requested_effort = match requested_effort {
        Some(effort) => Some(effort),
        None if thinking_enabled && field == EffortField::OutputConfig => {
            Some(DEFAULT_THINKING_EFFORT_LEVEL.to_string())
        }
        None => None,
    };

    // 档位归一化到该模型允许的枚举（上游声明优先，缺失时官方静态表兜底）
    let resolved_effort = requested_effort.as_deref().and_then(|effort| {
        resolve_effort_level(
            effort_profile.as_ref(),
            &model_id,
            effort,
            thinking_disabled,
        )
    });

    // max_tokens 随 AMRF 下发：客户端设的输出长度上限必须送到上游，否则
    // 上游按自己的默认窗口生成，客户端的输出意图丢失。clamp 到该模型的输出
    // 窗口（上游 tokenLimits.maxOutputTokens），下限 1024（官方各模型的
    // max_tokens 最小值都是 1024）。
    // 仅在上游 schema 明确声明 max_tokens 字段时才下发——上游没声明的字段
    // 硬发可能被当未知字段拒收。
    let max_output_tokens = request.max_tokens.map(|value| {
        let value = value as i64;
        let limit = effort_profile
            .as_ref()
            .and_then(|profile| profile.output_limit(&model_id));
        match limit {
            Some(limit) if limit > 0 => value.clamp(1024, limit),
            _ => value.max(1024),
        }
    });
    let max_output_tokens = max_output_tokens.filter(|_| {
        effort_profile
            .as_ref()
            .map(|profile| profile.declares_max_tokens(&model_id))
            .unwrap_or(false)
    });

    let additional_model_request_fields = match field {
        // GPT-5.6 系：只发 reasoning.effort，不带 Claude 信封；
        // effort 独立于 thinking 生效（官方表：GPT-5.6 默认 effort=high）。
        EffortField::Reasoning => resolved_effort.map(|effort| {
            serde_json::json!({
                "reasoning": { "effort": effort }
            })
        }),
        // Claude 系：output_config.effort 与 thinking 信封各自独立——
        // 官方明确「effort works whether or not thinking is enabled」，
        // 所以只发 effort 不发 thinking 时 effort 仍必须下发；
        // 客户端显式 thinking.type="disabled" 时不下发 thinking 信封。
        EffortField::OutputConfig => {
            if !supports_amrf {
                None
            } else {
                let mut fields = serde_json::Map::new();
                // G-2：官方 `tAu` 只在 `thinking.type.enum` 同时含 disabled+adaptive
                // 时才认为该模型有思考开关；没有开关的模型官方 UI 不给选，我们也不发。
                // 上游清单取不到时 profile 为 None，保持旧行为（照发）。
                let thinking_toggleable = effort_profile
                    .as_ref()
                    .and_then(|profile| profile.thinking_toggleable(&model_id))
                    .unwrap_or(true);
                if thinking_enabled && thinking_toggleable {
                    // 官方原生 AMRF 的 thinking 信封恒为 `{thinking:{type}}`：
                    // 官方扩展 `rAu` 只写 type，不写 display——`display` 是
                    // `/v1/responses` 路径的**输入**字段（`Tui` 读它映射为
                    // `reasoning.summary`），原生链路不消费它。
                    fields.insert(
                        "thinking".to_string(),
                        serde_json::json!({ "type": "adaptive" }),
                    );
                }
                if let Some(effort) = resolved_effort {
                    fields.insert(
                        "output_config".to_string(),
                        serde_json::json!({ "effort": effort }),
                    );
                }
                if let Some(max_tokens) = max_output_tokens {
                    fields.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
                }
                if fields.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(fields))
                }
            }
        }
    };

    // 始终设置 agent_continuation_id 和 agent_task_type
    // 根据抓包验证，Kiro API 在所有情况下都接受这两个字段
    Ok(KiroPayload {
        conversation_state: ConversationState {
            chat_trigger_type: "MANUAL".to_string(),
            conversation_id: conversation_id.clone(),
            agent_continuation_id: Some(agent_continuation_id),
            agent_task_type: Some("vibe".to_string()),
            current_message: CurrentMessage {
                user_input_message: UserInputMessage {
                    content: current_content,
                    model_id,
                    origin: "AI_EDITOR".to_string(),
                    cache_point: None,
                    client_cache_config: None,
                    documents: documents_option(current_documents),
                    images: images_option(current_images),
                    user_input_message_context: build_user_context(
                        tools_for_current,
                        current_tool_results,
                    ),
                    user_intent: None,
                },
            },
            history,
            customization_arn: None,
            workspace_id: None,
        },
        profile_arn,
        additional_model_request_fields,
    })
}
