#[derive(Debug, Clone, PartialEq)]
pub enum KiroEvent {
    Text(String),
    Thinking(String),
    ThinkingSignature(String),
    /// 上游屏蔽掉的思考（`reasoningContentEvent.redactedContent`，base64 字符串）。
    /// 官方 `Zfd` 对 `redactedContent` 原样取串、Uint8Array 才转 base64；
    /// 出站按 Anthropic `redacted_thinking` / Responses `reasoning.encrypted_content` 下发。
    ThinkingRedacted(String),
    ToolUseStart {
        id: String,
        name: String,
    },
    ToolUseInputDelta {
        id: String,
        name: Option<String>,
        input_delta: String,
    },
    ToolUseStop {
        id: String,
    },
    Usage {
        input_tokens: i32,
        output_tokens: i32,
        cache_read_input_tokens: Option<i32>,
        cache_creation_input_tokens: Option<i32>,
        /// 上游 metadataEvent.stopReason 原值（G-8）
        stop_reason: Option<String>,
        /// 上游 metadataEvent.stopDetails.refusal 的解释/分类（G-9）
        refusal: Option<String>,
    },
    ContextUsage {
        percentage: f32,
    },
    Metering {
        unit: String,
        unit_plural: String,
        usage: f64,
    },
    Citation {
        text: Option<String>,
        link: String,
        target: serde_json::Value,
    },
}

