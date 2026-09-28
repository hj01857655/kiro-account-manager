//! 摘要生成、token 估算与缓存键计算。

use crate::clients::kiro_client::build_generate_assistant_response_url;
use crate::gateway::converter::build_kiro_payload;
use crate::gateway::models::NormalizedMessage;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::parse::parse_summary_from_eventstream;

/// 计算消息列表的哈希值
pub(super) fn calculate_messages_hash(messages: &[NormalizedMessage]) -> String {
    let mut hasher = DefaultHasher::new();

    for msg in messages {
        msg.role.hash(&mut hasher);

        // 优化：避免重复的 to_string()
        if let Some(content) = &msg.content {
            match content {
                Value::String(s) => s.hash(&mut hasher),
                Value::Array(arr) => {
                    for item in arr {
                        item.to_string().hash(&mut hasher);
                    }
                }
                other => other.to_string().hash(&mut hasher),
            }
        }

        // 包含 tool_calls 到哈希中（避免遗漏）
        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                tc.function.name.hash(&mut hasher);
                tc.function.arguments.hash(&mut hasher);
            }
        }
    }

    format!("{:x}", hasher.finish())
}

/// 调用 LLM 生成对话摘要（返回摘要和 token 统计）
pub(super) async fn generate_summary_with_tokens(
    http: &reqwest::Client,
    access_token: &str,
    region: &str,
    messages: &[NormalizedMessage],
    model_id: &str,
) -> Result<(String, i32, i32), String> {
    let (summary, api_input_tokens, api_output_tokens) =
        generate_summary(http, access_token, region, messages, model_id).await?;

    // 优先使用 API 返回的 token，fallback 到本地估算
    let input_tokens = if let Some(tokens) = api_input_tokens {
        log::info!("[压缩] 使用 API 返回的输入 token: {}", tokens);
        tokens
    } else {
        let estimated = estimate_tokens_for_messages(messages);
        log::info!(
            "[压缩] API 未返回 token，使用本地估算输入 token: {}",
            estimated
        );
        estimated
    };

    let output_tokens = if let Some(tokens) = api_output_tokens {
        log::info!("[压缩] 使用 API 返回的输出 token: {}", tokens);
        tokens
    } else {
        let estimated = (summary.len() / 4) as i32; // 粗略估算：4 字符 ≈ 1 token
        log::info!(
            "[压缩] API 未返回 token，使用本地估算输出 token: {}",
            estimated
        );
        estimated
    };

    Ok((summary, input_tokens, output_tokens))
}

/// 估算消息的 token 数量
pub(super) fn estimate_tokens_for_messages(messages: &[NormalizedMessage]) -> i32 {
    let total_chars: usize = messages
        .iter()
        .filter_map(|m| m.content.as_ref())
        .map(|c| c.to_string().len())
        .sum();
    (total_chars / 4) as i32 // 粗略估算：4 字符 ≈ 1 token
}

/// 调用 LLM 生成对话摘要（返回摘要和可能的 token 信息）
pub(super) async fn generate_summary(
    http: &reqwest::Client,
    access_token: &str,
    region: &str,
    messages: &[NormalizedMessage],
    model_id: &str,
) -> Result<(String, Option<i32>, Option<i32>), String> {
    // 构建摘要提示词
    let conversation_text = format_messages_for_summary(messages);

    let summary_prompt = format!(
        r#"[系统指令：这是一个自动摘要请求，不是用户消息]

请为以下对话历史生成结构化摘要。要求：

1. 使用第三人称，不要用对话口吻
2. 使用项目符号列表格式
3. 过滤掉寒暄、客套话等无关内容
4. 重点记录：
   - 讨论的主要话题和问题
   - 执行的工具调用及结果
   - 分享的代码或技术信息
   - 已解决的问题和方案

输出格式：

## 对话摘要
* 话题1：关键信息
* 话题2：关键信息

## 工具执行
* 工具X：结果Y

## 代码实现
* 实现1：说明

## 已解决问题
* 问题1：解决方案

---

待摘要的对话内容：

{}
"#,
        conversation_text
    );

    // 构建请求
    let summary_request = vec![NormalizedMessage {
        role: "user".to_string(),
        content: Some(Value::String(summary_prompt)),
        tool_calls: None,
        tool_call_id: None,
        metadata: None,
    }];

    // 调用 LLM
    log::info!("[压缩] 调用 LLM 生成摘要...");

    let payload = build_kiro_payload(
        http,
        &crate::gateway::models::NormalizedRequest {
            model: model_id.to_string(),
            messages: summary_request,
            stream: false,
            max_tokens: Some(2000),
            temperature: Some(0.3),
            top_p: None,
            stop: None,
            tools: None,
            tool_choice: None,
            previous_response_id: None,
            thinking: None,
            include_usage: false,
            tool_name_map: std::collections::HashMap::new(),
            server_tool_names: Vec::new(),
        },
        None,
        None,
    )
    .await
    .map_err(|e| format!("构建摘要请求失败: {}", e))?;

    // 发送请求
    let upstream_url = build_generate_assistant_response_url(region);

    let response = http
        .post(&upstream_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/vnd.amazon.eventstream")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("发送摘要请求失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("摘要请求失败 ({}): {}", status, body));
    }

    // 解析响应
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("读取摘要响应失败: {}", e))?;

    // 解析 EventStream 响应（现在返回 token 信息）
    let (summary, input_tokens, output_tokens) = parse_summary_from_eventstream(&body)?;

    Ok((summary, input_tokens, output_tokens))
}

/// 格式化消息用于摘要
pub(super) fn format_messages_for_summary(messages: &[NormalizedMessage]) -> String {
    let mut result = String::new();

    for (idx, msg) in messages.iter().enumerate() {
        let role = match msg.role.as_str() {
            "user" => "用户",
            "assistant" => "助手",
            "tool" => "工具",
            _ => "系统",
        };

        result.push_str(&format!("\n[消息 {}] {}:\n", idx + 1, role));

        if let Some(content) = &msg.content {
            let text = match content {
                Value::String(s) => s.clone(),
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| {
                        v.get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => content.to_string(),
            };

            // 限制每条消息的长度
            let truncated = if text.len() > 1000 {
                // 回退到最近的字符边界，避免在多字节字符（中文/emoji）中间切断导致 panic
                let mut end = 1000;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...[已截断]", &text[..end])
            } else {
                text
            };

            result.push_str(&truncated);
            result.push('\n');
        }

        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                result.push_str(&format!(
                    "  [工具调用] {}: {}\n",
                    tc.function.name, tc.function.arguments
                ));
            }
        }
    }

    result
}
