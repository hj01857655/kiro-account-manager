//! 下游响应体构造：Anthropic / OpenAI Responses 的响应与引用（citation）拼装。

use super::*;

pub fn slice_text_by_char_range(text: &str, start: usize, end: usize) -> Option<String> {
    if end < start {
        return None;
    }

    let chars: Vec<char> = text.chars().collect();
    if start > chars.len() || end > chars.len() {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

pub fn infer_citation_text(citation: &stream::AggregatedCitation, message_text: &str) -> String {
    if let Some(text) = citation.text.as_ref() {
        return text.clone();
    }

    citation
        .target
        .get("range")
        .and_then(|range| {
            let start = range.get("start").and_then(Value::as_u64)? as usize;
            let end = range.get("end").and_then(Value::as_u64)? as usize;
            slice_text_by_char_range(message_text, start, end)
        })
        .unwrap_or_default()
}

pub fn extract_anthropic_citation_bounds(
    citation: &stream::AggregatedCitation,
    message_text: &str,
) -> Option<(usize, usize)> {
    if let Some(range) = citation.target.get("range") {
        let start = range.get("start").and_then(Value::as_u64)? as usize;
        let end = range.get("end").and_then(Value::as_u64)? as usize;
        if end < start {
            return None;
        }
        return Some((start, end));
    }

    let start = citation.target.get("location").and_then(Value::as_u64)? as usize;
    let cited_text = infer_citation_text(citation, message_text);
    Some((start, start + cited_text.chars().count()))
}

pub fn build_anthropic_text_citation(
    citation: &stream::AggregatedCitation,
    message_text: &str,
) -> Option<Value> {
    let (start_char_index, end_char_index) =
        extract_anthropic_citation_bounds(citation, message_text)?;
    let cited_text = infer_citation_text(citation, message_text);

    Some(json!({
        "type": "char_location",
        "cited_text": cited_text,
        "document_index": 0,
        "document_title": citation.link,
        "start_char_index": start_char_index,
        "end_char_index": end_char_index,
        "file_id": Value::Null
    }))
}

pub fn build_anthropic_text_citations(
    citations: &[stream::AggregatedCitation],
    message_text: &str,
) -> Option<Value> {
    let mapped: Vec<Value> = citations
        .iter()
        .filter_map(|citation| build_anthropic_text_citation(citation, message_text))
        .collect();

    if mapped.is_empty() {
        None
    } else {
        Some(Value::Array(mapped))
    }
}

pub fn build_anthropic_citation_delta_event(
    index: usize,
    citation: &stream::AggregatedCitation,
    message_text: &str,
) -> Option<Value> {
    Some(json!({
        "type": "content_block_delta",
        "index": index,
        "delta": {
            "type": "citations_delta",
            "citation": build_anthropic_text_citation(citation, message_text)?
        }
    }))
}

pub fn build_anthropic_content_blocks(
    aggregated: &stream::AggregatedKiroResponse,
) -> Vec<AnthropicContentBlock> {
    let mut content = Vec::new();
    if !aggregated.thinking.is_empty() {
        content.push(AnthropicContentBlock {
            block_type: "thinking".to_string(),
            text: None,
            thinking: Some(aggregated.thinking.clone()),
            signature: aggregated.thinking_signature.clone(),
            data: None,
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
            citations: None,
        });
    }
    // 被屏蔽思考单独成块：Anthropic 侧对应 `redacted_thinking{data}`，
    // 官方 Kiro 把 `redactedContent` 当不透明密文透传，客户端多轮原样回传即可
    // 保住 reasoning 连续性（G-7）。
    if let Some(redacted) = aggregated.thinking_redacted.as_ref() {
        if !redacted.is_empty() {
            content.push(AnthropicContentBlock {
                block_type: "redacted_thinking".to_string(),
                text: None,
                thinking: None,
                signature: None,
                data: Some(redacted.clone()),
                id: None,
                name: None,
                input: None,
                tool_use_id: None,
                content: None,
                citations: None,
            });
        }
    }
    if !aggregated.text.is_empty() {
        content.push(AnthropicContentBlock {
            block_type: "text".to_string(),
            text: Some(aggregated.text.clone()),
            thinking: None,
            signature: None,
            data: None,
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
            citations: build_anthropic_text_citations(&aggregated.citations, &aggregated.text),
        });
    }
    for (id, name, arguments) in &aggregated.tool_calls {
        content.push(AnthropicContentBlock {
            block_type: "tool_use".to_string(),
            text: None,
            thinking: None,
            signature: None,
            data: None,
            id: Some(id.clone()),
            name: Some(name.clone()),
            input: Some(serde_json::from_str(arguments).unwrap_or_else(|_| json!({}))),
            tool_use_id: None,
            content: None,
            citations: None,
        });
    }
    content
}

pub fn build_anthropic_response(model: &str, aggregated: &stream::AggregatedKiroResponse) -> Value {
    build_anthropic_response_with_stop(model, aggregated, None)
}

/// 构建 Anthropic 响应；命中 stop 序列时 stop_reason 为 `stop_sequence`
/// 并回填命中的 stop_sequence（Anthropic 协议要求）。
pub fn build_anthropic_response_with_stop(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    stop_sequence: Option<&str>,
) -> Value {
    let content = build_anthropic_content_blocks(aggregated);
    // 优先级：stop_sequence 命中 > 上游 stopReason 映射（G-8）> 消息内容启发式
    let stop_reason = if stop_sequence.is_some() {
        "stop_sequence".to_string()
    } else if let Some(reason) = aggregated.stop_reason.as_deref() {
        let mapped = stream::map_stop_reason_to_anthropic(reason);
        // 上游说 end_turn 但累积到了工具调用（stop 事件缺失等）：tool_use 优先，
        // 否则客户端不会执行已下发的 tool_use 块
        if mapped == "end_turn" && !aggregated.tool_calls.is_empty() {
            "tool_use".to_string()
        } else {
            mapped.to_string()
        }
    } else if aggregated.tool_calls.is_empty() {
        "end_turn".to_string()
    } else {
        "tool_use".to_string()
    };
    serde_json::to_value(AnthropicMessagesResponse {
        id: format!("msg_{}", short_uuid()),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content,
        model: model.to_string(),
        stop_reason: Some(stop_reason),
        stop_sequence: stop_sequence.map(str::to_string),
        usage: AnthropicUsage {
            input_tokens: aggregated.input_tokens,
            output_tokens: aggregated.output_tokens,
            cache_creation_input_tokens: aggregated.cache_creation_input_tokens,
            cache_read_input_tokens: aggregated.cache_read_input_tokens,
        },
    })
    .unwrap_or_else(|_| json!({}))
}

pub fn build_responses_citation_annotations(citations: &[stream::AggregatedCitation]) -> Vec<Value> {
    citations
        .iter()
        .map(|citation| {
            let mut value = json!({
                "type": "url_citation",
                "url": citation.link,
                "target": citation.target,
                "citationLink": citation.link
            });
            if let Some(range) = citation.target.get("range") {
                if let Some(start_index) = range.get("start").and_then(Value::as_u64) {
                    value["start_index"] = Value::from(start_index);
                }
                if let Some(end_index) = range.get("end").and_then(Value::as_u64) {
                    value["end_index"] = Value::from(end_index);
                }
            }
            if let Some(text) = citation.text.as_ref() {
                value["citationText"] = Value::String(text.clone());
            }
            value
        })
        .collect()
}

pub fn build_responses_annotation_added_event(
    response_id: &str,
    message_id: &str,
    annotation: Value,
    annotation_index: usize,
    sequence_number: usize,
) -> Value {
    json!({
        "type": "response.output_text.annotation.added",
        "response_id": response_id,
        "item_id": message_id,
        "output_index": 0,
        "content_index": 0,
        "annotation_index": annotation_index,
        "annotation": annotation,
        "sequence_number": sequence_number
    })
}

pub fn build_responses_output_text(aggregated: &stream::AggregatedKiroResponse) -> ResponsesOutputText {
    let text = aggregated.text.clone();
    let annotations = build_responses_citation_annotations(&aggregated.citations);

    ResponsesOutputText { text, annotations }
}

/// 官方 `C6i`：`encrypted_content` 是 JSON 信封字符串 —— redacted 优先
/// （`{"redactedContent":<b64>}`，b64 原值即上游 `redactedContent`），否则
/// `{"reasoningText":{text,signature?}}`。signature 仅在有值时携带。
pub fn build_responses_reasoning_encrypted_content(
    thinking: &str,
    signature: Option<&str>,
    redacted: Option<&str>,
) -> Option<String> {
    if let Some(redacted) = redacted.filter(|value| !value.is_empty()) {
        return Some(json!({ "redactedContent": redacted }).to_string());
    }
    if thinking.is_empty() {
        return None;
    }
    let mut envelope = json!({ "reasoningText": { "text": thinking } });
    if let Some(signature) = signature.filter(|value| !value.is_empty()) {
        envelope["reasoningText"]["signature"] = json!(signature);
    }
    Some(envelope.to_string())
}

/// 官方 `ifd`：reasoning 是 output[] 独立 item，`summary` 恒为空数组，
/// 思考内容（文本+签名 / 屏蔽密文）全部走 `encrypted_content` 的 C6i JSON
/// 信封，不混进 message.content。
pub fn build_responses_reasoning_item(aggregated: &stream::AggregatedKiroResponse) -> Option<Value> {
    let encrypted = build_responses_reasoning_encrypted_content(
        &aggregated.thinking,
        aggregated.thinking_signature.as_deref(),
        aggregated.thinking_redacted.as_deref(),
    )?;
    Some(json!({
        "id": format!("rs_{}", short_uuid()),
        "type": "reasoning",
        "summary": [],
        "encrypted_content": encrypted
    }))
}

pub fn build_responses_message_content(aggregated: &stream::AggregatedKiroResponse) -> Vec<Value> {
    let output_text = build_responses_output_text(aggregated);
    let mut content = Vec::new();
    if !output_text.text.is_empty() {
        content.push(json!({
            "type": "output_text",
            "text": output_text.text,
            "annotations": output_text.annotations
        }));
    }
    for (id, name, arguments) in &aggregated.tool_calls {
        content.push(json!({
            "type": "function_call",
            "call_id": id,
            "name": name,
            "arguments": arguments
        }));
    }
    content
}

#[allow(dead_code)]
pub fn build_responses_response(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    previous_response_id: Option<&str>,
) -> Value {
    build_responses_response_with_ids(
        model,
        aggregated,
        &format!("resp_{}", short_uuid()),
        &format!("msg_{}", short_uuid()),
        chrono::Utc::now().timestamp(),
        previous_response_id,
    )
}

pub fn build_responses_response_with_ids(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    response_id: &str,
    message_id: &str,
    created_at: i64,
    previous_response_id: Option<&str>,
) -> Value {
    let output_text = build_responses_output_text(aggregated);
    let content = build_responses_message_content(aggregated);

    // 官方 `ifd`：reasoning 是 output[] 独立 item（`summary` 恒空数组、内容走
    // `encrypted_content` 的 C6i JSON 信封），不混进 message.content。
    let mut output = Vec::new();
    if let Some(reasoning_item) = build_responses_reasoning_item(aggregated) {
        output.push(reasoning_item);
    }
    output.push(json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "content": content
    }));

    // G-8b：上游截断/拦截终态必须报 incomplete + incomplete_details，不得一律 completed
    let incomplete_reason = aggregated
        .stop_reason
        .as_deref()
        .and_then(stream::map_stop_reason_to_responses_incomplete);
    let mut response = json!({
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "status": stream::responses_item_status(aggregated),
        "model": model,
        "previous_response_id": previous_response_id,
        "output": output,
        "output_text": output_text.text,
        "usage": {
            "input_tokens": aggregated.input_tokens,
            "output_tokens": aggregated.output_tokens,
            "total_tokens": aggregated.input_tokens + aggregated.output_tokens,
            "cache_creation_input_tokens": aggregated.cache_creation_input_tokens,
            "cache_read_input_tokens": aggregated.cache_read_input_tokens
        }
    });
    if let Some(reason) = incomplete_reason {
        response["incomplete_details"] = json!({ "reason": reason });
    }
    response
}

