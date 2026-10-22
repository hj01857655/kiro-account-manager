//! 三协议「字段 × 消费判定」表。
//!
//! 问题：三个入口各自 `Deserialize` 到强类型结构体，**没被结构体收录的字段
//! 会被 serde 静默丢弃**；被收录但标了 `#[allow(dead_code)]` 的字段同样
//! 解析后不用。客户端发 `response_format`、`top_k` 时以为生效了，
//! 行为差异最后被归因成「模型不听话」。
//!
//! 本模块把判定变成可执行的真料：每个字段明确标为
//! - [`FieldSupport::Consumed`] 真的会改变上游请求或响应
//! - [`FieldSupport::Ignored`]   有意不支持，静默丢弃（附带原因）
//! - [`FieldSupport::Rejected`]  明确 400，不让客户端误以为生效
//!
//! `normalize_*` 之外还有一道 `reject_unsupported_fields` 负责把
//! `Rejected` 变成 400。判定表与那层过滤靠 `assert_table_matches_rejections`
//! 测试保持一致——改了一处忘另一处会红。

use serde_json::Value;

/// 单个协议字段的支持程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSupport {
    /// 消费：字段值会进入 `NormalizedRequest`，真正影响上游请求或响应。
    Consumed,
    /// 有意忽略：记录在案但不实现。客户端发了不报错，也不生效。
    Ignored {
        /// 为什么不实现——给排查的人看，也防止后人重复讨论。
        reason: &'static str,
    },
    /// 拒绝：返回 400。用于「不支持却容易被误以为支持」的字段。
    Rejected {
        /// 客户端可见的原因（写入 400 响应体）。
        reason: &'static str,
    },
}

/// 协议标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// `POST /v1/messages`（Anthropic Messages）
    AnthropicMessages,
    /// `POST /v1/chat/completions`（OpenAI Chat Completions）
    OpenAIChatCompletions,
    /// `POST /v1/responses`（OpenAI Responses）
    OpenAIResponses,
}

impl Protocol {
    /// 该协议的字段判定表。
    pub fn field_support(&self) -> &'static [(&'static str, FieldSupport)] {
        match self {
            Self::AnthropicMessages => ANTHROPIC_MESSAGES_FIELDS,
            Self::OpenAIChatCompletions => OPENAI_CHAT_FIELDS,
            Self::OpenAIResponses => OPENAI_RESPONSES_FIELDS,
        }
    }

    /// 查一个字段的判定；表里没有的字段按 [`FieldSupport::Ignored`] 处理
    /// （未知字段不该让请求失败——客户端升级时常带上新字段）。
    pub fn support_of(&self, field: &str) -> FieldSupport {
        self.field_support()
            .iter()
            .find(|(name, _)| *name == field)
            .map(|(_, support)| *support)
            .unwrap_or(FieldSupport::Ignored {
                reason: "未知字段：协议演进中客户端可能提前发送，不因此拒绝请求",
            })
    }
}

// ===== Anthropic Messages =====

/// `top_k` 上游无对应参数；`metadata` 仅作客户端侧标记，转发无意义。
static ANTHROPIC_MESSAGES_FIELDS: &[(&str, FieldSupport)] = &[
    ("model", FieldSupport::Consumed),
    ("messages", FieldSupport::Consumed),
    ("system", FieldSupport::Consumed),
    ("max_tokens", FieldSupport::Consumed),
    ("stream", FieldSupport::Consumed),
    ("temperature", FieldSupport::Consumed),
    ("top_p", FieldSupport::Consumed),
    ("stop_sequences", FieldSupport::Consumed),
    ("tools", FieldSupport::Consumed),
    ("tool_choice", FieldSupport::Consumed),
    ("thinking", FieldSupport::Consumed),
    // 官方 effort 档位在顶层 output_config.effort（与 thinking 正交），
    // 归一化阶段已消费进 NormalizedRequest.effort
    ("output_config", FieldSupport::Consumed),
    // 上游 Kiro 不接受 top_k，转发会被忽略，不如明说
    (
        "top_k",
        FieldSupport::Rejected {
            reason: "top_k 暂不支持：上游 Kiro 未暴露该参数",
        },
    ),
    (
        "metadata",
        FieldSupport::Ignored {
            reason: "客户端侧标记，转发到上游无意义",
        },
    ),
];

// ===== OpenAI Chat Completions =====

