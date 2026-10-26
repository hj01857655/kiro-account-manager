//! OTLP 导出：请求日志元数据转 OpenTelemetry 指标（OTLP/HTTP JSON）推送到
//! 外部观测栈。只在 `otlp_endpoint` 配置非空时启用；只发元数据（模型/账号/
//! 会话/tokens/credits/状态），不含 prompt 与响应内容。

use serde_json::{json, Value};

use super::GatewayRequestLogEntry;

/// 单条请求日志 → OTLP ExportMetricsServiceRequest（JSON 编码）。
///
/// 指标按 tokens 拆成 4 个独立计数器（输入/输出/缓存读/缓存写），
/// credits 与 duration 各一个，属性携带维度键。失败请求也上报
/// （tokens 全 0，带 error_type），便于观测栈统计错误率。
pub fn entry_to_otlp_metrics(entry: &GatewayRequestLogEntry) -> Value {
    let attrs = |extra: &[(&str, Value)]| {
        let mut a: Vec<Value> = vec![
            json!({ "key": "model", "value": { "stringValue": entry.model.clone().unwrap_or_else(|| "unknown".into()) } }),
            json!({ "key": "endpoint", "value": { "stringValue": entry.endpoint } }),
            json!({ "key": "account.id", "value": { "stringValue": entry.account_id.clone().unwrap_or_else(|| "unknown".into()) } }),
            json!({ "key": "session.id", "value": { "stringValue": entry.session_id.clone().unwrap_or_else(|| "unknown".into()) } }),
            json!({ "key": "status.code", "value": { "intValue": entry.status_code } }),
            json!({ "key": "stream", "value": { "boolValue": entry.stream } }),
        ];
        for (k, v) in extra {
            a.push(json!({ "key": k, "value": v }));
        }
        a
    };

    let now_nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default();

    let counter = |name: &str, value: i64, extra: &[(&str, Value)]| {
        json!({
            "name": name,
            "sum": {
                "dataPoints": [{
                    "asInt": value.to_string(),
                    "timeUnixNano": now_nanos.to_string(),
                    "attributes": attrs(extra),
                }],
                "aggregationTemporality": 2, // CUMULATIVE 不适用流式单点；用 DELTA 语义逐点上报
            }
        })
    };

    let error_type_attr: Vec<(&str, Value)> = entry
        .error_type
        .as_ref()
        .map(|t| vec![("error.type", json!(t))])
        .unwrap_or_default();

    let metrics = vec![
        counter(
            "kiro.gateway.tokens.input",
            i64::from(entry.input_tokens.unwrap_or(0)),
            &[],
        ),
        counter(
            "kiro.gateway.tokens.output",
            i64::from(entry.output_tokens.unwrap_or(0)),
            &[],
        ),
        counter(
            "kiro.gateway.tokens.cache_read",
            i64::from(entry.cache_read_input_tokens.unwrap_or(0)),
            &[],
        ),
        counter(
            "kiro.gateway.tokens.cache_write",
            i64::from(entry.cache_creation_input_tokens.unwrap_or(0)),
            &[],
        ),
        counter("kiro.gateway.requests", 1, &error_type_attr),
        {
            // credits 是浮点，走 DoubleGauge 单点
            json!({
                "name": "kiro.gateway.credits",
                "gauge": {
                    "dataPoints": [{
                        "asDouble": entry.credits.unwrap_or(0.0),
                        "timeUnixNano": now_nanos.to_string(),
                        "attributes": attrs(&[]),
                    }]
                }
            })
        },
        {
            json!({
                "name": "kiro.gateway.duration_ms",
                "gauge": {
                    "dataPoints": [{
                        "asInt": entry.duration_ms.to_string(),
                        "timeUnixNano": now_nanos.to_string(),
                        "attributes": attrs(&[]),
                    }]
                }
            })
        },
    ];

    json!({
        "resourceMetrics": [{
            "resource": {
                "attributes": [
                    { "key": "service.name", "value": { "stringValue": "kiro-account-manager-gateway" } },
                ]
            },
            "scopeMetrics": [{
                "scope": { "name": "kiro.gateway" },
                "metrics": metrics,
            }],
        }]
    })
}

/// 解析 otlp_headers 配置（JSON 对象字符串 → 键值对）。坏配置静默降级为空。
pub fn parse_headers(cfg: Option<&str>) -> Vec<(String, String)> {
    let Some(raw) = cfg else { return Vec::new() };
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw.trim()) else {
        return Vec::new();
    };
    map.into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> GatewayRequestLogEntry {
        serde_json::from_str(
            r#"{
                "occurredAt": "2026-10-26 12:00:00",
                "requestId": "req-1",
                "requestIndex": 1,
                "endpoint": "/v1/messages",
                "clientIp": "127.0.0.1",
                "model": "claude-sonnet-4-5",
                "stream": true,
                "statusCode": 200,
                "outcome": "success",
                "durationMs": 1234,
                "inputTokens": 100,
                "outputTokens": 50,
                "cacheReadInputTokens": 10,
                "cacheCreationInputTokens": 20,
                "credits": 0.12,
                "accountId": "acc-1",
                "sessionId": "sess-1"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn entry_to_otlp_metrics_has_service_and_counters() {
        let v = entry_to_otlp_metrics(&sample_entry());
        let metrics = v["resourceMetrics"][0]["scopeMetrics"][0]["metrics"]
            .as_array()
            .unwrap();
        assert_eq!(metrics.len(), 7);
        assert_eq!(
            v["resourceMetrics"][0]["resource"]["attributes"][0]["key"],
            "service.name"
        );
        let input = metrics
            .iter()
            .find(|m| m["name"] == "kiro.gateway.tokens.input")
            .unwrap();
        assert_eq!(input["sum"]["dataPoints"][0]["asInt"], "100");
    }

    #[test]
    fn parse_headers_handles_bad_and_good_config() {
        assert!(parse_headers(None).is_empty());
        assert!(parse_headers(Some("not json")).is_empty());
        let h = parse_headers(Some(r#"{"Authorization":"Bearer x"}"#));
        assert_eq!(h, vec![("Authorization".to_string(), "Bearer x".to_string())]);
    }
}
