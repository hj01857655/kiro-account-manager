mod compress;
mod converter;
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
    http::HeaderMap,
    response::{Json, Response},
    routing::{get, post},
    Router,
};
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

use crate::clients::http_client::is_supported_kiro_region;
use crate::gateway::token_cache::TokenCache;

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
}

fn default_cache_ttl() -> u64 { 180 }

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

fn default_true_val() -> bool { true }
fn default_mapping_type() -> String { "replace".to_string() }

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
                    let idx = ROUND_ROBIN.fetch_add(1, Ordering::Relaxed) % rule.target_models.len();
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
    pub request_index: u64,
    pub endpoint: String,
    pub client_ip: String,
    pub model: Option<String>,
    pub stream: bool,
    pub upstream_source: Option<String>,
    pub region: Option<String>,
    pub status_code: u16,
    pub outcome: String,
    pub duration_ms: u64,
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

const CONFIG_DIR: &str = ".kiro-account-manager";
const CONFIG_FILE: &str = "gateway-config.json";
const LOGS_DIR: &str = "logs";
const REQUEST_LOG_FILE: &str = "gateway-request-log.jsonl";
const DEFAULT_AGENT_MODE: &str = "q-developer-converse";

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
        .filter(|value| !value.starts_with("#disabled#")) // 过滤禁用的 Key
    {
        keys.push(key.to_string());
    }

    for key in config
        .client_api_keys
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .filter(|item| !item.starts_with("#disabled#")) // 过滤禁用的 Key
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
        "single" | "group" | "pool" => {}
        "local" => {
            return Err("反代不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string());
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
    dirs::data_dir()
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        })
        .join(CONFIG_DIR)
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
        let max_duration_ms = all_logs.iter()
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
                || entry.cache_creation_input_tokens.unwrap_or(0) > 0 {
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
    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/messages", post(messages_handler))
        .route("/v1/messages/count_tokens", post(count_tokens_handler))
        .route("/v1/responses", post(responses_handler))
        .route("/v1/chat/completions", post(openai_chat_handler))
        .with_state(state)
}

async fn spawn_runtime(config: GatewayConfig) -> Result<GatewayRuntime, String> {
    ensure_config_valid(&config)?;

    let request_count = Arc::new(AtomicU64::new(0));
    let last_error = Arc::new(AsyncMutex::new(None));
    let responses_sessions = Arc::new(AsyncMutex::new(HashMap::new()));
    let token_cache = Arc::new(AsyncMutex::new(TokenCache::new()));

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
    let cache_dir = dirs::data_dir()
        .map(|p| p.join(".kiro-account-manager").join("cache"));
    let response_cache = Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
        cache_config,
        cache_dir,
    )));

    let state = RouterState {
        config: config.clone(),
        request_count: request_count.clone(),
        last_error: last_error.clone(),
        responses_sessions,
        token_cache,
        load_balancer,
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

async fn count_tokens_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::count_tokens_handler(state, addr, headers, payload).await
}

async fn messages_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::proxy_handler(state, addr, headers, payload, ResponseFormat::Anthropic).await
}

