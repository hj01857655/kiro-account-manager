mod compress;
pub mod field_support;
mod converter;
mod eventstream;
pub(crate) mod load_balancer;
pub mod log_store;
pub mod masking;
pub mod otlp;
pub mod prompt_info;
mod models;
pub mod prompt_cache;
pub mod prompt_filter;
mod proxy;
pub mod response_cache;
mod stream;
pub(crate) mod stop_sequence;
mod thinking_parser;
mod token_cache;
mod token_estimator;

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method},
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use reqwest::Client;
use tower_http::cors::{AllowOrigin, CorsLayer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Write},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
use tauri::{AppHandle, Manager};
use tokio::{
    net::TcpListener,
    sync::{oneshot, Mutex as AsyncMutex},
    task::JoinHandle,
};

use crate::clients::http_client::{
    build_streaming_http_client_with_upstream_proxy, is_supported_kiro_region,
};
use crate::gateway::token_cache::TokenCache;

/// 获取可用模型列表（仅用于前端展示，返回 model ID 列表）
pub fn get_available_models() -> Vec<String> {
    converter::get_available_models()
        .into_iter()
        .map(|model| model.id)
        .collect()
}

#[cfg(test)]
thread_local! {
    static REQUEST_LOG_PATH_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn request_log_path_override() -> Option<PathBuf> {
    REQUEST_LOG_PATH_OVERRIDE.with(|cell| cell.borrow().clone())
}

#[cfg(test)]
fn set_request_log_path_override(path: Option<PathBuf>) {
    REQUEST_LOG_PATH_OVERRIDE.with(|cell| *cell.borrow_mut() = path);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// 客户端鉴权密钥：固定为 `kiro`，不再可配（见 `CLIENT_API_KEY`）。
    ///
    /// 以下三个历史字段仅做兼容读取，认证一律走常量：
    /// - `access_token` / `client_api_keys` / `client_keys` 是多 Key 时代的
    ///   存储与文本镜像（`name:key`、`#disabled#` 编码），已整体废弃。
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub client_api_keys: Vec<String>,
    #[serde(default)]
    pub client_keys: Vec<serde_json::Value>,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_account_mode")]
    pub account_mode: String,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub pool_account_ids: Vec<String>,
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(default = "default_threshold")]
    pub threshold: i32,
    #[serde(default = "default_local_only")]
    pub local_only: bool,
    #[serde(default)]
    pub allowed_ips: Vec<String>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// 系统提示过滤：检测 Claude Code 系统提示并替换为精简版
    #[serde(default)]
    pub filter_claude_code: bool,
    /// 系统提示过滤：去掉 --- SYSTEM PROMPT --- 边界标记
    #[serde(default)]
    pub filter_strip_boundaries: bool,
    /// 系统提示过滤：去掉环境噪音行（git status、recent commits 等）
    #[serde(default)]
    pub filter_env_noise: bool,
    /// 自定义提示过滤规则
    #[serde(default)]
    pub prompt_filter_rules: Vec<PromptFilterRule>,
    /// 网关上游（Kiro API）显式代理：非空时优先于账号代理与应用代理链。
    /// 支持 http/https/socks5 URL；空 = 维持原隐式解析（账号代理 > 应用代理模式 > 环境变量）。
    #[serde(default)]
    pub upstream_proxy: Option<String>,
    /// 工具描述长度上限（字节）。超过的描述外移到系统提示的 Tool Documentation 段，
    /// 防止超长工具描述触发上游 4xx（社区 kiro2api 同款配置项）。
    /// 默认 10237 = 与旧版硬编码一致，行为不变；0 = 不限制。
    #[serde(default = "default_max_tool_desc_len")]
    pub max_tool_description_length: u32,
    /// 是否记录请求日志
    #[serde(default = "default_true_val")]
    pub log_requests: bool,
    /// 响应缓存：是否启用
    #[serde(default = "default_true_val")]
    pub response_cache_enabled: bool,
    /// 响应缓存：TTL（秒）
    #[serde(default = "default_cache_ttl")]
    pub response_cache_ttl: u64,
    /// WebSearch 子请求处理：Claude Code 强制 web_search 工具调用时，
    /// 用账号凭证调 Kiro 远程 MCP（runtime /mcp）执行搜索并注入结果。
    /// 关闭后此类子请求会因 tool_choice 无法映射而被拒绝。
    #[serde(default = "default_true_val")]
    pub web_search_enabled: bool,
    /// Tool Search 工具发现规范（Anthropic defer_loading）：拆分 defer_loading
    /// 工具并注入合成 tool_search 工具，模型按需检索定义（见
    /// proxy/tool_search.rs）。非流式直改请求；流式经缓冲代答环处理，
    /// 客户端仍收标准 SSE。关闭后 defer_loading 字段被忽略，行为与旧版一致。
    #[serde(default = "default_true_val")]
    pub tool_search_enabled: bool,
    /// 会话粘性：同客户端 IP 固定路由到同一账号。
    /// 上游 prompt cache 按账号维度生效，同客户端在账号间跳动会反复丢缓存，
    /// 固定路由可显著提升缓存命中（日志缓存读列可见）；顺带分散风控面。
    /// 指定 x-account-id 时该头优先，粘性不介入。
    #[serde(default)]
    pub session_affinity: bool,
    /// 记录流式逐事件明细（SSE event log）：每个流式请求多落一个事件文件。
    /// 排查流式协议问题（toolUse 分片、reasoning 分片顺序）时打开，
    /// 平时关闭省磁盘。关闭不影响请求日志主条目。
    #[serde(default)]
    pub log_stream_events: bool,
    /// OTLP 导出：请求日志转 OpenTelemetry 指标推送到外部观测栈（OTLP/HTTP JSON）。
    /// 空 = 关闭。显式开启才外发，且只发元数据（模型/账号/会话/tokens/credits），
    /// 不含 prompt 与响应内容。
    #[serde(default)]
    pub otlp_endpoint: Option<String>,
    /// OTLP 请求附带的鉴权头（如 Authorization: Bearer ...），JSON 对象序列化字符串
    #[serde(default)]
    pub otlp_headers: Option<String>,
    /// 隐私遮蔽：请求离开本机前把敏感值换成占位符，响应回来再换回真值。
    /// 默认关：开启后所有发往上游的文本（messages/system/tools/input）
    /// 都会过一遍规则引擎，极小概率误伤（见 masking.rs 的规则清单）。
    /// 真实值只在内存，不落盘；遮蔽失败时 fail-closed 拒绝请求。
    #[serde(default)]
    pub privacy_mask_enabled: bool,
    /// 遮蔽规则的用户覆盖：按类别改动作/停用。空 = 全部内置默认。
    #[serde(default)]
    pub mask_kind_overrides: Vec<masking::MaskKindOverride>,
    /// 用户自定义遮蔽规则（自己的正则）。编译失败的规则在启动时跳过。
    #[serde(default)]
    pub mask_custom_rules: Vec<masking::MaskCustomRule>,
    /// 永不遮蔽的值清单（精确匹配优先于一切规则）。
    #[serde(default)]
    pub mask_never_mask: Vec<masking::MaskNeverMask>,
    /// 遮蔽的同形替换：密钥类改送「同形假密钥」而非 `{{API_KEY_xxx}}` 占位符。
    ///
    /// 占位符会被 agent/工具的 key 格式校验判为非法而拒收，模型也会
    /// 特意提一句「你的密钥格式不对」；同形假密钥（同前缀、同长度、
    /// 同字符类别）两关都能过。短密钥与含特殊字符的值会自动降级回占位符。
    /// 默认关：保持传统占位符行为。
    #[serde(default)]
    pub mask_shape: bool,
}

