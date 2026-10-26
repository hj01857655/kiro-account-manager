use crate::gateway::GatewayRequestLogEntry;
/// 内存日志存储模块
///
use std::sync::Arc;
use tokio::sync::RwLock;

/// 内存日志存储
#[derive(Debug, Clone)]
pub struct LogStore {
    inner: Arc<RwLock<LogStoreInner>>,
}

#[derive(Debug)]
struct LogStoreInner {
    /// 日志列表（按请求索引排序，最新的在后面）
    logs: Vec<GatewayRequestLogEntry>,
    /// 最大保存条数
    max_logs: usize,
    /// 本次运行的起点：启动恢复的历史条目数。get_last_session 只取这之后的条目
    session_start: usize,
}
impl LogStore {
    /// 创建新的日志存储
    ///
    /// # Arguments
    /// * `max_logs` - 最大保存的日志条数（默认 10000）
    pub fn new(max_logs: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LogStoreInner {
                logs: Vec::with_capacity(max_logs.min(1000)),
                max_logs,
                session_start: 0,
            })),
        }
    }

    /// 添加一条日志
    pub async fn add(&self, entry: GatewayRequestLogEntry) {
        let mut inner = self.inner.write().await;

        // 添加到存储
        inner.logs.push(entry.clone());

        // 超过最大数量时删除最旧的
        if inner.logs.len() > inner.max_logs {
            // 保留最新的 max_logs 条
            let drain_count = inner.logs.len() - inner.max_logs;
            inner.logs.drain(0..drain_count);
            // 会话起点是相对索引，截断后前移
            inner.session_start = inner.session_start.saturating_sub(drain_count);
        }
    }

    /// 获取所有日志
    pub async fn get_all(&self) -> Vec<GatewayRequestLogEntry> {
        let inner = self.inner.read().await;
        inner.logs.clone()
    }

    /// 获取最近 N 条日志
    pub async fn get_last(&self, count: usize) -> Vec<GatewayRequestLogEntry> {
        let inner = self.inner.read().await;
        let start = inner.logs.len().saturating_sub(count);
        inner.logs[start..].to_vec()
    }

    /// 标记启动恢复的历史条数：之后的 add 才算「本次运行」的流量
    pub async fn set_session_start(&self, restored: usize) {
        let mut inner = self.inner.write().await;
        inner.session_start = restored.min(inner.logs.len());
    }

    /// 获取本次运行（会话）内的最近 N 条——不含启动时从文件回灌的历史。
    /// 概览「最近请求」用它：看到的永远是正在发生的流量，不是上一轮会话的旧数据。
    pub async fn get_last_session(&self, count: usize) -> Vec<GatewayRequestLogEntry> {
        let inner = self.inner.read().await;
        let session = &inner.logs[inner.session_start.min(inner.logs.len())..];
        let start = session.len().saturating_sub(count);
        session[start..].to_vec()
    }

    /// 清空所有日志
    pub async fn clear(&self) {
        let mut inner = self.inner.write().await;
        inner.logs.clear();
        inner.session_start = 0;
    }

    /// 获取日志统计信息
    pub async fn get_stats(&self) -> LogStats {
        let inner = self.inner.read().await;

        let mut stats = LogStats {
            total: inner.logs.len(),
            success: 0,
            error: 0,
            streaming: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read_tokens: 0,
            total_cache_creation_tokens: 0,
            total_credits: 0.0,
            avg_duration_ms: 0,
            requests_with_cache: 0,
        };

        if inner.logs.is_empty() {
            return stats;
        }

        let mut total_duration: u64 = 0;

        for log in &inner.logs {
            if log.status_code < 400 {
                stats.success += 1;
            } else {
                stats.error += 1;
            }

            if log.stream {
                stats.streaming += 1;
            }

            stats.total_input_tokens += log.input_tokens.unwrap_or(0);
            stats.total_output_tokens += log.output_tokens.unwrap_or(0);
            stats.total_cache_read_tokens += log.cache_read_input_tokens.unwrap_or(0);
            stats.total_cache_creation_tokens += log.cache_creation_input_tokens.unwrap_or(0);
            stats.total_credits += log.credits.unwrap_or(0.0);

            if log.cache_read_input_tokens.unwrap_or(0) > 0
                || log.cache_creation_input_tokens.unwrap_or(0) > 0
            {
                stats.requests_with_cache += 1;
            }

            total_duration += log.duration_ms;
        }

        stats.avg_duration_ms = (total_duration as f64 / inner.logs.len() as f64) as u64;

        stats
    }

    /// 按模型分组统计
    pub async fn get_model_stats(&self) -> Vec<ModelStat> {
        let inner = self.inner.read().await;

        let mut model_map: std::collections::HashMap<String, ModelStat> =
            std::collections::HashMap::new();

        for log in &inner.logs {
            let model = log.model.clone().unwrap_or_else(|| "unknown".to_string());
            let stat = model_map.entry(model.clone()).or_insert(ModelStat {
                model,
                count: 0,
                success: 0,
                error: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
            });

            stat.count += 1;
            if log.outcome == "success" {
                stat.success += 1;
            } else if log.outcome == "error" {
                stat.error += 1;
            }
            stat.total_input_tokens += log.input_tokens.unwrap_or(0);
            stat.total_output_tokens += log.output_tokens.unwrap_or(0);
        }

        let mut stats: Vec<ModelStat> = model_map.into_values().collect();
        stats.sort_by(|a, b| b.count.cmp(&a.count));
        stats
    }

    /// 按端点分组统计
    pub async fn get_endpoint_stats(&self) -> Vec<EndpointStat> {
        let inner = self.inner.read().await;

        let mut endpoint_map: std::collections::HashMap<String, EndpointStat> =
            std::collections::HashMap::new();

        for log in &inner.logs {
            let stat = endpoint_map
                .entry(log.endpoint.clone())
                .or_insert(EndpointStat {
                    endpoint: log.endpoint.clone(),
                    count: 0,
                    success: 0,
                    error: 0,
                });

            stat.count += 1;
            if log.outcome == "success" {
                stat.success += 1;
            } else if log.outcome == "error" {
                stat.error += 1;
            }
        }

        let mut stats: Vec<EndpointStat> = endpoint_map.into_values().collect();
        stats.sort_by(|a, b| b.count.cmp(&a.count));
        stats
    }

    /// 按客户端 UA 分组统计。
    ///
    /// 回答的是「谁在用这个网关」。密钥固定为 `kiro` 后 Key 不再是区分性维度，
    /// 只按 UA 产品名分组；表里的「客户端」列即 UA。
    pub async fn get_client_stats(&self) -> Vec<ClientStat> {
        let inner = self.inner.read().await;

        let mut map: std::collections::HashMap<Option<String>, ClientStat> =
            std::collections::HashMap::new();

        for log in &inner.logs {
            let product = log.client_user_agent.as_deref().and_then(ua_product);
            let stat = map.entry(product.clone()).or_insert(ClientStat {
                product,
                count: 0,
                success: 0,
                error: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
            });

            stat.count += 1;
            if log.status_code < 400 {
                stat.success += 1;
            } else {
                stat.error += 1;
            }
            stat.total_input_tokens += log.input_tokens.unwrap_or(0);
            stat.total_output_tokens += log.output_tokens.unwrap_or(0);
        }

        let mut stats: Vec<ClientStat> = map.into_values().collect();
        stats.sort_by(|a, b| b.count.cmp(&a.count));
        stats
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new(10000) // 默认保存 10000 条日志
    }
}

