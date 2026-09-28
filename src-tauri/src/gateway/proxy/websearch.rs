//! Claude Code WebSearch 强制工具调用子请求的处理。
//!
//! ## 背景（逆向结论，2026-09-28 实测）
//!
//! Claude Code（`cc_entrypoint=claude-desktop-3p`）执行内置 WebSearch 时，会向
//! Anthropic 兼容端点发起一个独立子请求：
//!
//! - `tool_choice: {"type":"tool","name":"web_search"}` —— 强制工具调用；
//! - `tools: [{"name":"web_search","type":"web_search_20250305","max_uses":8}]`
//!   —— **服务端工具**（无 input_schema），官方 API 中由 Anthropic 服务端执行搜索；
//! - system 含 "You are an assistant for performing a web search tool use"；
//! - 末条 user 消息即固定模板 "Perform a web search for the query: <query>"。
//!
//! Kiro 上游没有 tool_choice 语义、更没有服务端 web_search，直接转发必然失败。
//! 但 Kiro 官方在 runtime 面托管了同名远程 MCP 工具（IDE 的 `remote_web_search`
//! 即此通道，见 kiro_agent bundle `GBi`/`RemoteToolWrapper`）：
//!
//! ```text
//! POST https://runtime.{region}.kiro.dev/mcp
//! Authorization: Bearer <access_token>
//! content-type: application/json
//! accept: application/json, text/event-stream
//! x-amzn-kiro-profile-arn: <profile_arn>
//!
//! {"jsonrpc":"2.0","id":"...","method":"tools/call",
//!  "params":{"name":"web_search","arguments":{"query":"..."}}}
//! ```
//!
//! 响应 `result.content[0].text` 为 JSON 字符串
//! `{"results":[{title,url,snippet,publishedDate,id,domain,...}]}`（官方客户端优先读
//! `result.structuredContent.results`，content[].text 为兜底，此处对齐双通道）。
//!
//! ## 处理策略
//!
//! 命中检测后把子请求**原地改写**为"注入搜索结果 → 模型直答"：
//! 1. 从末条 user 消息提取 query（模板解析失败则整条消息兜底，截断 200 字符——
//!    与服务端工具 schema `query ≤ 200 chars` 一致）；
//! 2. 用当前账号凭证调远程 MCP `tools/call`；
//! 3. 把结果摘要追加进该 user 消息，并剥掉 tools/tool_choice（Kiro 无强制工具
//!    语义，残留 webSearch 工具只会让模型发出客户端无法执行的 tool_use）；
//! 4. `thinking: {"type":"disabled"}` 一并剥掉（payload 构建层 `is_some()` 即注入
//!    thinking 标签，会把显式禁用误开）。
//!
//! 改写后走既有流式/非流式管线，下游（Claude Code）拿到纯文本即认为搜索完成。
//! 搜索失败时注入"不可用"提示降级为模型直答，不产生硬错误。

use super::*;

