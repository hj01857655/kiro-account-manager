//! LoadBalancer 主体：取号、健康检查、加权轮询与故障转移，以及配额估算。

use super::*;

/// 粘性哈希：进程内稳定即可（无需跨进程一致，进程重启映射自然重建）。
///
/// `DefaultHasher::new` 是零种子，同 key 同结果；`RandomState` 的种子每次
/// 启动随机，会让「同 key 同账号」的跨重启稳定性失效，故不用它。
fn stable_hash(key: &str) -> usize {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(key.as_bytes());
    hasher.finish() as usize
}

/// 负载均衡器
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
    /// 账号状态的唯一共享副本（= `AppState.store`）。
    ///
    /// 封禁等持久化必须落到这一份，否则与 Tauri 命令各写各的快照、互相覆盖。
    store: crate::state::SharedAccountStore,
}

/// 连接位守卫：持有期间该账号的 `active_connections` 保持 +1，
/// Drop 时归零。用于把连接计数覆盖到整个请求周期。
pub struct ConnectionGuard {
    balancer: Arc<LoadBalancer>,
    account_id: String,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        // Drop 不能 await，因此这里用 try_write 尽力而为：
        // 拿不到写锁时计数会偏高一格，但账号不会被永久卡死
        // （`select_least_connections` 只读瞬时值，下次请求会自然回落）。
        if let Ok(mut health_map) = self.balancer.health_map.try_write() {
            if let Some(health) = health_map.get_mut(&self.account_id) {
                health.decrement_connections();
            }
        }
    }
}

impl LoadBalancer {
    /// 仅供测试使用：自建一份独立账号副本。
    ///
    /// 生产路径必须用 `with_store` 注入 `AppState.store`，否则又会回到
    /// 各写各的快照、互相覆盖的老问题。
    #[allow(dead_code)]
    pub fn new(strategy: LoadBalancerStrategy) -> Self {
        Self::with_store(strategy, std::sync::Arc::new(std::sync::Mutex::new(
            crate::core::account::AccountStore::new(),
        )))
    }

    /// 注入共享账号副本；gateway 运行时用这个，保证与管理器同一份状态。
    pub fn with_store(
        strategy: LoadBalancerStrategy,
        store: crate::state::SharedAccountStore,
    ) -> Self {
        Self {
            strategy,
            current_index: Arc::new(RwLock::new(0)),
            health_map: Arc::new(RwLock::new(HashMap::new())),
            health_check_interval: Duration::from_secs(30),
            rate_limited_accounts: Arc::new(RwLock::new(HashMap::new())),
            store,
        }
    }

