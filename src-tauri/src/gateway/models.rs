use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct NormalizedRequest {
    pub model: String,
    pub messages: Vec<NormalizedMessage>,
    #[serde(default)]
    pub stream: bool,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub tools: Option<Vec<Tool>>,
    #[allow(dead_code)] // 当前归一化层保留该字段用于协议对齐与测试校验，后续 Kiro payload 尚未消费
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search: Option<WebSearchToolOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebSearchToolOptions {
    pub max_uses: Option<i32>,
    pub allowed_domains: Option<Vec<String>>,
    pub blocked_domains: Option<Vec<String>>,
    pub user_location: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroPayload {
    pub conversation_state: ConversationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationState {
    pub chat_trigger_type: String,
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_continuation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_task_type: Option<String>,
    pub current_message: CurrentMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<HistoryItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customization_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMessage {
    pub user_input_message: UserInputMessage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputMessage {
    pub content: String,
    pub model_id: String,
    pub origin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input_message_context: Option<UserInputMessageContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<InferenceConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageBlock {
    pub format: String,
    pub source: ImageSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageSource {
    pub bytes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputMessageContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<KiroTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<KiroToolResult>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroTool {
    pub tool_specification: KiroToolSpec,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: KiroInputSchema,
}

#[derive(Debug, Clone, Serialize)]
pub struct KiroInputSchema {
    pub json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroToolResult {
    pub content: Vec<KiroToolResultContent>,
    pub status: String,
    pub tool_use_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KiroToolResultContent {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum HistoryItem {
    User {
        #[serde(rename = "userInputMessage")]
        user_input_message: HistoryUserMessage,
    },
    Assistant {
        #[serde(rename = "assistantResponseMessage")]
        assistant_response_message: HistoryAssistantMessage,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryUserMessage {
    pub content: String,
    pub model_id: String,
    pub origin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input_message_context: Option<UserInputMessageContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<InferenceConfig>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAssistantMessage {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_uses: Option<Vec<KiroToolUse>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroToolUse {
    pub name: String,
    pub input: serde_json::Value,
    pub tool_use_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: i32,
    #[serde(default)]
    pub system: Option<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<AnthropicTool>>,
    pub tool_choice: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicTool {
    #[serde(default)]
    pub r#type: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub max_uses: Option<i32>,
    #[serde(default)]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(default)]
    pub blocked_domains: Option<Vec<String>>,
    #[serde(default)]
    pub user_location: Option<serde_json::Value>,
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
}