fn default_cache_ttl() -> u64 {
    180
}

fn default_max_tool_desc_len() -> u32 {
    crate::gateway::converter::TOOL_DESCRIPTION_MAX_LENGTH as u32
}

/// 自定义提示过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFilterRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    /// regex | lines-containing
    pub rule_type: String,
    /// 匹配模式（正则表达式或子串）
    pub match_pattern: String,
    /// 替换内容（仅 regex 类型使用，空 = 删除匹配）
    #[serde(default)]
    pub replace: String,
}

fn default_true_val() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub running: bool,
    pub host: String,
    pub port: u16,
    pub request_count: u64,
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_config: Option<GatewayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRequestLogEntry {
    pub occurred_at: String,
    /// 全局唯一的请求 ID（UUID）
    pub request_id: String,
    pub request_index: u64,
    pub endpoint: String,
    pub client_ip: String,
    /// 客户端发来的 User-Agent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_user_agent: Option<String>,
    /// 本次请求实际使用的账号 ID（额度/账单按账号聚合的依据）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// 会话标识：x-session-id 请求头，缺省时回退「客户端 IP|UA」派生键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub status_code: u16,
    pub outcome: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    /// Prompt Caching: 输入 tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i32>,
    /// Prompt Caching: 输出 tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i32>,
    /// Prompt Caching: 缓存读取 tokens（节省 90% 成本）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i32>,
    /// Prompt Caching: 缓存写入 tokens（首次写入成本 +25%）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i32>,
    /// 上游 meteringEvent 给出的本次请求计费消耗（credits，真实值非估算）。
    /// 仅流式路径可得：非流式响应体不带该事件时为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<f64>,
    /// 上游 contextUsageEvent 给出的上下文窗口占用百分比（0-100）。
    /// 仅流式路径可得：上游在流末帧报告本次请求占了模型窗口多少。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_usage_percentage: Option<f32>,
    /// Prompt 构成（system/tools/memory/files/results/chat）：
    /// 请求到达时按 body 估算，上游 usage 回来后等比校准（counted=true）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<prompt_info::PromptInfo>,
    /// 错误类型（如 invalid_request_error, authentication_error 等）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// 流式响应信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_info: Option<StreamInfo>,
    /// 请求摘要信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_summary: Option<RequestSummary>,
    /// 响应摘要信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_summary: Option<ResponseSummary>,
}

/// 流式响应信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub chunk_count: usize,
    /// 首字节时间（毫秒）
    pub first_chunk_ms: u64,
}

/// 请求摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSummary {
    pub message_count: usize,
    pub tool_count: usize,
    pub total_content_length: usize,
    pub has_images: bool,
}

/// 响应摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseSummary {
    pub content_length: usize,
    pub tool_calls_count: usize,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRequestStats {
    pub total: usize,
    pub success: usize,
    pub error: usize,
    pub streaming: usize,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_creation_tokens: i64,
    /// 上游 meteringEvent 的真实计费消耗合计（credits）
    pub total_credits: f64,
    pub requests_with_cache: usize,
    pub max_duration_ms: u64,
    pub avg_duration_ms: u64,
}

pub struct GatewayRuntime {
    pub config: GatewayConfig,
    pub request_count: Arc<AtomicU64>,
    pub last_error: Arc<AsyncMutex<Option<String>>>,
    pub log_store: Arc<log_store::LogStore>,
    pub response_cache: Arc<AsyncMutex<response_cache::ResponseCache>>,
    pub load_balancer: Arc<load_balancer::LoadBalancer>,
    /// 隐私遮蔽引擎（值→占位符映射表跨请求保留，同值永远同占位符）。
    /// `Arc<AsyncMutex<..>>`：mask_text 需要可变访问映射表。
    pub mask_engine: Arc<AsyncMutex<masking::MaskEngine>>,
    /// 遮蔽审计记录（内存环形缓冲，只存元数据不留真值）。
    pub mask_audit: Arc<masking::MaskAuditLog>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_task: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResponsesSessionEntry {
    #[allow(dead_code)]
    pub response_id: String,
    pub previous_response_id: Option<String>,
    pub request_messages: Vec<crate::gateway::models::NormalizedMessage>,
    pub request_tools: Option<Vec<crate::gateway::models::Tool>>,
    pub request_tool_choice: Option<serde_json::Value>,
    pub response_text: String,
    pub tool_calls: Vec<(String, String, String)>,
    pub updated_at: Instant,
}

pub(crate) type ResponsesSessionStore = Arc<AsyncMutex<HashMap<String, ResponsesSessionEntry>>>;

#[derive(Clone)]
struct RouterState {
    config: GatewayConfig,
    request_count: Arc<AtomicU64>,
    last_error: Arc<AsyncMutex<Option<String>>>,
    http: Client,
    responses_sessions: ResponsesSessionStore,
    #[allow(dead_code)]
    token_cache: Arc<AsyncMutex<TokenCache>>,
    load_balancer: Arc<load_balancer::LoadBalancer>,
    log_store: Arc<log_store::LogStore>,
    response_cache: Arc<AsyncMutex<response_cache::ResponseCache>>,
    /// 隐私遮蔽引擎（值→占位符映射表跨请求保留，同值永远同占位符）。
    /// `Arc<AsyncMutex<..>>`：mask_text 需要可变访问映射表。
    mask_engine: Arc<AsyncMutex<masking::MaskEngine>>,
    /// 遮蔽审计记录（内存环形缓冲，只存元数据不留真值）。
    mask_audit: Arc<masking::MaskAuditLog>,
    /// 账号状态的唯一共享副本（= `AppState.store`）。
    ///
    /// gateway 不得自行 `AccountStore::new()`：那会得到与 Tauri 命令
    /// 互不知情的独立快照，后写覆盖前写，静默丢失 token 与失败计数。
    store: crate::state::SharedAccountStore,
}

#[derive(Debug, Clone, Copy)]
enum ResponseFormat {
    Anthropic,
    Responses,
    OpenAI,
}

/// 应用数据目录（= Roaming/.kiro-account-manager）。
/// 详见 `core::paths`。
fn config_dir() -> PathBuf {
    crate::core::paths::app_data_dir_or_default()
}
const CONFIG_FILE: &str = "gateway-config.json";
const LOGS_DIR: &str = "logs";
const REQUEST_LOG_FILE: &str = "gateway-request-log.jsonl";
const DEFAULT_AGENT_MODE: &str = "vibe";

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8765
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_account_mode() -> String {
    "pool".to_string()
}

fn default_strategy() -> String {
    "round_robin".to_string()
}

fn default_threshold() -> i32 {
    90
}

fn default_local_only() -> bool {
    true
}

fn default_log_level() -> String {
    "debug".to_string()
}

fn build_bind_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    let normalized = host.trim();
    if normalized.is_empty() {
        return Err("监听地址不能为空".to_string());
    }