    /// 选择账号。
    ///
    /// `affinity_key` 非空时启用会话粘性：同 key 固定映射到健康集合中的
    /// 同一个账号（key 哈希取模），让上游 prompt cache 按账号维度持续命中。
    /// 无粘性 key 或健康集合变化时自然回落到当前策略。
    pub async fn select_account(
        &self,
        accounts: &[Account],
        affinity_key: Option<&str>,
    ) -> Option<Account> {
        if accounts.is_empty() {
            return None;
        }

        // 过滤健康的账号
        let healthy_accounts = self.filter_healthy_accounts(accounts).await;

        if healthy_accounts.is_empty() {
            // 如果没有健康的，尝试重置健康状态
            self.reset_health_if_all_unhealthy(accounts).await;
            return accounts.first().cloned();
        }

        // 会话粘性：同客户端固定选号，优先于策略
        if let Some(key) = affinity_key {
            return Some(self.select_sticky(&healthy_accounts, key));
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

    /// 粘性选号：key 哈希在健康集合里固定取一个。
    ///
    /// 有意不做「账号-客户端」映射表：映射表要维护 TTL 与清理，而按 id 排序后
    /// 哈希取模即可保证同一 key 在集合不变时永远命中同一账号；账号增减只影响
    /// 落在边界上的少数 key，代价可接受（换来的是 cache 命中的大多数场景）。
    fn select_sticky(&self, accounts: &[Account], key: &str) -> Account {
        let mut sorted: Vec<&Account> = accounts.iter().collect();
        // 先按 id 排序：账号 Vec 的顺序来自账号库遍历，不排序时同一 key
        // 可能因底层顺序变化而落到不同账号
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let hash = stable_hash(key);
        sorted[hash % sorted.len()].clone()
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
                    health.consecutive_failures = 0;
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

    /// 减少账号的活跃连接数。
    ///
    /// 常规路径应使用 [`LoadBalancer::acquire_connection`] 返回的 guard，
    /// 由 Drop 自动归零；这个手动版本只在确知不会中途返回时使用。
    #[allow(dead_code)]
    pub async fn decrement_connections(&self, account_id: &str) {
        let mut health_map = self.health_map.write().await;
        if let Some(health) = health_map.get_mut(account_id) {
            health.decrement_connections();
        }
    }

    /// 为一次请求占住一个连接位，返回 RAII guard。
    ///
    /// 连接计数必须覆盖**整个请求周期**才有意义：原先加/减都挤在取凭据那一段，
    /// 请求还没发出去计数就归零，`LeastConnections` 因此永远看到 0。
    /// guard 在 Drop 时归零，panic、提前 return、连接中断都能收尾，
    /// 不会把账号永久卡在「有连接」状态。
    pub async fn acquire_connection(self: &Arc<Self>, account_id: &str) -> ConnectionGuard {
        self.increment_connections(account_id).await;
        ConnectionGuard {
            balancer: Arc::clone(self),
            account_id: account_id.to_string(),
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
            // 封禁是账号级结论，不是「失败很多次」的累积结果；
            // 原实现塞 recent_failures = 999 会让成功率统计永久为 0
            health.consecutive_failures = usize::MAX;
        }
        drop(health_map); // 释放锁

        // 持久化到数据库
        if let Err(e) = self.persist_banned_status(account_id).await {
            log::error!(
                "[LoadBalancer] 持久化账号 {} 的封禁状态失败: {}",
                account_id,
                e
            );
        }
    }

    /// 持久化封禁状态到数据库
    async fn persist_banned_status(&self, account_id: &str) -> Result<(), String> {
        use crate::commands::common::update_account_status;

        let mut store = self
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

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
            health.consecutive_failures = 0;
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

/// 计算账号剩余配额比例（0-1；WeightedRandom 会 ×100 转百分比）
///
/// 真实形状是 usageBreakdownList（usageLimit/currentUsage），此前读的
/// usage_data.remaining/total 两个键全仓无人写入，导致 MostQuota 恒平手、
/// WeightedRandom 恒等权，两个策略退化为随机/固定选号。无 breakdown
/// 数据时回落 1.0（= 旧默认 100%）。
fn remaining_quota(account: &Account) -> f64 {
    crate::core::usage::UsageDetails::from_usage_data(account.usage_data.as_ref())
        .map(|d| {
            let total_limit = d.main_limit + d.trial_limit + d.bonus_limit + d.overage_cap;
            if total_limit <= 0.0 {
                1.0
            } else {
                (d.remaining() / total_limit).clamp(0.0, 1.0)
            }
        })
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account_with_usage(usage: Option<serde_json::Value>) -> Account {
        let mut acc = Account::new_enterprise("u1".to_string(), "test".to_string());
        acc.usage_data = usage;
        acc
    }

    #[test]
    fn remaining_quota_reads_usage_breakdown() {
        // 100 总配额用掉 30 → 0.7；没有 trial/bonus/overage
        let acc = account_with_usage(Some(serde_json::json!({
            "usageBreakdownList": [{
                "usageLimit": 100.0,
                "currentUsage": 30.0
            }]
        })));
        assert!((remaining_quota(&acc) - 0.7).abs() < 0.01);
    }

    #[test]
    fn remaining_quota_falls_back_without_breakdown() {
        // 旧代码读的 remaining/total 键并不是真实形状：这里保证不再被它误导
        let acc = account_with_usage(Some(serde_json::json!({ "remaining": 5, "total": 100 })));
        assert!((remaining_quota(&acc) - 1.0).abs() < 0.01);
        assert_eq!(remaining_quota(&account_with_usage(None)), 1.0);
    }
}
