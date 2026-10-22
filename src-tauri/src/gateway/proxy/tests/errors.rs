//! 上游错误映射与安全截断的回归测试。

use super::*;

#[test]
fn safe_truncate_never_splits_multibyte_chars() {
    // '中' 是 3 字节。构造一个字节长度超过上限、且上限不落在字符边界上的串。
    // 上限 100：'中'.repeat(40) = 120 字节，字符边界在 0,3,6,...,99,102；100 不是边界。
    // 旧代码 &s[..100] 会 panic（byte index 100 is not a char boundary）。
    let s = "中".repeat(40);
    let end = safe_truncate(&s, 100);
    // 回退到最近的合法边界 99（= 33 个 '中'）
    assert!(s.is_char_boundary(end), "回退点必须是合法字符边界");
    assert!(end <= 100, "不得超过字节上限");
    // 真正切一刀，确认不 panic
    let _ = &s[..end];
    assert_eq!(end, 99);

    // ASCII：上限正好落在边界，原样返回
    let ascii = "A".repeat(120);
    assert_eq!(safe_truncate(&ascii, 100), 100);

    // 串本身比上限短：返回全长
    assert_eq!(safe_truncate("abc", 100), 3);

    // 上限 0：返回 0，不 panic
    assert_eq!(safe_truncate(&s, 0), 0);

    // emoji（4 字节）边界回退
    let emoji = "😀".repeat(10); // 40 字节，边界 0,4,8,...,40
    let e = safe_truncate(&emoji, 10); // 10 不是 4 的倍数 → 回退到 8
    assert_eq!(e, 8);
    let _ = &emoji[..e];
}

#[test]
fn map_upstream_error_detects_invalid_bearer_token() {
    let body =
        r#"{"message":"The bearer token included in the request is invalid.","reason":null}"#;

    let (status, error_type, message) = map_upstream_error(StatusCode::FORBIDDEN, body);

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error_type, "token_expired_error");
    assert_eq!(
        message,
        "The bearer token included in the request is invalid."
    );
}

#[test]
fn detect_upstream_error_body_maps_success_status_error_payloads() {
    let error = detect_upstream_error_body(
        r#"{"error":{"message":"Invalid model. Please select a different model to continue.","type":"invalid_request_error"}}"#,
    )
    .expect("error payload should be detected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
    assert_eq!(error.1, "invalid_request_error");
    assert!(error.2.contains("Invalid model"));
}

#[test]
fn gateway_error_carries_status_and_type_without_string_parsing() {
    // 状态码与错误类型必须是类型的一部分，调用方无需 starts_with 解析
    let err = GatewayError::PreferredAccountUnavailable {
        account_id: "acc-1".to_string(),
    };
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    assert_eq!(err.error_type(), "invalid_request_error");
    assert!(err.message().contains("acc-1"));

    let err = GatewayError::NoAvailableAccount;
    assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(err.error_type(), "unavailable");

    let err = GatewayError::BalancerNoSelection;
    assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(err.error_type(), "unavailable");
}

#[test]
fn classify_gateway_error_round_trips_display() {
    // Display 保留 __NNN__ 前缀（迁移期兼容），classify 必须能还原成同一类型
    for original in [
        GatewayError::PreferredAccountUnavailable {
            account_id: "acc-42".to_string(),
        },
        GatewayError::NoAvailableAccount,
        GatewayError::BalancerNoSelection,
    ] {
        let rendered = original.to_string();
        let restored = classify_gateway_error(&rendered)
            .expect("rendered error should be classifiable");
        assert_eq!(restored.status(), original.status());
        assert_eq!(restored.error_type(), original.error_type());
        assert_eq!(restored.message(), original.message());
    }
}

#[test]
fn classify_gateway_error_returns_none_for_plain_messages() {
    // 非网关自身产生的错误（上游报文、限流文本）不应被误分类
    assert!(classify_gateway_error("账号 acc-1 已不可用").is_none());
    assert!(classify_gateway_error("429 too many requests").is_none());
    assert!(classify_gateway_error("").is_none());
}

/// 封禁判据：`map_upstream_error` 必须把 403 + TEMPORARILY_SUSPENDED 映射成
/// `account_banned_error` + `BANNED:` 前缀。
///
/// 这是 gateway 回写账号状态的唯一依据（`upstream.rs` 的封禁分支按 `error_type`
/// 判断，不自行解析 body）。判据若漂移，账号列表会重新和日志分裂——即 #154。
#[test]
fn map_upstream_error_flags_banned_with_account_banned_error_type() {
    let body = r#"{"message":"Your User ID is temporarily suspended.","reason":"TEMPORARILY_SUSPENDED"}"#;
    let (status, error_type, message) = map_upstream_error(StatusCode::FORBIDDEN, body);

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        error_type, "account_banned_error",
        "封禁必须映射为 account_banned_error，供上层按类型分支"
    );
    assert!(
        message.starts_with("BANNED:"),
        "封禁消息须带 BANNED: 前缀，供旧调用方识别"
    );
}

/// token 失效与封禁是两种不同的 403，不能互相掩盖。
///
/// `upstream.rs` 里 token_expired_error 分支排在前，若封禁被误判成 token 失效，
/// 就会走刷新重试而不是标记 banned。
#[test]
fn map_upstream_error_separates_token_invalid_from_banned() {
    let token_body = r#"{"message":"Bearer token is invalid or expired"}"#;
    let (_, token_error_type, _) = map_upstream_error(StatusCode::FORBIDDEN, token_body);
    assert_eq!(token_error_type, "token_expired_error");

    let banned_body = r#"{"message":"Your User ID is temporarily suspended.","reason":"TEMPORARILY_SUSPENDED"}"#;
    let (_, banned_error_type, _) = map_upstream_error(StatusCode::FORBIDDEN, banned_body);
    assert_eq!(banned_error_type, "account_banned_error");
}