fn json_value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedCitation {
    pub text: Option<String>,
    pub link: String,
    pub target: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AggregatedKiroResponse {
    pub text: String,
    pub thinking: String,
    pub thinking_signature: Option<String>,
    /// 上游屏蔽思考（`reasoningContentEvent.redactedContent`，base64）。
    /// 与 `thinking` 互斥：该轮既无正文也无签名时只有它。
    pub thinking_redacted: Option<String>,
    pub tool_calls: Vec<(String, String, String)>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_input_tokens: Option<i32>,
    pub cache_creation_input_tokens: Option<i32>,
    pub context_usage_percentage: Option<f32>,
    pub metering_usage: Option<f64>,
    pub citations: Vec<AggregatedCitation>,
    /// 上游终止语义原值（END_TURN / MAX_TOKENS / TOOL_USE / CONTENT_FILTERED / ...），
    /// None=上游未给出，出站继续用启发式
    pub stop_reason: Option<String>,
    /// 内容拦截时的 refusal 文本（explanation 优先，退到 category）
    pub refusal: Option<String>,
}

pub fn parse_kiro_event_full(json_str: &str) -> Option<KiroEvent> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

    // 添加调试：只记录真正的 token/usage 事件（meteringEvent、contextUsageEvent、metadataEvent）
    let json_lower = json_str.to_lowercase();
    let is_metering_or_usage_event = json_lower.contains("\"meteringevent\"")
        || json_lower.contains("\"contextusageevent\"")
        || json_lower.contains("\"metadataevent\"")
        || (json_lower.contains("\"usage\"") && json_lower.contains("\"inputtokens\""));

    if is_metering_or_usage_event {
        log::debug!(
            "[Token 解析] 发现 token/usage 事件: bytes={}, chars={}, top_keys={:?}",
            json_str.len(),
            json_str.chars().count(),
            value
                .as_object()
                .map(|object| object.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        );
    }

    // 解析 metadataEvent 中的 tokenUsage
    // Kiro IDE 的 token 信息在 metadataEvent.tokenUsage 中
    if let Some(metadata_event) = value.get("metadataEvent").and_then(|item| item.as_object()) {
        log::debug!(
            "[Token 解析] 发现 metadataEvent: {:?}",
            metadata_event.keys().collect::<Vec<_>>()
        );

        // 上游终止语义（G-8/G-8b/G-9）：metadataEvent 除 tokenUsage 外还携 stopReason
        // 与 stopDetails.refusal，旧实现只取了 tokenUsage 导致截断/拦截被当 end_turn
        let stop_reason = metadata_event
            .get("stopReason")
            .and_then(|item| item.as_str())
            .filter(|item| !item.is_empty())
            .map(|item| {
                if !is_kiro_stop_reason(item) {
                    log::warn!("[Token 解析] 未识别的上游 stopReason: {}", item);
                }
                item.to_string()
            });
        let refusal = metadata_event
            .get("stopDetails")
            .and_then(|item| item.get("refusal"))
            .and_then(|item| {
                item.get("explanation")
                    .or_else(|| item.get("category"))
                    .and_then(|value| value.as_str())
                    .filter(|text| !text.is_empty())
                    .map(|text| text.to_string())
            });
        if let Some(token_usage) = metadata_event
            .get("tokenUsage")
            .and_then(|item| item.as_object())
        {
            // Kiro IDE 的字段名：
            // - uncachedInputTokens (未缓存的输入 tokens)
            // - cacheReadInputTokens (缓存读取 tokens)
            // - cacheWriteInputTokens (缓存写入 tokens)
            // - outputTokens (输出 tokens)
            // - totalTokens (总 tokens)
            let uncached_input_tokens = token_usage
                .get("uncachedInputTokens")
                .and_then(|item| item.as_i64())
                .unwrap_or(0) as i32;
            let cache_read_input_tokens = token_usage
                .get("cacheReadInputTokens")
                .and_then(|item| item.as_i64())
                .map(|v| v as i32);
            let cache_creation_input_tokens = token_usage
                .get("cacheWriteInputTokens")
                .and_then(|item| item.as_i64())
                .map(|v| v as i32);
            let output_tokens = token_usage
                .get("outputTokens")
                .and_then(|item| item.as_i64())
                .unwrap_or(0) as i32;

            // input_tokens 仅为未缓存的输入（符合 Anthropic 官方规范）
            // 客户端会单独看到 cache_read_input_tokens 和 cache_creation_input_tokens 字段
            // 三者相加才是真实总输入量。如果这里包含 cache_*，客户端账单会双重计费虚高
            // 参考：chaogei/Kiro-account-manager v1.6.6 修复
            let input_tokens = uncached_input_tokens;

            log::info!(
                "[Token 解析] ✅ 发现 metadataEvent.tokenUsage: keys={:?}, 未缓存输入={}, 缓存读取={:?}, 缓存写入={:?}, 输出={}",
                token_usage.keys().cloned().collect::<Vec<_>>(),
                uncached_input_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                output_tokens
            );
            log::info!(
                "[Token 解析] 已解析: 未缓存输入={}, 缓存读取={:?}, 缓存写入={:?}, 输出={}, 总输入={}",
                uncached_input_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                output_tokens,
                input_tokens
            );

            if input_tokens > 0 || output_tokens > 0 {
                return Some(KiroEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_input_tokens,
                    cache_creation_input_tokens,
                    stop_reason,
                    refusal,
                });
            } else if stop_reason.is_some() || refusal.is_some() {
                // 纯终止帧：tokens 全 0，下游不得据此覆盖已累计的 usage
                return Some(KiroEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_input_tokens,
                    cache_creation_input_tokens,
                    stop_reason,
                    refusal,
                });
            } else {
                log::warn!("[Token 解析] ⚠️ 发现 tokenUsage 但所有 token 都为 0");
            }
        } else if stop_reason.is_some() || refusal.is_some() {
            // 部分上游终帧只带 stopReason/stopDetails，不带 tokenUsage
            return Some(KiroEvent::Usage {
                input_tokens: 0,
                output_tokens: 0,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
                stop_reason,
                refusal,
            });
        } else {
            log::warn!("[Token 解析] ⚠️ 发现 metadataEvent 但没有 tokenUsage 字段");
        }
    }

    // 兼容旧格式：解析顶层的 usage 字段（用于其他格式的响应）
    if let Some(usage) = value.get("usage").and_then(|item| item.as_object()) {
        let input_tokens = usage
            .get("inputTokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|item| item.as_i64())
            .unwrap_or(0) as i32;
        let output_tokens = usage
            .get("outputTokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|item| item.as_i64())
            .unwrap_or(0) as i32;

        let cache_read_input_tokens = usage
            .get("cachedReadTokens")
            .or_else(|| usage.get("cacheReadInputTokens"))
            .or_else(|| usage.get("cache_read_input_tokens"))
            .and_then(|item| item.as_i64())
            .map(|v| v as i32);
        let cache_creation_input_tokens = usage
            .get("cachedWriteTokens")
            .or_else(|| usage.get("cacheCreationInputTokens"))
            .or_else(|| usage.get("cache_creation_input_tokens"))
            .and_then(|item| item.as_i64())
            .map(|v| v as i32);

        log::info!(
            "[Token 解析] 发现旧格式 usage: keys={:?}, input={}, output={}, cache_read={:?}, cache_creation={:?}",
            usage.keys().cloned().collect::<Vec<_>>(),
            input_tokens,
            output_tokens,
            cache_read_input_tokens,
            cache_creation_input_tokens
        );

        if input_tokens > 0
            || output_tokens > 0
            || cache_read_input_tokens.is_some()
            || cache_creation_input_tokens.is_some()
        {
            return Some(KiroEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                stop_reason: None,
                refusal: None,
            });
        }
    }

    // 解析 contextUsageEvent
    if let Some(context_event) = value
        .get("contextUsageEvent")
        .and_then(|item| item.as_object())
    {
        if let Some(percentage) = context_event
            .get("contextUsagePercentage")
            .and_then(|item| item.as_f64())
        {
            return Some(KiroEvent::ContextUsage {
                percentage: percentage as f32,
            });
        }
    }

    if let Some(metering) = value.get("meteringEvent").and_then(|item| item.as_object()) {
        let unit = metering
            .get("unit")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let unit_plural = metering
            .get("unitPlural")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let usage = metering
            .get("usage")
            .and_then(|item| item.as_f64())
            .unwrap_or(0.0);

        if !unit.is_empty() {
            return Some(KiroEvent::Metering {
                unit,
                unit_plural,
                usage,
            });
        }
    }
    if let Some(text) = parse_reasoning_text(&value) {
        if !text.is_empty() {
            return Some(KiroEvent::Thinking(text));
        }
    }
    // 提取 reasoning signature（可能在单独的事件中，也可能和 text 一起）
    if let Some(sig) = parse_reasoning_signature(&value) {
        if !sig.is_empty() {
            return Some(KiroEvent::ThinkingSignature(sig));
        }
    }
    // 被屏蔽的思考：只有它时该轮思考既无正文也无签名（G-7）。
    // 官方 Zfd 要求 text/signature/redacted 三者全空才不产出，因此必须放在
    // 前两者之后单独一档，不能与它们合并返回。
    if let Some(redacted) = parse_reasoning_redacted(&value) {
        if !redacted.is_empty() {
            return Some(KiroEvent::ThinkingRedacted(redacted));
        }
    }

    // 解析 codeReferenceEvent
    if let Some(code_ref) = value.get("codeReferenceEvent") {
        if let Some(citation) = parse_code_reference_event(code_ref) {
            return Some(KiroEvent::Citation {
                text: citation.text,
                link: citation.link,
                target: citation.target,
            });
        }
    }

    // 解析 supplementaryWebLinksEvent
    if let Some(web_links) = value.get("supplementaryWebLinksEvent") {
        if let Some(citation) = parse_supplementary_web_links_event(web_links) {
            return Some(KiroEvent::Citation {
                text: citation.text,
                link: citation.link,
                target: citation.target,
            });
        }
    }

    if let Some(citation) = parse_citation_event(&value) {
        return Some(KiroEvent::Citation {
            text: citation.text,
            link: citation.link,
            target: citation.target,
        });
    }

    if let Some(tool_use_id) = value
        .get("toolUseId")
        .and_then(|item| item.as_str())
        .or_else(|| {
            value
                .get("toolUseEvent")
                .and_then(|e| e.get("toolUseId"))
                .and_then(|item| item.as_str())
        })
    {
        // 支持 toolUseEvent 包装格式
        let tool_data = value.get("toolUseEvent").unwrap_or(&value);

        let name = tool_data
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();

        if tool_data.get("stop").and_then(|item| item.as_bool()) == Some(true) {
            return Some(KiroEvent::ToolUseStop {
                id: tool_use_id.to_string(),
            });
        }

        if let Some(input) = tool_data.get("input") {
            let input_delta = if let Some(text) = input.as_str() {
                log::trace!(
                    "[Tool Use] 收到 input 字符串分片: id={tool_use_id}, byte_len={}, char_len={}",
                    text.len(),
                    text.chars().count()
                );
                text.to_string()
            } else if input.is_object() || input.is_array() {
                let serialized = serde_json::to_string(input).unwrap_or_default();
                log::trace!(
                    "[Tool Use] 收到 input 对象/数组分片: id={tool_use_id}, value_type={}, byte_len={}, char_len={}",
                    json_value_kind(input),
                    serialized.len(),
                    serialized.chars().count()
                );
                serialized
            } else {
                log::warn!(
                    "[Tool Use] input 类型未知: value_type={}",
                    json_value_kind(input)
                );
                String::new()
            };
            if !input_delta.is_empty() {
                log::trace!(
                    "[Tool Use] 解析 ToolUseInputDelta: id={tool_use_id}, byte_len={}",
                    input_delta.len()
                );
                return Some(KiroEvent::ToolUseInputDelta {
                    id: tool_use_id.to_string(),
                    name: if name.is_empty() { None } else { Some(name) },
                    input_delta,
                });
            }
        }

        if !name.is_empty() {
            log::debug!(
                "[Tool Use] 发送 ToolUseStart: id={}, name={}",
                tool_use_id,
                name
            );
            return Some(KiroEvent::ToolUseStart {
                id: tool_use_id.to_string(),
                name,
            });
        }
    }

    if let Some(tool) = value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("toolUses"))
        .and_then(|item| item.as_array())
        .and_then(|items| items.first())
    {
        let id = tool
            .get("toolUseId")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let name = tool
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();

        if !name.is_empty() {
            return Some(KiroEvent::ToolUseStart { id, name });
        }
    }

    parse_text_content(&value).map(KiroEvent::Text)
}

