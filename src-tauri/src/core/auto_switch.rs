// 自动换号策略模块
#![allow(dead_code)]

use crate::core::account::Account;
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
            strategy_str
                .map(SwitchStrategy::from_str)
                .unwrap_or_default(),
            threshold.unwrap_or(90),
            group_id,
        )
    }

    /// 检查账号是否需要切换
    ///
    /// 口径说明：分母只用**主配额**（不含超额封顶）。开了超额 = 用户授权
    /// 上限封顶前的烧钱，但换号策略的意图是「主额度耗尽前主动切换、保住
    /// 主额度」——把 10000 的 overage_cap 计入分母会把打满主额度的号判成
    /// 9% 可用、继续被选中，接下来的请求直接产生 $0.04/Credit 真实扣费。
    /// 要烧超额是显式动作，不该是调度静默带来的。
    pub fn should_switch(&self, account: &Account) -> bool {
        if !account.is_available() {
            return true;
        }

        // 从 usage_data 中解析配额信息
        if let Some((current, limit)) = extract_usage_totals(account.usage_data.as_ref()) {
            if limit > 0.0 {
                #[allow(clippy::cast_precision_loss)]
                let usage_percent = (current / limit) * 100.0;
                return usage_percent >= f64::from(self.threshold);
            }
        }

        false
    }

    /// 选择下一个可用账号
    pub fn select_next<'a>(
        &mut self,
        accounts: &'a [Account],
        current_id: &str,
    ) -> Option<&'a Account> {
        let available: Vec<&Account> = accounts
            .iter()
            .filter(|a| {
                if a.id == current_id {
                    return false;
                }
                if !a.is_available() {
                    return false;
                }
                if let Some(ref gid) = self.group_id {
                    if a.group_id.as_ref() != Some(gid) {
                        return false;
                    }
                }
                true
            })
            .collect();

        if available.is_empty() {
            return None;
        }

        match self.strategy {
            SwitchStrategy::RoundRobin => {
                self.round_robin_index = (self.round_robin_index + 1) % available.len();
                Some(available[self.round_robin_index])
            }
            SwitchStrategy::MostQuota => {
                // 选择剩余配额最多的账号
                available
                    .iter()
                    .max_by_key(|a| {
                        if let Some((current, limit)) = extract_usage_totals(a.usage_data.as_ref())
                        {
                            #[allow(clippy::cast_possible_truncation)]
                            // i64 → i32 转换用于排序，配额值不会超过 i32 范围
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
    matches!(
        status,
        "banned"
            | "封禁"
            | "已封禁"
            | "invalid"
            | "失效"
            | "已失效"
            | "Token已失效"
            | "expired"
            | "过期"
            | "已过期"
    )
}

fn extract_usage_totals(usage_data: Option<&Value>) -> Option<(f64, f64)> {
    // 复用 usage::UsageBreakdown：带 *WithPrecision 精度回退（上游给小数时
    // 旧的 i64 直读会截断成 0/丢精度），并保证与 usage.rs 同一解析口径
    let b = crate::core::usage::UsageBreakdown::from_usage_data(usage_data)?;
    Some((b.current, b.limit))
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

#[cfg(test)]
mod tests {
    use super::{is_unavailable_status, AccountSwitcher, SwitchStrategy};
    use crate::core::account::Account;
    use crate::core::usage::is_usage_capped;

    #[test]
    fn capped_status_allows_switching() {
        assert!(!is_unavailable_status("capped"));
        assert!(!is_unavailable_status("封顶"));
    }

    #[test]
    fn capped_usage_requires_switch() {
        let mut account = Account::new("capped@example.com".to_string(), "capped".to_string());
        account.usage_data = Some(serde_json::json!({
            "overageConfiguration": {
                "overageStatus": "DISABLED"
            },
            "usageBreakdownList": [
                {
                    "currentUsage": 50,
                    "usageLimit": 50
                }
            ]
        }));

        let switcher = AccountSwitcher::new(SwitchStrategy::RoundRobin, 90, None);

        assert!(is_usage_capped(account.usage_data.as_ref()));
        assert!(switcher.should_switch(&account));
    }

    /// 精度回退：上游只给 *WithPrecision 小数字段时，旧 i64 直读会得 0
    /// （永远判「不需要切换」）；复用 UsageBreakdown 后应正确解析并触发切换。
    #[test]
    fn precision_fields_are_read_for_switch_decision() {
        let mut account = Account::new("precision@example.com".to_string(), "precision".to_string());
        account.usage_data = Some(serde_json::json!({
            "overageConfiguration": { "overageStatus": "DISABLED" },
            "usageBreakdownList": [
                {
                    "currentUsageWithPrecision": 495.5,
                    "usageLimitWithPrecision": 500.0
                }
            ]
        }));

        let switcher = AccountSwitcher::new(SwitchStrategy::RoundRobin, 90, None);
        // 495.5/500 = 99.1% >= 90 → 需要切换；旧代码 current 读 0 → 0% 不切
        assert!(switcher.should_switch(&account));
    }

    /// 超额已开的打满号：主额度口径下 100% 必须判切——分母不含 overage_cap，
    /// 不能因为开了超额就把它当成 9% 可用继续调度（否则直接烧 $0.04/Credit）。
    #[test]
    fn overage_enabled_full_account_still_switches() {
        let mut account = Account::new("overage@example.com".to_string(), "overage".to_string());
        account.usage_data = Some(serde_json::json!({
            "overageConfiguration": { "overageStatus": "ENABLED" },
            "subscriptionInfo": { "overageCapability": "OVERAGE_CAPABLE" },
            "usageBreakdownList": [
                {
                    "currentUsage": 1000,
                    "usageLimit": 1000,
                    "overageCap": 10000
                }
            ]
        }));

        let switcher = AccountSwitcher::new(SwitchStrategy::RoundRobin, 90, None);
        // 主额度口径：1000/1000 = 100% >= 90 → 切
        assert!(switcher.should_switch(&account));
    }
}
