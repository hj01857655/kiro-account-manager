//! 请求归一化：把 Anthropic Messages / OpenAI Chat / OpenAI Responses
//! 三种下游协议统一转换为内部 `NormalizedRequest`。
//!
//! 按协议拆分为子模块；本文件放三种协议共用的归一化骨架。

use super::*;

mod anthropic;
mod openai_chat;
mod responses;

pub use anthropic::*;
pub use openai_chat::*;
pub use responses::*;

pub fn create_tool_results_message(tool_results: &[(String, String)]) -> NormalizedMessage {
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

pub fn build_normalized_request_from_payload(
    payload: &Value,
    model: String,
    mut messages: Vec<NormalizedMessage>,
    tools: Option<Vec<Tool>>,
    tool_name_map: std::collections::HashMap<String, String>,
) -> NormalizedRequest {
    // 处理 response_format：注入 system prompt 让模型按格式返回
    if let Some(response_format) = payload.get("response_format").and_then(|v| v.as_object()) {
        let format_type = response_format.get("type").and_then(Value::as_str);
        let instruction = match format_type {
            Some("json_object") => {
                Some("You must respond with a valid JSON object. Do not include any explanatory text outside the JSON.".to_string())
            }
            Some("json_schema") => {
                let name = response_format
                    .get("json_schema")
                    .and_then(|s| s.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("output");
                let description = response_format
                    .get("json_schema")
                    .and_then(|s| s.get("description"))
                    .and_then(Value::as_str)
                    .filter(|d| !d.is_empty());
                match description {
                    Some(desc) => Some(format!(
                        "You must respond with a valid JSON object conforming to the \"{}\" schema. Description: {}. Do not include any explanatory text outside the JSON.",
                        name, desc
                    )),
                    None => Some(format!(
                        "You must respond with a valid JSON object conforming to the \"{}\" schema. Do not include any explanatory text outside the JSON.",
                        name
                    )),
                }
            }
            _ => None,
        };
        if let Some(instr) = instruction {
            if let Some(first) = messages.first_mut() {
                if first.role == "system" {
                    if let Some(Value::String(text)) = &mut first.content {
                        text.push_str("\n\n");
                        text.push_str(&instr);
                    } else {
                        first.content = Some(Value::String(instr));
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
            } else {
                messages.push(NormalizedMessage {
                    role: "system".to_string(),
                    content: Some(Value::String(instr)),
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: None,
                });
            }
        }
    }

    // 处理 parallel_tool_calls=false：注入 system prompt 让模型一次只调用一个工具
    if payload.get("parallel_tool_calls").and_then(Value::as_bool) == Some(false) {
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

    // 处理 reasoning.effort → Kiro thinking 映射（OpenAI Responses API）
    let thinking = payload.get("reasoning").and_then(|r| r.get("effort")).and_then(Value::as_str).map(|effort| {
        let budget_tokens = match effort {
            "low" => 1024,
            "medium" => 4096,
            "high" => 16384,
            _ => 4096,
        };
        log::info!("[模型映射] reasoning.effort=\"{}\" → thinking enabled, budget_tokens={}", effort, budget_tokens);
        Thinking {
            thinking_type: "enabled".to_string(),
            budget_tokens,
        }
    });

    NormalizedRequest {
        model,
        messages,
        stream: payload
            .get("stream")
            .and_then(Value::as_bool)
            .unwrap_or(true), // 默认使用流式响应
        max_tokens: payload
            .get("max_output_tokens")
            .or_else(|| payload.get("max_completion_tokens"))
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
        thinking,
        include_usage: payload
            .get("stream_options")
            .and_then(|opts| opts.get("include_usage"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        tool_name_map,
        // Responses 协议的服务端工具项（web_search_call 等）是消息内容而非工具定义
        server_tool_names: Vec::new(),
    }
}

/// 检测模型名是否包含 "thinking" 后缀，若包含则覆写 thinking 配置
///
/// 根据 Anthropic 官方文档 (https://platform.claude.com/docs/en/docs/about-claude/models):
///
/// **Adaptive Thinking** (type: "adaptive"):
/// - Claude Opus 4.7
/// - Claude Sonnet 4.6
///
/// **Extended Thinking** (type: "enabled"):
/// - Claude Haiku 4.5
/// - Claude Sonnet 4.5
/// - Claude Opus 4.5
///
/// budget_tokens 固定为 20000
pub fn override_thinking_from_model_name(request: &mut NormalizedRequest) {
    let model_lower = request.model.to_lowercase();
    if !model_lower.contains("thinking") {
        return;
    }

    // 判断是否支持 Adaptive Thinking
    let supports_adaptive =
        // Claude Opus 4.7
        (model_lower.contains("opus") && (model_lower.contains("4-7") || model_lower.contains("4.7")))
        ||
        // Claude Sonnet 4.6
        (model_lower.contains("sonnet") && (model_lower.contains("4-6") || model_lower.contains("4.6")));

    let thinking_type = if supports_adaptive {
        "adaptive"
    } else {
        "enabled"
    };

    log::info!(
        "[Gateway] 模型名 {} 包含 thinking 后缀，覆写 thinking 配置为 {}",
        request.model,
        thinking_type
    );

    use crate::gateway::models::Thinking;
    request.thinking = Some(Thinking {
        thinking_type: thinking_type.to_string(),
        budget_tokens: 20000,
    });
}
