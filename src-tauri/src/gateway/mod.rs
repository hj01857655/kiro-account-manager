mod compress;
pub(crate) mod converter;
mod eventstream;
pub(crate) mod load_balancer;
pub mod log_store;
mod models;
pub mod prompt_cache;
pub mod prompt_filter;
mod proxy;
pub mod response_cache;
mod stream;
mod thinking_parser;
mod token_cache;
mod token_estimator;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, Method},
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use reqwest::Client;
use tower_http::cors::{Any, CorsLayer};
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

use crate::clients::http_client::{build_streaming_http_client, is_supported_kiro_region};
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
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub client_api_keys: Vec<String>,
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
    #[serde(default)]
    pub model_mappings: Vec<ModelMappingRule>,
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
}

fn default_cache_ttl() -> u64 {
    180
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

/// 模型映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMappingRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    /// replace | alias | loadbalance
    #[serde(default = "default_mapping_type")]
    pub rule_type: String,
    pub source_model: String,
    pub target_models: Vec<String>,
    #[serde(default)]
    pub weights: Vec<u32>,
}

fn default_true_val() -> bool {
    true
}
fn default_mapping_type() -> String {
    "replace".to_string()
}

/// 根据模型映射规则解析实际模型名
pub fn resolve_model_mapping(config: &GatewayConfig, requested_model: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static ROUND_ROBIN: AtomicUsize = AtomicUsize::new(0);

    for rule in &config.model_mappings {
        if !rule.enabled {
            continue;
        }
        if rule.source_model != requested_model {
            continue;
        }
        if rule.target_models.is_empty() {
            continue;
        }

        match rule.rule_type.as_str() {
            "replace" | "alias" => {
                return rule.target_models[0].clone();
            }
            "loadbalance" => {
                if rule.weights.is_empty() || rule.weights.len() != rule.target_models.len() {
                    // 无权重或权重数量不匹配，简单轮询
                    let idx =
                        ROUND_ROBIN.fetch_add(1, Ordering::Relaxed) % rule.target_models.len();
                    return rule.target_models[idx].clone();
                }
                // 加权轮询
                let total_weight: u32 = rule.weights.iter().sum();
                if total_weight == 0 {
                    return rule.target_models[0].clone();
                }
                let tick = ROUND_ROBIN.fetch_add(1, Ordering::Relaxed) as u32 % total_weight;
                let mut cumulative = 0u32;
                for (i, &w) in rule.weights.iter().enumerate() {
                    cumulative += w;
                    if tick < cumulative {
                        return rule.target_models[i].clone();
                    }
                }
                return rule.target_models[0].clone();
            }
            _ => {}
        }
    }

    requested_model.to_string()
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
    pub requests_with_cache: usize,
    pub max_duration_ms: u64,
    pub avg_duration_ms: u64,
}

