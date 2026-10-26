//! 上游请求头与调用上下文的回归测试。

use super::*;

#[test]
fn get_available_models_call_context_uses_account_machine_id_and_effective_profile_arn() {
    let upstream = UpstreamCredentials {
        account_id: "test-account".to_string(),
        access_token: "token-models".to_string(),
        machine_id: "account-machine-id".to_string(),
        profile_arn: Some(
            "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX".to_string(),
        ),
        available_models_profile_arn: Some(
            "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX".to_string(),
        ),
        provider: Some("BuilderId".to_string()),
        region: "us-east-1".to_string(),
        source_label: "single:test".to_string(),
        user_agent: "KiroIDE 0.11.34 account-machine-id".to_string(),
        auth_method: Some("IdC".to_string()),
        send_opt_out: true,
        http: reqwest::Client::new(),
    };

    let (machine_id, profile_arn) = get_available_models_call_context(&upstream);

    assert_eq!(machine_id, "account-machine-id");
    assert_eq!(
        profile_arn,
        Some("arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX")
    );
}

#[test]
fn add_kiro_upstream_headers_adds_generate_request_headers() {
    let upstream = UpstreamCredentials {
        account_id: "test-account".to_string(),
        access_token: "token-1".to_string(),
        machine_id: "machine-123".to_string(),
        profile_arn: None,
        available_models_profile_arn: None,
        provider: None,
        region: "us-east-1".to_string(),
        source_label: "single:test".to_string(),
        user_agent: "KiroIDE 0.11.34 machine-123".to_string(),
        auth_method: Some("external_idp".to_string()),
        send_opt_out: true,
        http: reqwest::Client::new(),
    };

    let request = add_kiro_upstream_headers(
        reqwest::Client::new()
            .post("https://runtime.us-east-1.kiro.dev/generateAssistantResponse"),
        &upstream,
        "application/vnd.amazon.eventstream",
        true,
        true,
        false,
        1,
    )
    .build()
    .expect("request should build");

    assert_eq!(
        request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("Bearer token-1")
    );
    assert_eq!(
        request
            .headers()
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok()),
        Some("KiroIDE 0.11.34 machine-123")
    );
    let x_amz_user_agent = request
        .headers()
        .get("x-amz-user-agent")
        .and_then(|value| value.to_str().ok())
        .expect("x-amz-user-agent header");
    assert!(x_amz_user_agent.starts_with("aws-sdk-js/1.0.39 KiroIDE-"));
    assert!(x_amz_user_agent.ends_with("-machine-123"));
    // G-6：amz-sdk-request 的 attempt/max 与实际重试上界同源（MAX_RETRIES=5）
    assert_eq!(
        request
            .headers()
            .get("amz-sdk-request")
            .and_then(|value| value.to_str().ok()),
        Some("attempt=1; max=5")
    );
    assert_eq!(
        request
            .headers()
            .get("x-amzn-codewhisperer-optout")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );
    assert_eq!(
        request
            .headers()
            .get("x-amzn-kiro-agent-mode")
            .and_then(|value| value.to_str().ok()),
        Some(DEFAULT_AGENT_MODE)
    );
    // TokenType header 已移除（会导致某些接口 403 错误）
    assert!(request.headers().get("TokenType").is_none());
    assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
    assert!(request.headers().get("redirect-for-internal").is_none());
}

#[test]
fn add_kiro_upstream_headers_adds_profile_arn_for_enterprise_generate() {
    let upstream = UpstreamCredentials {
        account_id: "ent-account".to_string(),
        access_token: "token-ent".to_string(),
        machine_id: "machine-ent".to_string(),
        profile_arn: Some(
            "arn:aws:codewhisperer:us-east-1:123456789012:profile/AAAACCCCXXXX".to_string(),
        ),
        available_models_profile_arn: Some(
            "arn:aws:codewhisperer:us-east-1:123456789012:profile/AAAACCCCXXXX".to_string(),
        ),
        provider: Some("Enterprise".to_string()),
        region: "us-east-1".to_string(),
        source_label: "single:ent".to_string(),
        user_agent: "KiroIDE 0.11.34 machine-ent".to_string(),
        auth_method: Some("IdC".to_string()),
        send_opt_out: true,
        http: reqwest::Client::new(),
    };

    // 与 call_generate_assistant_response 的调用点一致：有真实 ARN 才带头
    let include_profile_arn = upstream
        .profile_arn
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let request = add_kiro_upstream_headers(
        reqwest::Client::new()
            .post("https://runtime.us-east-1.kiro.dev/generateAssistantResponse"),
        &upstream,
        "application/vnd.amazon.eventstream",
        true,
        true,
        include_profile_arn,
        1,
    )
    .build()
    .expect("request should build");

    assert_eq!(
        request
            .headers()
            .get("x-amzn-kiro-profile-arn")
            .and_then(|value| value.to_str().ok()),
        Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/AAAACCCCXXXX")
    );
}

