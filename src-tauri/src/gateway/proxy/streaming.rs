//! 流式转发：SSE 事件生成与下游流式响应处理。

use super::*;


/// stream_proxy_response 的可变翻译状态（原闭包平铺变量收拢）。
/// 纯状态容器：转换逻辑仍按 format 分支处理，字段访问顺序与原变量一致。
#[derive(Default)]
struct StreamState {
    /// 工具调用参数累积器：toolUseId -> (name, arguments-json)
    tool_accumulators: HashMap<String, (String, String)>,
    message_started: bool,
    next_block_index: usize,
    text_block_index: Option<usize>,
    thinking_block_index: Option<usize>,
    tool_block_indexes: HashMap<String, usize>,
    openai_tool_call_indexes: HashMap<String, i32>,
    openai_next_tool_index: i32,
    saw_tool_calls: bool,
    responses_sequence_number: usize,
    responses_next_output_index: usize,
    responses_tool_output_indexes: HashMap<String, usize>,
}

pub fn stream_proxy_response(
    state: RouterState,
    upstream_resp: reqwest::Response,
    format: ResponseFormat,
    model: String,
    request_messages: Vec<NormalizedMessage>,
    request_tools: Option<Vec<Tool>>,
    request_tool_choice: Option<Value>,
    previous_response_id: Option<String>,
    tool_name_map: std::collections::HashMap<String, String>,
    include_usage: bool,
    log_context: RequestLogContext<'static>,
) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(2048);
    tokio::spawn(async move {
        // 辅助函数：还原工具名称（sanitized -> original）
        let restore_tool_name = |sanitized: &str| -> String {
            tool_name_map
                .get(sanitized)
                .cloned()
                .unwrap_or_else(|| sanitized.to_string())
        };
        let mut upstream_stream = upstream_resp.bytes_stream();
        let mut raw_buffer = Vec::new();
        let mut parser = ThinkingParser::new();
        let mut aggregated = stream::AggregatedKiroResponse::default();
        let mut st = StreamState::default();
        let mut input_tokens = 0i32;
        let mut output_tokens = 0i32;
        let anthropic_id = format!("msg_{}", short_uuid());
        let response_id = format!("resp_{}", short_uuid());
        let message_id = format!("msg_{}", short_uuid());
        let created_at = chrono::Utc::now().timestamp();
        let completion_id = format!("chatcmpl-{}", short_uuid());

        if matches!(format, ResponseFormat::Responses) {
            let created = json!({
                "type": "response.created",
                "response": {
                    "id": response_id,
                    "object": "response",
                    "created_at": created_at,
                    "status": "in_progress",
                    "model": model,
                    "output": []
                }
            });
            if !send_data(&tx, &created.to_string()).await {
                return;
            }

            let output_item_added = json!({
                "type": "response.output_item.added",
                "response_id": response_id,
                "output_index": 0,
                "item": {
                    "id": message_id,
                    "type": "message",
                    "status": "in_progress",
                    "role": "assistant",
                    "content": []
                }
            });
            if !send_data(&tx, &output_item_added.to_string()).await {
                return;
            }
        } else if matches!(format, ResponseFormat::OpenAI) {
            let completion_id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
            let created = chrono::Utc::now().timestamp();
            let delta = crate::gateway::models::OpenAIChatDelta {
                role: Some("assistant".to_string()),
                content: Some("".to_string()),
                tool_calls: None,
audio: None,
function_call: None,
            };
            let chunk =
                stream::build_openai_chunk(&completion_id, created, &model, delta, None, None);
            if let Ok(chunk_json) = serde_json::to_string(&chunk) {
                if !send_data(&tx, &chunk_json).await {
                    return;
                }
            }
        }

        const STALLED_STREAM_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(300);

        loop {
            let chunk_result =
                match tokio::time::timeout(STALLED_STREAM_TIMEOUT, upstream_stream.next()).await {
                    Ok(Some(result)) => result,
                    Ok(None) => break,
                    Err(_) => {
                        log::error!("流式响应超时: 5分钟内未收到数据");
                        let data = json!({
                            "type": "error",
                            "message": "流式响应超时: 5分钟内未收到数据"
                        });
                        send_data(&tx, &data.to_string()).await;
                        break;
                    }
                };

            match chunk_result {
                Ok(bytes) => {
                    // 累积二进制数据
                    raw_buffer.extend_from_slice(&bytes);
                    // 逐个解码 EventStream 消息
                    loop {
                        match decode_message(&raw_buffer) {
                            Ok(Some((msg, consumed_bytes))) => {
                                // 成功解码一个消息
                                let message_type =
                                    msg.headers.get(":message-type").map(String::as_str);
                                let event_type = msg.headers.get(":event-type").map(String::as_str);

                                if matches!(message_type, Some("error") | Some("exception")) {
                                    let error_text = String::from_utf8_lossy(&msg.payload);
                                    log::error!(
                                        "EventStream 上游错误: message_type={:?}, event_type={:?}, payload_bytes={}",
                                        message_type,
                                        event_type,
                                        msg.payload.len()
                                    );
                                    let data = json!({
                                        "type": "error",
                                        "message": sanitize_error(error_text.as_ref())
                                    });
                                    send_data(&tx, &data.to_string()).await;
                                    raw_buffer.drain(..consumed_bytes);
                                    break;
                                }

                                if !matches!(message_type, Some("event")) {
                                    raw_buffer.drain(..consumed_bytes);
                                    continue;
                                }

                                // 将 payload 转换为文本
                                let json_text = String::from_utf8_lossy(&msg.payload);

                                // 写入每个 EventStream 事件到文件
                                {
                                    let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
                                    let _ = std::fs::create_dir_all(&log_dir);
                                    let entry = format!(
                                        "[{}] kind=kiro_event idx={} event={} bytes={} chars={} body={}\n",
                                        chrono::Local::now().format("%H:%M:%S%.3f"),
                                        log_context.request_index,
                                        event_type.unwrap_or("unknown"),
                                        json_text.len(),
                                        json_text.chars().count(),
                                        json_text
                                    );
                                    let _ = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(log_dir.join("kiro-response-eventstream.log"))
                                        .and_then(|mut f| {
                                            std::io::Write::write_all(&mut f, entry.as_bytes())
                                        });
                                }

                                // 解析 JSON 事件
                                if let Some(event) = parse_kiro_event_full(&json_text) {
                                    let event_name = match &event {
                                        KiroEvent::Text(_) => "Text",
                                        KiroEvent::Thinking(_) => "Thinking",
                                        KiroEvent::ThinkingSignature(_) => "ThinkingSignature",
                                        KiroEvent::ToolUseStart { .. } => "ToolUseStart",
                                        KiroEvent::ToolUseInputDelta { .. } => "ToolUseInputDelta",
                                        KiroEvent::ToolUseStop { .. } => "ToolUseStop",
                                        KiroEvent::Usage { .. } => "Usage",
                                        KiroEvent::ContextUsage { .. } => "ContextUsage",
                                        KiroEvent::Metering { .. } => "Metering",
                                        KiroEvent::Citation { .. } => "Citation",
                                    };
                                    // 记录每个 Kiro API 事件（trace 级别），只打印元信息
                                    log::trace!(
                                        "[Kiro API 响应事件] event={}, bytes={}, chars={}",
                                        event_name,
                                        msg.payload.len(),
                                        json_text.chars().count()
                                    );
                                    match event {
                                        KiroEvent::Usage {
                                            input_tokens: input,
                                            output_tokens: output,
                                            cache_read_input_tokens,
                                            cache_creation_input_tokens,
                                        } => {
                                            log::info!(
                                                "[Stream] ✅ Received Usage event: input={}, output={}, cache_read={:?}, cache_write={:?}",
                                                input,
                                                output,
                                                cache_read_input_tokens,
                                                cache_creation_input_tokens
                                            );
                                            input_tokens = input;
                                            output_tokens = output;
                                            aggregated.input_tokens = input;
                                            aggregated.output_tokens = output;
                                            aggregated.cache_read_input_tokens =
                                                cache_read_input_tokens;
                                            aggregated.cache_creation_input_tokens =
                                                cache_creation_input_tokens;
                                        }
                                        KiroEvent::ContextUsage { percentage } => {
                                            aggregated.context_usage_percentage = Some(percentage);
                                            if matches!(format, ResponseFormat::Anthropic) {
                                                let data = json!({"type":"context_usage","percentage":percentage});
                                                send_event(
                                                    &tx,
                                                    Some("context_usage"),
                                                    &data.to_string(),
                                                )
                                                .await;
                                            }
                                        }
                                        KiroEvent::Thinking(text) => {
                                            aggregated.thinking.push_str(&text);
                                            handle_stream_text(
                                                &tx,
                                                format,
                                                &model,
                                                &anthropic_id,
                                                &response_id,
                                                &completion_id,
                                                created_at,
                                                &text,
                                                true,
                                                &mut st.message_started,
                                                &mut st.next_block_index,
                                                &mut st.text_block_index,
                                                &mut st.thinking_block_index,
                                                input_tokens,
                                                output_tokens,
                                                aggregated.cache_read_input_tokens,
                                                aggregated.cache_creation_input_tokens,
                                            )
                                            .await;
                                        }
                                        KiroEvent::ThinkingSignature(sig) => {
                                            aggregated.thinking_signature = Some(sig);
                                        }
                                        KiroEvent::Text(text) => {
                                            aggregated.text.push_str(&text);
                                            for segment in parser.push_and_parse(&text) {
                                                handle_stream_text(
                                                    &tx,
                                                    format,
                                                    &model,
                                                    &anthropic_id,
                                                    &response_id,
                                                    &completion_id,
                                                    created_at,
                                                    &segment.content,
                                                    segment.segment_type == SegmentType::Thinking,
                                                    &mut st.message_started,
                                                    &mut st.next_block_index,
                                                    &mut st.text_block_index,
                                                    &mut st.thinking_block_index,
                                                    input_tokens,
                                                    output_tokens,
                                                    aggregated.cache_read_input_tokens,
                                                    aggregated.cache_creation_input_tokens,
                                                )
                                                .await;
                                            }
                                        }
                                        KiroEvent::ToolUseStart { id, name } => {
                                            st.saw_tool_calls = true;
                                            // 还原工具名称
                                            let original_name = restore_tool_name(&name);
                                            // 修复：用还原后的原始工具名发给客户端，否则 Claude Code 收到 sanitized 名会报 "No such tool available"
                                            let name = original_name.clone();
                                            st.tool_accumulators
                                                .entry(id.clone())
                                                .or_insert((original_name.clone(), String::new()));
                                            match format {
                                                ResponseFormat::Anthropic => {
                                                    ensure_anthropic_message_start(
                                                        &tx,
                                                        &mut st.message_started,
                                                        &anthropic_id,
                                                        &model,
                                                        aggregated.input_tokens,
                                                        aggregated.output_tokens,
                                                        aggregated.cache_read_input_tokens,
                                                        aggregated.cache_creation_input_tokens,
                                                    )
                                                    .await;
                                                    close_content_block(&tx, &mut st.text_block_index)
                                                        .await;
                                                    close_content_block(
                                                        &tx,
                                                        &mut st.thinking_block_index,
                                                    )
                                                    .await;
                                                    let index = st.next_block_index;
                                                    st.next_block_index += 1;
                                                    st.tool_block_indexes.insert(id.clone(), index);
                                                    let data = json!({
                                                        "type": "content_block_start",
                                                        "index": index,
                                                        "content_block": {
                                                            "type": "tool_use",
                                                            "id": id,
                                                            "name": name,
                                                            "input": {}
                                                        }
                                                    });
                                                    send_event(
                                                        &tx,
                                                        Some("content_block_start"),
                                                        &data.to_string(),
                                                    )
                                                    .await;
                                                }
                                                ResponseFormat::Responses => {
                                                    let output_index = st.responses_next_output_index;
                                                    st.responses_next_output_index += 1;
                                                    st.responses_tool_output_indexes
                                                        .insert(id.clone(), output_index);
                                                    let data = json!({
                                                        "type": "response.output_item.added",
                                                        "response_id": response_id,
                                                        "output_index": output_index,
                                                        "item": {
                                                            "id": id,
                                                            "type": "function_call",
                                                            "status": "in_progress",
                                                            "call_id": id,
                                                            "name": name,
                                                            "arguments": ""
                                                        }
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                                ResponseFormat::OpenAI => {
                                                    // OpenAI Chat Completions: 发送工具调用开始 chunk
                                                    let tool_index = st.openai_next_tool_index;
                                                    st.openai_next_tool_index += 1;
                                                    st.openai_tool_call_indexes
                                                        .insert(id.clone(), tool_index);

                                                    let chunk = stream::build_openai_chunk(
                                                        &completion_id,
                                                        created_at,
                                                        &model,
                                                        crate::gateway::models::OpenAIChatDelta {
                                                            role: None,
                                                            content: None,
                                                            tool_calls: Some(vec![
                                                                crate::gateway::models::OpenAIDeltaToolCall {
                                                                    index: tool_index,
                                                                    id: id.clone(),
                                                                    call_type: "function".to_string(),
                                                                    function: crate::gateway::models::OpenAIToolCallFunction {
                                                                        name: name.clone(),
                                                                        arguments: "".to_string(),
                                                                    },
                                                                }
                                                            ]),
                                                            audio: None,
                                                            function_call: None,
                                                        },
                                                        None,
                                                        None,
                                                    );
                                                    if let Ok(chunk_json) =
                                                        serde_json::to_string(&chunk)
                                                    {
                                                        send_data(&tx, &chunk_json).await;
                                                    }
                                                }
                                            }
                                        }
                                        KiroEvent::ToolUseInputDelta {
                                            id,
                                            name,
                                            input_delta,
                                        } => {
                                            // 当 input delta 先于 start 到达时（Kiro 流可能乱序），
                                            // 用 delta 中携带的 name 主动发起 start 事件，避免客户端卡死
                                            let mut started_from_delta = false;
                                            if let Some((existing_name, current_input)) =
                                                st.tool_accumulators.get_mut(&id)
                                            {
                                                if existing_name.is_empty() {
                                                    if let Some(n) = name.as_ref() {
                                                        *existing_name = restore_tool_name(n);
                                                    }
                                                }
                                                current_input.push_str(&input_delta);
                                            } else {
                                                let resolved_name = name
                                                    .as_ref()
                                                    .map(|n| restore_tool_name(n))
                                                    .unwrap_or_default();
                                                st.tool_accumulators.insert(
                                                    id.clone(),
                                                    (resolved_name, input_delta.clone()),
                                                );
                                                started_from_delta = true;
                                            }

                                            // 如果是 delta 先到，并且携带了 name，则补发 start 事件
                                            if started_from_delta {
                                                if let Some(raw_name) = name.as_ref() {
                                                    let original_name = restore_tool_name(raw_name);
                                                    st.saw_tool_calls = true;
                                                    match format {
                                                        ResponseFormat::Anthropic => {
                                                            if !st.tool_block_indexes.contains_key(&id)
                                                            {
                                                                ensure_anthropic_message_start(
                                                                    &tx,
                                                                    &mut st.message_started,
                                                                    &anthropic_id,
                                                                    &model,
                                                                    aggregated.input_tokens,
                                                                    aggregated.output_tokens,
                                                                    aggregated.cache_read_input_tokens,
                                                                    aggregated.cache_creation_input_tokens,
                                                                )
                                                                .await;
                                                                close_content_block(
                                                                    &tx,
                                                                    &mut st.text_block_index,
                                                                )
                                                                .await;
                                                                close_content_block(
                                                                    &tx,
                                                                    &mut st.thinking_block_index,
                                                                )
                                                                .await;
                                                                let index = st.next_block_index;
                                                                st.next_block_index += 1;
                                                                st.tool_block_indexes
                                                                    .insert(id.clone(), index);
                                                                let data = json!({
                                                                    "type": "content_block_start",
                                                                    "index": index,
                                                                    "content_block": {
                                                                        "type": "tool_use",
                                                                        "id": id,
                                                                        "name": original_name,
                                                                        "input": {}
                                                                    }
                                                                });
                                                                send_event(
                                                                    &tx,
                                                                    Some("content_block_start"),
                                                                    &data.to_string(),
                                                                )
                                                                .await;
                                                            }
                                                        }
                                                        ResponseFormat::Responses => {
                                                            if !st.responses_tool_output_indexes
                                                                .contains_key(&id)
                                                            {
                                                                let output_index =
                                                                    st.responses_next_output_index;
                                                                st.responses_next_output_index += 1;
                                                                st.responses_tool_output_indexes
                                                                    .insert(
                                                                        id.clone(),
                                                                        output_index,
                                                                    );
                                                                let data = json!({
                                                                    "type": "response.output_item.added",
                                                                    "response_id": response_id,
                                                                    "output_index": output_index,
                                                                    "item": {
                                                                        "id": id,
                                                                        "type": "function_call",
                                                                        "status": "in_progress",
                                                                        "call_id": id,
                                                                        "name": original_name,
                                                                        "arguments": ""
                                                                    }
                                                                });
                                                                send_data(&tx, &data.to_string())
                                                                    .await;
                                                            }
                                                        }
                                                        ResponseFormat::OpenAI => {
                                                            if !st.openai_tool_call_indexes
                                                                .contains_key(&id)
                                                            {
                                                                let tool_index =
                                                                    st.openai_next_tool_index;
                                                                st.openai_next_tool_index += 1;
                                                                st.openai_tool_call_indexes
                                                                    .insert(id.clone(), tool_index);
                                                                let chunk = stream::build_openai_chunk(
                                                                    &completion_id,
                                                                    created_at,
                                                                    &model,
                                                                    crate::gateway::models::OpenAIChatDelta {
                                                                        role: None,
                                                                        content: None,
                                                                        tool_calls: Some(vec![
                                                                            crate::gateway::models::OpenAIDeltaToolCall {
                                                                                index: tool_index,
                                                                                id: id.clone(),
                                                                                call_type: "function".to_string(),
                                                                                function: crate::gateway::models::OpenAIToolCallFunction {
                                                                                    name: original_name.clone(),
                                                                                    arguments: "".to_string(),
                                                                                },
                                                                            }
                                                                        ]),
                                                                        audio: None,
                                                                        function_call: None,
                                                                    },
                                                                    None,
                                                                    None,
                                                                );
                                                                if let Ok(chunk_json) =
                                                                    serde_json::to_string(&chunk)
                                                                {
                                                                    send_data(&tx, &chunk_json)
                                                                        .await;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // 不再立即转发片段，避免客户端收到不完整的 JSON（参考 Kiro-Go）
                                        }
                                        KiroEvent::ToolUseStop { id } => match format {
                                            ResponseFormat::Anthropic => {
                                                // 在 ToolUseStop 时，一次性发送完整的 input（参考 Kiro-Go）
                                                if let Some((name, input)) =
                                                    st.tool_accumulators.remove(&id)
                                                {
                                                    aggregated.tool_calls.push((
                                                        id.clone(),
                                                        name,
                                                        input.clone(),
                                                    ));

                                                    // 发送完整的 input_json_delta
                                                    if let Some(index) =
                                                        st.tool_block_indexes.get(&id).copied()
                                                    {
                                                        if !input.is_empty() {
                                                            let data = json!({
                                                                "type": "content_block_delta",
                                                                "index": index,
                                                                "delta": {
                                                                    "type": "input_json_delta",
                                                                    "partial_json": input
                                                                }
                                                            });
                                                            send_event(
                                                                &tx,
                                                                Some("content_block_delta"),
                                                                &data.to_string(),
                                                            )
                                                            .await;
                                                        }
                                                    }
                                                }
                                                if let Some(index) = st.tool_block_indexes.remove(&id)
                                                {
                                                    let data = json!({
                                                        "type": "content_block_stop",
                                                        "index": index
                                                    });
                                                    send_event(
                                                        &tx,
                                                        Some("content_block_stop"),
                                                        &data.to_string(),
                                                    )
                                                    .await;
                                                }
                                            }
                                            ResponseFormat::Responses => {
                                                if let Some((name, input)) =
                                                    st.tool_accumulators.remove(&id)
                                                {
                                                    aggregated.tool_calls.push((
                                                        id.clone(),
                                                        name.clone(),
                                                        input.clone(),
                                                    ));
                                                    let done = build_stream_responses_function_call_arguments_done_event(
                                                        &response_id,
                                                        &id,
                                                        &input,
                                                    );
                                                    send_data(&tx, &done.to_string()).await;
                                                    let output_index =
                                                        st.responses_tool_output_indexes
                                                            .remove(&id)
                                                            .unwrap_or_else(|| {
                                                                let idx =
                                                                    st.responses_next_output_index;
                                                                st.responses_next_output_index += 1;
                                                                idx
                                                            });
                                                    let data = json!({
                                                        "type": "response.output_item.done",
                                                        "response_id": response_id,
                                                        "output_index": output_index,
                                                        "item": {
                                                            "id": id,
                                                            "type": "function_call",
                                                            "status": "completed",
                                                            "call_id": id,
                                                            "name": name,
                                                            "arguments": input
                                                        }
                                                    });
                                                    send_data(&tx, &data.to_string()).await;
                                                }
                                            }
                                            ResponseFormat::OpenAI => {
                                                if let Some((name, input)) =
                                                    st.tool_accumulators.remove(&id)
                                                {
                                                    aggregated.tool_calls.push((
                                                        id.clone(),
                                                        name.clone(),
                                                        input.clone(),
                                                    ));

                                                    // OpenAI 格式：在 ToolUseStop 时发送完整的 arguments
                                                    if let Some(&tool_index) =
                                                        st.openai_tool_call_indexes.get(&id)
                                                    {
                                                        let chunk = stream::build_openai_chunk(
                                                            &completion_id,
                                                            created_at,
                                                            &model,
                                                            crate::gateway::models::OpenAIChatDelta {
                                                                role: None,
                                                                content: None,
                                                                tool_calls: Some(vec![
                                                                    crate::gateway::models::OpenAIDeltaToolCall {
                                                                        index: tool_index,
                                                                        id: "".to_string(),
                                                                        call_type: "function".to_string(),
                                                                        function: crate::gateway::models::OpenAIToolCallFunction {
                                                                            name: "".to_string(),
                                                                            arguments: input,
                                                                        },
                                                                    }
                                                                ]),
                                                                audio: None,
                                                                function_call: None,
                                                            },
                                                            None,
                                                            None,
                                                        );
                                                        if let Ok(chunk_json) =
                                                            serde_json::to_string(&chunk)
                                                        {
                                                            send_data(&tx, &chunk_json).await;
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        KiroEvent::Citation { text, link, target } => {
                                            let citation =
                                                stream::AggregatedCitation { text, link, target };
                                            aggregated.citations.push(citation.clone());

                                            match format {
                                                ResponseFormat::Anthropic => {
                                                    ensure_anthropic_message_start(
                                                        &tx,
                                                        &mut st.message_started,
                                                        &anthropic_id,
                                                        &model,
                                                        aggregated.input_tokens,
                                                        aggregated.output_tokens,
                                                        aggregated.cache_read_input_tokens,
                                                        aggregated.cache_creation_input_tokens,
                                                    )
                                                    .await;
                                                    close_content_block(
                                                        &tx,
                                                        &mut st.thinking_block_index,
                                                    )
                                                    .await;
                                                    if st.text_block_index.is_none() {
                                                        let index = st.next_block_index;
                                                        st.next_block_index += 1;
                                                        st.text_block_index = Some(index);
                                                        let data = json!({
                                                            "type": "content_block_start",
                                                            "index": index,
                                                            "content_block": {
                                                                "type": "text",
                                                                "text": ""
                                                            }
                                                        });
                                                        send_event(
                                                            &tx,
                                                            Some("content_block_start"),
                                                            &data.to_string(),
                                                        )
                                                        .await;
                                                    }
                                                    if let Some(index) = st.text_block_index {
                                                        if let Some(data) =
                                                            build_anthropic_citation_delta_event(
                                                                index,
                                                                &citation,
                                                                &aggregated.text,
                                                            )
                                                        {
                                                            send_event(
                                                                &tx,
                                                                Some("content_block_delta"),
                                                                &data.to_string(),
                                                            )
                                                            .await;
                                                        }
                                                    }
                                                }
                                                ResponseFormat::Responses => {
                                                    if let Some(annotation) =
                                                        build_responses_citation_annotations(
                                                            std::slice::from_ref(&citation),
                                                        )
                                                        .into_iter()
                                                        .next()
                                                    {
                                                        let data =
                                                            build_responses_annotation_added_event(
                                                                &response_id,
                                                                &message_id,
                                                                annotation,
                                                                aggregated.citations.len() - 1,
                                                                st.responses_sequence_number,
                                                            );
                                                        st.responses_sequence_number += 1;
                                                        send_data(&tx, &data.to_string()).await;
                                                    }
                                                }
                                                ResponseFormat::OpenAI => {
                                                    // OpenAI Chat Completions stream should not emit
                                                    // Responses API events like response.annotation.added.
                                                    // Citations are not part of the Chat Completions API.
                                                }
                                            }
                                        }
                                        KiroEvent::Metering {
                                            unit,
                                            unit_plural,
                                            usage,
                                        } => {
                                            // 记录 metering 信息到聚合响应
                                            aggregated.metering_usage = Some(usage);

                                            // 如果是 Anthropic 格式，发送 metering 事件
                                            if matches!(format, ResponseFormat::Anthropic) {
                                                let data = json!({
                                                    "type": "metering",
                                                    "unit": unit,
                                                    "unitPlural": unit_plural,
                                                    "usage": usage
                                                });
                                                send_event(
                                                    &tx,
                                                    Some("metering"),
                                                    &data.to_string(),
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                } else {
                                    log::trace!(
                                        "[Kiro API 响应事件] event=unparsed, bytes={}, chars={}",
                                        msg.payload.len(),
                                        json_text.chars().count()
                                    );
                                }

                                // 清理已处理的字节
                                raw_buffer.drain(..consumed_bytes);
                            }
                            Ok(None) => {
                                // 缓冲区数据不足，等待更多数据
                                break;
                            }
                            Err(error) => {
                                // 解码失败，记录错误并清空缓冲区
                                log::error!("EventStream 解码失败: {}", error);
                                raw_buffer.clear();
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    log::error!("流式读取错误: {:?}", error);
                    let error_msg = format!("流式读取失败: {error}");
                    log::error!("错误详情: {}", error_msg);
                    let data = json!({"type":"error","message":sanitize_error(&error_msg)});
                    send_data(&tx, &data.to_string()).await;
                    break;
                }
            }
        }

        for segment in parser.flush() {
            handle_stream_text(
                &tx,
                format,
                &model,
                &anthropic_id,
                &response_id,
                &completion_id,
                created_at,
                &segment.content,
                segment.segment_type == SegmentType::Thinking,
                &mut st.message_started,
                &mut st.next_block_index,
                &mut st.text_block_index,
                &mut st.thinking_block_index,
                input_tokens,
                output_tokens,
                aggregated.cache_read_input_tokens,
                aggregated.cache_creation_input_tokens,
            )
            .await;
        }
        // 收集未关闭的工具调用（没有收到 stop 事件的），不要直接 push 到 aggregated.tool_calls
        // 因为 Anthropic 末尾分支需要区分"已正常 stop"和"未 stop"的，避免重复发送事件
        let unstopped_tools: Vec<(String, String, String)> = st.tool_accumulators
            .drain()
            .filter(|(_, (name, input))| !name.is_empty() || !input.is_empty())
            .map(|(id, (name, input))| {
                log::warn!("[流式] 收集未关闭的工具调用: id={}, name={}", id, name);
                (id, name, input)
            })
            .collect();
        for tool in &unstopped_tools {
            aggregated.tool_calls.push(tool.clone());
        }
        aggregated.tool_calls = stream::deduplicate_tool_calls(aggregated.tool_calls);

        // 流式结束后，使用本地估算 token（在发送响应之前）
        let token_source = if aggregated.input_tokens == 0 || aggregated.output_tokens == 0 {
            // 估算输入 tokens（从请求消息中）
            let request_text = serde_json::to_string(&request_messages).unwrap_or_default();
            aggregated.input_tokens =
                crate::gateway::token_estimator::estimate_tokens(&request_text, &model);

            // 估算输出 tokens（从响应文本中）
            let response_text = format!("{}{}", aggregated.text, aggregated.thinking);
            aggregated.output_tokens =
                crate::gateway::token_estimator::estimate_tokens(&response_text, &model);

            log::info!(
                "[流式] 估算的 tokens: input={}, output={} (model={})",
                aggregated.input_tokens,
                aggregated.output_tokens,
                model
            );
            "estimated"
        } else {
            log::info!(
                "[流式] 使用响应中的 token 信息: input={}, output={}",
                aggregated.input_tokens,
                aggregated.output_tokens
            );
            "upstream"
        };

        // Prompt Cache 模拟：如果响应中没有缓存信息，用模拟器填充
        if aggregated.cache_read_input_tokens.is_none()
            && aggregated.cache_creation_input_tokens.is_none()
        {
            let tracker = crate::gateway::prompt_cache::global_prompt_cache_tracker();
            let messages_json: Vec<serde_json::Value> = request_messages
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "role": m.role,
                        "content": m.content
                    })
                })
                .collect();
            let tools_json: Option<Vec<serde_json::Value>> = request_tools.as_ref().map(|tools| {
                tools
                    .iter()
                    .map(|t| serde_json::to_value(t).unwrap_or_default())
                    .collect()
            });

            if let Some(profile) = tracker.build_profile(
                None,
                &messages_json,
                tools_json.as_deref(),
                aggregated.input_tokens as usize,
                &model,
            ) {
                let account_id = model.as_str();
                let cache_usage = tracker.compute(account_id, &profile);
                tracker.update(account_id, &profile);

                if cache_usage.cache_read_input_tokens > 0 {
                    aggregated.cache_read_input_tokens =
                        Some(cache_usage.cache_read_input_tokens as i32);
                }
                if cache_usage.cache_creation_input_tokens > 0 {
                    aggregated.cache_creation_input_tokens =
                        Some(cache_usage.cache_creation_input_tokens as i32);
                }

                log::info!(
                    "[流式] Prompt Cache 模拟: read={}, creation={}",
                    cache_usage.cache_read_input_tokens,
                    cache_usage.cache_creation_input_tokens
                );
            }
        }

        log::info!(
            "[流式响应完成] model={}, text_len={}, thinking_len={}, tool_calls={}, input_tokens={}, output_tokens={}, cache_read_input_tokens={}, cache_creation_input_tokens={}, token_source={}",
            model,
            aggregated.text.len(),
            aggregated.thinking.len(),
            aggregated.tool_calls.len(),
            aggregated.input_tokens,
            aggregated.output_tokens,
            aggregated
                .cache_read_input_tokens
                .map(|item| item.to_string())
                .unwrap_or_else(|| "-".to_string()),
            aggregated
                .cache_creation_input_tokens
                .map(|item| item.to_string())
                .unwrap_or_else(|| "-".to_string()),
            token_source
        );

        match format {
            ResponseFormat::Anthropic => {
                close_content_block(&tx, &mut st.text_block_index).await;
                close_content_block(&tx, &mut st.thinking_block_index).await;

                // 只处理"未收到 stop 事件"的工具调用，避免重复发送已经在流中正常 stop 过的
                for (id, name, input) in &unstopped_tools {
                    // 如果之前已经发过 content_block_start（delta 先到时），直接补 delta+stop
                    let block_index = if let Some(idx) = st.tool_block_indexes.remove(id) {
                        idx
                    } else {
                        let idx = st.next_block_index;
                        st.next_block_index += 1;
                        let start = json!({
                            "type": "content_block_start",
                            "index": idx,
                            "content_block": {
                                "type": "tool_use",
                                "id": id,
                                "name": name,
                                "input": {}
                            }
                        });
                        send_event(&tx, Some("content_block_start"), &start.to_string()).await;
                        idx
                    };
                    let parsed_input: Value =
                        serde_json::from_str(input).unwrap_or_else(|_| json!({}));
                    let delta = json!({
                        "type": "content_block_delta",
                        "index": block_index,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": serde_json::to_string(&parsed_input).unwrap_or_else(|_| "{}".to_string())
                        }
                    });
                    send_event(&tx, Some("content_block_delta"), &delta.to_string()).await;
                    let stop = json!({
                        "type": "content_block_stop",
                        "index": block_index
                    });
                    send_event(&tx, Some("content_block_stop"), &stop.to_string()).await;
                    st.saw_tool_calls = true;
                }

                // 兜底关闭：如果 st.tool_block_indexes 还有遗留（理论上 unstopped_tools 已经覆盖，
                // 但万一有 start 事件发了但既没 stop 也没在 unstopped_tools 里），统一发 stop
                for (_, idx) in st.tool_block_indexes.drain() {
                    let stop = json!({
                        "type": "content_block_stop",
                        "index": idx
                    });
                    send_event(&tx, Some("content_block_stop"), &stop.to_string()).await;
                }

                let mut usage = json!({
                    "input_tokens": aggregated.input_tokens,
                    "output_tokens": aggregated.output_tokens
                });

                // 添加 cache token 信息（如果存在）
                if let Some(cache_read) = aggregated.cache_read_input_tokens {
                    usage["cache_read_input_tokens"] = json!(cache_read);
                }
                if let Some(cache_creation) = aggregated.cache_creation_input_tokens {
                    usage["cache_creation_input_tokens"] = json!(cache_creation);
                }

                let finish = json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": if st.saw_tool_calls { "tool_use" } else { "end_turn" },
                        "stop_sequence": Value::Null
                    },
                    "usage": usage
                });
                send_event(&tx, Some("message_delta"), &finish.to_string()).await;
                send_event(&tx, Some("message_stop"), "{\"type\":\"message_stop\"}").await;
            }
            ResponseFormat::Responses => {
                let output_text = build_responses_output_text(&aggregated);
                if !output_text.text.is_empty() {
                    let text_done = build_stream_responses_output_text_done_event(
                        &response_id,
                        &output_text.text,
                    );
                    send_data(&tx, &text_done.to_string()).await;
                }
                if !aggregated.thinking.is_empty() {
                    let reasoning_done = build_stream_responses_reasoning_done_event(
                        &response_id,
                        &aggregated.thinking,
                    );
                    send_data(&tx, &reasoning_done.to_string()).await;
                }
                let content = build_responses_message_content(&aggregated);
                let output_item_done = json!({
                    "type": "response.output_item.done",
                    "response_id": response_id,
                    "output_index": 0,
                    "item": {
                        "id": message_id,
                        "type": "message",
                        "status": "completed",
                        "role": "assistant",
                        "content": content
                    }
                });
                send_data(&tx, &output_item_done.to_string()).await;

                let completed = build_stream_responses_completed_event(
                    &model,
                    &aggregated,
                    &response_id,
                    &message_id,
                    created_at,
                    previous_response_id.as_deref(),
                );
                send_data(&tx, &completed.to_string()).await;
                persist_responses_session_entry(
                    &state,
                    &response_id,
                    request_messages.clone(),
                    request_tools.clone(),
                    request_tool_choice.clone(),
                    previous_response_id.clone(),
                    &aggregated,
                )
                .await;
                send_data(&tx, "[DONE]").await;
            }
            ResponseFormat::OpenAI => {
                // OpenAI: finish 帧只带 finish_reason；include_usage 时再发空 choices + usage
                let finish_reason = if st.saw_tool_calls { "tool_calls" } else { "stop" };
                let finish_chunk = stream::build_openai_chunk(
                    &completion_id,
                    created_at,
                    &model,
                    crate::gateway::models::OpenAIChatDelta {
                        role: None,
                        content: None,
                        tool_calls: None,
                        audio: None,
                        function_call: None,
                    },
                    Some(finish_reason.to_string()),
                    None,
                );
                let finish_json = serde_json::to_string(&finish_chunk).unwrap_or_default();
                send_data(&tx, &finish_json).await;
                if include_usage {
                    let usage_chunk = stream::build_openai_usage_chunk(
                        &completion_id,
                        created_at,
                        &model,
                        stream::build_openai_chat_usage(
                            aggregated.input_tokens,
                            aggregated.output_tokens,
                        ),
                    );
                    let usage_json = serde_json::to_string(&usage_chunk).unwrap_or_default();
                    send_data(&tx, &usage_json).await;
                }
                send_data(&tx, "[DONE]").await;
            }
        }

        // 记录请求日志（token 已经在发送响应前估算好了）
        let response_body_log = if aggregated.text.is_empty() {
            None
        } else {
            Some(aggregated.text.clone())
        };

        // 写入客户端响应到日志文件
        {
            let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");

            // 构建完整的响应体（根据格式）
            let response_body = match format {
                ResponseFormat::Anthropic => {
                    serde_json::to_string(&build_anthropic_response(&model, &aggregated))
                        .unwrap_or_default()
                }
                ResponseFormat::Responses => {
                    serde_json::to_string(&build_responses_response_with_ids(
                        &model,
                        &aggregated,
                        &response_id,
                        &message_id,
                        created_at,
                        previous_response_id.as_deref(),
                    ))
                    .unwrap_or_default()
                }
                ResponseFormat::OpenAI => {
                    serde_json::to_string(&stream::build_openai_response(&model, &aggregated))
                        .unwrap_or_default()
                }
            };

            let body_end = safe_truncate(&response_body, 50000);
            let entry = format!(
                "[{}] kind=client_response idx={} endpoint={} stream=true status=200 bytes={} truncated={} body={}\n",
                chrono::Local::now().format("%H:%M:%S"),
                log_context.request_index,
                log_context.endpoint,
                response_body.len(),
                body_end < response_body.len(),
                &response_body[..body_end]
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_dir.join(format!(
                    "{}-response.log",
                    get_client_log_prefix_for_endpoint(log_context.endpoint)
                )))
                .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));

            // 也写入 upstream 成功响应
            let upstream_entry = format!(
                "[{}] kind=kiro_response_summary idx={} status=200 text_len={} thinking_len={} tool_calls={:?} input={} output={}\n",
                chrono::Local::now().format("%H:%M:%S"),
                log_context.request_index,
                aggregated.text.len(),
                aggregated.thinking.len(),
                aggregated
                    .tool_calls
                    .iter()
                    .map(|(id, name, args)| format!(
                        "{}({})={}",
                        name,
                        id,
                        &args[..safe_truncate(args, 100)]
                    ))
                    .collect::<Vec<_>>(),
                aggregated.input_tokens,
                aggregated.output_tokens,
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_dir.join("kiro-request.log"))
                .and_then(|mut f| std::io::Write::write_all(&mut f, upstream_entry.as_bytes()));
        }

        write_request_log(
            &log_context,
            StatusCode::OK,
            "stream",
            None,
            None, // error_type
            response_body_log.as_deref(),
            Some(aggregated.input_tokens),
            Some(aggregated.output_tokens),
            aggregated.cache_read_input_tokens,
            aggregated.cache_creation_input_tokens,
            &state,
        );
    }); // tokio::spawn 闭合

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        )
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .header(header::CONNECTION, HeaderValue::from_static("keep-alive"))
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_stream_text(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    format: ResponseFormat,
    model: &str,
    anthropic_id: &str,
    response_id: &str,
    completion_id: &str,
    created: i64,
    text: &str,
    is_thinking: bool,
    message_started: &mut bool,
    next_block_index: &mut usize,
    text_block_index: &mut Option<usize>,
    thinking_block_index: &mut Option<usize>,
    input_tokens: i32,
    output_tokens: i32,
    cache_read_input_tokens: Option<i32>,
    cache_creation_input_tokens: Option<i32>,
) {
    if text.is_empty() {
        return;
    }

    match format {
        ResponseFormat::Anthropic => {
            ensure_anthropic_message_start(
                tx,
                message_started,
                anthropic_id,
                model,
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
            )
            .await;

            if is_thinking {
                close_content_block(tx, text_block_index).await;
                if thinking_block_index.is_none() {
                    let index = *next_block_index;
                    *next_block_index += 1;
                    *thinking_block_index = Some(index);
                    let data = json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": {
                            "type": "thinking",
                            "thinking": ""
                        }
                    });
                    send_event(tx, Some("content_block_start"), &data.to_string()).await;
                }
                let data = json!({
                    "type": "content_block_delta",
                    "index": thinking_block_index.unwrap_or_default(),
                    "delta": {
                        "type": "thinking_delta",
                        "thinking": text
                    }
                });
                send_event(tx, Some("content_block_delta"), &data.to_string()).await;
            } else {
                close_content_block(tx, thinking_block_index).await;
                if text_block_index.is_none() {
                    let index = *next_block_index;
                    *next_block_index += 1;
                    *text_block_index = Some(index);
                    let data = json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": {
                            "type": "text",
                            "text": ""
                        }
                    });
                    send_event(tx, Some("content_block_start"), &data.to_string()).await;
                }
                let data = json!({
                    "type": "content_block_delta",
                    "index": text_block_index.unwrap_or_default(),
                    "delta": {
                        "type": "text_delta",
                        "text": text
                    }
                });
                send_event(tx, Some("content_block_delta"), &data.to_string()).await;
            }
        }
        ResponseFormat::Responses => {
            let data = json!({
                "type": if is_thinking { "response.reasoning.delta" } else { "response.output_text.delta" },
                "response_id": response_id,
                "delta": text
            });
            send_data(tx, &data.to_string()).await;
        }
        ResponseFormat::OpenAI => {
            if is_thinking {
                return;
            }
            let delta = crate::gateway::models::OpenAIChatDelta {
                role: if !*message_started {
                    *message_started = true;
                    Some("assistant".to_string())
                } else {
                    None
                },
                content: Some(text.to_string()),
                tool_calls: None,
audio: None,
function_call: None,
            };
            let chunk = crate::gateway::stream::build_openai_chunk(
                completion_id,
                created,
                model,
                delta,
                None,
                None,
            );
            if let Ok(chunk_json) = serde_json::to_string(&chunk) {
                send_data(tx, &chunk_json).await;
            }
        }
    }
}

pub async fn ensure_anthropic_message_start(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    message_started: &mut bool,
    anthropic_id: &str,
    model: &str,
    input_tokens: i32,
    output_tokens: i32,
    cache_read_input_tokens: Option<i32>,
    cache_creation_input_tokens: Option<i32>,
) {
    if *message_started {
        return;
    }

    let mut usage = json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens
    });

    // 添加 cache token 信息（如果存在）
    if let Some(cache_read) = cache_read_input_tokens {
        usage["cache_read_input_tokens"] = json!(cache_read);
    }
    if let Some(cache_creation) = cache_creation_input_tokens {
        usage["cache_creation_input_tokens"] = json!(cache_creation);
    }

    let data = json!({
        "type": "message_start",
        "message": {
            "id": anthropic_id,
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": model,
            "stop_reason": Value::Null,
            "stop_sequence": Value::Null,
            "usage": usage
        }
    });
    send_event(tx, Some("message_start"), &data.to_string()).await;
    *message_started = true;
}

pub async fn close_content_block(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    index: &mut Option<usize>,
) {
    if let Some(current) = index.take() {
        let data = json!({
            "type": "content_block_stop",
            "index": current
        });
        send_event(tx, Some("content_block_stop"), &data.to_string()).await;
    }
}

pub async fn send_event(
    tx: &mpsc::Sender<Result<Bytes, Infallible>>,
    event: Option<&str>,
    payload: &str,
) -> bool {
    // 写入发给客户端的每个 SSE 事件到文件
    {
        let log_dir = crate::core::paths::app_data_dir_or_default().join("logs");
        let body_end = safe_truncate(payload, 2000);
        let entry = if let Some(event_name) = event {
            format!(
                "[{}] kind=client_sse event={} bytes={} truncated={} data={}\n",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                event_name,
                payload.len(),
                body_end < payload.len(),
                &payload[..body_end]
            )
        } else {
            format!(
                "[{}] kind=client_sse event=data bytes={} truncated={} data={}\n",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                payload.len(),
                body_end < payload.len(),
                &payload[..body_end]
            )
        };
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(get_client_sse_log_file(event, payload)))
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    }

    let chunk = if let Some(event) = event {
        format!("event: {event}\ndata: {payload}\n\n")
    } else {
        format!("data: {payload}\n\n")
    };
    tx.send(Ok(Bytes::from(chunk))).await.is_ok()
}

pub async fn send_data(tx: &mpsc::Sender<Result<Bytes, Infallible>>, payload: &str) -> bool {
    send_event(tx, None, payload).await
}

/// 从请求中提取会话 ID（用于缓存）
pub fn extract_session_id_from_request(request: &NormalizedRequest) -> Option<String> {
    // 尝试从 previous_response_id 提取会话 ID
    if let Some(prev_id) = &request.previous_response_id {
        // 从 response ID 中提取会话部分（假设格式为 "session_xxx_response_yyy"）
        if let Some(session_part) = prev_id.split('_').nth(1) {
            return Some(format!("session_{}", session_part));
        }
        // 如果格式不匹配，直接使用 previous_response_id 作为会话标识
        return Some(prev_id.clone());
    }

    // 如果没有 previous_response_id，使用消息内容的哈希作为会话标识
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for msg in &request.messages {
        msg.role.hash(&mut hasher);
        if let Some(content) = &msg.content {
            content.to_string().hash(&mut hasher);
        }
    }
    Some(format!("session_{:x}", hasher.finish()))
}