#[derive(Debug)]
pub struct GatewayRuntime {
    pub config: GatewayConfig,
    pub request_count: Arc<AtomicU64>,
    pub last_error: Arc<AsyncMutex<Option<String>>>,
    pub log_store: Arc<log_store::LogStore>,
    pub response_cache: Arc<AsyncMutex<response_cache::ResponseCache>>,
    pub load_balancer: Arc<load_balancer::LoadBalancer>,
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
            model_mappings: Vec::new(),
            filter_claude_code: false,
            filter_strip_boundaries: false,
            filter_env_noise: false,
            prompt_filter_rules: Vec::new(),
            log_requests: true,
            response_cache_enabled: true,
            response_cache_ttl: default_cache_ttl(),
            web_search_enabled: true,
        }
    }
}
pub(crate) fn effective_client_api_keys(config: &GatewayConfig) -> Vec<String> {
    let mut keys = Vec::new();

    if let Some(key) = config
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.starts_with("#disabled#"))
    // 过滤禁用的 Key
    {
        keys.push(key.to_string());
    }

    for key in config
        .client_api_keys
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .filter(|item| !item.starts_with("#disabled#"))
    // 过滤禁用的 Key
    {
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.to_string());
        }
    }

    keys
}

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
            return Err("2API不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string());
        }
        _ => return Err("accountMode 必须是 single/group/pool".to_string()),
    }
    if !matches!(
        config.log_level.as_str(),
        "debug" | "info" | "warn" | "error"
    ) {
        return Err("logLevel 必须是 debug/info/warn/error".to_string());
    }
    if effective_client_api_keys(config).is_empty() {
        return Err("必须配置客户端 API Key".to_string());
    }
    if !config.local_only && config.allowed_ips.is_empty() {
        return Err("允许远程访问时必须至少配置一个白名单来源 IP".to_string());
    }
    for entry in &config.allowed_ips {
        if !is_valid_allowlist_entry(entry) {
            return Err(format!("白名单条目无效: {entry}"));
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
    normalized.access_token = normalized
        .access_token
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    normalized.region = normalized.region.trim().to_string();
    normalized.account_mode = normalized.account_mode.trim().to_string();
    normalized.strategy = normalized.strategy.trim().to_string();
    normalized.log_level = normalized.log_level.trim().to_ascii_lowercase();
    normalized.client_api_keys = effective_client_api_keys(&normalized);
    normalized.access_token = normalized.client_api_keys.first().cloned();
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

/// 收敛网关日志目录体积：把超过 [`RAW_LOG_MAX_BYTES`] 的 `*.log` 轮转为 `*.log.1`，
/// 让后续追加从头开始。旧备份会被覆盖，故每个文件最多约占 2×上限。
///
/// - 由 `proxy_handler` 按固定请求间隔低频调用，不在热路径上每次执行。
/// - 轮转失败（文件被占用等）静默忽略，下次再试；不会影响正常请求。
/// - `*.log.1` 备份本身因扩展名不是 `log` 会被跳过，不会反复轮转。
pub fn enforce_raw_log_caps() {
    let log_dir = crate::core::paths::app_data_dir_or_default().join(LOGS_DIR);
    let Ok(entries) = std::fs::read_dir(&log_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("log") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() || meta.len() <= RAW_LOG_MAX_BYTES {
            continue;
        }
        let backup = path.with_extension("log.1");
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
    let max_items = limit.unwrap_or(100).clamp(1, 500);
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

    fs::remove_file(path).map_err(|e| format!("清空请求日志失败: {e}"))
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
        // 从内存存储获取
        let logs = log_store.get_last(limit.unwrap_or(50)).await;
        return Ok(logs);
    }

    // 如果 gateway 未运行，从文件读取
    let path = request_log_path()?;
    get_gateway_request_logs_from_path(&path, limit)
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
    let config = normalize_config(&config);
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

    let runtime = spawn_runtime(config.clone()).await?;
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

fn router(state: RouterState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

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

async fn spawn_runtime(config: GatewayConfig) -> Result<GatewayRuntime, String> {
    ensure_config_valid(&config)?;

    let request_count = Arc::new(AtomicU64::new(0));
    let last_error = Arc::new(AsyncMutex::new(None));
    let responses_sessions = Arc::new(AsyncMutex::new(HashMap::new()));
    let token_cache = Arc::new(AsyncMutex::new(TokenCache::new()));

    let http = build_streaming_http_client().map_err(|e| format!("初始化 HTTP 客户端失败: {e}"))?;

    // 初始化负载均衡器
    let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
    let load_balancer = Arc::new(load_balancer::LoadBalancer::new(strategy));

    // 初始化内存日志存储（保存最近 10000 条日志）
    let log_store = Arc::new(log_store::LogStore::new(10000));

    // 从文件加载历史日志到内存（启动时恢复）
    if let Ok(path) = request_log_path() {
        if let Ok(history) = get_gateway_request_logs_from_path(&path, Some(500)) {
            let store_clone = log_store.clone();
            tokio::spawn(async move {
                // 历史日志是倒序的（最新在前），需要反转后逐条添加
                for entry in history.into_iter().rev() {
                    store_clone.add(entry).await;
                }
            });
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
    };

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

    Ok(GatewayRuntime {
        config,
        request_count,
        last_error,
        log_store,
        response_cache,
        load_balancer,
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