    if normalized.eq_ignore_ascii_case("localhost") {
        return Ok(SocketAddr::from(([127, 0, 0, 1], port)));
    }

    let bind_target = if normalized.contains(':') {
        format!("[{normalized}]:{port}")
    } else {
        format!("{normalized}:{port}")
    };

    bind_target
        .parse::<SocketAddr>()
        .map_err(|e| format!("监听地址无效: {e}"))
}
impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_host(),
            port: default_port(),
            access_token: None,
            client_api_keys: Vec::new(),
            client_keys: Vec::new(),
            region: default_region(),
            account_mode: default_account_mode(),
            account_id: None,
            group_id: None,
            pool_account_ids: Vec::new(),
            strategy: default_strategy(),
            threshold: default_threshold(),
            local_only: default_local_only(),
            allowed_ips: Vec::new(),
            log_level: default_log_level(),
            filter_claude_code: false,
            filter_strip_boundaries: false,
            filter_env_noise: false,
            prompt_filter_rules: Vec::new(),
            upstream_proxy: None,
            max_tool_description_length: default_max_tool_desc_len(),
            log_requests: true,
            response_cache_enabled: true,
            response_cache_ttl: default_cache_ttl(),
            web_search_enabled: true,
            tool_search_enabled: true,
            session_affinity: false,
            log_stream_events: false,
            otlp_endpoint: None,
            otlp_headers: None,
            privacy_mask_enabled: false,
            mask_kind_overrides: Vec::new(),
            mask_custom_rules: Vec::new(),
            mask_never_mask: Vec::new(),
            mask_shape: false,
        }
    }
}
/// 客户端鉴权密钥：固定为 `kiro`。
///
/// 多 Key 管理（生成 / 命名 / 启停 / 配额 / RPM）已整体移除。保留固定密钥
/// 是为了维持「客户端必须携带鉴权头」的协议形状，也是为了和把鉴权头当
/// 透传字段的客户端保持兼容。
pub const CLIENT_API_KEY: &str = "kiro";

impl GatewayStatus {
    pub fn stopped(config: &GatewayConfig) -> Self {
        Self {
            running: false,
            host: config.host.clone(),
            port: config.port,
            request_count: 0,
            last_error: None,
            runtime_config: None,
        }
    }
}

fn ensure_config_valid(config: &GatewayConfig) -> Result<(), String> {
    build_bind_addr(&config.host, config.port)?;
    if config.port == 0 {
        return Err("端口必须大于 0".to_string());
    }

    let region = config.region.trim();
    if region.is_empty() {
        return Err("region 不能为空".to_string());
    }
    if !is_supported_kiro_region(region) {
        return Err(format!("region 不受支持: {region}"));
    }
    match config.account_mode.as_str() {
        "single"
            if config
                .account_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            return Err("single 模式必须选择账号".to_string());
        }
        "group"
            if config
                .group_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            return Err("group 模式必须选择分组".to_string());
        }
        "pool" if config.pool_account_ids.is_empty() => {
            return Err("pool 模式必须至少选择一个账号".to_string());
        }
        "single" | "group" | "pool" => {}
        "local" => {
            return Err("kiro2api 不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string());
        }
        _ => return Err("accountMode 必须是 single/group/pool".to_string()),
    }
    if !matches!(
        config.log_level.as_str(),
        "debug" | "info" | "warn" | "error"
    ) {
        return Err("logLevel 必须是 debug/info/warn/error".to_string());
    }
    if config.max_tool_description_length > 100_000 {
        return Err("maxToolDescriptionLength 不能超过 100000".to_string());
    }
    if !config.local_only && config.allowed_ips.is_empty() {
        return Err("允许远程访问时必须至少配置一个白名单来源 IP".to_string());
    }
    for entry in &config.allowed_ips {
        if !is_valid_allowlist_entry(entry) {
            return Err(format!("白名单条目无效: {entry}"));
        }
    }
    if let Some(proxy) = config.upstream_proxy.as_deref() {
        let proxy = proxy.trim();
        if proxy.is_empty() {
            return Err("upstreamProxy 不能是空白字符串，留空请设为 null".to_string());
        }
        let scheme = proxy
            .split("://")
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let has_scheme = proxy.contains("://");
        if !has_scheme
            || !matches!(scheme.as_str(), "http" | "https" | "socks5" | "socks5h")
        {
            return Err(format!(
                "upstreamProxy 仅支持 http/https/socks5 URL，收到: {proxy}"
            ));
        }
    }
    Ok(())
}

fn is_valid_allowlist_entry(entry: &str) -> bool {
    let trimmed = entry.trim();
    !trimmed.is_empty()
        && (trimmed.parse::<IpAddr>().is_ok() || trimmed.parse::<ipnet::IpNet>().is_ok())
}

