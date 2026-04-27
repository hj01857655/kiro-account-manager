use crate::auth::{DesktopRefreshResponse, DESKTOP_AUTH_API};

/// 生成PKCE `code_verifier`（32字节，base64url）
pub fn generate_code_verifier_social() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    base64_url_encode(&bytes)
}

/// 生成PKCE `code_challenge`（SHA256哈希，base64url）
pub fn generate_code_challenge_social(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    base64_url_encode(&result)
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD.encode(data)
}

/// 使用授权码交换Social token（POST /oauth/token）
pub async fn exchange_social_code_for_token(
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
    machineid: &str,
) -> Result<DesktopRefreshResponse, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "code": code,
        "code_verifier": code_verifier,
        "redirect_uri": redirect_uri
    });

    let kiro_ide_version = "0.6.18";
    let user_agent = format!("KiroIDE-{kiro_ide_version}-{machineid}");

    let response = client
        .post(format!("{DESKTOP_AUTH_API}/oauth/token"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("user-agent", user_agent)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OAuth token request failed: {e}"))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("OAuth token exchange failed ({status}): {body}"));
    }

    let token_resp: DesktopRefreshResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse OAuth token response: {e}"))?;

    Ok(token_resp)
}
