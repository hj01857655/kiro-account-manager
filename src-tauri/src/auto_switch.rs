// 自动换号策略模块
#![allow(dead_code)]

use crate::account::Account;
use rand::seq::SliceRandom;
use serde_json::Value;

/// 换号策略枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchStrategy {
    /// 轮询：按顺序依次选择账号
    #[default]
    RoundRobin,
    /// 最多配额：选择剩余配额最多的账号
    MostQuota,
    /// 随机：随机选择一个可用账号
    Random,
}

impl SwitchStrategy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "most_quota" => Self::MostQuota,
            "random" => Self::Random,
            _ => Self::RoundRobin,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::MostQuota => "most_quota",
            Self::Random => "random",
        }
    }
}

/// 自动换号选择器
pub struct AccountSwitcher {
    strategy: SwitchStrategy,
    round_robin_index: usize,
    threshold: i32,
    group_id: Option<String>,
}

impl AccountSwitcher {
    pub fn new(strategy: SwitchStrategy, threshold: i32, group_id: Option<String>) -> Self {
        Self {
            strategy,
            round_robin_index: 0,
            threshold: threshold.clamp(0, 100),
            group_id,
        }
    }

    pub fn from_settings(
        strategy_str: Option<&str>,
        threshold: Option<i32>,
        group_id: Option<String>,
    ) -> Self {
        Self::new(
            strategy_str.map(SwitchStrategy::from_str).unwrap_or_default(),
            threshold.unwrap_or(90),
            group_id,
        )
    }

    /// 检查账号是否需要切换
    pub fn should_switch(&self, account: &Account) -> bool {
        if is_unavailable_status(&account.status) {
            return true;
        }

        // 从 usage_data 中解析配额信息
        if let Some((current, limit)) = extract_usage_totals(account.usage_data.as_ref()) {
            if limit > 0 {
                #[allow(clippy::cast_precision_loss)] // i64 → f64 转换用于百分比计算，精度损失可接受
                let usage_percent = (current as f64 / limit as f64) * 100.0;
                return usage_percent >= f64::from(self.threshold);
            }
        }

        false
    }

    /// 选择下一个可用账号
    pub fn select_next<'a>(&mut self, accounts: &'a [Account], current_id: &str) -> Option<&'a Account> {
        let available: Vec<&Account> = accounts.iter()
            .filter(|a| {
                if a.id == current_id { return false; }
                if !a.is_available() { return false; }
                if let Some(ref gid) = self.group_id {
                    if a.group_id.as_ref() != Some(gid) { return false; }
                }
                true
            })
            .collect();

        if available.is_empty() { return None; }

        match self.strategy {
            SwitchStrategy::RoundRobin => {
                self.round_robin_index = (self.round_robin_index + 1) % available.len();
                Some(available[self.round_robin_index])
            }
            SwitchStrategy::MostQuota => {
                // 选择剩余配额最多的账号
                available.iter()
                    .max_by_key(|a| {
                        if let Some((current, limit)) = extract_usage_totals(a.usage_data.as_ref()) {
                            #[allow(clippy::cast_possible_truncation)] // i64 → i32 转换用于排序，配额值不会超过 i32 范围
                            return (limit - current) as i32;
                        }
                        0
                    })
                    .copied()
            }
            SwitchStrategy::Random => {
                let mut rng = rand::thread_rng();
                available.choose(&mut rng).copied()
            }
        }
    }
}

fn is_unavailable_status(status: &str) -> bool {
    matches!(status, "banned" | "封禁" | "已封禁" | "已过期")
}

fn extract_usage_totals(usage_data: Option<&Value>) -> Option<(i64, i64)> {
    let breakdown = usage_data?
        .get("usageBreakdownList")?
        .as_array()?
        .first()?;

    let current = breakdown
        .get("currentUsage")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let limit = breakdown
        .get("usageLimit")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    Some((current, limit))
}

/// 换号结果
#[derive(Debug, Clone)]
pub struct SwitchResult {
    pub success: bool,
    pub new_account_id: Option<String>,
    pub new_account_email: Option<String>,
    pub message: String,
}

impl SwitchResult {
    pub fn success(account: &Account) -> Self {
        Self {
            success: true,
            new_account_id: Some(account.id.clone()),
            new_account_email: account.email.clone(),
            message: format!("已切换到账号: {}", account.get_display_id()),
        }
    }

    pub fn no_available() -> Self {
        Self {
            success: false,
            new_account_id: None,
            new_account_email: None,
            message: "没有可用的账号可切换".to_string(),
        }
    }

    pub fn not_needed() -> Self {
        Self {
            success: false,
            new_account_id: None,
            new_account_email: None,
            message: "当前账号配额充足，无需切换".to_string(),
        }
    }
}