fn normalize_config(config: &GatewayConfig) -> GatewayConfig {
    let mut normalized = config.clone();
    normalized.host = normalized.host.trim().to_string();
    // 端口不再可配：固定 8765，Base URL 因此恒定，客户端只需配一次。
    // 旧配置里存的其它端口在这里被覆盖，无需用户手工改配置文件。
    // 字段本身保留，是为了不破坏已有配置文件的兼容读取。
    normalized.port = default_port();
    // 客户端密钥固定为 kiro，不再从配置读取。历史字段在归一化时清空，
    // 避免旧配置里的多 Key 数据继续以“真相”的姿态存在于配置文件中。
    normalized.access_token = None;
    normalized.client_api_keys = Vec::new();
    normalized.client_keys = Vec::new();
    normalized.region = normalized.region.trim().to_string();
    normalized.account_mode = normalized.account_mode.trim().to_string();
    normalized.strategy = normalized.strategy.trim().to_string();
    normalized.log_level = normalized.log_level.trim().to_ascii_lowercase();
    normalized.allowed_ips = normalized
        .allowed_ips
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .fold(Vec::new(), |mut acc, item| {
            if !acc.contains(&item) {
                acc.push(item);
            }
            acc
        });
    // 空串归一成 None，避免前端清空输入框后残留 "" 触发校验失败
    normalized.upstream_proxy = normalized
        .upstream_proxy
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    normalized
}

fn config_path() -> Result<PathBuf, String> {
    Ok(ensure_gateway_data_dir()?.join(CONFIG_FILE))
}

fn request_log_path() -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Some(path) = request_log_path_override() {
        return Ok(path);
    }
    Ok(gateway_log_dir_raw()?.join(REQUEST_LOG_FILE))
}

fn append_gateway_request_log_to_path(
    path: &Path,
    entry: &GatewayRequestLogEntry,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建请求日志目录失败: {e}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("打开请求日志失败: {e}"))?;
    let serialized =
        serde_json::to_string(entry).map_err(|e| format!("序列化请求日志失败: {e}"))?;
    writeln!(file, "{serialized}").map_err(|e| format!("写入请求日志失败: {e}"))
}

/// 单个网关原始报文日志的体积上限。超过即轮转为 `<name>.log.1`。
///
/// 背景：原始报文转储（`kiro-request.log` / `anthropic-messages-*.log` 等）原先是
/// 纯 `OpenOptions::append` 无限追加、从不回收，实测单文件涨到 221MB、
/// 整个 logs 目录合计约 372MB，只能靠用户手动清理。
const RAW_LOG_MAX_BYTES: u64 = 32 * 1024 * 1024;

/// 结构化请求日志（JSONL）的体积上限。
///
/// 它比 `*.log` 更容易涨爆：`request_body` / `response_body` 原样入库、
/// 没有截断，Claude Code 一个请求就能写进几百 KB。轮转为 `.jsonl.1`，
/// 备份扩展名不是 `jsonl`，不会反复轮转。
const REQUEST_LOG_MAX_BYTES: u64 = 32 * 1024 * 1024;

/// 受管日志扩展名对应的体积上限；返回 `None` 表示该文件不纳入轮转。
///
/// 独立成函数是为了可测：轮转本身要动真实日志目录，而这套「管哪些、不管哪些」
/// 的规则必须能被锁住——`.jsonl` 曾被漏掉，正是因为它扩展名不是 `log`。
fn managed_log_cap(extension: &str) -> Option<u64> {
    match extension {
        "log" => Some(RAW_LOG_MAX_BYTES),
        "jsonl" => Some(REQUEST_LOG_MAX_BYTES),
        _ => None,
    }
}

/// 收敛网关日志目录体积：把超过上限的 `*.log` / `*.jsonl` 轮转为 `<name>.1`，
/// 让后续追加从头开始。旧备份会被覆盖，故每个文件最多约占 2×上限。
///
/// - 由 `proxy_handler` 按固定请求间隔低频调用，不在热路径上每次执行。
/// - 轮转失败（文件被占用等）静默忽略，下次再试；不会影响正常请求。
/// - 备份扩展名（`log.1` / `jsonl.1`）不在受管集合内，不会反复轮转。
pub fn enforce_raw_log_caps() {
    let log_dir = crate::core::paths::app_data_dir_or_default().join(LOGS_DIR);
    let Ok(entries) = std::fs::read_dir(&log_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        let Some(cap) = managed_log_cap(extension) else {
            continue;
        };
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() || meta.len() <= cap {
            continue;
        }
        let backup = path.with_extension(format!("{extension}.1"));
        let _ = std::fs::remove_file(&backup);
        let _ = std::fs::rename(&path, &backup);
    }
}

fn get_gateway_request_logs_from_path(
    path: &Path,
    limit: Option<usize>,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path).map_err(|e| format!("读取请求日志失败: {e}"))?;
    let reader = BufReader::new(file);
    // limit 上限 20000：内存 LogStore 容量就是 10000，文件路径允许更大是给
    // 「7 天趋势」这类跨会话聚合用的（Usage/遮蔽页拉全量历史算时间轴）。
    // clamp 到 1 防 0 条的空洞语义。
    let max_items = limit.unwrap_or(100).clamp(1, 20_000);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<GatewayRequestLogEntry>(trimmed) {
            entries.push(entry);
        }
    }

    let start = entries.len().saturating_sub(max_items);
    let mut recent = entries.split_off(start);
    recent.reverse();
    Ok(recent)
}

fn clear_gateway_request_logs_at_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path).map_err(|e| format!("清空请求日志失败: {e}"))?;
    // 轮转备份（32MB 上限触发的 `.jsonl.1`）也一并删：
    // 只删主文件时，重启网关启动恢复会从备份把旧日志灌回内存，
    // 表现为「清空后过一会儿日志又回来了」。
    let backup = path.with_extension(format!(
        "{}.1",
        path.extension().and_then(|e| e.to_str()).unwrap_or("jsonl")
    ));
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    Ok(())
}

fn gateway_data_dir() -> PathBuf {
    // 统一走 paths 模块
    config_dir()
}

fn ensure_gateway_data_dir() -> Result<PathBuf, String> {
    let dir = gateway_data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    Ok(dir)
}

