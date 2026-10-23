use super::*;

#[tokio::test]
async fn runtime_serves_health_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .bearer_auth("kiro")
        .send()
        .await
        .expect("health request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_serves_models_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/v1/models"))
        .bearer_auth("kiro")
        .send()
        .await
        .expect("models request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: Value = response
        .json()
        .await
        .expect("models response should be json");
    assert_eq!(payload.get("object").and_then(Value::as_str), Some("list"));
    assert!(
        payload
            .get("data")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty()),
        "models response should include at least one model"
    );
    stop_runtime(&mut runtime).await;
}

/// 四个本地端点共用 `generate_local_response`，客户端 Key 在那里统一校验；
/// 无 Key 一律 401。同时锁定 CORS 收敛：本地来源回显、外地来源不回显。
#[tokio::test]
async fn runtime_rejects_models_without_client_key() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/v1/models"))
        .send()
        .await
        .expect("models request should complete");

    // 四个本地端点（/health、/v1/models、两个 token 计数）共用
    // generate_local_response，客户端 Key 校验在那里统一完成。
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "/v1/models without a client key must be rejected"
    );

    // CORS 收敛：本地来源拿到 Allow-Origin 回执，外地来源拿不到。
    // 无 Origin 的请求（curl / SDK / 探活工具）不受 CORS 约束，必须放行。
    let local = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .header("Origin", "http://localhost:3000")
        .send()
        .await
        .expect("local-origin health request should complete");
    assert_eq!(local.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        local
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:3000"),
        "local origin should be echoed back"
    );

    let evil = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .header("Origin", "http://evil.com")
        .send()
        .await
        .expect("remote-origin health request should complete");
    assert_eq!(
        evil.headers().get("access-control-allow-origin"),
        None,
        "remote origin must not be echoed back"
    );

    stop_runtime(&mut runtime).await;
}

/// 打包后的 Tauri WebView 页来源不是 localhost（Windows 是 http://tauri.localhost，
/// Linux/macOS 是 tauri://localhost）。它必须被当作本机来源，否则应用内的
/// 协议实测（浏览器 fetch 直连网关）会被 CORS 拦死——而开发模式跑在
/// localhost:1420 上，看不出这个问题。
#[tokio::test]
async fn runtime_allows_tauri_webview_origin() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    for origin in ["http://tauri.localhost", "tauri://localhost", "https://tauri.localhost"] {
        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .header("Origin", origin)
            .send()
            .await
            .expect("tauri-origin request should complete");
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some(origin),
            "Tauri WebView origin {origin} should be echoed back"
        );
    }

    // 不能因为放宽了 Tauri 来源就把外形相似的域名一起放进来
    let lookalike = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .header("Origin", "http://tauri.localhost.evil.com")
        .send()
        .await
        .expect("lookalike-origin request should complete");
    assert_eq!(
        lookalike.headers().get("access-control-allow-origin"),
        None,
        "a lookalike host must not be taken for the Tauri WebView"
    );

    stop_runtime(&mut runtime).await;
}

/// 协议实测会发 x-account-id 与 anthropic-version；这两个头不在 CORS 放行列表里时，
/// 浏览器预检失败，请求根本到不了 handler。
#[tokio::test]
async fn runtime_cors_preflight_allows_protocol_test_headers() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .request(
            reqwest::Method::OPTIONS,
            format!("http://127.0.0.1:{port}/v1/messages"),
        )
        .header("Origin", "http://tauri.localhost")
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            "authorization,content-type,anthropic-version,x-account-id",
        )
        .send()
        .await
        .expect("preflight request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let allowed = response
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    for needed in ["authorization", "content-type", "anthropic-version", "x-account-id"] {
        assert!(
            allowed.contains(needed),
            "preflight must allow {needed}, got: {allowed}"
        );
    }

    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_serves_count_tokens_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/messages/count_tokens"))
        .bearer_auth("kiro")
        .header("content-type", "application/json")
        .body(
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{ "role": "user", "content": "hello world" }]
            })
            .to_string(),
        )
        .send()
        .await
        .expect("count tokens request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: Value = response
        .json()
        .await
        .expect("count tokens response should be json");
    // 该端点估算的是整个序列化请求（含 model / messages 结构），实测 27
    assert_eq!(payload.get("input_tokens").and_then(Value::as_u64), Some(27));
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_rejects_unauthenticated_health_requests_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("health request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_rejects_raw_authorization_header_without_bearer_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .header("Authorization", "kiro")
        .send()
        .await
        .expect("health request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_rejects_unauthenticated_models_requests_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/v1/models"))
        .send()
        .await
        .expect("models request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_rejects_unauthenticated_count_tokens_requests_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/messages/count_tokens"))
        .header("content-type", "application/json")
        .body(
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "input": [{ "role": "user", "content": "hello" }]
            })
            .to_string(),
        )
        .send()
        .await
        .expect("count tokens request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_rejects_unauthenticated_responses_requests_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/responses"))
        .header("content-type", "application/json")
        .body(
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "input": [{ "role": "user", "content": "hello" }]
            })
            .to_string(),
        )
        .send()
        .await
        .expect("responses request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_rejects_unauthenticated_messages_requests_over_real_http() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = runtime_test_gateway_config(port);
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/messages"))
        .header("content-type", "application/json")
        .body(
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{ "role": "user", "content": "hello" }]
            })
            .to_string(),
        )
        .send()
        .await
        .expect("messages request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}

#[tokio::test]
async fn runtime_requires_client_api_key_even_when_local_only() {
    let guard = RuntimeHttpTestGuard::new();
    let port = guard.port;

    let config = GatewayConfig {
        port,
        local_only: true,
        account_mode: "single".to_string(),
        account_id: Some("test-account".to_string()),
                ..GatewayConfig::default()
    };
    let mut runtime = spawn_runtime(config, test_shared_store())
        .await
        .expect("runtime should start");

    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/responses"))
        .header("content-type", "application/json")
        .body(
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "input": [{ "role": "user", "content": "hello" }]
            })
            .to_string(),
        )
        .send()
        .await
        .expect("responses request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    stop_runtime(&mut runtime).await;
}