/// 流式终态事件：按上游 stopReason 在 response.completed / response.incomplete 间选择
///（官方 Dfd 同样区分两种终帧）。
pub fn build_stream_responses_completed_event(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    response_id: &str,
    message_id: &str,
    created_at: i64,
    previous_response_id: Option<&str>,
) -> Value {
    let response = build_responses_response_with_ids(
        model,
        aggregated,
        response_id,
        message_id,
        created_at,
        previous_response_id,
    );
    let event_type = if response.get("status").and_then(Value::as_str) == Some("incomplete") {
        "response.incomplete"
    } else {
        "response.completed"
    };
    json!({
        "type": event_type,
        "response": response
    })
}

pub fn build_stream_responses_function_call_arguments_done_event(
    response_id: &str,
    call_id: &str,
    arguments: &str,
) -> Value {
    json!({
        "type": "response.function_call_arguments.done",
        "response_id": response_id,
        "call_id": call_id,
        "arguments": arguments
    })
}

pub fn build_stream_responses_output_text_done_event(response_id: &str, text: &str) -> Value {
    json!({
        "type": "response.output_text.done",
        "response_id": response_id,
        "text": text
    })
}

pub fn build_stream_responses_reasoning_done_event(response_id: &str, text: &str) -> Value {
    json!({
        "type": "response.reasoning.done",
        "response_id": response_id,
        "text": text
    })
}

/// R3：官方 Anthropic 流式契约在 thinking 块末尾以 `signature_delta` 增量下发签名；
/// 非流式侧由 content block 的 signature 字段承载。
pub fn build_anthropic_signature_delta_event(index: usize, signature: &str) -> Value {
    json!({
        "type": "content_block_delta",
        "index": index,
        "delta": {
            "type": "signature_delta",
            "signature": signature
        }
    })
}