fn gateway_log_dir_raw() -> Result<PathBuf, String> {
    let dir = ensure_gateway_data_dir()?.join(LOGS_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
    Ok(dir)
}

pub fn load_gateway_config() -> Result<GatewayConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(GatewayConfig::default());
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    let cfg = serde_json::from_str::<GatewayConfig>(&content)
        .map_err(|e| format!("解析配置失败: {e}"))?;
    Ok(normalize_config(&cfg))
}

pub fn get_gateway_config() -> Result<GatewayConfig, String> {
    load_gateway_config()
}

/// 就地丢弃 `pool_account_ids` 里指向已删除账号的 id。
///
/// 与 `prune_deleted_pool_accounts` 同一个目的：那一个是删除账号后回写文件，
/// 这一个在启动前就地校正内存里的配置。删账号时的清理可能失败（磁盘只读、
/// 配置被占用），悬空 id 会在启动这一刻被拦住，而不是带病把网关拉起来。
fn prune_deleted_pool_accounts_in_place(config: &mut GatewayConfig, store: &crate::state::SharedAccountStore) {
    if config.pool_account_ids.is_empty() {
        return;
    }
    let existing_ids: Vec<String> = {
        // 与 proxy/auth.rs 的 lock_shared_store 同一约定：poison 时仍取 into_inner，
        // 不因为一次 panic 就让启动路径整体失败
        let guard = store.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.accounts.iter().map(|account| account.id.clone()).collect()
    };
    config.pool_account_ids.retain(|id| existing_ids.iter().any(|existing| existing == id));
}

/// 丢弃 `pool_account_ids` 里指向已删除账号的 id。
///
/// 删除账号不会通知网关配置，池里的 id 会永久残留：账号全删光之后，
/// 「pool 模式」仍带着 N 个失效 id 通过 `ensure_config_valid`，运行期却一个
/// 账号都匹配不到（`proxy/auth.rs` 与 `proxy/upstream.rs` 都按 id 过滤），
/// 请求全部失败。删除账号是唯一会制造这种悬空引用的动作，故在此收敛。
pub fn prune_deleted_pool_accounts(existing_ids: &[String]) -> Result<(), String> {
    let mut config = load_gateway_config()?;
    if config.pool_account_ids.is_empty() {
        return Ok(());
    }
    let before = config.pool_account_ids.len();
    config
        .pool_account_ids
        .retain(|id| existing_ids.iter().any(|existing| existing == id));
    if config.pool_account_ids.len() == before {
        return Ok(());
    }
    // 池被清空时 pool 模式会变成非法配置。这里仍照实写盘，不走 save_gateway_config：
    // 那会让 ensure_config_valid 把这次清理整个挡掉，悬空 id 继续留在文件里。
    // 合法性由下次启动时暴露给用户，而不是拿一份看似有效的配置去启动网关。
    let path = config_path()?;
    let content = serde_json::to_string_pretty(&normalize_config(&config))
        .map_err(|e| format!("序列化配置失败: {e}"))?;
    fs::write(path, content).map_err(|e| format!("写入配置失败: {e}"))
}

pub fn save_gateway_config(config: &GatewayConfig) -> Result<(), String> {
    let normalized = normalize_config(config);
    ensure_config_valid(&normalized)?;
    let path = config_path()?;
    let content =
        serde_json::to_string_pretty(&normalized).map_err(|e| format!("序列化配置失败: {e}"))?;
    fs::write(path, content).map_err(|e| format!("写入配置失败: {e}"))
}

pub fn append_gateway_request_log(entry: &GatewayRequestLogEntry) -> Result<(), String> {
    let path = request_log_path()?;
    append_gateway_request_log_to_path(&path, entry)
}

pub async fn get_gateway_request_logs(
    state: &tauri::State<'_, crate::state::AppState>,
    limit: Option<usize>,
    session_only: bool,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    // 尝试从运行中的 gateway 的内存存储获取
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        // session_only=true 只取本次运行的条目（概览「最近请求」），
        // 不把启动回灌的历史误当成实时流量
        let logs = if session_only {
            log_store.get_last_session(limit.unwrap_or(50)).await
        } else {
            log_store.get_last(limit.unwrap_or(50)).await
        };
        return Ok(logs);
    }

    // gateway 未运行：session_only 语义下没有「本次运行」可言
    if session_only {
        return Ok(Vec::new());
    }

    // 如果 gateway 未运行，从文件读取
    let path = request_log_path()?;
    get_gateway_request_logs_from_path(&path, limit)
}

/// 读落盘的全量请求日志（跨会话历史，与内存视图分离）。
///
/// 与 `get_gateway_request_logs` 的区别：**无条件走文件**——网关运行中也一样。
/// 内存 LogStore 上限 10000 条且重启即丢，而「7 天趋势」这类跨会话聚合
/// 需要文件里的完整历史（Usage / 遮蔽审计页的时间轴）。
/// 文件不存在（从未产生过请求）时返回空 Vec。
pub async fn get_gateway_request_log_history(
    limit: Option<usize>,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    let path = request_log_path()?;
    get_gateway_request_logs_from_path(&path, limit)
}

/// 额度页单账号条目：上游配额快照 + 本地聚合的消耗量。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayQuotaEntry {
    pub account_id: String,
    pub email: Option<String>,
    pub provider: Option<String>,
    pub region: Option<String>,
    /// 上游 getUsageLimits 原始 JSON（窗口、剩余额度）
    pub usage: serde_json::Value,
    /// 本地历史日志按账号聚合的 credits 消耗（account_id 字段上线后的请求才计入）
    pub credits_consumed: f64,
    pub total_requests: u64,
    pub total_tokens: i64,
    /// 该账号查询失败时的错误信息（token 失效 / 封禁等），None 表示查询成功
    pub error: Option<String>,
}