#[test]
fn add_kiro_upstream_headers_keeps_runtime_requests_minimal() {
    let upstream = UpstreamCredentials {
        account_id: "test-account".to_string(),
        access_token: "token-2".to_string(),
        machine_id: "machine-456".to_string(),
        profile_arn: None,
        available_models_profile_arn: None,
        provider: None,
        region: "us-east-1".to_string(),
        source_label: "single:test".to_string(),
        user_agent: "KiroIDE 0.11.34 machine-456".to_string(),
        auth_method: Some("social".to_string()),
        send_opt_out: true,
        http: reqwest::Client::new(),
    };

    let request = add_kiro_upstream_headers(
        reqwest::Client::new()
            .get("https://runtime.us-east-1.kiro.dev/ListAvailableModels?origin=AI_EDITOR"),
        &upstream,
        "application/json",
        false,
        false,
        false,
        1,
    )
    .build()
    .expect("request should build");

    let x_amz_user_agent = request
        .headers()
        .get("x-amz-user-agent")
        .and_then(|value| value.to_str().ok())
        .expect("x-amz-user-agent header");
    assert!(x_amz_user_agent.starts_with("aws-sdk-js/1.0.39 KiroIDE-"));
    assert!(x_amz_user_agent.ends_with("-machine-456"));
    assert!(request
        .headers()
        .get("x-amzn-codewhisperer-optout")
        .is_none());
    assert!(request.headers().get("x-amzn-kiro-agent-mode").is_none());
    assert!(request.headers().get("TokenType").is_none());
    assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
    assert!(request.headers().get("redirect-for-internal").is_none());
}

#[test]
fn add_kiro_upstream_headers_adds_mcp_profile_arn_header() {
    let upstream = UpstreamCredentials {
        account_id: "test-account".to_string(),
        access_token: "token-3".to_string(),
        machine_id: "machine-789".to_string(),
        profile_arn: Some(
            "arn:aws:codewhisperer:us-east-1:123456789012:profile/test".to_string(),
        ),
        available_models_profile_arn: Some(
            "arn:aws:codewhisperer:us-east-1:123456789012:profile/test".to_string(),
        ),
        provider: None,
        region: "us-east-1".to_string(),
        source_label: "single:test".to_string(),
        user_agent: "KiroIDE 0.11.34 machine-789".to_string(),
        auth_method: Some("social".to_string()),
        send_opt_out: true,
        http: reqwest::Client::new(),
    };

    let request = add_kiro_upstream_headers(
        reqwest::Client::new().post(crate::clients::kiro_client::build_mcp_url("us-east-1")),
        &upstream,
        "application/json",
        false,
        false,
        true,
        1,
    )
    .build()
    .expect("request should build");

    assert_eq!(
        request
            .headers()
            .get("x-amzn-kiro-profile-arn")
            .and_then(|value| value.to_str().ok()),
        Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/test")
    );
    assert!(request.headers().get("redirect-for-internal").is_none());
}

#[test]
fn add_kiro_upstream_headers_adds_redirect_for_internal_only_for_internal_provider() {
    let upstream = UpstreamCredentials {
        account_id: "test-account".to_string(),
        access_token: "token-4".to_string(),
        machine_id: "machine-999".to_string(),
        profile_arn: None,
        available_models_profile_arn: None,
        provider: Some("Internal".to_string()),
        region: "us-east-1".to_string(),
        source_label: "single:test".to_string(),
        user_agent: "KiroIDE 0.11.34 machine-999".to_string(),
        auth_method: Some("IdC".to_string()),
        send_opt_out: true,
        http: reqwest::Client::new(),
    };

    let request = add_kiro_upstream_headers(
        reqwest::Client::new()
            .post("https://runtime.us-east-1.kiro.dev/generateAssistantResponse"),
        &upstream,
        "application/vnd.amazon.eventstream",
        true,
        true,
        false,
        1,
    )
    .build()
    .expect("request should build");

    assert_eq!(
        request
            .headers()
            .get("redirect-for-internal")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );
}