static OPENAI_CHAT_FIELDS: &[(&str, FieldSupport)] = &[
    ("model", FieldSupport::Consumed),
    ("messages", FieldSupport::Consumed),
    ("stream", FieldSupport::Consumed),
    ("stream_options", FieldSupport::Consumed),
    ("max_tokens", FieldSupport::Consumed),
    ("temperature", FieldSupport::Consumed),
    ("top_p", FieldSupport::Consumed),
    ("stop", FieldSupport::Consumed),
    ("tools", FieldSupport::Consumed),
    ("tool_choice", FieldSupport::Consumed),
    // 明确拒绝：这些字段「看起来该生效」但没有上游对应物，
    // 静默丢弃会让客户端误判模型行为。
    (
        "response_format",
        FieldSupport::Rejected {
            reason: "response_format 暂不支持：上游 Kiro 无结构化输出开关",
        },
    ),
    (
        "modalities",
        FieldSupport::Rejected {
            reason: "modalities 暂不支持：上游仅文本",
        },
    ),
    (
        "audio",
        FieldSupport::Rejected {
            reason: "audio 暂不支持：上游仅文本",
        },
    ),
    (
        "prediction",
        FieldSupport::Rejected {
            reason: "prediction 暂不支持：上游无预设输出机制",
        },
    ),
    (
        "logprobs",
        FieldSupport::Rejected {
            reason: "logprobs 暂不支持：上游不回对数概率",
        },
    ),
    (
        "top_logprobs",
        FieldSupport::Rejected {
            reason: "top_logprobs 暂不支持：上游不回对数概率",
        },
    ),
    // 采样与计费类：实现无意义或会改变计费口径
    (
        "n",
        FieldSupport::Rejected {
            reason: "n 暂不支持：一次请求只产生一个 completion",
        },
    ),
    (
        "seed",
        FieldSupport::Ignored {
            reason: "确定性种子：上游不保证可复现，设置了反而误导",
        },
    ),
    (
        "presence_penalty",
        FieldSupport::Ignored {
            reason: "惩罚项：上游 Kiro 未暴露",
        },
    ),
    (
        "frequency_penalty",
        FieldSupport::Ignored {
            reason: "惩罚项：上游 Kiro 未暴露",
        },
    ),
    (
        "logit_bias",
        FieldSupport::Ignored {
            reason: "词元偏置：上游 Kiro 未暴露",
        },
    ),
    (
        "max_completion_tokens",
        FieldSupport::Ignored {
            reason: "旧参数字段：请使用 max_tokens",
        },
    ),
    (
        "store",
        FieldSupport::Ignored {
            reason: "服务端留存开关：本服务不按请求持久化对话",
        },
    ),
    (
        "metadata",
        FieldSupport::Ignored {
            reason: "客户端侧标记，转发无意义",
        },
    ),
    (
        "user",
        FieldSupport::Ignored {
            reason: "终端用户标识：归因请使用客户端 Key",
        },
    ),
    (
        "parallel_tool_calls",
        FieldSupport::Ignored {
            reason: "并行工具开关：由 tool_choice 决定，无独立控制",
        },
    ),
    (
        "service_tier",
        FieldSupport::Ignored {
            reason: "服务分层：上游单一链路",
        },
    ),
];

// ===== OpenAI Responses =====

/// `text.format`：Responses 的结构化输出开关，上游无对应物。
/// 作为命名常量供表与路径判定共用，避免两处各写一份文案。
const REJECTED_RESPONSES_TEXT_FORMAT: FieldSupport = FieldSupport::Rejected {
    reason: "text.format 暂不支持：上游 Kiro 无结构化输出开关",
};

static OPENAI_RESPONSES_FIELDS: &[(&str, FieldSupport)] = &[
    ("format", REJECTED_RESPONSES_TEXT_FORMAT),
    ("model", FieldSupport::Consumed),
    ("input", FieldSupport::Consumed),
    ("previous_response_id", FieldSupport::Consumed),
    ("stream", FieldSupport::Consumed),
    ("max_output_tokens", FieldSupport::Consumed),
    ("temperature", FieldSupport::Consumed),
    ("top_p", FieldSupport::Consumed),
    ("tools", FieldSupport::Consumed),
    ("tool_choice", FieldSupport::Consumed),
    ("instructions", FieldSupport::Consumed),
    // `text` 本身是合法容器（可含 citation 等），只有 `text.format`
    // 是结构化输出开关——由 find_rejected_path 按路径判定，此处不整体拒绝。
    (
        "text",
        FieldSupport::Ignored {
            reason: "文本输出容器：其中的 format 子字段单独判定",
        },
    ),
    (
        "truncation",
        FieldSupport::Ignored {
            reason: "截断策略：上游自行决定",
        },
    ),
    (
        "include",
        FieldSupport::Ignored {
            reason: "附加输出项：响应构造已固定",
        },
    ),
    (
        "metadata",
        FieldSupport::Ignored {
            reason: "客户端侧标记，转发无意义",
        },
    ),
    (
        "parallel_tool_calls",
        FieldSupport::Ignored {
            reason: "并行工具开关：由 tool_choice 决定",
        },
    ),
    (
        "store",
        FieldSupport::Ignored {
            reason: "服务端留存开关：会话仅保留在内存（见 responses_sessions）",
        },
    ),
    (
        "reasoning",
        FieldSupport::Consumed,
    ),
];