/// 额度页：网关作用域内每个账号的配额与本地消耗聚合。
///
/// 上游配额走 `get_usage_by_account`（与账号管理页同一实现，不刷新 token，
/// 失效即报错不静默）；本地消耗从落盘 JSONL 按 account_id 聚合
/// （上限 20000 条，account_id 字段上线前的历史请求无法归属，不计入）。
pub async fn get_gateway_quota(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<GatewayQuotaEntry>, String> {
    // 网关作用域：运行中用运行配置，否则用落盘配置
    let config = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;
        guard.as_ref().map(|rt| rt.config.clone())
    }
    .unwrap_or_else(|| load_gateway_config().unwrap_or_default());

    // 先收集作用域内账号的 owned 副本并立即释放锁：
    // std MutexGuard 跨 .await 会让 future 变成非 Send（Tauri 命令要求 Send）
    let scoped: Vec<crate::core::account::Account> = {
        let store_guard = state
            .store
            .lock()
            .map_err(|_| "获取账号存储失败".to_string())?;
        match config.account_mode.as_str() {
            "pool" => store_guard
                .accounts
                .iter()
                .filter(|a| config.pool_account_ids.contains(&a.id))
                .cloned()
                .collect(),
            _ => store_guard
                .accounts
                .iter()
                .filter(|a| config.account_id.as_deref() == Some(a.id.as_str()))
                .cloned()
                .collect(),
        }
    };

    // 本地消耗聚合：按 account_id 累加 credits / 请求数 / tokens
    let history = get_gateway_request_log_history(Some(20_000))
        .await
        .unwrap_or_default();
    let mut agg: std::collections::HashMap<String, (f64, u64, i64)> =
        std::collections::HashMap::new();
    for log in &history {
        let Some(ref id) = log.account_id else { continue };
        let e = agg.entry(id.clone()).or_insert((0.0, 0, 0));
        e.0 += log.credits.unwrap_or(0.0);
        e.1 += 1;
        e.2 += i64::from(log.input_tokens.unwrap_or(0))
            + i64::from(log.output_tokens.unwrap_or(0));
    }

    let mut out = Vec::new();
    for account in scoped {
        let mut entry = GatewayQuotaEntry {
            account_id: account.id.clone(),
            email: account.email.clone(),
            provider: account.provider.clone(),
            region: account.region.clone(),
            usage: serde_json::Value::Null,
            credits_consumed: agg.get(&account.id).map(|e| e.0).unwrap_or(0.0),
            total_requests: agg.get(&account.id).map(|e| e.1).unwrap_or(0),
            total_tokens: agg.get(&account.id).map(|e| e.2).unwrap_or(0),
            error: None,
        };

        match &account.access_token {
            Some(token) if !token.is_empty() => {
                match crate::commands::common::get_usage_by_account(&account, token).await {
                    Ok(u) => entry.usage = u.usage_data,
                    Err(e) => entry.error = Some(e),
                }
            }
            _ => entry.error = Some("无 access token".to_string()),
        }
        out.push(entry);
    }
    Ok(out)
}

pub async fn get_gateway_request_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<GatewayRequestStats, String> {
    // 尝试从运行中的 gateway 的内存存储获取
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        // 从内存存储获取统计
        let stats = log_store.get_stats().await;
        let all_logs = log_store.get_all().await;

        // 计算最大延迟
        let max_duration_ms = all_logs
            .iter()
            .map(|log| log.duration_ms)
            .max()
            .unwrap_or(0);

        return Ok(GatewayRequestStats {
            total: stats.total,
            success: stats.success,
            error: stats.error,
            streaming: stats.streaming,
            total_input_tokens: stats.total_input_tokens as i64,
            total_output_tokens: stats.total_output_tokens as i64,
            total_cache_read_tokens: stats.total_cache_read_tokens as i64,
            total_cache_creation_tokens: stats.total_cache_creation_tokens as i64,
            total_credits: stats.total_credits,
            requests_with_cache: stats.requests_with_cache,
            max_duration_ms,
            avg_duration_ms: stats.avg_duration_ms,
        });
    }

    // 如果 gateway 未运行，从文件读取
    let path = request_log_path()?;
    get_gateway_request_stats_from_path(&path)
}

pub async fn get_gateway_model_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<log_store::ModelStat>, String> {
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        return Ok(log_store.get_model_stats().await);
    }

    Ok(Vec::new())
}

pub async fn get_gateway_endpoint_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<log_store::EndpointStat>, String> {
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        return Ok(log_store.get_endpoint_stats().await);
    }

    Ok(Vec::new())
}

pub async fn get_gateway_client_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<log_store::ClientStat>, String> {
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        return Ok(log_store.get_client_stats().await);
    }

    Ok(Vec::new())
}

fn get_gateway_request_stats_from_path(path: &Path) -> Result<GatewayRequestStats, String> {
    if !path.exists() {
        return Ok(GatewayRequestStats {
            total: 0,
            success: 0,
            error: 0,
            streaming: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read_tokens: 0,
            total_cache_creation_tokens: 0,
            total_credits: 0.0,
            requests_with_cache: 0,
            max_duration_ms: 0,
            avg_duration_ms: 0,
        });
    }

    let file = fs::File::open(path).map_err(|e| format!("读取请求日志失败: {e}"))?;
    let reader = BufReader::new(file);

    let mut total = 0;
    let mut success = 0;
    let mut error = 0;
    let mut streaming = 0;
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cache_read_tokens: i64 = 0;
    let mut total_cache_creation_tokens: i64 = 0;
    let mut total_credits: f64 = 0.0;
    let mut requests_with_cache = 0;
    let mut max_duration_ms = 0u64;
    let mut total_duration_ms = 0u64;

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<GatewayRequestLogEntry>(trimmed) {
            total += 1;

            if entry.status_code < 400 {
                success += 1;
            } else {
                error += 1;
            }

            if entry.stream {
                streaming += 1;
            }

            total_input_tokens += entry.input_tokens.unwrap_or(0) as i64;
            total_output_tokens += entry.output_tokens.unwrap_or(0) as i64;
            total_cache_read_tokens += entry.cache_read_input_tokens.unwrap_or(0) as i64;
            total_cache_creation_tokens += entry.cache_creation_input_tokens.unwrap_or(0) as i64;
            total_credits += entry.credits.unwrap_or(0.0);

            if entry.cache_read_input_tokens.unwrap_or(0) > 0
                || entry.cache_creation_input_tokens.unwrap_or(0) > 0
            {
                requests_with_cache += 1;
            }

            max_duration_ms = max_duration_ms.max(entry.duration_ms);
            total_duration_ms += entry.duration_ms;
        }
    }

    let avg_duration_ms = if total > 0 {
        total_duration_ms / total as u64
    } else {
        0
    };

    Ok(GatewayRequestStats {
        total,
        success,
        error,
        streaming,
        total_input_tokens,
        total_output_tokens,
        total_cache_read_tokens,
        total_cache_creation_tokens,
        total_credits,
        requests_with_cache,
        max_duration_ms,
        avg_duration_ms,
    })
}

pub async fn clear_gateway_request_logs(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<(), String> {
    // 清空内存存储
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        log_store.clear().await;
    }

    // 清空文件
    let path = request_log_path()?;
    clear_gateway_request_logs_at_path(&path)
}

