//! OpenAI Chat Completions 协议模型。

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub max_completion_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub tools: Option<Vec<OpenAITool>>,
    pub tool_choice: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    pub modalities: Option<Vec<String>>,
    #[serde(default)]
    #[allow(dead_code)]
    pub audio: Option<AudioParams>,
    #[serde(default)]
    #[allow(dead_code)]
    pub prediction: Option<PredictionConfig>,
    #[serde(default)]
    pub stream_options: Option<StreamOptions>,
    #[serde(default)]
    #[allow(dead_code)]
    pub response_format: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    pub store: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    pub user: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub n: Option<i32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub seed: Option<i64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub logit_bias: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    pub logprobs: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub top_logprobs: Option<i32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub service_tier: Option<String>,
    /// OpenAI Chat 的 effort 档位（reasoning_effort）
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// 旧版 function 角色用 name 标识函数；映射为 tool 时作 tool_call_id 回退
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub audio: Option<AudioInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAIChatChoice>,
    pub usage: OpenAIChatUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatChoice {
    pub index: i32,
    pub message: OpenAIChatResponseMessage,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub refusal: Option<String>,
    pub annotations: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIResponseToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<OpenAIFunctionCall>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIResponseToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIToolCallFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatPromptTokensDetails {
    pub cached_tokens: i32,
    pub audio_tokens: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatCompletionTokensDetails {
    pub reasoning_tokens: i32,
    pub audio_tokens: i32,
    pub accepted_prediction_tokens: i32,
    pub rejected_prediction_tokens: i32,
}

// OpenAI Chat Completions API 使用量统计

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub prompt_tokens_details: OpenAIChatPromptTokensDetails,
    pub completion_tokens_details: OpenAIChatCompletionTokensDetails,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    pub choices: Vec<OpenAIChatChunkChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIChatUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatChunkChoice {
    pub index: i32,
    pub delta: OpenAIChatDelta,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: Option<String>,
}

// 已废弃的 function_call (案例6 - Functions, 向后兼容)

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

// 音频输入参数 (案例10 - Audio input)

#[derive(Debug, Clone, Deserialize)]
pub struct AudioInput {
    #[allow(dead_code)]
    pub id: String,
}

// 音频配置参数 (案例10 - Audio parameters)

#[derive(Debug, Clone, Deserialize)]
pub struct AudioParams {
    #[allow(dead_code)]
    pub voice: String,
    #[allow(dead_code)]
    pub format: String,
}

// 预测输出配置 (案例 - Predicted outputs)

#[derive(Debug, Clone, Deserialize)]
pub struct PredictionConfig {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub prediction_type: String,
    #[allow(dead_code)]
    pub content: serde_json::Value,
}

// 流式选项 (用于获取usage统计)

#[derive(Debug, Clone, Deserialize)]
pub struct StreamOptions {
    #[serde(default)]
    pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIDeltaToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<OpenAIFunctionCall>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIDeltaToolCall {
    pub index: i32,
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIToolCallFunction,
}

// 音频输出 (案例10 - With audio)

#[derive(Debug, Clone, Serialize)]
pub struct AudioOutput {
    pub id: String,
    pub expires_at: i64,
    pub data: String,        // base64 编码的音频数据
    pub transcript: String,
}
