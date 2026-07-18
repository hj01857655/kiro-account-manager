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
        )
        .build()
        .expect("request should build");

        assert!(
            request.headers().get("redirect-for-internal").is_none(),
            "provider {provider} should not add redirect-for-internal"
        );
    }
}
