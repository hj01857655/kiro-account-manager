//! 协议转换层：把下游（Anthropic Messages / OpenAI Chat / OpenAI Responses）
//! 请求归一化为内部结构，再组装成 Kiro 上游 payload。
//!
//! 按职责拆分为若干子模块，本文件负责共享导入、模块声明与对外 API 汇聚。

use crate::clients::http_client::apply_app_proxy;
use crate::gateway::models::{
    AnthropicMessagesRequest, ConversationState, CurrentMessage, HistoryAssistantMessage,
    HistoryItem, HistoryUserMessage, ImageBlock, ImageSource, KiroInputSchema, KiroPayload,
    KiroTool, KiroToolResult, KiroToolResultContent, KiroToolSpec, KiroToolUse, ModelInfo,
    NormalizedMessage, NormalizedRequest, OpenAIChatRequest, Thinking, Tool, ToolCall,
    ToolCallFunction, ToolFunction, UserInputMessage, UserInputMessageContext,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::Client;
use serde_json::{json, Map, Value};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};
use tokio::net::lookup_host;
use uuid::Uuid;

mod normalize;
mod models;
mod tools;
mod images;
mod content;
mod history;
mod payload;
mod effort;

pub use normalize::*;
pub use models::*;
pub use tools::*;
pub use images::*;
pub use content::*;
pub use history::*;
pub use payload::*;
pub use effort::*;

/// 工具描述长度上限，超过则外移到 system prompt 的 Tool Documentation 段。
pub const TOOL_DESCRIPTION_MAX_LENGTH: usize = 10237;
const MAX_IMAGE_SOURCE_BYTES: usize = 5 * 1024 * 1024;
const MAX_IMAGE_REDIRECTS: usize = 3;
const IMAGE_FETCH_TIMEOUT_SECONDS: u64 = 15;

#[cfg(test)]
mod tests;