async fn responses_handler(
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
    proxy::proxy_handler(state, addr, headers, payload, ResponseFormat::OpenAI).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue, Method, Request, StatusCode};
    use serde_json::json;
    use std::{
        process,
        sync::{
            atomic::{AtomicU64, Ordering},
            Mutex,
        },
    };
    use tower::util::ServiceExt;

    static REQUEST_LOG_TEST_MUTEX: Mutex<()> = Mutex::new(());
    static REQUEST_LOG_TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct RequestLogTestFixture {
        path: PathBuf,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl RequestLogTestFixture {
        fn new() -> Self {
            let guard = REQUEST_LOG_TEST_MUTEX
                .lock()
                .expect("request log test mutex should lock");
            let dir = std::env::temp_dir().join(format!(
                "kiro-gateway-request-log-test-{}-{}",
                process::id(),
                REQUEST_LOG_TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).expect("request log test dir should create");
            let path = dir.join(REQUEST_LOG_FILE);
            set_request_log_path_override(Some(path.clone()));
            Self {
                path,
                _guard: guard,
            }
        }
    }

    impl Drop for RequestLogTestFixture {
        fn drop(&mut self) {
            set_request_log_path_override(None);
            if let Some(dir) = self.path.parent() {
                let _ = fs::remove_dir_all(dir);
            }
        }
    }

    fn gateway_runtime_test_state() -> RouterState {
        let config = GatewayConfig {
            access_token: Some("sk-test".to_string()),
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            ..GatewayConfig::default()
        };
        let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
        RouterState {
            config,
            request_count: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(AsyncMutex::new(None)),
            responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
            load_balancer: Arc::new(load_balancer::LoadBalancer::new(strategy)),
            log_store: Arc::new(log_store::LogStore::new(1000)),
            response_cache: Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
                response_cache::CacheConfig::default(),
                None,
            ))),
        }
    }

    fn auth_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer sk-test"));
        headers
    }

    

    

    

    

    fn test_router_state() -> RouterState {
        let config = GatewayConfig::default();
        let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
        RouterState {
            config,
            request_count: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(AsyncMutex::new(None)),
            responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
            load_balancer: Arc::new(load_balancer::LoadBalancer::new(strategy)),
            log_store: Arc::new(log_store::LogStore::new(1000)),
            response_cache: Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
                response_cache::CacheConfig::default(),
                None,
            ))),
        }
    }

    fn runtime_test_gateway_config(port: u16) -> GatewayConfig {
        GatewayConfig {
            port,
            local_only: false,
            allowed_ips: vec!["127.0.0.1".to_string()],
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        }
    }

    #[tokio::test]
    async fn health_route_is_reachable() {
        let app = router(test_router_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn responses_route_is_reachable() {
        let app = router(test_router_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "claude-sonnet-4-5-20250929",
                            "input": [{ "role": "user", "content": "hello" }]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn openai_chat_completions_endpoint_accepts_requests() {
        let app = router(test_router_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "claude-sonnet-4-5-20250929",
                            "messages": [{ "role": "user", "content": "hello" }]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn lightweight_routes_increment_request_count_and_write_logs() {
        let fixture = RequestLogTestFixture::new();
        let state = gateway_runtime_test_state();
        let client_addr: SocketAddr = "127.0.0.1:4317".parse().expect("socket addr should parse");

        let health = proxy::health_handler(state.clone(), client_addr, auth_headers()).await;
        assert_eq!(health.status(), StatusCode::OK);

        let models = proxy::models_handler(state.clone(), client_addr, auth_headers()).await;
        assert_eq!(models.status(), StatusCode::OK);

        let count_tokens = proxy::count_tokens_handler(
            state.clone(),
            client_addr,
            auth_headers(),
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{ "role": "user", "content": "hello world" }]
            }),
        )
        .await;
        assert_eq!(count_tokens.status(), StatusCode::OK);

        assert_eq!(state.request_count.load(Ordering::Relaxed), 3);

        let logs = get_gateway_request_logs_from_path(fixture.path.as_path(), Some(10))
            .expect("request logs should read");
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].endpoint, "count_tokens");
        assert_eq!(logs[1].endpoint, "models");
        assert_eq!(logs[2].endpoint, "health");
        assert_eq!(logs[0].status_code, 200);
        assert_eq!(logs[1].status_code, 200);
        assert_eq!(logs[2].status_code, 200);
        assert_eq!(logs[0].outcome, "success");
        assert_eq!(logs[1].outcome, "success");
        assert_eq!(logs[2].outcome, "success");
        assert_eq!(logs[0].client_ip, "127.0.0.1");
        assert!(
            logs[0].request_body.is_none(),
            "request body should not be logged by default"
        );
        assert!(
            logs[0].response_body.is_none(),
            "response body should not be logged by default"
        );
    }

    #[test]
    fn rejects_unsupported_region() {
        let config = GatewayConfig {
            region: "moon-east-1".to_string(),
            ..GatewayConfig::default()
        };

        let err = ensure_config_valid(&config).expect_err("unsupported region should fail");
        assert!(err.contains("region 不受支持"));
    }

    #[test]
    fn rejects_local_account_mode_for_gateway() {
        let config = GatewayConfig {
            account_mode: "local".to_string(),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        };

        let err = ensure_config_valid(&config).expect_err("local mode should fail");
        assert!(
            err.contains("不再支持 local 模式"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn accepts_known_regions() {
        let mut config = GatewayConfig {
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        };
        for region in [
            "us-east-1",
            "eu-central-1",
            "us-west-2",
            "ap-northeast-1",
            "ap-southeast-1",
            "us-gov-west-1",
        ] {
            config.region = region.to_string();
            ensure_config_valid(&config).expect("known region should pass validation");
        }
    }

    #[test]
    fn rejects_remote_access_without_api_key() {
        let config = GatewayConfig {
            local_only: false,
            account_id: Some("test-account".to_string()),
            access_token: None,
            ..GatewayConfig::default()
        };

        let err =
            ensure_config_valid(&config).expect_err("remote access without api key should fail");
        assert!(err.contains("API Key"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_remote_access_without_allowlist() {
        let config = GatewayConfig {
            local_only: false,
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            allowed_ips: Vec::new(),
            ..GatewayConfig::default()
        };

        let err =
            ensure_config_valid(&config).expect_err("remote access without allowlist should fail");
        assert!(err.contains("白名单"), "unexpected error: {err}");
    }

    #[test]
    fn normalize_config_promotes_legacy_access_token_to_client_api_keys() {
        let config = GatewayConfig {
            access_token: Some(" sk-primary ".to_string()),
            client_api_keys: Vec::new(),
            ..GatewayConfig::default()
        };

        let normalized = normalize_config(&config);

        assert_eq!(normalized.client_api_keys, vec!["sk-primary".to_string()]);
        assert_eq!(normalized.access_token.as_deref(), Some("sk-primary"));
    }

    #[test]
    fn normalize_config_deduplicates_client_api_keys() {
        let config = GatewayConfig {
            access_token: Some("sk-primary".to_string()),
            client_api_keys: vec![
                " sk-primary ".to_string(),
                "sk-secondary".to_string(),
                "".to_string(),
                "sk-secondary".to_string(),
            ],
            ..GatewayConfig::default()
        };

        let normalized = normalize_config(&config);

        assert_eq!(
            normalized.client_api_keys,
            vec!["sk-primary".to_string(), "sk-secondary".to_string()]
        );
    }

    #[test]
    fn rejects_config_without_any_client_api_keys() {
        let config = GatewayConfig {
            access_token: Some("   ".to_string()),
            client_api_keys: vec!["".to_string(), "   ".to_string()],
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            ..GatewayConfig::default()
        };

        let err = ensure_config_valid(&normalize_config(&config))
            .expect_err("missing client api keys should fail");
        assert!(err.contains("客户端 API Key"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn runtime_serves_health_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .bearer_auth("sk-test")
            .send()
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_serves_models_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/v1/models"))
            .bearer_auth("sk-test")
            .send()
            .await
            .expect("models request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value = response.json().await.expect("models response should be json");
        assert_eq!(payload.get("object").and_then(Value::as_str), Some("list"));
        assert!(
            payload
                .get("data")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty()),
            "models response should include at least one model"
        );
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_serves_count_tokens_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages/count_tokens"))
            .bearer_auth("sk-test")
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "messages": [{ "role": "user", "content": "hello world" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("count tokens request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value = response
            .json()
            .await
            .expect("count tokens response should be json");
        assert_eq!(payload.get("input_tokens").and_then(Value::as_u64), Some(2));
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_health_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .send()
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_raw_authorization_header_without_bearer_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .header("Authorization", "sk-test")
            .send()
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_models_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/v1/models"))
            .send()
            .await
            .expect("models request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_count_tokens_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages/count_tokens"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "input": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("count tokens request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_responses_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/responses"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "input": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("responses request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_messages_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/messages"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "messages": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("messages request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_requires_client_api_key_even_when_local_only() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = GatewayConfig {
            port,
            local_only: true,
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        };
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/responses"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "input": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("responses request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }
}
