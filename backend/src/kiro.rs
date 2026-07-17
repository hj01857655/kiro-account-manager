use crate::{
    db,
    error::{AppError, AppResult},
    models::{AccountRecord, AccountSecrets},
    state::AppState,
};
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use reqwest::{Client, Proxy};
use serde_json::{json, Value};
use std::time::Duration as StdDuration;
use url::Url;
use uuid::Uuid;

pub async fn usage(state: &AppState, id: &str) -> AppResult<Value> {
    let record = db::get_account_record(&state.pool, id).await?;
    let secrets = db::decrypt_secrets(&state.crypto, &record)?;
    let value = request_usage(&record, &secrets)
        .await
        .map_err(upstream_error)?;
    sqlx::query("UPDATE accounts SET usage_json=?,status='active',disabled_reason=NULL,success_count=success_count+1,last_checked_at=?,updated_at=? WHERE id=?")
        .bind(value.to_string()).bind(db::now()).bind(db::now()).bind(id).execute(&state.pool).await?;
    Ok(value)
}

pub async fn models(state: &AppState, id: &str) -> AppResult<Value> {
    let record = db::get_account_record(&state.pool, id).await?;
    let secrets = db::decrypt_secrets(&state.crypto, &record)?;
    let value = request_models(&record, &secrets)
        .await
        .map_err(upstream_error)?;
    sqlx::query("UPDATE accounts SET models_json=?,success_count=success_count+1,last_checked_at=?,updated_at=? WHERE id=?")
        .bind(value.to_string()).bind(db::now()).bind(db::now()).bind(id).execute(&state.pool).await?;
    Ok(value)
}

pub async fn check(state: &AppState, id: &str) -> AppResult<Value> {
    let record = db::get_account_record(&state.pool, id).await?;
    let secrets = db::decrypt_secrets(&state.crypto, &record)?;
    let result = request_usage(&record, &secrets).await;
    match result {
        Ok(value) => {
            sqlx::query("UPDATE accounts SET usage_json=?,status='active',disabled_reason=NULL,success_count=success_count+1,last_checked_at=?,updated_at=? WHERE id=?")
                .bind(value.to_string()).bind(db::now()).bind(db::now()).bind(id).execute(&state.pool).await?;
            Ok(json!({ "available": true, "usage": value }))
        }
        Err(message) => {
            let status = if message.starts_with("BANNED:") {
                "banned"
            } else {
                "error"
            };
            sqlx::query("UPDATE accounts SET status=?,disabled_reason=?,failure_count=failure_count+1,last_failure_at=?,last_checked_at=?,updated_at=? WHERE id=?")
                .bind(status).bind(public_error(&message)).bind(db::now()).bind(db::now()).bind(db::now()).bind(id).execute(&state.pool).await?;
            Ok(json!({ "available": false, "status": status, "message": public_error(&message) }))
        }
    }
}

