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
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
            citations: None,
        });
    }
    if !aggregated.text.is_empty() {
        content.push(AnthropicContentBlock {
            block_type: "text".to_string(),
            text: Some(aggregated.text.clone()),
            thinking: None,
            signature: None,
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
    let stop_reason = if stop_sequence.is_some() {
        "stop_sequence".to_string()
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
    if !aggregated.thinking.is_empty() {
        content.push(json!({
            "type": "reasoning",
            "summary": aggregated.thinking
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

    let output = vec![json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "content": content
    })];

    json!({
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "status": "completed",
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
    })
}

pub fn build_stream_responses_completed_event(
    model: &str,
    aggregated: &stream::AggregatedKiroResponse,
    response_id: &str,
    message_id: &str,
    created_at: i64,
    previous_response_id: Option<&str>,
) -> Value {
    json!({
        "type": "response.completed",
        "response": build_responses_response_with_ids(
            model,
            aggregated,
            response_id,
            message_id,
            created_at,
            previous_response_id,
        )
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
