use super::*;

#[test]
fn rejects_unsupported_region() {
    let config = GatewayConfig {
        region: "moon-east-1".to_string(),
        ..GatewayConfig::default()
    };

    let err = ensure_config_valid(&config).expect_err("unsupported region should fail");
    assert!(err.contains("region 不受支持"));
}

#[test]
fn rejects_local_account_mode_for_gateway() {
    let config = GatewayConfig {
        account_mode: "local".to_string(),
        ..GatewayConfig::default()
    };

    let err = ensure_config_valid(&config).expect_err("local mode should fail");
    assert!(
        err.contains("不再支持 local 模式"),
        "unexpected error: {err}"
    );
}

#[test]
fn accepts_known_regions() {
    let mut config = GatewayConfig {
        account_mode: "single".to_string(),
        account_id: Some("test-account".to_string()),
        ..GatewayConfig::default()
    };
    for region in [
        "us-east-1",
        "eu-central-1",
        "us-west-2",
        "ap-northeast-1",
        "ap-southeast-1",
        "us-gov-west-1",
    ] {
        config.region = region.to_string();
        ensure_config_valid(&config).expect("known region should pass validation");
    }
}

#[test]
fn rejects_remote_access_without_allowlist() {
    let config = GatewayConfig {
        local_only: false,
        account_mode: "single".to_string(),
        account_id: Some("test-account".to_string()),
        allowed_ips: Vec::new(),
        ..GatewayConfig::default()
    };

    let err =
        ensure_config_valid(&config).expect_err("remote access without allowlist should fail");
    assert!(err.contains("白名单"), "unexpected error: {err}");
}

#[test]
fn normalize_config_clears_legacy_key_fields() {
    // 客户端密钥固定为 kiro 后，旧配置里的多 Key 数据在归一化时必须被清空
    let config = GatewayConfig {
        access_token: Some("Key 2:sk-TVV1F9".to_string()),
        client_api_keys: vec!["Key 2:sk-TVV1F9".to_string()],
        client_keys: vec![serde_json::json!({"id": "ck-1", "key": "sk-x", "enabled": true})],
        ..GatewayConfig::default()
    };

    let normalized = normalize_config(&config);

    assert_eq!(normalized.access_token, None);
    assert!(normalized.client_api_keys.is_empty());
    assert!(normalized.client_keys.is_empty());
}

#[test]
fn log_cap_covers_jsonl_and_log_only() {
    // `.jsonl` 与 `.log` 都受管；其它扩展名（备份、配置）不能碰
    assert!(managed_log_cap("log").is_some());
    assert!(managed_log_cap("jsonl").is_some());
    assert_eq!(managed_log_cap("log"), managed_log_cap("jsonl"));
    assert!(managed_log_cap("log.1").is_none());
    assert!(managed_log_cap("jsonl.1").is_none());
    assert!(managed_log_cap("json").is_none());
    assert!(managed_log_cap("").is_none());
}

#[test]
fn local_origin_accepts_loopback_and_rejects_lookalikes() {
    // 本机来源放行
    assert!(is_local_origin("http://localhost"));
    assert!(is_local_origin("http://localhost:3000"));
    assert!(is_local_origin("http://127.0.0.1"));
    assert!(is_local_origin("http://127.0.0.1:8765"));
    assert!(is_local_origin("http://[::1]:8080"));

    // 关键：前缀匹配不能放过「长得像本机」的域
    assert!(!is_local_origin("http://localhost.evil.com"));
    assert!(!is_local_origin("http://127.0.0.1.evil.com"));
    assert!(!is_local_origin("https://localhost"));
    assert!(!is_local_origin("http://evil.com"));
    assert!(!is_local_origin("null"));
    assert!(!is_local_origin(""));
}
