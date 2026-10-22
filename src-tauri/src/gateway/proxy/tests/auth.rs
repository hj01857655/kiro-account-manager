//! 客户端鉴权的回归测试。

use super::*;

#[test]
fn verify_client_auth_accepts_fixed_kiro_key() {
    let config = GatewayConfig::default();

    let mut bearer_headers = HeaderMap::new();
    bearer_headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer kiro"),
    );
    assert!(verify_client_auth(&bearer_headers, &config).is_ok());

    let mut x_api_key_headers = HeaderMap::new();
    x_api_key_headers.insert("x-api-key", HeaderValue::from_static("kiro"));
    assert!(verify_client_auth(&x_api_key_headers, &config).is_ok());

    // 多 Key 时代的旧密钥一律不再认：固定 Key 落地后历史配置里的 sk- 值必须失效
    let mut legacy_headers = HeaderMap::new();
    legacy_headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer sk-secondary"),
    );
    assert!(verify_client_auth(&legacy_headers, &config).is_err());

    let mut invalid_headers = HeaderMap::new();
    invalid_headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer sk-unknown"),
    );
    assert!(verify_client_auth(&invalid_headers, &config).is_err());
}