pub async fn start_gateway(
    state: &tauri::State<'_, crate::state::AppState>,
    config: GatewayConfig,
) -> Result<GatewayStatus, String> {
    let mut config = normalize_config(&config);
    // 启动前丢掉指向已删除账号的池 id。留着它们 pool 模式照样通过
    // ensure_config_valid，运行期却一个账号都匹配不到（proxy/auth.rs 与
    // proxy/upstream.rs 都按 id 过滤），请求全部失败且界面看不出原因。
    // 清空后 pool 模式变成非法配置，这里的校验会把这个状态明确报出来。
    prune_deleted_pool_accounts_in_place(&mut config, &state.store);
    ensure_config_valid(&config)?;

    let existing = {
        let mut guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;
        guard.take()
    };

    if let Some(mut rt) = existing {
        stop_runtime(&mut rt).await;
    }

    let runtime = spawn_runtime(config.clone(), state.store.clone()).await?;
    let status = GatewayStatus {
        running: true,
        host: config.host.clone(),
        port: config.port,
        request_count: 0,
        last_error: None,
        runtime_config: Some(config.clone()),
    };

    let mut guard = state
        .gateway
        .lock()
        .map_err(|_| "获取 gateway 状态失败".to_string())?;
    *guard = Some(runtime);

    Ok(status)
}

pub async fn stop_gateway(state: &tauri::State<'_, crate::state::AppState>) -> Result<(), String> {
    let existing = {
        let mut guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;
        guard.take()
    };

    if let Some(mut rt) = existing {
        stop_runtime(&mut rt).await;
    }

    Ok(())
}

pub async fn get_gateway_status(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<GatewayStatus, String> {
    let snapshot = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| {
            (
                rt.config.clone(),
                rt.request_count.load(Ordering::Relaxed),
                rt.last_error.clone(),
                rt.server_task.is_some(),
            )
        })
    };

    if let Some((config, request_count, last_error, running)) = snapshot {
        let last_error_text = last_error.lock().await.clone();
        Ok(GatewayStatus {
            running,
            host: config.host.clone(),
            port: config.port,
            request_count,
            last_error: last_error_text,
            runtime_config: Some(config),
        })
    } else {
        let cfg = load_gateway_config().unwrap_or_default();
        Ok(GatewayStatus::stopped(&cfg))
    }
}

/// 判断一个 Origin 是否为「本机来源」。
///
/// 匹配两类：
/// 1. 浏览器开发/调试来源：`http://localhost[:port]`、`http://127.0.0.1[:port]`、`http://[::1][:port]`；
/// 2. Tauri WebView 自身的来源：`tauri://localhost`（Linux/macOS）、
///    `http://tauri.localhost` / `https://tauri.localhost`（Windows）。
///    打包后的应用页就是这个来源，漏掉它则应用内的「协议实测」等
///    浏览器 fetch 调用会被 CORS 拦死，开发模式却看不出问题。
///
/// 前缀后必须是端口分隔符或结束——否则 `http://localhost.evil.com` 会被
/// `starts_with("http://localhost")` 误放行。
fn is_local_origin(origin: &str) -> bool {
    // Tauri WebView：scheme 与主机名固定，没有端口概念，整串精确匹配。
    // 不把 tauri:// 做成前缀匹配，避免 `tauri://localhost.evil.com` 蒙对。
    if matches!(
        origin,
        "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost"
    ) {
        return true;
    }
    for prefix in ["http://localhost", "http://127.0.0.1", "http://[::1]"] {
        if let Some(rest) = origin.strip_prefix(prefix) {
            return rest.is_empty() || rest.starts_with(':');
        }
    }
    false
}

fn router(state: RouterState) -> Router {
    // CORS 收敛：不再 `allow_origin(Any)`。
    //
    // 原来的全开意味着同机上任意网页都能用 JS 读走 /health、/v1/models
    // （本机边界挡得住远程，挡不住本机浏览器）。现在统一只放行本机来源：
    // local_only 与远程模式同等对待——浏览器 Origin 不在 IP 白名单语义内，
    // 要为别人开放得另行处理，不在这层开口子。
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            origin.to_str().is_ok_and(is_local_origin)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        // x-account-id 与 anthropic-version 是三个入口真实会用到的请求头：
        // 前者让调用方强制指定账号（proxy/auth.rs 读它），后者是 Anthropic
        // 协议的版本声明。它们不在放行列表里时，浏览器预检会先失败，
        // 请求根本到不了 handler——所以必须显式列出。
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-account-id"),
            HeaderName::from_static("anthropic-version"),
        ]);

    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/messages", post(anthropic_messages_handler))
        .route(
            "/v1/messages/count_tokens",
            post(anthropic_count_tokens_handler),
        )
        .route("/v1/responses", post(openai_responses_handler))
        .route("/v1/responses/input_tokens", post(openai_tokens_handler))
        .route("/v1/chat/completions", post(openai_chat_handler))
        .layer(cors)
        .with_state(state)
}