pub async fn refresh(state: &AppState, id: &str) -> AppResult<Value> {
    let mut record = db::get_account_record(&state.pool, id).await?;
    let mut secrets = db::decrypt_secrets(&state.crypto, &record)?;
    let refresh_token = secrets
        .refresh_token
        .clone()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::bad_request("this account has no refresh token"))?;
    let client = client_for(&record, &secrets).map_err(upstream_error)?;
    let auth_method = record
        .auth_method
        .as_deref()
        .unwrap_or("social")
        .to_ascii_lowercase();
    let response = if auth_method.contains("idc") || auth_method.contains("sso") {
        let client_id = secrets
            .client_id
            .as_deref()
            .ok_or_else(|| AppError::bad_request("IdC account is missing clientId"))?;
        let client_secret = secrets
            .client_secret
            .as_deref()
            .ok_or_else(|| AppError::bad_request("IdC account is missing clientSecret"))?;
        let region = region(&record);
        client.post(format!("https://oidc.{region}.amazonaws.com/token"))
            .json(&json!({ "clientId": client_id, "clientSecret": client_secret, "grantType": "refresh_token", "refreshToken": refresh_token }))
            .send().await.map_err(|_| upstream_error("token refresh request failed"))?
    } else if auth_method.contains("external") {
        let endpoint = secrets.token_endpoint.as_deref().ok_or_else(|| {
            AppError::bad_request("external IdP account is missing tokenEndpoint")
        })?;
        validate_external_token_endpoint(endpoint)?;
        let client_id = secrets
            .client_id
            .as_deref()
            .ok_or_else(|| AppError::bad_request("external IdP account is missing clientId"))?;
        client
            .post(endpoint)
            .form(&[
                ("client_id", client_id),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
            ])
            .send()
            .await
            .map_err(|_| upstream_error("token refresh request failed"))?
    } else {
        client
            .post("https://prod.us-east-1.auth.desktop.kiro.dev/refreshToken")
            .header(
                "user-agent",
                format!("KiroIDE-0.6.18-{}", record.machine_id),
            )
            .json(&json!({ "refreshToken": refresh_token }))
            .send()
            .await
            .map_err(|_| upstream_error("token refresh request failed"))?
    };
    if !response.status().is_success() {
        let status = response.status();
        record_refresh_failure(state, id, status.as_u16()).await?;
        return Err(AppError::new(
            StatusCode::BAD_GATEWAY,
            "token_refresh_failed",
            format!("upstream token refresh returned HTTP {}", status.as_u16()),
        ));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| upstream_error("token refresh response was invalid"))?;
    secrets.access_token =
        string_field(&body, &["accessToken", "access_token"]).or(secrets.access_token);
    secrets.refresh_token =
        string_field(&body, &["refreshToken", "refresh_token"]).or(secrets.refresh_token);
    secrets.id_token = string_field(&body, &["idToken", "id_token"]).or(secrets.id_token);
    let expires_at = number_field(&body, &["expiresIn", "expires_in"])
        .map(|seconds| (Utc::now() + Duration::seconds(seconds)).to_rfc3339())
        .or(record.expires_at.take());
    let encrypted = state
        .crypto
        .encrypt_json(&secrets)
        .map_err(|_| AppError::internal())?;
    sqlx::query("UPDATE accounts SET secret_blob=?,expires_at=?,status='active',disabled_reason=NULL,success_count=success_count+1,last_refreshed_at=?,updated_at=? WHERE id=?")
        .bind(encrypted).bind(&expires_at).bind(db::now()).bind(db::now()).bind(id).execute(&state.pool).await?;
    Ok(json!({ "refreshed": true, "expiresAt": expires_at }))
}

pub async fn request_usage(
    record: &AccountRecord,
    secrets: &AccountSecrets,
) -> Result<Value, String> {
    let token = access_token(secrets)?;
    let region = region(record);
    let url = format!("https://management.{region}.kiro.dev/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&resourceType=AGENTIC_REQUEST");
    let response = client_for(record, secrets)?
        .get(url)
        .bearer_auth(token)
        .header(
            "user-agent",
            format!("KiroIDE-0.6.18-{}", record.machine_id),
        )
        .header("x-amz-user-agent", "aws-sdk-rust/1.0 api/kiro-management")
        .header("amz-sdk-invocation-id", Uuid::new_v4().to_string())
        .header("amz-sdk-request", "attempt=1; max=1")
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|_| "getUsageLimits request failed".to_string())?;
    parse_response("getUsageLimits", response).await
}

pub async fn request_models(
    record: &AccountRecord,
    secrets: &AccountSecrets,
) -> Result<Value, String> {
    let token = access_token(secrets)?;
    let region = region(record);
    let mut body = json!({ "origin": "AI_EDITOR" });
    if let Some(profile_arn) = record
        .profile_arn
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        body["profileArn"] = Value::String(profile_arn.clone());
    }
    let response = client_for(record, secrets)?
        .post(format!("https://management.{region}.kiro.dev"))
        .bearer_auth(token)
        .header("content-type", "application/x-amz-json-1.0")
        .header(
            "x-amz-target",
            "KiroControlPlaneBearerService.ListAvailableModels",
        )
        .header("x-amz-user-agent", "aws-sdk-js/1.0.0")
        .header("amz-sdk-invocation-id", Uuid::new_v4().to_string())
        .header("amz-sdk-request", "attempt=1; max=3")
        .json(&body)
        .send()
        .await
        .map_err(|_| "ListAvailableModels request failed".to_string())?;
    parse_response("ListAvailableModels", response).await
}

