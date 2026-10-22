//! Kiro 上游（CodeWhisperer）请求 / 响应 / EventStream 模型。

use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroPayload {
    pub conversation_state: ConversationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_arn: Option<String>,
    /// 官方 thinking 通道：4.6+ 模型接受顶层 additionalModelRequestFields，
    /// 上游由此返回原生 reasoningContentEvent（带签名）；4.5 代际与 GPT-5.6
    /// 不支持该字段，必须保持 None（盲传会被上游 400 / 整体跳过）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_request_fields: Option<serde_json::Value>,
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
    pub cache_point: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_cache_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<DocumentBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input_message_context: Option<UserInputMessageContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageBlock {
    pub format: String,
    pub source: ImageSource,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ImageSource {
    Bytes {
        bytes: String, // Base64 编码的图片数据
    },
    Other {
        #[serde(flatten)]
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentBlock {
    pub format: String,
    pub name: String,
    pub source: DocumentSource,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum DocumentSource {
    Bytes {
        bytes: String, // Base64 编码的字节数据
    },
    // 官方 DocumentSource union 仅含 bytes（extension.js Smithy schema
    // DocumentSource$ 与 ImageSource$ 同构），无 Files API file_id 通道。
    Other {
        #[serde(flatten)]
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputMessageContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_studio_context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_state: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_state: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_state: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_state: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_state: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<KiroToolResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<KiroTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_settings: Option<serde_json::Value>,
}

// Tool 是联合类型

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum KiroTool {
    CachePoint {
        #[serde(rename = "cachePoint")]
        cache_point: serde_json::Value,
    },
    ToolSpecification {
        #[serde(rename = "toolSpecification")]
        tool_specification: KiroToolSpec,
    },
    Other {
        #[serde(flatten)]
        data: serde_json::Value,
    },
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

// ToolResultContentBlock 是联合类型

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum KiroToolResultContent {
    Text {
        text: String,
    },
    Json {
        json: serde_json::Value,
    },
    Other {
        #[serde(flatten)]
        data: serde_json::Value,
    },
}

// AWS EventStream 响应事件类型

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AssistantResponseEvent {
    pub content: String,
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_continuation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ToolUseEvent {
    pub tool_use_id: String,
    pub name: String,
    pub input: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MessageMetadataEvent {
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_continuation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UsageInfo {
    pub input_tokens: i32,
    pub output_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i32>,
}

// ============================================================================
// OpenAI Responses API 结构体
// ============================================================================

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
    pub documents: Option<Vec<DocumentBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input_message_context: Option<UserInputMessageContext>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAssistantMessage {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_uses: Option<Vec<KiroToolUse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplementary_web_links: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_prompt: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_point: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroToolUse {
    pub name: String,
    pub input: serde_json::Value,
    pub tool_use_id: String,
}
