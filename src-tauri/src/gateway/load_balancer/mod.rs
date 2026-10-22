//! 负载均衡模块：多种策略、健康检查、加权轮询与故障转移。

// 负载均衡模块
// 提供多种负载均衡策略，包括健康检查、加权轮询、故障转移等

use crate::core::account::Account;
use rand::{distributions::WeightedIndex, prelude::*, seq::SliceRandom};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

mod types;
mod balancer;

pub use types::*;
pub use balancer::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_health_score() {
        let mut health = AccountHealth::new("test".to_string());

        // 新账号，默认满分
        assert_eq!(health.health_score(), 100);

        // 记录成功
        health.record_success(100);
        assert!(health.health_score() >= 90);

        // 记录失败（连续失败 3 次即标记不健康，与曾经是否成功无关）
        health.record_failure();
        health.record_failure();
        health.record_failure();
        assert!(!health.is_healthy);
        assert!(health.health_score() < 50);

        // 一次成功即中断连续失败，恢复健康
        health.record_success(100);
        assert!(health.is_healthy);

        // 全新账号，纯失败场景
        let mut health2 = AccountHealth::new("test2".to_string());
        health2.record_failure();
        health2.record_failure();
        health2.record_failure();
        assert!(!health2.is_healthy);
        assert_eq!(health2.health_score(), 0);
    }

    #[test]
    fn consecutive_failures_reset_on_success() {
        let mut health = AccountHealth::new("acc".to_string());

        // 差一次到阈值，仍健康
        health.record_failure();
        health.record_failure();
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 2);

        // 成功把连续计数清零：之后要从头再攒 3 次
        health.record_success(50);
        assert_eq!(health.consecutive_failures, 0);

        health.record_failure();
        health.record_failure();
        assert!(health.is_healthy, "成功后的两次失败不该触发不健康");
        health.record_failure();
        assert!(!health.is_healthy, "连续第三次失败应标记不健康");
    }

    #[test]
    fn window_truncates_instead_of_decaying_forever() {
        let mut health = AccountHealth::new("acc".to_string());

        // 灌满远超窗口的成功，再灌失败
        for _ in 0..500 {
            health.record_success(10);
        }
        for _ in 0..5 {
            health.record_failure();
        }

        // 真截断：总量不超过窗口，且早期记录被丢弃
        assert!(
            health.recent_successes + health.recent_failures <= AccountHealth::WINDOW_SIZE,
            "window should be truncated, got {} + {}",
            health.recent_successes,
            health.recent_failures
        );
        // 成功率不被 500 次前的历史稀释
        assert!(health.health_score() < 100);
    }

    #[test]
    fn connection_guard_releases_on_drop() {
        let lb = std::sync::Arc::new(LoadBalancer::new(LoadBalancerStrategy::RoundRobin));
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime should build");

        rt.block_on(async {
            let guard = lb.acquire_connection("acc-1").await;
            assert_eq!(
                lb.get_all_health().await.get("acc-1").map(|h| h.active_connections),
                Some(1),
                "guard 持有期间计数应为 1"
            );
            drop(guard);
            assert_eq!(
                lb.get_all_health().await.get("acc-1").map(|h| h.active_connections),
                Some(0),
                "guard Drop 后计数应归零"
            );
        });
    }

    #[tokio::test]
    async fn test_load_balancer_round_robin() {
        let lb = LoadBalancer::new(LoadBalancerStrategy::RoundRobin);
        let accounts = vec![
            Account {
                id: "1".to_string(),
                email: Some("test1@example.com".to_string()),
                password: None,
                label: "Account 1".to_string(),
                status: "active".to_string(),
                added_at: "2024-01-01".to_string(),
                access_token: Some("token1".to_string()),
                refresh_token: None,
                expires_at: None,
                provider: None,
                user_id: None,
                auth_method: None,
                client_id: None,
                client_secret: None,
                region: Some("us-east-1".to_string()),
                client_id_hash: None,
                sso_session_id: None,
                id_token: None,
                start_url: None,
                profile_arn: None,
                token_endpoint: None,
                issuer_url: None,
                scopes: None,
                usage_data: None,
                group_id: None,
                tag_links: vec![],
                machine_id: None,
                available_models_cache: None,
                failure_count: 0,
                last_failure_at: None,
                disabled_reason: None,
                success_count: 0,
                enabled: true,
                proxy_config: None,
            },
            Account {
                id: "2".to_string(),
                email: Some("test2@example.com".to_string()),
                password: None,
                label: "Account 2".to_string(),
                status: "active".to_string(),
                added_at: "2024-01-01".to_string(),
                access_token: Some("token2".to_string()),
                refresh_token: None,
                expires_at: None,
                provider: None,
                user_id: None,
                auth_method: None,
                client_id: None,
                client_secret: None,
                region: Some("us-east-1".to_string()),
                client_id_hash: None,
                sso_session_id: None,
                id_token: None,
                start_url: None,
                profile_arn: None,
                token_endpoint: None,
                issuer_url: None,
                scopes: None,
                usage_data: None,
                group_id: None,
                tag_links: vec![],
                machine_id: None,
                available_models_cache: None,
                failure_count: 0,
                last_failure_at: None,
                disabled_reason: None,
                success_count: 0,
                enabled: true,
                proxy_config: None,
            },
        ];

        let acc1 = lb.select_account(&accounts).await.unwrap();
        let acc2 = lb.select_account(&accounts).await.unwrap();
        let acc3 = lb.select_account(&accounts).await.unwrap();

        assert_eq!(acc1.id, "1");
        assert_eq!(acc2.id, "2");
        assert_eq!(acc3.id, "1"); // 轮回到第一个
    }
}
