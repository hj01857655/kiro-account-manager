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

/// 负载均衡策略
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancerStrategy {
    /// 轮询（Round Robin）- 按顺序轮流使用账号
    RoundRobin,
    /// 随机（Random）- 随机选择账号
    Random,
    /// 均衡（Balanced）- 优先使用成功次数最少的账号
    Balanced,
    /// 最多配额（Most Quota）- 优先使用剩余配额最多的账号
    MostQuota,
    /// 加权随机（Weighted Random）- 根据配额和成功率加权随机
    WeightedRandom,
    /// 最少连接（Least Connections）- 优先使用活跃连接最少的账号
    LeastConnections,
}

impl LoadBalancerStrategy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "round_robin" => Self::RoundRobin,
            "random" => Self::Random,
            "balanced" => Self::Balanced,
            "most_quota" => Self::MostQuota,
            "weighted_random" => Self::WeightedRandom,
            "least_connections" => Self::LeastConnections,
            _ => Self::RoundRobin, // 默认轮询
        }
    }
}

/// 账号健康状态
#[derive(Debug, Clone)]
pub struct AccountHealth {
    /// 账号 ID
    #[allow(dead_code)]
    pub account_id: String,
    /// 活跃连接数
    pub active_connections: usize,
    /// 最近失败次数（滑动窗口）
    pub recent_failures: usize,
    /// 最近成功次数（滑动窗口）
    pub recent_successes: usize,
    /// 最后一次健康检查时间
    pub last_check: Instant,
    /// 是否健康
    pub is_healthy: bool,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: u64,
}

/// 可序列化的健康状态（用于API响应）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableAccountHealth {
    pub account_id: String,
    pub active_connections: usize,
    pub recent_failures: usize,
    pub recent_successes: usize,
    pub is_healthy: bool,
    pub avg_response_time_ms: u64,
    pub health_score: u32,
}

impl From<&AccountHealth> for SerializableAccountHealth {
    fn from(health: &AccountHealth) -> Self {
        Self {
            account_id: health.account_id.clone(),
            active_connections: health.active_connections,
            recent_failures: health.recent_failures,
            recent_successes: health.recent_successes,
            is_healthy: health.is_healthy,
            avg_response_time_ms: health.avg_response_time_ms,
            health_score: health.health_score(),
        }
    }
}

impl AccountHealth {
    pub fn new(account_id: String) -> Self {
        Self {
            account_id,
            active_connections: 0,
            recent_failures: 0,
            recent_successes: 0,
            last_check: Instant::now(),
            is_healthy: true,
            avg_response_time_ms: 0,
        }
    }

    /// 计算健康分数（0-100）
    pub fn health_score(&self) -> u32 {
        if !self.is_healthy {
            return 0;
        }

        let total_requests = self.recent_successes + self.recent_failures;
        if total_requests == 0 {
            return 100; // 新账号，默认满分
        }

        // 成功率（0-100）
        let success_rate = (self.recent_successes as f64 / total_requests as f64 * 100.0) as u32;

        // 连接负载惩罚（每 10 个连接减 5 分）
        let connection_penalty = (self.active_connections / 10) as u32 * 5;

        // 响应时间惩罚（每 1000ms 减 10 分）
        let response_penalty = (self.avg_response_time_ms / 1000) as u32 * 10;

        success_rate
            .saturating_sub(connection_penalty)
            .saturating_sub(response_penalty)
    }

    /// 记录成功
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.recent_successes += 1;
        self.is_healthy = true;
        self.last_check = Instant::now();

        // 更新平均响应时间（简单移动平均）
        if self.avg_response_time_ms == 0 {
            self.avg_response_time_ms = response_time_ms;
        } else {
            self.avg_response_time_ms = (self.avg_response_time_ms * 9 + response_time_ms) / 10;
        }

        // 滑动窗口：保持最近 100 次请求的统计
        if self.recent_successes + self.recent_failures > 100 {
            self.recent_successes = (self.recent_successes * 9) / 10;
            self.recent_failures = (self.recent_failures * 9) / 10;
        }
    }

    /// 记录失败
    pub fn record_failure(&mut self) {
        self.recent_failures += 1;
        self.last_check = Instant::now();

        // 连续失败 3 次标记为不健康
        if self.recent_failures >= 3 && self.recent_successes == 0 {
            self.is_healthy = false;
        }

        // 滑动窗口
        if self.recent_successes + self.recent_failures > 100 {
            self.recent_successes = (self.recent_successes * 9) / 10;
            self.recent_failures = (self.recent_failures * 9) / 10;
        }
    }

    /// 增加活跃连接
    pub fn increment_connections(&mut self) {
        self.active_connections += 1;
    }

    /// 减少活跃连接
    pub fn decrement_connections(&mut self) {
        self.active_connections = self.active_connections.saturating_sub(1);
    }
}

