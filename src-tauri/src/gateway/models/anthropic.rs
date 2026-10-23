//! Anthropic Messages 协议模型。

use super::*;

fn default_max_tokens() -> i32 {
    4096
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    #[serde(default)]
    pub system: Option<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    #[allow(dead_code)]
    pub top_k: Option<i32>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<AnthropicTool>>,
    pub tool_choice: Option<serde_json::Value>,
    pub thinking: Option<Thinking>,
    /// 顶层 output_config（官方 effort 档位放在 output_config.effort）
    #[serde(default)]
    pub output_config: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnthropicTool {
    #[serde(default)]
    pub r#type: Option<String>,
    /// 客户端自定义工具必填；官方 client toolset（`computer_toolset_20260801` /
    /// `browser_toolset_20260801`）是「一个无 name 的条目，声明固定成员集」——
    /// 缺省为空串，否则整个请求在反序列化阶段就 400。
    #[serde(default)]
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub cache_control: Option<serde_json::Value>,
    /// Anthropic Tool Search 规范（tool_search_tool_*）：客户端标记按需加载。
    /// normalize 层收集名单，请求侧拆分见 proxy/tool_search.rs。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// 限定谁能调用该工具：`direct`（模型直接调，缺省即此）/ `code_execution_*`
    /// （仅在代码执行沙箱内可调）。上游 Kiro 无代码执行环境，因此只声明了
    /// `code_execution_*` 的工具实际无法被调用——normalize 层据此告警。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<String>>,
    /// client toolset（computer/browser）的成员开关：`{成员名: {enabled, defer_loading}}`。
    /// 成员 schema 由官方固定，本网关暂不展开（见 normalize/anthropic.rs 的告警）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Anthropic thinking block signature (from Kiro reasoningContentEvent.signature)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Anthropic `redacted_thinking` 块的密文（原样来自 Kiro `redactedContent`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i32>,
}