pub fn deduplicate_tool_calls(
    tool_calls: Vec<(String, String, String)>,
) -> Vec<(String, String, String)> {
    use std::collections::HashMap;

    if tool_calls.is_empty() {
        return tool_calls;
    }

    let mut by_id: HashMap<String, (String, String, String)> = HashMap::new();
    let mut order = Vec::new();

    for (id, name, args) in tool_calls {
        if !by_id.contains_key(&id) {
            order.push(id.clone());
        }
        by_id.insert(id.clone(), (id, name, args));
    }

    order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect()
}

/// 从已切分好的 JSON payload 数组直接聚合（每帧直接解析，无需 extract_json 重新切分）
pub fn aggregate_kiro_response_from_payloads(payloads: &[String]) -> AggregatedKiroResponse {
    let mut aggregated = AggregatedKiroResponse::default();
    let mut tool_accumulators: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    let mut found_usage = false;
    let mut json_count = 0;
    let mut event_counts = std::collections::HashMap::new();

    log::info!("[聚合] 开始聚合，共 {} 个 payload", payloads.len());

    for json_str in payloads {
        let json_str = json_str.trim();
        if json_str.is_empty() {
            continue;
        }
        json_count += 1;

        log::debug!(
            "[聚合] JSON #{}: bytes={}, chars={}",
            json_count,
            json_str.len(),
            json_str.chars().count()
        );

        if let Some(event) = parse_kiro_event_full(json_str) {
            let event_type = match &event {
                KiroEvent::Text(_) => "Text",
                KiroEvent::Thinking(_) => "Thinking",
                KiroEvent::ThinkingSignature(_) => "ThinkingSignature",
                KiroEvent::ThinkingRedacted(_) => "ThinkingRedacted",
                KiroEvent::ToolUseStart { .. } => "ToolUseStart",
                KiroEvent::ToolUseInputDelta { .. } => "ToolUseInputDelta",
                KiroEvent::ToolUseStop { .. } => "ToolUseStop",
                KiroEvent::Usage { .. } => "Usage",
                KiroEvent::ContextUsage { .. } => "ContextUsage",
                KiroEvent::Metering { .. } => "Metering",
                KiroEvent::Citation { .. } => "Citation",
            };
            *event_counts.entry(event_type).or_insert(0) += 1;

            match event {
                KiroEvent::Text(text) => {
                    log::debug!("[聚合] 文本事件: {} 字符", text.len());
                    aggregated.text.push_str(&text);
                }
                KiroEvent::Thinking(text) => {
                    log::debug!("[聚合] 思考事件: {} 字符", text.len());
                    aggregated.thinking.push_str(&text);
                }
                KiroEvent::ThinkingSignature(sig) => {
                    log::debug!("[聚合] 思考签名: {} 字符", sig.len());
                    aggregated.thinking_signature = Some(sig);
                }
                KiroEvent::ThinkingRedacted(redacted) => {
                    log::debug!("[聚合] 屏蔽思考: {} 字符", redacted.len());
                    aggregated.thinking_redacted = Some(redacted);
                }
                KiroEvent::ToolUseStart { id, name } => {
                    log::debug!("[聚合] 工具使用开始: id={}, name={}", id, name);
                    let entry = tool_accumulators
                        .entry(id)
                        .or_insert((String::new(), String::new()));
                    if entry.0.is_empty() {
                        entry.0 = name;
                    }
                }
                KiroEvent::ToolUseInputDelta {
                    id,
                    name,
                    input_delta,
                } => {
                    log::debug!(
                        "[聚合] 工具输入增量: id={}, delta_len={}",
                        id,
                        input_delta.len()
                    );
                    if let Some((existing_name, current_input)) = tool_accumulators.get_mut(&id) {
                        if existing_name.is_empty() {
                            if let Some(name) = name {
                                *existing_name = name;
                            }
                        }
                        if input_delta.trim_start().starts_with('{')
                            && current_input.trim_start().starts_with('{')
                        {
                            log::debug!("[聚合] 检测到完整 JSON input，替换而不是追加");
                            *current_input = input_delta;
                        } else {
                            current_input.push_str(&input_delta);
                        }
                    } else {
                        tool_accumulators.insert(id, (name.unwrap_or_default(), input_delta));
                    }
                }
                KiroEvent::ToolUseStop { id } => {
                    log::debug!("[聚合] 工具使用结束: id={}", id);
                    if let Some((name, input)) = tool_accumulators.remove(&id) {
                        log::debug!(
                            "[聚合] 完整的工具调用: id={}, name={}, input_len={}",
                            id,
                            name,
                            input.len()
                        );
                        aggregated.tool_calls.push((id, name, input));
                    } else {
                        log::warn!("[聚合] ⚠️  工具使用结束但未找到对应的累加器: id={}", id);
                    }
                }
                KiroEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_input_tokens,
                    cache_creation_input_tokens,
                    stop_reason,
                    refusal,
                } => {
                    // 纯终止帧（四个 token 字段全空）：只更新 stop_reason/refusal，
                    // 不得覆盖已累计的 usage；旧格式帧可能只带 cache tokens，仍要收下
                    if input_tokens > 0
                        || output_tokens > 0
                        || cache_read_input_tokens.is_some()
                        || cache_creation_input_tokens.is_some()
                    {
                        found_usage = true;
                        aggregated.input_tokens = input_tokens;
                        aggregated.output_tokens = output_tokens;
                        aggregated.cache_read_input_tokens = cache_read_input_tokens;
                        aggregated.cache_creation_input_tokens = cache_creation_input_tokens;
                    }
                    if stop_reason.is_some() {
                        aggregated.stop_reason = stop_reason;
                    }
                    if refusal.is_some() {
                        aggregated.refusal = refusal;
                    }
                    log::info!(
                        "[聚合] ✅ 发现 usage 信息: input={}, output={}, cache_read={:?}, cache_creation={:?}",
                        input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens
                    );
                }
                KiroEvent::ContextUsage { percentage } => {
                    log::debug!("[聚合] 上下文使用: {}%", percentage);
                    aggregated.context_usage_percentage = Some(percentage);
                }
                KiroEvent::Metering { usage, .. } => {
                    log::debug!("[聚合] 计量: {}", usage);
                    aggregated.metering_usage = Some(usage);
                }
                KiroEvent::Citation { text, link, target } => {
                    log::debug!("[聚合] 引用: link={}", link);
                    aggregated
                        .citations
                        .push(AggregatedCitation { text, link, target });
                }
            }
        } else {
            log::warn!(
                "[聚合] 解析事件失败: json_index={}, bytes={}, chars={}",
                json_count,
                json_str.len(),
                json_str.chars().count()
            );
        }
    }

    // 收集未关闭的 tool_accumulators
    for (id, (name, input)) in tool_accumulators {
        if !name.is_empty() || !input.is_empty() {
            log::warn!("[聚合] ⚠️ 未关闭的工具调用: id={}, name={}", id, name);
            aggregated.tool_calls.push((id, name, input));
        }
    }

    aggregated.tool_calls = deduplicate_tool_calls(aggregated.tool_calls);

    log::info!("[聚合] 完成: 处理了 {} 个 JSON 对象", json_count);
    log::info!("[聚合] 事件计数: {:?}", event_counts);
    log::info!(
        "[聚合] 结果: 文本={} 字符, 思考={} 字符, 工具调用={}, 引用={}",
        aggregated.text.len(),
        aggregated.thinking.len(),
        aggregated.tool_calls.len(),
        aggregated.citations.len()
    );

    if !found_usage {
        log::warn!("[聚合] ⚠️ Kiro 响应中未找到 usage 信息!");
    }

    aggregated
}