/// 负载均衡器
#[derive(Debug)]
pub struct LoadBalancer {
    /// 负载均衡策略
    strategy: LoadBalancerStrategy,
    /// 当前轮询索引（用于 RoundRobin）
    current_index: Arc<RwLock<usize>>,
    /// 账号健康状态
    health_map: Arc<RwLock<HashMap<String, AccountHealth>>>,
    /// 健康检查间隔
    #[allow(dead_code)]
    health_check_interval: Duration,
    /// 速率限制的账号（临时屏蔽）
    rate_limited_accounts: Arc<RwLock<HashMap<String, Instant>>>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancerStrategy) -> Self {
        Self {
            strategy,
            current_index: Arc::new(RwLock::new(0)),
            health_map: Arc::new(RwLock::new(HashMap::new())),
            health_check_interval: Duration::from_secs(30),
            rate_limited_accounts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 选择账号
    pub async fn select_account(&self, accounts: &[Account]) -> Option<Account> {
        if accounts.is_empty() {
            return None;
        }

        // 过滤健康的账号
        let healthy_accounts = self.filter_healthy_accounts(accounts).await;

        if healthy_accounts.is_empty() {
            // 如果没有健康的账号，尝试重置健康状态
            self.reset_health_if_all_unhealthy(accounts).await;
            return accounts.first().cloned();
        }

        match self.strategy {
            LoadBalancerStrategy::RoundRobin => self.select_round_robin(&healthy_accounts).await,
            LoadBalancerStrategy::Random => self.select_random(&healthy_accounts),
            LoadBalancerStrategy::Balanced => self.select_balanced(&healthy_accounts),
            LoadBalancerStrategy::MostQuota => self.select_most_quota(&healthy_accounts),
            LoadBalancerStrategy::WeightedRandom => {
                self.select_weighted_random(&healthy_accounts).await
            }
            LoadBalancerStrategy::LeastConnections => {
                self.select_least_connections(&healthy_accounts).await
            }
        }
    }

    /// 过滤健康的账号（排除速率限制和已封禁的账号）
    async fn filter_healthy_accounts(&self, accounts: &[Account]) -> Vec<Account> {
        let health_map = self.health_map.read().await;
        let rate_limited = self.rate_limited_accounts.read().await;

        accounts
            .iter()
            .filter(|acc| {
                // 检查账号是否启用（数据库持久化状态）
                if !acc.enabled {
                    log::debug!("[LoadBalancer] 跳过已禁用的账号: {} (enabled=false)", acc.label);
                    return false;
                }

                // 检查是否被速率限制
                if let Some(blocked_at) = rate_limited.get(&acc.id) {
                    if blocked_at.elapsed().as_secs() < 60 {
                        log::debug!("[LoadBalancer] 跳过被速率限制的账号: {}", acc.label);
                        return false;
                    }
                }

                // 检查健康状态
                health_map
                    .get(&acc.id)
                    .map(|h| h.is_healthy)
                    .unwrap_or(true) // 新账号默认健康
            })
            .cloned()
            .collect()
    }

    /// 如果所有账号都不健康，重置健康状态
    async fn reset_health_if_all_unhealthy(&self, accounts: &[Account]) {
        let mut health_map = self.health_map.write().await;

        let all_unhealthy = accounts.iter().all(|acc| {
            health_map
                .get(&acc.id)
                .map(|h| !h.is_healthy)
                .unwrap_or(false)
        });

        if all_unhealthy {
            log::warn!("[LoadBalancer] 所有账号都不健康，执行自愈机制重置健康状态");
            for acc in accounts {
                if let Some(health) = health_map.get_mut(&acc.id) {
                    health.is_healthy = true;
                    health.recent_failures = 0;
                }
            }
        }
    }

    /// 轮询选择
    async fn select_round_robin(&self, accounts: &[Account]) -> Option<Account> {
        let mut index = self.current_index.write().await;
        let account = accounts.get(*index % accounts.len()).cloned();
        *index = (*index + 1) % accounts.len();
        account
    }

    /// 随机选择
    fn select_random(&self, accounts: &[Account]) -> Option<Account> {
        let mut rng = thread_rng();
        accounts.choose(&mut rng).cloned()
    }

    /// 均衡选择（优先使用成功次数最少的账号）
    fn select_balanced(&self, accounts: &[Account]) -> Option<Account> {
        accounts.iter().min_by_key(|acc| acc.success_count).cloned()
    }

    /// 最多配额选择
    fn select_most_quota(&self, accounts: &[Account]) -> Option<Account> {
        accounts
            .iter()
            .max_by(|a, b| {
                let quota_a = remaining_quota(a);
                let quota_b = remaining_quota(b);
                quota_a
                    .partial_cmp(&quota_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// 加权随机选择（根据健康分数和配额加权）
    async fn select_weighted_random(&self, accounts: &[Account]) -> Option<Account> {
        let health_map = self.health_map.read().await;

        // 计算权重：健康分数 × 剩余配额百分比
        let weights: Vec<u32> = accounts
            .iter()
            .map(|acc| {
                let health_score = health_map
                    .get(&acc.id)
                    .map(|h| h.health_score())
                    .unwrap_or(100);

                let quota_percent = (remaining_quota(acc) * 100.0) as u32;

                // 权重 = 健康分数 × (配额百分比 + 1)
                // +1 避免配额为 0 时权重为 0
                health_score * (quota_percent + 1)
            })
            .collect();

        // 如果所有权重都是 0，返回第一个账号
        if weights.iter().all(|&w| w == 0) {
            return accounts.first().cloned();
        }

        // 加权随机选择
        let dist = WeightedIndex::new(&weights).ok()?;
        let mut rng = thread_rng();
        let index = dist.sample(&mut rng);

        accounts.get(index).cloned()
    }

    /// 最少连接选择
    async fn select_least_connections(&self, accounts: &[Account]) -> Option<Account> {
        let health_map = self.health_map.read().await;

        accounts
            .iter()
            .min_by_key(|acc| {
                health_map
                    .get(&acc.id)
                    .map(|h| h.active_connections)
                    .unwrap_or(0)
            })
            .cloned()
    }

    /// 增加账号的活跃连接数
    pub async fn increment_connections(&self, account_id: &str) {
        let mut health_map = self.health_map.write().await;
        health_map
            .entry(account_id.to_string())
            .or_insert_with(|| AccountHealth::new(account_id.to_string()))
            .increment_connections();
    }

    /// 减少账号的活跃连接数
    pub async fn decrement_connections(&self, account_id: &str) {
        let mut health_map = self.health_map.write().await;
        if let Some(health) = health_map.get_mut(account_id) {
            health.decrement_connections();
        }
    }

    /// 记录账号请求成功
    pub async fn record_success(&self, account_id: &str, response_time_ms: u64) {
        let mut health_map = self.health_map.write().await;
        health_map
            .entry(account_id.to_string())
            .or_insert_with(|| AccountHealth::new(account_id.to_string()))
            .record_success(response_time_ms);
    }

    /// 记录账号请求失败
    pub async fn record_failure(&self, account_id: &str) {
        let mut health_map = self.health_map.write().await;
        health_map
            .entry(account_id.to_string())
            .or_insert_with(|| AccountHealth::new(account_id.to_string()))
            .record_failure();
    }

    /// 标记账号为速率限制（临时屏蔽 60 秒）
    pub async fn mark_rate_limited(&self, account_id: &str) {
        let mut rate_limited = self.rate_limited_accounts.write().await;
        rate_limited.insert(account_id.to_string(), Instant::now());
        log::info!(
            "[LoadBalancer] 账号 {} 被标记为速率限制，将屏蔽 60 秒",
            account_id
        );
    }

    /// 标记账号为已封禁（永久屏蔽，直到手动重置）
    ///
    /// 此方法会直接更新账号数据库，设置 status="banned" 和 enabled=false
    /// 不再使用内存标记，直接依赖数据库的 enabled 字段过滤
    pub async fn mark_account_banned(&self, account_id: &str) {
        log::warn!(
            "[LoadBalancer] 账号 {} 被标记为已封禁，正在持久化到数据库",
            account_id
        );

        // 标记为不健康
        let mut health_map = self.health_map.write().await;
        if let Some(health) = health_map.get_mut(account_id) {
            health.is_healthy = false;
            health.recent_failures = 999; // 设置高失败次数
        }
        drop(health_map); // 释放锁

        // 持久化到数据库
        if let Err(e) = Self::persist_banned_status(account_id).await {
            log::error!(
                "[LoadBalancer] 持久化账号 {} 的封禁状态失败: {}",
                account_id,
                e
            );
        }
    }

    /// 持久化封禁状态到数据库
    async fn persist_banned_status(account_id: &str) -> Result<(), String> {
        use crate::core::account::AccountStore;
        use crate::commands::common::update_account_status;

        let mut store = AccountStore::new();

        // 查找并更新账号
        let mut found = false;
        let accounts = &mut store.accounts;

        for account in accounts.iter_mut() {
            if account.id == account_id {
                found = true;

                // 更新状态为封禁，enabled 设为 false
                update_account_status(account, true, false);

                log::info!(
                    "[LoadBalancer] 正在持久化账号 {} 的封禁状态: status={}, enabled={}",
                    account_id,
                    account.status,
                    account.enabled
                );
                break;
            }
        }

        if !found {
            return Err(format!("账号 {} 未找到", account_id));
        }

        // 保存到文件
        store.try_save_to_file()?;

        log::info!("[LoadBalancer] 账号 {} 的封禁状态已持久化到数据库", account_id);
        Ok(())
    }

    /// 获取所有被速率限制的账号
    pub async fn get_rate_limited_accounts(&self) -> Vec<String> {
        let mut rate_limited = self.rate_limited_accounts.write().await;

        // 清理过期的屏蔽
        rate_limited.retain(|account_id, blocked_at| {
            if blocked_at.elapsed().as_secs() >= 60 {
                log::info!("[LoadBalancer] 账号 {} 速率限制已自动解除", account_id);
                false
            } else {
                true
            }
        });

        rate_limited.keys().cloned().collect()
    }

    /// 清除账号的速率限制标记
    pub async fn clear_rate_limit(&self, account_id: &str) {
        let mut rate_limited = self.rate_limited_accounts.write().await;
        if rate_limited.remove(account_id).is_some() {
            log::info!("[LoadBalancer] 手动清除账号 {} 的速率限制", account_id);
        }
    }

    /// 获取所有账号的健康状态
    pub async fn get_all_health(&self) -> HashMap<String, AccountHealth> {
        let health_map = self.health_map.read().await;
        health_map.clone()
    }

    /// 重置账号健康状态
    pub async fn reset_health(&self, account_id: &str) {
        let mut health_map = self.health_map.write().await;
        if let Some(health) = health_map.get_mut(account_id) {
            health.is_healthy = true;
            health.recent_failures = 0;
            health.recent_successes = 0;
        }
    }

    /// 清理过期的健康状态（超过 1 小时未使用）
    pub async fn cleanup_stale_health(&self) {
        let mut health_map = self.health_map.write().await;
        let now = Instant::now();
        let stale_duration = Duration::from_secs(3600); // 1 小时

        health_map.retain(|_, health| now.duration_since(health.last_check) < stale_duration);
    }
}

/// 计算账号剩余配额百分比
fn remaining_quota(account: &Account) -> f64 {
    // 从 usage_data 中提取配额信息
    if let Some(usage_data) = &account.usage_data {
        // 尝试提取 remaining 和 total
        if let (Some(remaining), Some(total)) = (
            usage_data.get("remaining").and_then(|v| v.as_i64()),
            usage_data.get("total").and_then(|v| v.as_i64()),
        ) {
            if total > 0 {
                return (remaining as f64 / total as f64) * 100.0;
            }
        }
    }
    100.0 // 默认 100%
}

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

        // 记录失败（有成功记录时不会标记为不健康，但分数会下降）
        health.record_failure();
        health.record_failure();
        health.record_failure();
        // 有 1 次成功 + 3 次失败 = 25% 成功率，仍然 is_healthy（因为有成功记录）
        assert!(health.is_healthy);
        assert!(health.health_score() < 50);

        // 全新账号，纯失败场景
        let mut health2 = AccountHealth::new("test2".to_string());
        health2.record_failure();
        health2.record_failure();
        health2.record_failure();
        assert!(!health2.is_healthy);
        assert_eq!(health2.health_score(), 0);
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
