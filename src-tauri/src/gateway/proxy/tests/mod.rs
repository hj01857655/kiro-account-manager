use super::*;
use crate::gateway::token_cache::TokenCache;
use serde_json::json;
use std::sync::{atomic::AtomicU64, Arc};
use tokio::sync::Mutex as AsyncMutex;

mod errors;
mod tokens;
mod auth;
mod session;
mod responses;
mod upstream;
mod handlers;
mod tool_search_loop;

fn proxy_test_state() -> RouterState {
    RouterState {
        config: GatewayConfig {
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            ..GatewayConfig::default()
        },
        request_count: Arc::new(AtomicU64::new(0)),
        last_error: Arc::new(AsyncMutex::new(None)),
        http: Client::new(),
        responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
        token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
        load_balancer: Arc::new(crate::gateway::load_balancer::LoadBalancer::new(
            crate::gateway::load_balancer::LoadBalancerStrategy::RoundRobin,
        )),
        log_store: Arc::new(crate::gateway::log_store::LogStore::new(1000)),
        response_cache: Arc::new(AsyncMutex::new(
            crate::gateway::response_cache::ResponseCache::new(
                crate::gateway::response_cache::CacheConfig::default(),
                None,
            ),
        )),
        mask_engine: Arc::new(AsyncMutex::new(crate::gateway::masking::MaskEngine::new())),
        mask_audit: Arc::new(crate::gateway::masking::MaskAuditLog::new(10)),
        store: Arc::new(std::sync::Mutex::new(
            crate::core::account::AccountStore::new(),
        )),
    }
}