pub fn client_for(record: &AccountRecord, secrets: &AccountSecrets) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(StdDuration::from_secs(45));
    if let Some(raw) = &record.proxy_json {
        let value: Value =
            serde_json::from_str(raw).map_err(|_| "proxy configuration is invalid")?;
        if value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        {
            let protocol = value
                .get("protocol")
                .and_then(Value::as_str)
                .unwrap_or("http");
            let host = value
                .get("host")
                .and_then(Value::as_str)
                .ok_or("proxy host is missing")?;
            let port = value
                .get("port")
                .and_then(Value::as_u64)
                .ok_or("proxy port is missing")?;
            let url = format!("{protocol}://{host}:{port}");
            let mut proxy = Proxy::all(&url).map_err(|_| "proxy URL is invalid")?;
            if let Some(username) = value.get("username").and_then(Value::as_str) {
                proxy = proxy.basic_auth(
                    username,
                    secrets.proxy_password.as_deref().unwrap_or_default(),
                );
            }
            builder = builder.proxy(proxy);
        }
    }
    builder
        .build()
        .map_err(|_| "failed to create HTTP client".to_string())
}

fn access_token(secrets: &AccountSecrets) -> Result<&str, String> {
    secrets
        .access_token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "AUTH_ERROR: access token is missing".to_string())
}

fn region(record: &AccountRecord) -> &str {
    record
        .region
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("us-east-1")
}

async fn parse_response(api: &str, response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => format!("AUTH_ERROR: {api} returned HTTP {}", status.as_u16()),
            423 => "BANNED: account suspended".to_string(),
            _ => format!("{api} returned HTTP {}", status.as_u16()),
        });
    }
    response
        .json()
        .await
        .map_err(|_| format!("{api} returned invalid JSON"))
}

fn upstream_error(message: impl Into<String>) -> AppError {
    AppError::new(
        StatusCode::BAD_GATEWAY,
        "kiro_upstream_error",
        public_error(&message.into()),
    )
}

fn public_error(message: &str) -> String {
    message.chars().take(400).collect()
}

async fn record_refresh_failure(state: &AppState, id: &str, status: u16) -> AppResult<()> {
    sqlx::query("UPDATE accounts SET status='error',disabled_reason=?,failure_count=failure_count+1,last_failure_at=?,updated_at=? WHERE id=?")
        .bind(format!("token refresh returned HTTP {status}")).bind(db::now()).bind(db::now()).bind(id).execute(&state.pool).await?;
    Ok(())
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value
            .get(*name)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn number_field(value: &Value, names: &[&str]) -> Option<i64> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_i64))
}

fn validate_external_token_endpoint(endpoint: &str) -> AppResult<()> {
    let url =
        Url::parse(endpoint).map_err(|_| AppError::bad_request("tokenEndpoint is invalid"))?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if url.scheme() != "https"
        || !(host == "login.microsoftonline.com" || host.ends_with(".login.microsoftonline.com"))
    {
        return Err(AppError::bad_request(
            "external tokenEndpoint must use HTTPS on login.microsoftonline.com",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_external_token_endpoint;

    #[test]
    fn blocks_arbitrary_external_refresh_endpoint() {
        assert!(validate_external_token_endpoint("https://example.com/token").is_err());
        assert!(
            validate_external_token_endpoint("http://login.microsoftonline.com/token").is_err()
        );
        assert!(validate_external_token_endpoint(
            "https://login.microsoftonline.com/tenant/oauth2/v2.0/token"
        )
        .is_ok());
    }
}
