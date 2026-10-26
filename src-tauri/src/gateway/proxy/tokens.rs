//! token 估算与 payload 裁剪：请求 token 预估、按 token 裁剪历史、payload 体积裁剪。

use super::*;

pub fn build_count_tokens_response(payload: &Value) -> Value {
    json!({ "input_tokens": estimate_payload_tokens(payload).max(1) })
}

pub fn build_openai_tokens_response(payload: &Value) -> Value {
    json!({
        "object": "response.input_tokens",
        "input_tokens": estimate_payload_tokens(payload).max(1)
    })
}

pub fn estimate_payload_tokens(payload: &Value) -> usize {
    let model_id = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tokenizer_type = TokenizerType::from_model_id(model_id);
    let serialized = serde_json::to_string(payload).unwrap_or_else(|_| payload.to_string());
    let raw_tokens = estimate_text_tokens(&serialized, tokenizer_type);
    ((raw_tokens as f64) * COUNT_TOKENS_SAFETY_MULTIPLIER).ceil() as usize
}

pub fn get_payload_size(payload: &Value) -> usize {
    serde_json::to_string(payload).map(|s| s.len()).unwrap_or(0)
}

/// Token 估算器类型（根据模型选择不同的估算方法）
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum TokenizerType {
    Claude,  // Anthropic Claude 模型
    OpenAI,  // OpenAI GPT 模型（使用 tiktoken）
    Llama,   // Meta Llama 模型
    Generic, // 通用估算（未知模型）
}

impl TokenizerType {
    /// 根据模型 ID 判断使用哪种估算方法
    pub(super) fn from_model_id(model_id: &str) -> Self {
        let model_lower = model_id.to_lowercase();

        // Claude 系列：4.5, 4.6, 4.7 ,4.8及所有变体
        if model_lower.contains("claude") {
            TokenizerType::Claude
        } else if model_lower.contains("gpt")
            || model_lower.contains("o1")
            || model_lower.contains("o3")
        {
            TokenizerType::OpenAI
        } else if model_lower.contains("llama") {
            TokenizerType::Llama
        } else {
            TokenizerType::Generic
        }
    }
}

/// 估算请求消息的 token 数量（支持多种模型）
///
/// 参考 Kiro IDE 源码：extension.js 行 310847-310873
/// - Claude: length / 4 + newlines * 0.5 + code_blocks * 2
/// - OpenAI: 使用 Generic 方法（tiktoken 需要额外依赖）
/// - Llama: length / 3.5
/// - Generic: length / 4 + newlines * 0.5 + code_blocks * 2
///
/// 注意：这是粗略估算，用于提前拒绝明显超长的请求
/// - Kiro API 的 max_input_tokens 是 200k
/// - Kiro IDE 在 80% (160k tokens) 时触发自动总结
/// - 网关在 160k tokens 时直接拒绝（无法实现 AI 总结）
#[allow(dead_code)]
pub fn estimate_request_tokens(messages: &[NormalizedMessage], model_id: &str) -> usize {
    let tokenizer_type = TokenizerType::from_model_id(model_id);

    messages
        .iter()
        .map(|msg| {
            let mut tokens = 0;

            // 估算 content 字段的 token 数
            if let Some(content) = &msg.content {
                let text = extract_plain_text(Some(content));
                tokens += estimate_text_tokens(&text, tokenizer_type);
            }

            // 估算 tool_calls 的 token 数
            if let Some(tool_calls) = &msg.tool_calls {
                for tool_call in tool_calls {
                    tokens += estimate_text_tokens(&tool_call.function.name, tokenizer_type);
                    tokens += estimate_text_tokens(&tool_call.function.arguments, tokenizer_type);
                }
            }
            tokens
        })
        .sum()
}