#[test]
fn add_kiro_upstream_headers_does_not_add_redirect_for_enterprise_or_builderid() {
    for provider in ["Enterprise", "BuilderId"] {
        let upstream = UpstreamCredentials {
            account_id: "test-account".to_string(),
            access_token: "token-5".to_string(),
            machine_id: "machine-1000".to_string(),
            profile_arn: None,
            available_models_profile_arn: None,
            provider: Some(provider.to_string()),
            region: "us-east-1".to_string(),
            source_label: "single:test".to_string(),
            user_agent: "KiroIDE 0.11.34 machine-1000".to_string(),
            auth_method: Some("IdC".to_string()),
            send_opt_out: true,
            http: reqwest::Client::new(),
        };

        let request = add_kiro_upstream_headers(
            reqwest::Client::new()
                .post("https://runtime.us-east-1.kiro.dev/generateAssistantResponse"),
            &upstream,
            "application/vnd.amazon.eventstream",
            true,
            true,
            false,
            1,
        )
        .build()
        .expect("request should build");

        assert!(
            request.headers().get("redirect-for-internal").is_none(),
            "provider {provider} should not add redirect-for-internal"
        );
    }
}

/// 遮蔽→回替 round-trip：请求侧遮蔽后响应里的占位符必须能换回真值。
///
/// 覆盖非流式出口的 `unmask_response_value`（流式路径走 send_event 的
/// task-local 回替，映射表同源）。嵌套数组/对象/纯文本三种形态都要回替。
#[tokio::test]
async fn unmask_response_value_round_trips_masked_payload() {
    use crate::gateway::masking::MaskEngine;

    let mut engine = MaskEngine::new();
    // 格式合法的假 key（sk- 前缀 + 32 位），请求侧先遮蔽
    let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
    let request_text = format!("my key is {fake_key}");
    let masked_request = engine.mask_text(&request_text);
    assert!(masked_request.text.contains("{{API_KEY_"), "请求侧应产生占位符");
    assert!(!masked_request.text.contains(&fake_key), "真值不应出现在遮蔽后请求里");

    // 模拟上游把占位符原样带回响应（模型引用了用户消息里的 key）
    let upstream_response = serde_json::json!({
        "content": [
            {"type": "text", "text": format!("You mentioned {}", masked_request.text)}
        ],
        "nested": {
            "tool_input": {"note": masked_request.text.clone()},
            "list": [masked_request.text.clone(), "plain text"]
        }
    });

    // 把引擎（含映射表）挂进 AsyncMutex——与 RouterState.mask_engine 同构
    let mask_engine = std::sync::Arc::new(tokio::sync::Mutex::new(engine));
    let unmasked = unmask_response_value(upstream_response, &mask_engine).await;

    // 客户端必须拿到真值，且嵌套各层都回替了
    let serialized = unmasked.to_string();
    assert!(serialized.contains(&fake_key), "回替后应包含真值: {serialized}");
    assert!(!serialized.contains("{{API_KEY_"), "回替后不应残留占位符");
    assert_eq!(
        unmasked["nested"]["tool_input"]["note"],
        serde_json::json!(request_text),
        "嵌套对象里的值应完整还原"
    );
    assert_eq!(unmasked["nested"]["list"][1], serde_json::json!("plain text"), "无关文本不动");
}

/// 白名单值不产生占位符，回替自然也碰不到它——白名单与回替互不干扰。
#[tokio::test]
async fn unmask_never_mask_values_pass_through_untouched() {
    use crate::gateway::masking::{MaskEngine, MaskNeverMask};

    let fake_key = ["sk-", "abc123DEF456ghi789JKL012mno345PQR"].concat();
    let engine = MaskEngine::with_overrides(
        &[],
        &[],
        &[MaskNeverMask {
            value: fake_key.clone(),
            note: "排查此 key".to_string(),
        }],
    )
    .expect("装配应成功");
    // 白名单值请求侧就没被遮蔽，响应里也不会有占位符
    let mask_engine = std::sync::Arc::new(tokio::sync::Mutex::new(engine));
    let response = serde_json::json!({"text": format!("the key {fake_key} is fine")});
    let unmasked = unmask_response_value(response, &mask_engine).await;
    assert_eq!(
        unmasked["text"],
        serde_json::json!(format!("the key {fake_key} is fine")),
        "白名单值应原样通过"
    );
}
