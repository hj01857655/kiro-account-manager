use crate::{
    db,
    error::{AppError, AppResult},
    models::ApiResponse,
    state::AppState,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{
        header::{COOKIE, SET_COOKIE},
        HeaderMap, Method, Request,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use subtle::ConstantTimeEq;
use uuid::Uuid;

const COOKIE_NAME: &str = "kiro_admin";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    csrf: String,
    jti: String,
    iat: usize,
    exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
    pub client_ip: String,
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<LoginInput>,
) -> AppResult<Response> {
    let ip = client_ip(&headers, peer, &state.config.trusted_proxy_ips);
    state.check_login_rate(&ip).await?;
    let row = sqlx::query_as::<_, (String,)>("SELECT password_hash FROM admins WHERE username = ?")
        .bind(input.username.trim())
        .fetch_optional(&state.pool)
        .await?;
    let password = input.password;
    let valid = if let Some((hash,)) = row {
        tokio::task::spawn_blocking(move || {
            PasswordHash::new(&hash).ok().is_some_and(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
        })
        .await
        .unwrap_or(false)
    } else {
        // Keep nonexistent-user requests intentionally expensive to reduce username probing.
        tokio::task::spawn_blocking(move || {
            use argon2::{
                password_hash::{rand_core::OsRng, SaltString},
                PasswordHasher,
            };
            let salt = SaltString::generate(&mut OsRng);
            let _ = Argon2::default().hash_password(password.as_bytes(), &salt);
            false
        })
        .await
        .unwrap_or(false)
    };

    if !valid {
        state.record_login_failure(&ip).await;
        tracing::warn!(client_ip = %ip, username = %input.username, "administrator login failed");
        db::audit(
            &state.pool,
            "warn",
            "auth.login_failed",
            Some("admin"),
            None,
            "administrator login failed",
            Some(&ip),
        )
        .await;
        return Err(AppError::new(
            axum::http::StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "invalid username or password",
        ));
    }

    state.clear_login_failures(&ip).await;
    let csrf: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    let now = Utc::now();
    let claims = Claims {
        sub: input.username.trim().to_string(),
        csrf: csrf.clone(),
        jti: Uuid::new_v4().to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(8)).timestamp() as usize,
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::internal())?;
    db::audit(
        &state.pool,
        "info",
        "auth.login",
        Some("admin"),
        None,
        "administrator signed in",
        Some(&ip),
    )
    .await;
    let mut response = Json(ApiResponse::ok(
        json!({ "username": claims.sub, "csrfToken": csrf }),
    ))
    .into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        cookie_value(&token, state.config.cookie_secure, false)
            .parse()
            .map_err(|_| AppError::internal())?,
    );
    Ok(response)
}

pub async fn me(
    State(_state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
    request: Request<Body>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(AppError::unauthorized)?;
    Ok(Json(ApiResponse::ok(
        json!({ "username": user.username, "csrfToken": claims.csrf }),
    )))
}

pub async fn logout(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
) -> AppResult<Response> {
    db::audit(
        &state.pool,
        "info",
        "auth.logout",
        Some("admin"),
        None,
        "administrator signed out",
        Some(&user.client_ip),
    )
    .await;
    let mut response = Json(ApiResponse::ok(json!({ "loggedOut": true }))).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        cookie_value("", state.config.cookie_secure, true)
            .parse()
            .map_err(|_| AppError::internal())?,
    );
    Ok(response)
}

pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    let token = cookie(request.headers(), COOKIE_NAME).ok_or_else(AppError::unauthorized)?;
    let validation = Validation::new(Algorithm::HS256);
    let claims = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::unauthorized())?
    .claims;
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| {
            client_ip(request.headers(), *peer, &state.config.trusted_proxy_ips)
        })
        .unwrap_or_else(|| "unknown".to_string());
    state
        .check_api_rate(&format!("{}:{ip}", claims.sub))
        .await?;
    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        let supplied = request
            .headers()
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !constant_time_equal(supplied.as_bytes(), claims.csrf.as_bytes()) {
            return Err(AppError::forbidden("invalid CSRF token"));
        }
    }
    request.extensions_mut().insert(AuthUser {
        username: claims.sub.clone(),
        client_ip: ip,
    });
    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

pub fn client_ip(headers: &HeaderMap, peer: SocketAddr, trusted_proxy_ips: &[IpAddr]) -> String {
    if trusted_proxy_ips.contains(&peer.ip()) {
        if let Some(value) = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
        {
            if let Some(ip) = value
                .split(',')
                .next()
                .map(str::trim)
                .and_then(|value| value.parse::<IpAddr>().ok())
            {
                return ip.to_string();
            }
        }
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<IpAddr>().ok())
        {
            return ip.to_string();
        }
    }
    peer.ip().to_string()
}

fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then(|| value.to_string())
        })
}

fn cookie_value(token: &str, secure: bool, clear: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    let age = if clear { 0 } else { 28_800 };
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={age}{secure}")
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bool::from(left.ct_eq(right))
}

#[cfg(test)]
mod tests {
    use super::{client_ip, constant_time_equal, cookie};
    use axum::http::{header::COOKIE, HeaderMap};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn parses_named_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            "first=1; kiro_admin=secret; last=2".parse().unwrap(),
        );
        assert_eq!(cookie(&headers, "kiro_admin").as_deref(), Some("secret"));
    }

    #[test]
    fn csrf_comparison_rejects_mismatch() {
        assert!(constant_time_equal(b"same", b"same"));
        assert!(!constant_time_equal(b"same", b"different"));
    }

    #[test]
    fn forwarded_ip_is_used_only_for_trusted_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.10".parse().unwrap());
        let trusted_peer = SocketAddr::from(([172, 29, 0, 1], 1234));
        let trusted = [IpAddr::V4(Ipv4Addr::new(172, 29, 0, 1))];
        assert_eq!(client_ip(&headers, trusted_peer, &trusted), "203.0.113.10");
        assert_eq!(client_ip(&headers, trusted_peer, &[]), "172.29.0.1");
    }
}