/// 检查 payload 里是否有该协议 `Rejected` 的字段，有则返回 400 文案。
///
/// 只查顶层字段；嵌套结构（如 `text.format`）由调用方按路径传入。
pub fn find_rejected_field(protocol: Protocol, payload: &Value) -> Option<(String, String)> {
    let obj = payload.as_object()?;
    for (name, support) in protocol.field_support() {
        if let FieldSupport::Rejected { reason } = support {
            // 字段存在且非 null 才算「客户端真的设了」
            if obj.get(*name).is_some_and(|v| !v.is_null()) {
                return Some((name.to_string(), reason.to_string()));
            }
        }
    }
    None
}

/// 按嵌套路径查 `Rejected` 字段（如 Responses 的 `text.format`）。
pub fn find_rejected_path(
    protocol: Protocol,
    payload: &Value,
    path: &[&str],
) -> Option<(String, String)> {
    let mut current = payload;
    for segment in path {
        current = current.get(segment)?;
    }
    if current.is_null() {
        return None;
    }
    // 路径命中时，用最后一段名查判定表
    let leaf = path.last()?;
    match protocol.support_of(leaf) {
        FieldSupport::Rejected { reason } => Some((leaf.to_string(), reason.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_protocol_table_is_non_empty_and_has_consumed_fields() {
        for protocol in [
            Protocol::AnthropicMessages,
            Protocol::OpenAIChatCompletions,
            Protocol::OpenAIResponses,
        ] {
            let table = protocol.field_support();
            assert!(!table.is_empty(), "{protocol:?} table must not be empty");
            assert!(
                table
                    .iter()
                    .any(|(_, s)| matches!(s, FieldSupport::Consumed)),
                "{protocol:?} must consume something"
            );
        }
    }

    #[test]
    fn unknown_fields_are_ignored_not_rejected() {
        // 协议演进中客户端会提前发新字段，不能因此让请求失败
        assert_eq!(
            Protocol::OpenAIChatCompletions.support_of("some_future_field"),
            FieldSupport::Ignored {
                reason: "未知字段：协议演进中客户端可能提前发送，不因此拒绝请求",
            }
        );
    }

    #[test]
    fn find_rejected_field_detects_present_but_ignores_null() {
        let payload = json!({ "model": "m", "response_format": { "type": "json" } });
        let found = find_rejected_field(Protocol::OpenAIChatCompletions, &payload)
            .expect("response_format should be rejected");
        assert_eq!(found.0, "response_format");

        // null 不算「客户端设了」
        let payload = json!({ "model": "m", "response_format": null });
        assert!(find_rejected_field(Protocol::OpenAIChatCompletions, &payload).is_none());
    }

    #[test]
    fn find_rejected_path_handles_nested_responses_text_format() {
        let payload = json!({ "model": "m", "text": { "format": { "type": "json" } } });
        let found = find_rejected_path(
            Protocol::OpenAIResponses,
            &payload,
            &["text", "format"],
        )
        .expect("text.format should be rejected");
        assert_eq!(found.0, "format");

        let payload = json!({ "model": "m", "text": { "format": null } });
        assert!(find_rejected_path(
            Protocol::OpenAIResponses,
            &payload,
            &["text", "format"]
        )
        .is_none());
    }

    #[test]
    fn anthropic_top_k_is_rejected() {
        let payload = json!({ "model": "m", "top_k": 40 });
        let found = find_rejected_field(Protocol::AnthropicMessages, &payload)
            .expect("top_k should be rejected");
        assert_eq!(found.0, "top_k");
    }

    /// R5：Anthropic 表收录 output_config（effort 真实消费）；
    /// Responses 表 reasoning 现在是 Consumed（reasoning.effort + reasoning item）。
    #[test]
    fn effort_fields_are_consumed() {
        assert_eq!(
            Protocol::AnthropicMessages.support_of("output_config"),
            FieldSupport::Consumed
        );
        assert_eq!(
            Protocol::OpenAIResponses.support_of("reasoning"),
            FieldSupport::Consumed
        );
    }
}