/// 检测是否为 Claude Code 的 WebSearch 强制工具子请求（归一化后调用）。
///
/// 归一化层会把工具名 sanitize 成 camelCase（`web_search` → `webSearch`）：
/// - 服务端工具（type 形如 web_search_20250305）已被剥离出 tools，名字记录在
///   `server_tool_names`；
/// - 客户端也可能传名为 web_search 的普通自定义工具（保留在 tools 中）。
/// 两侧任一命中 + tool_choice 强制同名即判定。
pub(super) fn is_forced_web_search_request(
    tool_choice: &Option<Value>,
    tools: &Option<Vec<Tool>>,
    server_tool_names: &[String],
) -> bool {
    let web_search_tool_present = tools
        .as_ref()
        .is_some_and(|items| {
            items
                .iter()
                .any(|tool| tool.function.name == WEB_SEARCH_SANITIZED_NAME)
        })
        || server_tool_names.iter().any(|name| name == WEB_SEARCH_SANITIZED_NAME);
    if !web_search_tool_present {
        return false;
    }

    let Some(choice) = tool_choice.as_ref() else {
        return false;
    };
    let choice_type = match choice {
        Value::String(raw) => raw.trim(),
        Value::Object(_) => match choice.get("type").and_then(Value::as_str) {
            Some(value) => value.trim(),
            None => return false,
        },
        _ => return false,
    };
    if !matches!(choice_type, "tool" | "function") {
        return false;
    }

    let name = choice
        .get("name")
        .or_else(|| choice.pointer("/function/name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    crate::gateway::converter::sanitize_tool_name(name) == WEB_SEARCH_SANITIZED_NAME
}

/// Claude Code 固定模板：`Perform a web search for the query: <query>`
fn extract_search_query(request: &NormalizedRequest) -> Option<String> {
    static QUERY_RE: OnceLock<Regex> = OnceLock::new();
    let re = QUERY_RE.get_or_init(|| {
        // (?is) 跨行匹配：content 可能带 EnvironmentContext 等后缀
        Regex::new(r"(?is)perform\s+a\s+web\s+search\s+for\s+the\s+query:\s*(.+)")
            .expect("web search query regex")
    });

    let last_user_text = request.messages.iter().rev().find_map(|message| {
        if message.role != "user" {
            return None;
        }
        match message.content.as_ref()? {
            Value::String(text) => Some(text.clone()),
            _ => None,
        }
    })?;

    let query = re
        .captures(&last_user_text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
        // 模板失配时整条消息就是查询本体（子请求的 user 消息只有这一句话）
        .unwrap_or_else(|| last_user_text.trim().to_string());

    // 服务端工具 schema：query ≤ 200 字符（按字符截断，避免切断中文/emoji）
    let query: String = query.chars().take(WEB_SEARCH_QUERY_MAX_CHARS).collect();
    (!query.trim().is_empty()).then_some(query)
}

/// 调用 Kiro runtime 远程 MCP 的 web_search 工具，返回原始结果条目数组。
async fn call_remote_web_search(
    http: &Client,
    user_agent: &str,
    access_token: &str,
    region: &str,
    profile_arn: Option<&str>,
    query: &str,
) -> Result<Vec<Value>, String> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": format!("ws-{}", uuid::Uuid::new_v4().simple()),
        "method": "tools/call",
        "params": {
            "name": WEB_SEARCH_TOOL_NAME,
            "arguments": { "query": query }
        }
    });

    let mut req = http
        .post(crate::clients::kiro_client::build_mcp_url(region))
        .header("Authorization", format!("Bearer {access_token}"))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("user-agent", user_agent)
        .timeout(Duration::from_secs(WEB_SEARCH_TIMEOUT_SECS))
        .json(&body);
    if let Some(arn) = profile_arn.map(str::trim).filter(|value| !value.is_empty()) {
        req = req.header("x-amzn-kiro-profile-arn", arn);
    }

    let resp = req.send().await.map_err(|e| format!("MCP 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("MCP 响应读取失败: {e}"))?;
    if !status.is_success() {
        return Err(format!("MCP HTTP {status}: {}", sanitize_error(&text)));
    }

    let payload = extract_jsonrpc_payload(&text)?;
    if let Some(err) = payload.get("error").filter(|value| !value.is_null()) {
        let message = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Err(format!("MCP JSON-RPC error: {message}"));
    }
    let result = payload.get("result").ok_or("MCP 响应缺少 result")?;
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let detail = first_text_content(result).unwrap_or_default();
        return Err(format!("web_search 执行失败: {detail}"));
    }

    // 双通道解析（对齐官方 Ixu.parseSearchResults）
    if let Some(results) = result
        .pointer("/structuredContent/results")
        .and_then(Value::as_array)
    {
        return Ok(results.clone());
    }
    let text_content =
        first_text_content(result).ok_or("web_search 结果 content 为空")?;
    let parsed: Value = serde_json::from_str(text_content.trim())
        .map_err(|e| format!("web_search 结果 JSON 解析失败: {e}"))?;
    parsed
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "web_search 结果缺少 results 数组".to_string())
}

/// JSON 或 SSE（`data:` 行）两种封装的 JSON-RPC 载荷提取。
fn extract_jsonrpc_payload(text: &str) -> Result<Value, String> {
    if text.trim_start().starts_with('{') {
        return serde_json::from_str(text).map_err(|e| format!("MCP 响应解析失败: {e}"));
    }
    for line in text.lines().rev() {
        if let Some(data) = line.trim_start().strip_prefix("data:") {
            let data = data.trim();
            if data.starts_with('{') {
                return serde_json::from_str(data).map_err(|e| format!("MCP SSE 解析失败: {e}"));
            }
        }
    }
    Err("MCP 响应既非 JSON 也非 SSE".to_string())
}

