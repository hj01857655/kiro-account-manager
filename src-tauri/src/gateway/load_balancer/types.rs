//! 策略枚举与账号健康度数据结构（含序列化形态）。

use super::*;

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
    /// **连续**失败次数：一次成功即清零。用于判断账号是否该被标记不健康。
    ///
    /// 与 `window_failures` 分开：前者回答「现在是不是连着坏」，
    /// 后者回答「最近一段时间坏的比例」。混用一个字段会让两者都算错。
    pub consecutive_failures: usize,
    /// 滑动窗口内的失败次数（用于成功率统计）
    pub recent_failures: usize,
    /// 滑动窗口内的成功次数
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
    pub consecutive_failures: usize,
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
            consecutive_failures: health.consecutive_failures,
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
            consecutive_failures: 0,
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
        // 成功即中断连续失败，否则「连续」语义不成立
        self.consecutive_failures = 0;
        self.is_healthy = true;
        self.last_check = Instant::now();

        // 更新平均响应时间（简单移动平均）
        if self.avg_response_time_ms == 0 {
            self.avg_response_time_ms = response_time_ms;
        } else {
            self.avg_response_time_ms = (self.avg_response_time_ms * 9 + response_time_ms) / 10;
        }

        self.truncate_window();
    }

    /// 连续失败达到该阈值即标记不健康。
    pub const CONSECUTIVE_FAILURE_THRESHOLD: usize = 3;

    /// 成功率统计窗口大小（次）。
    pub const WINDOW_SIZE: usize = 100;

    /// 窗口真截断：保留最近 [`Self::WINDOW_SIZE`] 次，丢弃更早的。
    ///
    /// 原实现是 `count * 9 / 10` 的指数衰减，永不归零也永不到达稳态：
    /// 一个 1024 次前的失败会一直以极小权重影响成功率，
    /// 「最近 100 次」名不副实。
    fn truncate_window(&mut self) {
        let total = self.recent_successes + self.recent_failures;
        if total <= Self::WINDOW_SIZE {
            return;
        }
        // 成比例缩减，保持成功率不变，只压总量
        let keep_successes = self.recent_successes * Self::WINDOW_SIZE / total;
        let keep_failures = Self::WINDOW_SIZE - keep_successes;
        self.recent_successes = keep_successes;
        self.recent_failures = keep_failures;
    }

    /// 记录失败
    ///
    /// 语义是**连续失败**：一次成功即把连续计数清零。
    /// 原实现用 `recent_failures >= 3 && recent_successes == 0` 判断，
    /// 那是「累计 3 次失败且从未成功」——一个曾经成功过 100 次、
    /// 期间失败 3 次的账号会被误判为一直不健康，与注释不符。
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.recent_failures += 1;
        self.last_check = Instant::now();

        if self.consecutive_failures >= Self::CONSECUTIVE_FAILURE_THRESHOLD {
            self.is_healthy = false;
        }

        self.truncate_window();
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