/// 日志统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogStats {
    pub total: usize,
    pub success: usize,
    pub error: usize,
    pub streaming: usize,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
    pub total_cache_read_tokens: i32,
    pub total_cache_creation_tokens: i32,
    /// 上游 meteringEvent 的真实计费消耗合计（credits）
    pub total_credits: f64,
    pub avg_duration_ms: u64,
    pub requests_with_cache: usize,
}

/// 模型统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStat {
    pub model: String,
    pub count: usize,
    pub success: usize,
    pub error: usize,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
}

/// 端点统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStat {
    pub endpoint: String,
    pub count: usize,
    pub success: usize,
    pub error: usize,
}

/// 一个客户端工具的用量。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStat {
    /// UA 首段的产品名，如 `claude-cli`、`cursor-agent`；无 UA 时为 None
    pub product: Option<String>,
    pub count: usize,
    pub success: usize,
    pub error: usize,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
}

/// UA 首段的产品名：`claude-cli/2.1.0 (external, cli)` → `claude-cli`。
///
/// 只取首段是故意的：版本号会让同一个客户端每次升级就多出一行，
/// 而括号里的平台信息对「谁在用」这个问题没有信息量。
fn ua_product(ua: &str) -> Option<String> {
    let first = ua.split('/').next()?.split_whitespace().next()?;
    let name = first.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_log(index: u64, outcome: &str, model: &str) -> GatewayRequestLogEntry {
        GatewayRequestLogEntry {
            occurred_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            request_index: index,
            endpoint: "messages".to_string(),
            client_ip: "127.0.0.1".to_string(),
            client_user_agent: None,
            account_id: Some("test-account".to_string()),
            session_id: Some("sess-test|127.0.0.1".to_string()),
            model: Some(model.to_string()),
            stream: false,
            upstream_source: Some("test".to_string()),
            region: Some("us-east-1".to_string()),
            status_code: if outcome == "success" { 200 } else { 500 },
            outcome: outcome.to_string(),
            duration_ms: 1000,
            error: if outcome == "error" {
                Some("test error".to_string())
            } else {
                None
            },
            request_body: None,
            response_body: None,
            input_tokens: Some(100),
            output_tokens: Some(50),
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            credits: None,
            context_usage_percentage: None,
            error_type: None,
            stream_info: None,
            request_summary: None,
            response_summary: None,
        }
    }

    #[tokio::test]
    async fn test_log_store_add_and_get() {
        let store = LogStore::new(100);

        let log1 = create_test_log(1, "success", "claude-sonnet-4.5");
        let log2 = create_test_log(2, "error", "claude-opus-4");

        store.add(log1.clone()).await;
        store.add(log2.clone()).await;

        let all_logs = store.get_all().await;
        assert_eq!(all_logs.len(), 2);
        assert_eq!(all_logs[0].request_index, 1);
        assert_eq!(all_logs[1].request_index, 2);
    }

    #[tokio::test]
    async fn test_log_store_max_limit() {
        let store = LogStore::new(5);

        // 添加 10 条日志
        for i in 0..10 {
            store
                .add(create_test_log(i, "success", "claude-sonnet-4.5"))
                .await;
        }

        let all_logs = store.get_all().await;
        assert_eq!(all_logs.len(), 5); // 只保留最新的 5 条
        assert_eq!(all_logs[0].request_index, 5); // 最旧的是 #5
        assert_eq!(all_logs[4].request_index, 9); // 最新的是 #9
    }

    #[tokio::test]
    async fn test_log_store_get_last() {
        let store = LogStore::new(100);

        for i in 0..10 {
            store
                .add(create_test_log(i, "success", "claude-sonnet-4.5"))
                .await;
        }

        let last_3 = store.get_last(3).await;
        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].request_index, 7);
        assert_eq!(last_3[2].request_index, 9);
    }

    #[tokio::test]
    async fn test_log_store_stats() {
        let store = LogStore::new(100);

        store
            .add(create_test_log(1, "success", "claude-sonnet-4.5"))
            .await;
        store
            .add(create_test_log(2, "success", "claude-sonnet-4.5"))
            .await;
        store
            .add(create_test_log(3, "error", "claude-opus-4"))
            .await;

        let stats = store.get_stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success, 2);
        assert_eq!(stats.error, 1);
        assert_eq!(stats.total_input_tokens, 300); // 3 * 100
        assert_eq!(stats.total_output_tokens, 150); // 3 * 50
    }

    #[tokio::test]
    async fn test_log_store_stats_counts_http_successes_and_stream_flag() {
        let store = LogStore::new(100);

        let mut stream_log = create_test_log(1, "stream", "claude-sonnet-4.5");
        stream_log.status_code = 200;
        stream_log.stream = true;

        let mut cached_log = create_test_log(2, "success (cached)", "claude-sonnet-4.5");
        cached_log.status_code = 200;

        store.add(stream_log).await;
        store.add(cached_log).await;
        store
            .add(create_test_log(3, "error", "claude-opus-4"))
            .await;

        let stats = store.get_stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success, 2);
        assert_eq!(stats.error, 1);
        assert_eq!(stats.streaming, 1);
    }

    #[tokio::test]
    async fn test_log_store_model_stats() {
        let store = LogStore::new(100);

        store
            .add(create_test_log(1, "success", "claude-sonnet-4.5"))
            .await;
        store
            .add(create_test_log(2, "success", "claude-sonnet-4.5"))
            .await;
        store
            .add(create_test_log(3, "error", "claude-opus-4"))
            .await;

        let model_stats = store.get_model_stats().await;
        assert_eq!(model_stats.len(), 2);

        // 按 count 降序排列
        assert_eq!(model_stats[0].model, "claude-sonnet-4.5");
        assert_eq!(model_stats[0].count, 2);
        assert_eq!(model_stats[0].success, 2);

        assert_eq!(model_stats[1].model, "claude-opus-4");
        assert_eq!(model_stats[1].count, 1);
        assert_eq!(model_stats[1].error, 1);
    }

    #[tokio::test]
    async fn test_log_store_client_stats_group_by_ua() {
        let store = LogStore::new(100);

        // 同一个工具的两个版本号：必须归为一行，否则「谁在用」会按版本号碎片化
        let mut a = create_test_log(1, "success", "claude-sonnet-4.5");
        a.client_user_agent = Some("claude-cli/2.1.0 (external, cli)".to_string());

        // 同一个工具的不同版本号：应归为同一行（只取 UA 首段）
        let mut b = create_test_log(2, "success", "claude-sonnet-4.5");
        b.client_user_agent = Some("claude-cli/2.2.3".to_string());

        let mut c = create_test_log(3, "error", "claude-sonnet-4.5");
        c.client_user_agent = Some("curl/8.4.0".to_string());

        // 没有 UA 的脚本：product 为 None，不能和任何有 UA 的请求混在一起
        let mut d = create_test_log(4, "success", "claude-sonnet-4.5");
        d.client_user_agent = None;

        store.add(a).await;
        store.add(b).await;
        store.add(c).await;
        store.add(d).await;

        let stats = store.get_client_stats().await;
        assert_eq!(stats.len(), 3, "claude-cli / curl / 无UA");

        let cli = stats
            .iter()
            .find(|s| s.product.as_deref() == Some("claude-cli"))
            .expect("claude-cli should be grouped");
        assert_eq!(cli.count, 2, "两个版本号归为同一行");
        assert_eq!(cli.error, 0);

        let curl = stats
            .iter()
            .find(|s| s.product.as_deref() == Some("curl"))
            .expect("curl should be its own row");
        assert_eq!(curl.error, 1);

        let no_ua = stats
            .iter()
            .find(|s| s.product.is_none())
            .expect("request without UA should be grouped");
        assert_eq!(no_ua.count, 1);
    }

    #[tokio::test]
    async fn test_log_store_clear() {
        let store = LogStore::new(100);

        store
            .add(create_test_log(1, "success", "claude-sonnet-4.5"))
            .await;
        store
            .add(create_test_log(2, "success", "claude-sonnet-4.5"))
            .await;

        assert_eq!(store.get_all().await.len(), 2);

        store.clear().await;

        assert_eq!(store.get_all().await.len(), 0);
    }
}
