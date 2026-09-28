//! 网关反向代理：把下游请求（Anthropic / OpenAI）转发到 Kiro 上游，
//! 并把上游响应与流式事件转换回下游协议。
//!
//! 按职责拆分为若干子模块，本文件负责共享导入、模块声明与对外 API 汇聚。

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::Local;
use futures_util::StreamExt;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    clients::{
        http_client::{
            build_kiro_custom_user_agent, build_kiro_x_amz_user_agent,
            build_streaming_http_client_for_account, resolve_kiro_upstream_region,
            should_add_redirect_for_internal, should_send_codewhisperer_optout,
        },
        kiro_client::{build_generate_assistant_response_url, build_kiro_runtime_host, KiroClient},
    },
    commands::common::{
        account_machine_id_or_new, get_usage_by_account, is_token_expired,
        refresh_token_by_provider_with_account_proxy, resolve_profile_arn_from_candidates,
        update_account_status, RefreshResult,
    },
    core::account::{Account, AccountStore},
};

const MAX_FAILURES_PER_ACCOUNT: u32 = 3;
const MAX_KIRO_PAYLOAD_SIZE: usize = 450 * 1024; // 450KB - Kiro API 的 HTTP 请求大小限制（更保守）

// Token 限制的默认值（当无法从 API 获取时使用）
#[allow(dead_code)]
const SUMMARIZATION_THRESHOLD_PERCENT: f64 = 0.55; // 55% 触发裁剪（预留更多安全空间，避免 Kiro IDE 上下文导致超限）
const COUNT_TOKENS_SAFETY_MULTIPLIER: f64 = 1.15;

use super::{
    append_gateway_request_log,
    converter::{
        build_kiro_payload, get_available_models, normalize_anthropic_request,
        normalize_openai_chat_payload, normalize_openai_responses_request,
    },
    effective_client_api_keys,
    eventstream::decode_message,
    models::{
        AnthropicContentBlock, AnthropicMessagesRequest, AnthropicMessagesResponse, AnthropicUsage,
        ModelsResponse, NormalizedMessage, NormalizedRequest, OpenAIChatRequest, Tool, ToolCall,
        ToolCallFunction,
    },
    stream::{self, parse_kiro_event_full, KiroEvent},
    thinking_parser::{SegmentType, ThinkingParser},
    GatewayConfig, GatewayRequestLogEntry, ResponseFormat, ResponsesSessionEntry, RouterState,
    DEFAULT_AGENT_MODE,
};

mod auth;
mod tokens;
mod logging;
mod handlers;
mod session;
mod responses;
mod errors;
mod upstream;
mod streaming;
mod websearch;

pub use auth::*;
pub use tokens::*;
pub use logging::*;
pub use handlers::*;
pub use session::*;
pub use responses::*;
pub use errors::*;
pub use upstream::*;
pub use streaming::*;

#[cfg(test)]
mod tests;