/// 智能裁剪消息列表到目标 token 数
///
/// 策略：
/// 1. 保留最后一条用户消息（当前请求）
/// 2. 保留系统消息（system）
/// 3. 从最旧的消息开始删除，直到满足目标 token 数
/// 4. 至少保留 2 条消息（system + 最后一条用户消息）
///
/// 返回：是否成功裁剪
#[allow(dead_code)]
pub fn trim_messages_by_tokens(
    messages: &mut Vec<NormalizedMessage>,
    target_tokens: usize,
    model_id: &str,
) -> bool {
    let current_tokens = estimate_request_tokens(messages, model_id);
    if current_tokens <= target_tokens {
        return true;
    }

    log::info!(
        "[网关] 开始裁剪消息：当前 {} tokens，目标 {} tokens",
        current_tokens,
        target_tokens
    );

    // 分离 system 消息和对话消息
    let mut system_messages: Vec<NormalizedMessage> = Vec::new();
    let mut conversation_messages: Vec<NormalizedMessage> = Vec::new();

    for msg in messages.iter() {
        if msg.role == "system" {
            system_messages.push(msg.clone());
        } else {
            conversation_messages.push(msg.clone());
        }
    }

    if conversation_messages.is_empty() {
        log::warn!("[网关] 没有对话消息可裁剪");
        return false;
    }

    // 确保最后一条是 user 消息（Claude API 要求）
    let last_msg = conversation_messages.last().unwrap();
    if last_msg.role != "user" {
        log::warn!("[网关] 最后一条消息不是 user，无法裁剪");
        return false;
    }

    // 策略1: 从后往前保留尽可能多的消息
    let mut kept_messages = Vec::new();

    // 从后往前遍历
    for msg in conversation_messages.iter().rev() {
        let mut test_messages = system_messages.clone();
        // 注意：kept_messages 是反向的，需要反转后添加
        let mut temp_kept = kept_messages.clone();
        temp_kept.reverse();
        temp_kept.insert(0, msg.clone());
        test_messages.extend(temp_kept);

        let test_tokens = estimate_request_tokens(&test_messages, model_id);
        if test_tokens <= target_tokens {
            kept_messages.push(msg.clone());
        } else {
            // 超过限制，停止添加
            break;
        }
    }

    // 反转回正确的顺序
    kept_messages.reverse();

    // 如果一条都保留不了，尝试截断最后一条 user 消息
    if kept_messages.is_empty() {
        log::warn!("[网关] 无法保留任何消息，尝试截断最后一条 user 消息");
        let mut last_user = conversation_messages.last().unwrap().clone();

        // 尝试截断消息内容
        if let Some(content) = &last_user.content {
            let content_str = match content {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    // 提取文本内容
                    arr.iter()
                        .filter_map(|item| {
                            item.get("text")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                _ => content.to_string(),
            };

            // 逐步减少内容长度，直到满足 token 限制
            let mut truncated_content = content_str;
            let mut ratio = 0.8;

            while ratio > 0.1 {
                let target_len = (truncated_content.len() as f64 * ratio) as usize;
                truncated_content = truncated_content
                    .chars()
                    .take(target_len)
                    .collect::<String>();
                truncated_content.push_str("...[内容已截断]");

                last_user.content = Some(Value::String(truncated_content.clone()));

                let mut test_messages = system_messages.clone();
                test_messages.push(last_user.clone());

                let test_tokens = estimate_request_tokens(&test_messages, model_id);
                if test_tokens <= target_tokens {
                    kept_messages.push(last_user);
                    log::info!("[网关] 成功截断消息内容到 {} 字符", truncated_content.len());
                    break;
                }

                ratio -= 0.1;
            }
        }
    }

    if kept_messages.is_empty() {
        log::error!("[网关] 裁剪失败：无法保留任何消息");
        return false;
    }

    // 重建消息列表
    let mut final_messages = system_messages;
    final_messages.extend(kept_messages);

    let final_tokens = estimate_request_tokens(&final_messages, model_id);
    log::info!(
        "[网关] 裁剪成功：{} → {} 条消息，{} → {} tokens",
        messages.len(),
        final_messages.len(),
        current_tokens,
        final_tokens
    );

    *messages = final_messages;
    true
}

pub fn extract_plain_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        item.get("content")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::Object(map)) => map
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| map.get("content").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

/// 使用 tiktoken 对 OpenAI 模型进行精确 token 计数
/// 使用 OnceLock 缓存编码器，避免重复初始化
pub fn count_openai_tokens(text: &str) -> usize {
    static ENCODING: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();
    let bpe = ENCODING.get_or_init(|| {
        tiktoken_rs::o200k_base().expect("无法初始化 o200k_base 编码器")
    });
    bpe.encode_with_special_tokens(text).len()
}

/// 估算单个文本的 token 数量（支持多种模型）
///
/// 参考 Kiro IDE 源码：extension.js 行 310878-310911
///
/// **Claude 估算**（行 310894-310895）：
/// ```javascript
/// estimateWithClaude(text) {
///   return Math.ceil(text.length / 4);
/// }
/// ```
///
/// **Llama 估算**（行 310888-310889）：
/// ```javascript
/// estimateWithLlama(text) {
///   return Math.ceil(text.length / 3.5);
/// }
/// ```
///
/// **Generic 估算**（行 310906-310911）：
/// ```javascript
/// estimateGeneric(text) {
///   const baseTokens = Math.ceil(text.length / 4);
///   const newlineTokens = Math.ceil(text.split('\n').length * 0.5);
///   const codeBlockTokens = (text.match(/```/g) || []).length * 2;
///   return baseTokens + newlineTokens + codeBlockTokens;
/// }
/// ```
#[allow(dead_code)]
pub fn estimate_text_tokens(text: &str, tokenizer_type: TokenizerType) -> usize {
    if text.is_empty() {
        return 0;
    }

    match tokenizer_type {
        TokenizerType::Claude => {
            // Claude: length / 4
            text.len().div_ceil(4)
        }
        TokenizerType::OpenAI => {
            // OpenAI: 使用 tiktoken-rs 精确计数（o200k_base 编码）
            count_openai_tokens(text)
        }
        TokenizerType::Llama => {
            // Llama: length / 3.5 (向上取整)
            ((text.len() as f64 / 3.5).ceil() as usize).max(1)
        }
        TokenizerType::Generic => estimate_generic_tokens(text),
    }
}

/// 通用 token 估算方法（Kiro IDE 的 estimateGeneric）
///
/// 公式：
/// - base_tokens = ceil(length / 4)
/// - newline_tokens = ceil(lines * 0.5)
/// - code_block_tokens = code_blocks * 2
/// - total = base_tokens + newline_tokens + code_block_tokens
#[allow(dead_code)]
pub fn estimate_generic_tokens(text: &str) -> usize {
    // 基础估算：4 字符 = 1 token（向上取整）
    let base_tokens = text.len().div_ceil(4);

    // 换行符：每行 +0.5 token（向上取整）
    let lines = text.lines().count();
    let newline_tokens = lines.div_ceil(2);

    // 代码块：每个 ``` +2 tokens
    let code_blocks = text.matches("```").count();
    let code_block_tokens = code_blocks * 2;

    base_tokens + newline_tokens + code_block_tokens
}

/// 获取模型的最大输入 token 数
///
/// 上下文窗口的唯一事实来源是 `prompt_info::context_window_for_model`
/// （Kiro 官方文档 https://kiro.dev/docs/models/）；本函数只是它的
/// 包装，认不出的模型回退到 200k（与旧行为一致）。
#[allow(dead_code)]
pub async fn get_model_max_input_tokens(model_id: &str) -> usize {
    crate::gateway::prompt_info::context_window_for_model(model_id)
        .unwrap_or(200_000) as usize
}

/// 智能裁剪 Kiro payload 历史记录
///
/// 策略：
/// 1. 识别 tool call/result 配对（Assistant with tool_uses + User with tool_results）
/// 2. 从最旧的完整对话单元开始删除
/// 3. 保留最近的对话（至少保留最后 2 条消息）
/// 4. 避免破坏 tool_calls 和 tool_results 的配对关系
pub fn trim_kiro_payload_history(payload: &mut Value, max_bytes: usize) -> bool {
    let original_size = get_payload_size(payload);
    if original_size <= max_bytes {
        return false;
    }

    let original_len = payload
        .pointer("/conversationState/history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    if original_len == 0 {
        return false;
    }

    // 循环删除最旧的消息单元，直到满足大小要求
    let mut removed_count = 0;
    loop {
        // 检查当前大小
        let current_size = get_payload_size(payload);
        if current_size <= max_bytes {
            break;
        }

        // 获取当前历史记录
        let Some(history) = payload
            .pointer_mut("/conversationState/history")
            .and_then(|v| v.as_array_mut())
        else {
            break;
        };

        // 至少保留 2 条消息
        if history.len() <= 2 {
            break;
        }

        // 检查第一条消息是否是 Assistant 消息且包含 tool_uses
        let first_is_assistant_with_tools = history
            .first()
            .and_then(|msg| msg.get("assistant_response_message"))
            .and_then(|msg| msg.get("tool_uses"))
            .and_then(|tools| tools.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

        if first_is_assistant_with_tools && history.len() > 1 {
            // 检查第二条消息是否是 User 消息且包含 tool_results
            let second_has_tool_results = history
                .get(1)
                .and_then(|msg| msg.get("user_input_message"))
                .and_then(|msg| msg.get("user_input_message_context"))
                .and_then(|ctx| ctx.get("tool_results"))
                .and_then(|results| results.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);

            if second_has_tool_results {
                // 这是一个 tool call/result 配对，必须一起删除
                if history.len() > 3 {
                    // 确保删除后还剩至少 2 条消息
                    history.remove(0);
                    history.remove(0); // 删除第二条（现在变成第一条了）
                    removed_count += 2;
                    log::debug!("[网关] 移除工具调用/结果对。剩余: {}", history.len());
                    continue;
                } else {
                    // 删除后会少于 2 条消息，停止裁剪
                    break;
                }
            }
        }

        // 单个消息可以安全删除
        history.remove(0);
        removed_count += 1;
        log::debug!("[网关] 移除单条消息。剩余: {}", history.len());
    }

    let final_len = payload
        .pointer("/conversationState/history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    let trimmed = final_len < original_len;

    if trimmed {
        log::info!(
            "[网关] 历史记录从 {} 条消息裁剪到 {} 条 (移除了 {} 条消息)",
            original_len,
            final_len,
            removed_count
        );
    }

    trimmed
}