fn parse_text_content(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.get("content").and_then(|item| item.as_str()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("delta")
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("contentBlockDelta")
        .and_then(|item| item.get("delta"))
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("content"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    None
}

fn parse_reasoning_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value
        .get("reasoningContentEvent")
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("delta")
        .and_then(|item| item.get("thinking"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("contentBlockDelta")
        .and_then(|item| item.get("delta"))
        .and_then(|item| item.get("thinking"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    None
}

/// 从 reasoningContentEvent 中提取 signature
fn parse_reasoning_signature(value: &serde_json::Value) -> Option<String> {
    value
        .get("reasoningContentEvent")
        .and_then(|item| item.get("signature"))
        .and_then(|item| item.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 从 reasoningContentEvent 中提取被屏蔽的思考（base64 字符串）
fn parse_reasoning_redacted(value: &serde_json::Value) -> Option<String> {
    value
        .get("reasoningContentEvent")
        .and_then(|item| item.get("redactedContent"))
        .and_then(|item| item.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 解析 codeReferenceEvent
fn parse_code_reference_event(value: &serde_json::Value) -> Option<AggregatedCitation> {
    let references = value.get("references")?.as_array()?;
    let first_ref = references.first()?;

    let repository = first_ref
        .get("repository")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let license_name = first_ref
        .get("licenseName")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 构造 link（使用 repository 作为链接）
    let link = if !repository.is_empty() {
        repository.to_string()
    } else {
        return None;
    };

    // 构造 text（显示许可证信息）
    let text = if !license_name.is_empty() {
        Some(format!("Code reference ({})", license_name))
    } else {
        Some("Code reference".to_string())
    };

    // 构造 target（保留原始的 recommendationContentSpan）
    let target = first_ref
        .get("recommendationContentSpan")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    Some(AggregatedCitation { text, link, target })
}

/// 解析 supplementaryWebLinksEvent
fn parse_supplementary_web_links_event(value: &serde_json::Value) -> Option<AggregatedCitation> {
    let links = value.get("supplementaryWebLinks")?.as_array()?;
    let first_link = links.first()?;

    let url = first_link
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())?
        .to_string();

    let title = first_link
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let snippet = first_link
        .get("snippet")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // 使用 title 或 snippet 作为显示文本
    let text = title.or(snippet);

    // 构造 target（保留原始的 url 和 snippet）
    let target = serde_json::json!({
        "url": url,
        "snippet": first_link.get("snippet").cloned().unwrap_or(serde_json::Value::Null)
    });

    Some(AggregatedCitation {
        text,
        link: url,
        target,
    })
}

fn parse_citation_event(value: &serde_json::Value) -> Option<AggregatedCitation> {
    let target = value.get("target")?.clone();
    let link = value
        .get("citationLink")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())?
        .to_string();
    let text = value
        .get("citationText")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string);
    ensure_citation_target_supported(&target)?;

    Some(AggregatedCitation { text, link, target })
}

fn ensure_citation_target_supported(target: &serde_json::Value) -> Option<()> {
    if let Some(range) = target.get("range") {
        let start_index = range.get("start").and_then(|item| item.as_u64())? as usize;
        let end_index = range.get("end").and_then(|item| item.as_u64())? as usize;
        if end_index < start_index {
            return None;
        }
        return Some(());
    }

    if target
        .get("location")
        .and_then(|item| item.as_u64())
        .is_some()
    {
        return Some(());
    }

    None
}

use crate::gateway::models::{
    OpenAIChatChoice, OpenAIChatChunk, OpenAIChatChunkChoice, OpenAIChatCompletionTokensDetails,
    OpenAIChatDelta, OpenAIChatPromptTokensDetails, OpenAIChatResponse, OpenAIChatResponseMessage,
    OpenAIChatUsage, OpenAIResponseToolCall, OpenAIToolCallFunction,
};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 上游 `metadataEvent.stopReason` 已知集合（内部枚举 `ldi` 7 值 + `Ofd` 终态）。
/// 取证：docs/kiro-re-1.4.5/stopreason-enum.txt；定策：docs/Kiro 1.4.5/协议面与网关对照.md §G-8b。
pub fn is_kiro_stop_reason(reason: &str) -> bool {
    matches!(
        reason,
        "END_TURN"
            | "MAX_TOKENS"
            | "TOOL_USE"
            | "CONTENT_FILTERED"
            | "PAUSE_TURN"
            | "MODEL_CONTEXT_WINDOW_EXCEEDED"
            | "UNKNOWN"
            | "COMPACTION_REQUIRED_FOR_MODEL_HANDOFF"
    )
}

/// G-8b 全量映射：上游 stopReason → Anthropic `stop_reason`。
/// 官方文档（docs/protocol/anthropic-messages-api.md §5.2）明确
/// 「实现应把停止原因作为可扩展字符串处理」，因此：
/// - MODEL_CONTEXT_WINDOW_EXCEEDED / COMPACTION 都映 max_tokens（输出被截）；
/// - PAUSE_TURN 映 pause_turn（Anthropic 本就保留该值给长回合模型）；
/// - 仅 UNKNOWN 退回 end_turn（上游自身也无法定性，无更多信息可传）。
pub fn map_stop_reason_to_anthropic(reason: &str) -> &'static str {
    match reason {
        "MAX_TOKENS" | "MODEL_CONTEXT_WINDOW_EXCEEDED" | "COMPACTION_REQUIRED_FOR_MODEL_HANDOFF" => {
            "max_tokens"
        }
        "TOOL_USE" => "tool_use",
        "CONTENT_FILTERED" => "refusal",
        "PAUSE_TURN" => "pause_turn",
        _ => "end_turn",
    }
}

/// G-8b 全量映射：上游 stopReason → OpenAI Chat `finish_reason`。
pub fn map_stop_reason_to_openai_finish(reason: &str) -> &'static str {
    match reason {
        "MAX_TOKENS" | "MODEL_CONTEXT_WINDOW_EXCEEDED" | "COMPACTION_REQUIRED_FOR_MODEL_HANDOFF" => {
            "length"
        }
        "TOOL_USE" => "tool_calls",
        "CONTENT_FILTERED" => "content_filter",
        _ => "stop",
    }
}

/// G-8b 全量映射：上游 stopReason → OpenAI Responses `incomplete_details.reason`；
/// None 表示终态 completed。OpenAI 标准的 reason 枚举只有 max_output_tokens /
/// content_filter，非标截断（窗口超限/压缩交接）归入 max_output_tokens，
/// 原值保留在解析日志，不发明非标 reason 去撞客户端解析器。
pub fn map_stop_reason_to_responses_incomplete(reason: &str) -> Option<&'static str> {
    match reason {
        "MAX_TOKENS" | "MODEL_CONTEXT_WINDOW_EXCEEDED" | "COMPACTION_REQUIRED_FOR_MODEL_HANDOFF" => {
            Some("max_output_tokens")
        }
        "CONTENT_FILTERED" => Some("content_filter"),
        _ => None,
    }
}

/// Responses 终态与 item 状态的单一来源（G-8 补丁）：
/// `response.output_item.done` 的 message item 与 response 主体必须同一
/// status，incomplete 响应下 item 不得仍报 completed。
pub fn responses_item_status(aggregated: &AggregatedKiroResponse) -> &'static str {
    if aggregated
        .stop_reason
        .as_deref()
        .and_then(map_stop_reason_to_responses_incomplete)
        .is_some()
    {
        "incomplete"
    } else {
        "completed"
    }
}

/// G-9：流式 OpenAI finish 帧的 delta.refusal —— 仅当最终 finish_reason 为
/// content_filter 且上游给了 refusal 文本时下发，否则不下发该字段。
pub fn finish_refusal_text(aggregated: &AggregatedKiroResponse, finish_reason: &str) -> Option<String> {
    if finish_reason == "content_filter" {
        aggregated.refusal.clone()
    } else {
        None
    }
}

pub fn build_openai_chat_usage(prompt_tokens: i32, completion_tokens: i32) -> OpenAIChatUsage {
    OpenAIChatUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens + completion_tokens,
        prompt_tokens_details: OpenAIChatPromptTokensDetails {
            cached_tokens: 0,
            audio_tokens: 0,
        },
        completion_tokens_details: OpenAIChatCompletionTokensDetails {
            reasoning_tokens: 0,
            audio_tokens: 0,
            accepted_prediction_tokens: 0,
            rejected_prediction_tokens: 0,
        },
    }
}

pub fn build_openai_chunk(
    completion_id: &str,
    created: i64,
    model: &str,
    delta: OpenAIChatDelta,
    finish_reason: Option<String>,
    usage: Option<OpenAIChatUsage>,
) -> OpenAIChatChunk {
    OpenAIChatChunk {
        id: completion_id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created,
        model: model.to_string(),
        system_fingerprint: None,
        choices: vec![OpenAIChatChunkChoice {
            index: 0,
            delta,
            logprobs: None,
            finish_reason,
        }],
        usage,
        service_tier: None,
    }
}

/// OpenAI stream_options.include_usage：finish 帧之后的空 choices + usage 帧
pub fn build_openai_usage_chunk(
    completion_id: &str,
    created: i64,
    model: &str,
    usage: OpenAIChatUsage,
) -> OpenAIChatChunk {
    OpenAIChatChunk {
        id: completion_id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created,
        model: model.to_string(),
        system_fingerprint: None,
        choices: vec![],
        usage: Some(usage),
        service_tier: None,
    }
}

/// R4：Kiro citations → Chat 官方 `annotations` 形状
/// `{type:"url_citation", url_citation:{end_index, start_index, title, url}}`。
/// title 用引用文本，起止从 Kiro target.range 映射；缺 range 时不出注解（官方
/// 索引必填，不能像 Responses 那样猜）。
fn build_openai_url_citation_annotations(
    aggregated: &AggregatedKiroResponse,
) -> Vec<serde_json::Value> {
    let mut annotations = Vec::new();
    for citation in &aggregated.citations {
        let range = citation.target.get("range");
        let (Some(start), Some(end)) = (
            range.and_then(|r| r.get("start")).and_then(serde_json::Value::as_u64),
            range.and_then(|r| r.get("end")).and_then(serde_json::Value::as_u64),
        ) else {
            continue;
        };
        annotations.push(serde_json::json!({
            "type": "url_citation",
            "url_citation": {
                "end_index": end,
                "start_index": start,
                "title": citation.text.clone().unwrap_or_else(|| citation.link.clone()),
                "url": citation.link
            }
        }));
    }
    annotations
}

pub fn build_openai_response(
    model: &str,
    aggregated: &AggregatedKiroResponse,
) -> OpenAIChatResponse {
    let completion_id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut message = OpenAIChatResponseMessage {
        role: "assistant".to_string(),
        content: if aggregated.text.is_empty() {
            None
        } else {
            Some(aggregated.text.clone())
        },
        // refusal 先置空，等 finish_reason 定下来后按 G-9 口径回填
        refusal: None,
        annotations: build_openai_url_citation_annotations(aggregated),
        tool_calls: None,
        audio: None,
        function_call: None,
    };

    if !aggregated.tool_calls.is_empty() {
        let tool_calls: Vec<OpenAIResponseToolCall> = aggregated
            .tool_calls
            .iter()
            .map(|(id, name, arguments)| OpenAIResponseToolCall {
                id: id.clone(),
                call_type: "function".to_string(),
                function: OpenAIToolCallFunction {
                    name: name.clone(),
                    arguments: arguments.clone(),
                },
            })
            .collect();
        message.tool_calls = Some(tool_calls);
    }
    // G-8：上游 stopReason 优先，启发式仅在缺失时兑底；
    // 有工具但映射结果仍是 stop 时纠正为 tool_calls，否则客户端不会执行工具
    let finish_reason = match aggregated
        .stop_reason
        .as_deref()
        .map(map_stop_reason_to_openai_finish)
    {
        Some(mapped) if !(mapped == "stop" && message.tool_calls.is_some()) => {
            mapped.to_string()
        }
        _ => {
            if message.tool_calls.is_some() {
                "tool_calls".to_string()
            } else {
                "stop".to_string()
            }
        }
    };

    // G-9：refusal 文本只在终态真是 content_filter 时下发，避免与
    // finish_reason=length/stop 共出矛盾组合（流式侧走同一个 finish_refusal_text）
    message.refusal = finish_refusal_text(aggregated, &finish_reason);

    OpenAIChatResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created,
        model: model.to_string(),
        choices: vec![OpenAIChatChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: Some(finish_reason),
        }],
        usage: build_openai_chat_usage(aggregated.input_tokens, aggregated.output_tokens),
        service_tier: None,
        system_fingerprint: None,
        moderation: None,
    }
}

#[cfg(test)]
mod tests;
