//! Responses 会话持久化：会话消息与请求选项的存取。

use super::*;

pub async fn restore_responses_session_messages(
    state: &RouterState,
    request: &NormalizedRequest,
) -> Vec<NormalizedMessage> {
    let Some(mut current_response_id) = request.previous_response_id.clone() else {
        return request.messages.clone();
    };

    let sessions = state.responses_sessions.lock().await;
    let mut chain = Vec::new();
    while let Some(entry) = sessions.get(&current_response_id) {
        chain.push(entry.clone());
        let Some(previous) = entry.previous_response_id.clone() else {
            break;
        };
        current_response_id = previous;
    }
    drop(sessions);

    if chain.is_empty() {
        return request.messages.clone();
    }

    // 收集当前请求中的 tool_result_id，用于过滤最后一轮的 tool_calls
    let current_tool_result_ids: std::collections::HashSet<String> = request
        .messages
        .iter()
        .filter(|message| message.role == "tool")
        .filter_map(|message| message.tool_call_id.clone())
        .collect();

    chain.reverse();
    let mut merged = Vec::new();
    let chain_len = chain.len();
    for (index, entry) in chain.into_iter().enumerate() {
        let is_latest_entry = index + 1 == chain_len;

        // 对最后一轮的 tool_calls 进行过滤：只保留当前请求有对应 tool_result 的
        let effective_tool_calls = if is_latest_entry && !current_tool_result_ids.is_empty() {
            let filtered: Vec<_> = entry
                .tool_calls
                .iter()
                .filter(|(id, _, _)| current_tool_result_ids.contains(id))
                .cloned()
                .collect();
            // 如果过滤后为空（可能是 ID 不匹配），回退到全部
            if filtered.is_empty() {
                entry.tool_calls.clone()
            } else {
                filtered
            }
        } else {
            entry.tool_calls.clone()
        };

        merged.extend(entry.request_messages.clone());
        merged.push(NormalizedMessage {
            role: "assistant".to_string(),
            content: Some(Value::String(entry.response_text.clone())),
            tool_calls: if effective_tool_calls.is_empty() {
                None
            } else {
                Some(
                    effective_tool_calls
                        .iter()
                        .map(|(id, name, arguments)| ToolCall {
                            id: id.clone(),
                            call_type: "function".to_string(),
                            function: ToolCallFunction {
                                name: name.clone(),
                                arguments: if arguments.is_empty() {
                                    "{}".to_string()
                                } else {
                                    arguments.clone()
                                },
                            },
                        })
                        .collect(),
                )
            },
            tool_call_id: None,
            metadata: None,
        });
    }
    merged.extend(request.messages.clone());
    merged
}

/// 从历史 session 继承 tools 和 tool_choice（Responses API 有状态对话）
///
/// 当客户端使用 previous_response_id 但不重传 tools 时，
/// 需要从历史 session 中继承工具定义。
pub async fn restore_responses_session_request_options(
    state: &RouterState,
    request: &NormalizedRequest,
) -> (Option<Vec<Tool>>, Option<Value>) {
    let Some(mut current_response_id) = request.previous_response_id.clone() else {
        return (None, None);
    };

    let sessions = state.responses_sessions.lock().await;
    let mut inherited_tools = None;
    let mut inherited_tool_choice = None;

    while let Some(entry) = sessions.get(&current_response_id) {
        if inherited_tools.is_none() {
            inherited_tools = entry.request_tools.clone();
        }
        if inherited_tool_choice.is_none() {
            inherited_tool_choice = entry.request_tool_choice.clone();
        }
        if inherited_tools.is_some() && inherited_tool_choice.is_some() {
            break;
        }
        let Some(previous) = entry.previous_response_id.clone() else {
            break;
        };
        current_response_id = previous;
    }

    (inherited_tools, inherited_tool_choice)
}

/// 会话缓存 TTL：超过 1 小时的条目视为过期。
const SESSION_TTL: Duration = Duration::from_secs(60 * 60);

/// 会话清理周期。
///
/// 必须独立于请求：只在插入时清理意味着「一小时内没有新请求，旧会话就一直占内存」，
/// 而闲置恰是内存该被释放的时候。
const SESSION_CLEANUP_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// 会话缓存条数上限。
///
/// 每条都存整段 `request_messages`，长对话一回合就是几十 KB；
/// 只有 TTL 没有容量上限时，一小时内的高频长对话会把内存线性推高。
const SESSION_MAX_ENTRIES: usize = 200;

/// 淘汰过期与超量会话。
///
/// 先按 TTL 清，再按 `updated_at` 从旧到新裁到条数上限——
/// 最近用过的会话最可能被 `previous_response_id` 续接，优先保留。
pub(crate) fn prune_sessions(sessions: &mut HashMap<String, ResponsesSessionEntry>) {
    sessions.retain(|_, entry| entry.updated_at.elapsed() < SESSION_TTL);
    if sessions.len() <= SESSION_MAX_ENTRIES {
        return;
    }
    let mut by_age: Vec<_> = sessions
        .iter()
        .map(|(id, entry)| (id.clone(), entry.updated_at))
        .collect();
    by_age.sort_by_key(|(_, at)| *at);
    let excess = sessions.len() - SESSION_MAX_ENTRIES;
    for (id, _) in by_age.into_iter().take(excess) {
        sessions.remove(&id);
    }
}

pub async fn persist_responses_session_entry(
    state: &RouterState,
    response_id: &str,
    request_messages: Vec<NormalizedMessage>,
    request_tools: Option<Vec<Tool>>,
    request_tool_choice: Option<Value>,
    previous_response_id: Option<String>,
    aggregated: &stream::AggregatedKiroResponse,
) {
    let mut sessions = state.responses_sessions.lock().await;
    prune_sessions(&mut sessions);
    sessions.insert(
        response_id.to_string(),
        ResponsesSessionEntry {
            response_id: response_id.to_string(),
            previous_response_id,
            request_messages,
            request_tools,
            request_tool_choice,
            response_text: aggregated.text.clone(),
            tool_calls: aggregated.tool_calls.clone(),
            updated_at: Instant::now(),
        },
    );
}

/// 启动会话缓存的独立清理周期。
///
/// 由 `spawn_runtime` 在网关启动时调用；清理频率远低于 TTL，
/// 因此即使定时器精度差也不会误删仍在使用的会话。
pub fn spawn_responses_session_cleanup(sessions: crate::gateway::ResponsesSessionStore) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SESSION_CLEANUP_INTERVAL);
        // 首次 tick 立即返回，跳过它以启动时不做无谓清理
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let mut sessions = sessions.lock().await;
            prune_sessions(&mut sessions);
        }
    });
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponsesOutputText {
    pub(super) text: String,
    pub(super) annotations: Vec<Value>,
}