async fn spawn_runtime(
    config: GatewayConfig,
    store: crate::state::SharedAccountStore,
) -> Result<GatewayRuntime, String> {
    ensure_config_valid(&config)?;

    let request_count = Arc::new(AtomicU64::new(0));
    let last_error = Arc::new(AsyncMutex::new(None));
    let responses_sessions = Arc::new(AsyncMutex::new(HashMap::new()));
    let token_cache = Arc::new(AsyncMutex::new(TokenCache::new()));

    let http = build_streaming_http_client_with_upstream_proxy(
        config.upstream_proxy.as_deref(),
    )
    .map_err(|e| format!("初始化 HTTP 客户端失败: {e}"))?;

    // 初始化负载均衡器
    let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
    let load_balancer = Arc::new(load_balancer::LoadBalancer::with_store(
        strategy,
        store.clone(),
    ));

    // 初始化内存日志存储（保存最近 10000 条日志）
    let log_store = Arc::new(log_store::LogStore::new(10000));

    // 隐私遮蔽：引擎（映射表跨请求保留）+ 审计记录（内存环形缓冲）。
    // 两者在 runtime 生命周期内存活，停止即释放——审计元数据刻意不落盘。
    // 用户规则覆盖/自定义规则/白名单从配置装配；坏自定义正则不阻断启动。
    let mask_engine = Arc::new(AsyncMutex::new(
        masking::MaskEngine::with_overrides(
            &config.mask_kind_overrides,
            &config.mask_custom_rules,
            &config.mask_never_mask,
        )
        .unwrap_or_else(|e| {
            log::warn!("[遮蔽] 自定义规则装配有问题（已跳过坏规则）: {e}");
            masking::MaskEngine::new()
        }),
    ));
    // 同形替换：装配后开关，失败回退的引擎同样要生效
    {
        let mut engine = mask_engine.lock().await;
        engine.set_shape(config.mask_shape);
    }
    let mask_audit = Arc::new(masking::MaskAuditLog::new(500));

    // 从文件加载历史日志到内存（启动时恢复，容量对齐内存上限 10000，
    // 避免重启后内存视图只覆盖最近 500 条的断层）
    if let Ok(path) = request_log_path() {
        if let Ok(history) = get_gateway_request_logs_from_path(&path, Some(10000)) {
            // 必须在服务器开始受理请求前同步完成：若 spawn 异步回灌，
            // session_start 迟迟未 set，概览「最近请求」会把历史当实时流量返回
            // 历史日志是倒序的（最新在前），需要反转后逐条添加
            let restored = history.len();
            for entry in history.into_iter().rev() {
                log_store.add(entry).await;
            }
            // 回灌的是历史：之后的 add 才算「本次运行」的流量（概览按会话取数）
            log_store.set_session_start(restored).await;
        }
    }

    // 初始化响应缓存
    let cache_config = response_cache::CacheConfig {
        summary_cache_enabled: config.response_cache_enabled,
        summary_cache_max_age_seconds: config.response_cache_ttl,
        ..response_cache::CacheConfig::default()
    };
    let cache_dir = crate::core::paths::app_data_dir().map(|p| p.join("cache"));
    let response_cache = Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
        cache_config,
        cache_dir,
    )));

    let state = RouterState {
        config: config.clone(),
        request_count: request_count.clone(),
        last_error: last_error.clone(),
        http,
        responses_sessions,
        token_cache,
        load_balancer: load_balancer.clone(),
        log_store: log_store.clone(),
        response_cache: response_cache.clone(),
        mask_engine: mask_engine.clone(),
        mask_audit: mask_audit.clone(),
        store,
    };

    // 会话缓存独立清理周期：不能只在插入时清，否则闲置期间内存不释放。
    // 必须在 router(state) 移动 state 之前取出 Arc。
    let session_cleanup_store = state.responses_sessions.clone();

    let app = router(state);
    let addr = build_bind_addr(&config.host, config.port)?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("绑定端口失败: {e}"))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = shutdown_rx.await;
    });

    let server_task = tokio::spawn(async move {
        if let Err(e) = server.await {
            log::error!("网关服务器错误: {e}");
        }
    });

    // 会话缓存独立清理周期：不能只在插入时清，否则闲置期间内存不释放
    crate::gateway::proxy::spawn_responses_session_cleanup(session_cleanup_store);

    Ok(GatewayRuntime {
        config,
        request_count,
        last_error,
        log_store,
        response_cache,
        load_balancer,
        mask_engine,
        mask_audit,
        shutdown_tx: Some(shutdown_tx),
        server_task: Some(server_task),
    })
}

async fn stop_runtime(runtime: &mut GatewayRuntime) {
    if let Some(tx) = runtime.shutdown_tx.take() {
        let _ = tx.send(());
    }
    if let Some(task) = runtime.server_task.take() {
        let _ = task.await;
    }
}

pub async fn auto_start_if_enabled(app: &AppHandle) -> Result<(), String> {
    let cfg = load_gateway_config()?;
    if !cfg.enabled {
        return Ok(());
    }

    let state = app.state::<crate::state::AppState>();
    let _ = start_gateway(&state, cfg).await?;
    Ok(())
}

pub fn gateway_log_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    gateway_log_dir_raw()
}

pub fn get_gateway_log_dir(app: &AppHandle) -> Result<String, String> {
    gateway_log_dir(app).map(|path| path.to_string_lossy().to_string())
}

pub fn open_gateway_log_dir(app: &AppHandle) -> Result<String, String> {
    let dir = gateway_log_dir(app)?;
    open::that(&dir).map_err(|e| format!("打开日志目录失败: {e}"))?;
    Ok(dir.to_string_lossy().to_string())
}

/// 读取某个请求的 SSE 逐事件明细。
///
/// SSE 明细体积不可控（一次流式请求可达数万条），所以只落文件、不进内存
/// 日志。日志行自 2026-10-23 起带 `req=<request_id>`；更早的行没有该字段，
/// 无法归属到具体请求（返回空而非误匹配）。
///
/// `endpoint` 用于定位文件：三个协议各写各的（见
/// `proxy::logging::get_client_sse_log_file`）。
pub fn read_gateway_sse_log_inner(
    app: &AppHandle,
    request_id: &str,
    endpoint: Option<&str>,
) -> Result<Vec<String>, String> {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return Err("request_id 不能为空".to_string());
    }
    let dir = gateway_log_dir(app)?;
    let file_name = match endpoint.unwrap_or_default() {
        p if p.contains("/responses") => "openai-responses-response-sse.log",
        p if p.contains("/chat/completions") => "openai-chat-response-sse.log",
        _ => "anthropic-messages-response-sse.log",
    };
    let path = dir.join(file_name);
    if !path.exists() {
        return Ok(Vec::new());
    }

    // 按 `req=<id>` 精确匹配：不能只查 substring，否则 request_id 前缀相同的
    // 另一条请求会被误带进来。
    let needle = format!("req={request_id} ");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 SSE 日志失败: {e}"))?;
    Ok(content
        .lines()
        .filter(|line| line.contains(&needle))
        .map(str::to_string)
        .collect())
}

async fn health_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
) -> Response {
    proxy::health_handler(state, addr, headers).await
}

async fn models_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
) -> Response {
    proxy::models_handler(state, addr, headers).await
}
async fn anthropic_count_tokens_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::anthropic_count_tokens_handler(state, addr, headers, payload).await
}

async fn openai_tokens_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::openai_tokens_handler(state, addr, headers, payload).await
}

async fn anthropic_messages_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::proxy_handler(state, addr, headers, payload, ResponseFormat::Anthropic).await
}

async fn openai_responses_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::proxy_handler(state, addr, headers, payload, ResponseFormat::Responses).await
}

async fn openai_chat_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::openai_chat_handler(state, addr, headers, payload).await
}

#[cfg(test)]
mod tests;
