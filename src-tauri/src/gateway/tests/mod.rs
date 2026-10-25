//! 网关模块的测试：路由可达性、配置校验、以及真实 HTTP 运行时用例。

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

mod routes;
mod config;
mod runtime_http;

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

static RUNTIME_HTTP_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// 真实 HTTP 测试的串行守卫。
///
/// 这些测试的取端口方式是「`bind("127.0.0.1:0")` 拿端口 → `drop` 释放 → 再交给
/// `spawn_runtime` 重新绑定」，中间存在 TOCTOU 窗口：并行执行时同一个临时端口
/// 可能被另一个测试重复分配，导致 `runtime_*_over_real_http` 系列偶发失败。
/// 持有该守卫即可让它们串行执行（与 `RequestLogTestFixture` 同一套约定）。
struct RuntimeHttpTestGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
    port: u16,
}

impl RuntimeHttpTestGuard {
    fn new() -> Self {
        let guard = RUNTIME_HTTP_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);
        Self {
            _guard: guard,
            port,
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
        http: Client::new(),
        responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
        token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
        load_balancer: Arc::new(load_balancer::LoadBalancer::new(strategy)),
        log_store: Arc::new(log_store::LogStore::new(1000)),
        response_cache: Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
            response_cache::CacheConfig::default(),
            None,
        ))),
        mask_engine: Arc::new(AsyncMutex::new(masking::MaskEngine::new())),
        mask_audit: Arc::new(masking::MaskAuditLog::new(10)),
        store: test_shared_store(),
    }
}

/// 测试用的账号状态共享副本。
///
/// 与 `AppState.store` 同一类型；测试中用它替换真实的账号文件，
/// 避免 `spawn_runtime` 触碰开发机上的真实账号数据。
fn test_shared_store() -> crate::state::SharedAccountStore {
    Arc::new(Mutex::new(crate::core::account::AccountStore::new()))
}

fn auth_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer kiro"));
    headers
}

fn test_router_state() -> RouterState {
    let config = GatewayConfig::default();
    let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
    RouterState {
        config,
        request_count: Arc::new(AtomicU64::new(0)),
        last_error: Arc::new(AsyncMutex::new(None)),
        http: Client::new(),
        responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
        token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
        load_balancer: Arc::new(load_balancer::LoadBalancer::new(strategy)),
        log_store: Arc::new(log_store::LogStore::new(1000)),
        response_cache: Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
            response_cache::CacheConfig::default(),
            None,
        ))),
        mask_engine: Arc::new(AsyncMutex::new(masking::MaskEngine::new())),
        mask_audit: Arc::new(masking::MaskAuditLog::new(10)),
        store: test_shared_store(),
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