fn first_text_content(result: &Value) -> Option<&str> {
    result
        .get("content")?
        .as_array()?
        .iter()
        .find(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
}

/// 把结果条目格式化为模型可读摘要（含发布日期，时效性查询的关键信号）。
fn format_search_digest(query: &str, results: &[Value]) -> String {
    let mut out = format!(
        "<web_search_results query=\"{}\">\n",
        query.replace('"', "'")
    );
    for (index, result) in results.iter().take(WEB_SEARCH_MAX_RESULTS).enumerate() {
        let title = result
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("(no title)");
        let url = result.get("url").and_then(Value::as_str).unwrap_or("");
        let snippet = result.get("snippet").and_then(Value::as_str).unwrap_or("");
        out.push_str(&format!("[{}] {title}\n", index + 1));
        if !url.is_empty() {
            out.push_str(&format!("URL: {url}\n"));
        }
        if let Some(date) = published_date_string(result) {
            out.push_str(&format!("Published: {date}\n"));
        }
        if !snippet.is_empty() {
            out.push_str(snippet);
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str("</web_search_results>");
    out
}

/// `publishedDate`：epoch 毫秒（Kiro 0.12.301+ 抓包实测），兼容 ISO 字符串。
fn published_date_string(result: &Value) -> Option<String> {
    match result.get("publishedDate")? {
        Value::Number(ms) => {
            let ms = ms.as_i64()?;
            let dt = chrono::DateTime::from_timestamp_millis(ms)?;
            Some(dt.format("%Y-%m-%d").to_string())
        }
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// 修改末条 user 消息（字符串内容）——注入点。
fn mutate_last_user_text(request: &mut NormalizedRequest, transform: impl FnOnce(&str) -> String) {
    if let Some(message) = request
        .messages
        .iter_mut()
        .rev()
        .find(|m| m.role == "user" && matches!(m.content.as_ref(), Some(Value::String(_))))
    {
        if let Some(Value::String(text)) = message.content.as_ref() {
            message.content = Some(Value::String(transform(text)));
        }
    }
}

/// 处理入口：命中即原地改写请求；任何失败都降级为"模型直答"，不产生硬错误。
pub(super) async fn handle_forced_web_search(
    upstream: &UpstreamCredentials,
    request: &mut NormalizedRequest,
) {
    if request
        .thinking
        .as_ref()
        .map(|t| t.thinking_type == "disabled")
        .unwrap_or(false)
    {
        request.thinking = None;
    }

    // 无论搜索成败都不再下发工具：Kiro 无 tool_choice 语义，
    // 残留 webSearch 工具只会让模型发出客户端无法执行的 tool_use。
    let had_tools = request.tools.is_some();
    request.tools = None;
    request.tool_choice = None;
    if !had_tools {
        return;
    }

    let Some(query) = extract_search_query(request) else {
        log::warn!("[WebSearch] 子请求命中但未能提取 query，降级为模型直答");
        mutate_last_user_text(request, |text| {
            format!(
                "{text}\n\nThe web search tool is unavailable for this request (the search \
                 query could not be determined). Answer from your own knowledge and clearly \
                 state that no search was performed."
            )
        });
        return;
    };

    let profile_arn = upstream
        .profile_arn
        .as_deref()
        .or(upstream.available_models_profile_arn.as_deref());
    log::info!(
        "[WebSearch] 强制工具子请求命中 | query={query:?} | region={} | profileArn={}",
        upstream.region,
        profile_arn.unwrap_or("none")
    );

    match call_remote_web_search(
        &upstream.http,
        &upstream.user_agent,
        &upstream.access_token,
        &upstream.region,
        profile_arn,
        &query,
    )
    .await
    {
        Ok(results) if results.is_empty() => {
            log::info!("[WebSearch] 搜索完成，无结果");
            mutate_last_user_text(request, |text| {
                format!(
                    "{text}\n\nThe web search completed but returned no results. Answer from \
                     your own knowledge and clearly state that the search found nothing."
                )
            });
        }
        Ok(results) => {
            log::info!(
                "[WebSearch] 搜索成功，{} 条结果（取前 {} 条注入）",
                results.len(),
                WEB_SEARCH_MAX_RESULTS.min(results.len())
            );
            let digest = format_search_digest(&query, &results);
            mutate_last_user_text(request, |text| {
                format!(
                    "{text}\n\n{digest}\n\nUsing the web search results above, provide the \
                     final answer to the original request. Include the source URLs where \
                     appropriate."
                )
            });
        }
        Err(e) => {
            log::warn!("[WebSearch] 搜索失败，降级为模型直答: {e}");
            mutate_last_user_text(request, |text| {
                format!(
                    "{text}\n\nThe web search tool is currently unavailable. Answer from your \
                     own knowledge and clearly state that no search was performed."
                )
            });
        }
    }
}

const WEB_SEARCH_TOOL_NAME: &str = "web_search";
/// sanitize_tool_name("web_search") 的结果
const WEB_SEARCH_SANITIZED_NAME: &str = "webSearch";
/// 服务端工具 schema：query ≤ 200 chars
const WEB_SEARCH_QUERY_MAX_CHARS: usize = 200;
const WEB_SEARCH_MAX_RESULTS: usize = 10;
const WEB_SEARCH_TIMEOUT_SECS: u64 = 60;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::models::ToolFunction;

    fn normalized_request(
        tool_choice: Option<Value>,
        tools: Option<Vec<Tool>>,
        user_text: &str,
    ) -> NormalizedRequest {
        NormalizedRequest {
            model: "claude-opus-4.8".to_string(),
            messages: vec![NormalizedMessage {
                role: "user".to_string(),
                content: Some(Value::String(user_text.to_string())),
                tool_calls: None,
                tool_call_id: None,
                metadata: None,
            }],
            stream: true,
            max_tokens: Some(32000),
            temperature: Some(1.0),
            top_p: None,
            stop: None,
            tools,
            tool_choice,
            previous_response_id: None,
            thinking: None,
            include_usage: false,
            tool_name_map: HashMap::new(),
            server_tool_names: Vec::new(),
        }
    }

    fn web_search_tool() -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "webSearch".to_string(),
                description: None,
                parameters: Some(json!({"type": "object"})),
            },
            cache_control: None,
        }
    }

    fn bash_tool() -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "bashTool".to_string(),
                description: None,
                parameters: Some(json!({"type": "object"})),
            },
            cache_control: None,
        }
    }

    #[test]
    fn detects_cc_web_search_subrequest() {
        let request = normalized_request(
            Some(json!({"type": "tool", "name": "web_search"})),
            Some(vec![web_search_tool()]),
            "Perform a web search for the query: 歌手 刘欢 最近 2026 近况",
        );
        assert!(is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names
        ));
    }

    #[test]
    fn detects_openai_function_shape() {
        let request = normalized_request(
            Some(json!({"type": "function", "function": {"name": "web_search"}})),
            Some(vec![web_search_tool()]),
            "Perform a web search for the query: test",
        );
        assert!(is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names
        ));
    }

    #[test]
    fn rejects_non_forced_or_non_web_search() {
        // 无 tool_choice
        let request = normalized_request(None, Some(vec![web_search_tool()]), "hi");
        assert!(!is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names
        ));
        // auto
        let request = normalized_request(
            Some(json!({"type": "auto"})),
            Some(vec![web_search_tool()]),
            "hi",
        );
        assert!(!is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names
        ));
        // 强制的是别的工具
        let request = normalized_request(
            Some(json!({"type": "tool", "name": "bash"})),
            Some(vec![bash_tool()]),
            "hi",
        );
        assert!(!is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names
        ));
        // 强制 web_search 但工具列表里没有
        let request = normalized_request(
            Some(json!({"type": "tool", "name": "web_search"})),
            Some(vec![bash_tool()]),
            "hi",
        );
        assert!(!is_forced_web_search_request(
            &request.tool_choice,
            &request.tools,
            &request.server_tool_names
        ));
    }

    #[test]
    fn extracts_query_from_cc_template() {
        let request = normalized_request(
            Some(json!({"type": "tool", "name": "web_search"})),
            Some(vec![web_search_tool()]),
            "Perform a web search for the query: 歌手 刘欢 最近 2026 近况",
        );
        assert_eq!(
            extract_search_query(&request).as_deref(),
            Some("歌手 刘欢 最近 2026 近况")
        );
    }

    #[test]
    fn falls_back_to_full_user_text_on_template_mismatch() {
        let request = normalized_request(None, None, "今天天气怎么样");
        assert_eq!(
            extract_search_query(&request).as_deref(),
            Some("今天天气怎么样")
        );
    }

    #[test]
    fn truncates_query_to_schema_limit() {
        let long = "x".repeat(500);
        let request = normalized_request(None, None, &long);
        let query = extract_search_query(&request).unwrap();
        assert_eq!(query.chars().count(), 200);
    }

    #[test]
    fn formats_digest_with_published_date() {
        let results = vec![json!({
            "title": "Kiro Docs",
            "url": "https://kiro.dev/docs",
            "snippet": "Kiro CLI implements ACP.",
            "publishedDate": 1785869351000i64
        })];
        let digest = format_search_digest("kiro acp", &results);
        assert!(digest.starts_with("<web_search_results query=\"kiro acp\">"));
        assert!(digest.contains("[1] Kiro Docs"));
        assert!(digest.contains("URL: https://kiro.dev/docs"));
        assert!(digest.contains("Published: 20"));
        assert!(digest.contains("Kiro CLI implements ACP."));
        assert!(digest.ends_with("</web_search_results>"));
    }

    #[test]
    fn extracts_jsonrpc_payload_from_json_and_sse() {
        let json_body = json!({"jsonrpc": "2.0", "id": "1", "result": {"ok": true}});
        assert_eq!(
            extract_jsonrpc_payload(&json_body.to_string()).unwrap(),
            json_body
        );

        let sse_body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":\"1\",\"result\":{\"ok\":true}}\n\n";
        assert_eq!(
            extract_jsonrpc_payload(sse_body).unwrap(),
            json!({"jsonrpc": "2.0", "id": "1", "result": {"ok": true}})
        );
    }

    #[test]
    fn jsonrpc_error_and_iserror_surface_as_failure_payload() {
        let body = json!({"jsonrpc": "2.0", "id": "1", "error": {"code": -32000, "message": "boom"}});
        let payload = extract_jsonrpc_payload(&body.to_string()).unwrap();
        assert_eq!(
            payload.get("error").unwrap().get("message").unwrap(),
            "boom"
        );

        let body = json!({"jsonrpc": "2.0", "id": "1",
            "result": {"isError": true, "content": [{"type": "text", "text": "quota exceeded"}]}});
        let payload = extract_jsonrpc_payload(&body.to_string()).unwrap();
        let result = payload.get("result").unwrap();
        assert!(result.get("isError").and_then(Value::as_bool).unwrap_or(false));
        assert_eq!(
            first_text_content(result),
            Some("quota exceeded")
        );
    }

    #[test]
    fn mutates_last_user_message_in_place() {
        let mut request = normalized_request(None, None, "Perform a web search for the query: abc");
        mutate_last_user_text(&mut request, |text| format!("{text}\n\n<web_search_results />"));
        match request.messages.last().unwrap().content.as_ref().unwrap() {
            Value::String(text) => {
                assert!(text.starts_with("Perform a web search for the query: abc"));
                assert!(text.ends_with("<web_search_results />"));
            }
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn anthropic_server_tools_are_recorded_stripped_and_detected() {
        let payload = json!({
            "model": "claude-opus-4.8",
            "max_tokens": 100,
            "stream": true,
            "messages": [{"role": "user",
                "content": [{"type": "text", "text": "Perform a web search for the query: test"}]}],
            "tools": [
                {"name": "web_search", "type": "web_search_20250305", "max_uses": 8},
                {"name": "my_tool", "type": "custom",
                 "input_schema": {"type": "object", "properties": {}}}
            ],
            "tool_choice": {"type": "tool", "name": "web_search"}
        });
        let request: AnthropicMessagesRequest = serde_json::from_value(payload).unwrap();
        let normalized = normalize_anthropic_request(&request);

        // 服务端工具被记录（sanitized）并从 tools 中剥离；自定义工具保留
        assert_eq!(normalized.server_tool_names, vec!["webSearch"]);
        let names: Vec<&str> = normalized
            .tools
            .as_ref()
            .unwrap()
            .iter()
            .map(|tool| tool.function.name.as_str())
            .collect();
        assert_eq!(names, vec!["myTool"]);

        // WebSearch 子请求检测经由 server_tool_names 命中
        assert!(is_forced_web_search_request(
            &normalized.tool_choice,
            &normalized.tools,
            &normalized.server_tool_names
        ));

        // 服务端工具在场但未强制 → 不命中
        let not_forced = normalized_request(None, None, "hi");
        let mut not_forced = not_forced;
        not_forced.server_tool_names = vec!["webSearch".to_string()];
        assert!(!is_forced_web_search_request(
            &not_forced.tool_choice,
            &not_forced.tools,
            &not_forced.server_tool_names
        ));
    }
}
