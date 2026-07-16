use crate::{
    db,
    error::{AppError, AppResult},
    models::ApiResponse,
    state::AppState,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{
        header::{COOKIE, SET_COOKIE},
        HeaderMap, Method, Request, StatusCode,
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
use std::net::SocketAddr;
use subtle::ConstantTimeEq;
use uuid::Uuid;

const COOKIE_NAME: &str = "kiro_user";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsInput {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    kind: String,
    csrf: String,
    jti: String,
    iat: usize,
    exp: usize,
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub id: String,
    pub email: String,
    pub client_ip: String,
}

pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<CredentialsInput>,
) -> AppResult<Response> {
    if !state.config.user_registration_enabled {
        return Err(AppError::forbidden("user registration is disabled"));
    }
    let ip = crate::auth::client_ip(&headers, peer, &state.config.trusted_proxy_ips);
    state
        .check_login_rate(&format!("user-register:{ip}"))
        .await?;
    let email = normalize_email(&input.email)?;
    validate_password(&input.password)?;

    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_one(&state.pool)
        .await?;
    if exists.0 > 0 {
        state
            .record_login_failure(&format!("user-register:{ip}"))
            .await;
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "email_exists",
            "this email is already registered",
        ));
    }

    let password = input.password;
    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::internal())?
    .map_err(|_| AppError::internal())?;

    let id = Uuid::new_v4().to_string();
    let now = db::now();
    sqlx::query(
        "INSERT INTO users (id,email,password_hash,created_at,updated_at,last_login_at) VALUES (?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&email)
    .bind(password_hash)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await?;
    state
        .clear_login_failures(&format!("user-register:{ip}"))
        .await;
    db::audit(
        &state.pool,
        "info",
        "user.register",
        Some("user"),
        Some(&id),
        "user account registered",
        Some(&ip),
    )
    .await;
    session_response(&state, id, email)
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<CredentialsInput>,
) -> AppResult<Response> {
    let ip = crate::auth::client_ip(&headers, peer, &state.config.trusted_proxy_ips);
    let rate_key = format!("user-login:{ip}");
    state.check_login_rate(&rate_key).await?;
    let email = normalize_email(&input.email)?;
    let row =
        sqlx::query_as::<_, (String, String)>("SELECT id,password_hash FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await?;
    let password = input.password;
    let valid = if let Some((id, hash)) = row {
        let verified = tokio::task::spawn_blocking(move || {
            PasswordHash::new(&hash).ok().is_some_and(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
        })
        .await
        .unwrap_or(false);
        verified.then_some(id)
    } else {
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let _ = Argon2::default().hash_password(password.as_bytes(), &salt);
        })
        .await
        .ok();
        None
    };

    let Some(id) = valid else {
        state.record_login_failure(&rate_key).await;
        tracing::warn!(client_ip = %ip, "user login failed");
        db::audit(
            &state.pool,
            "warn",
            "user.login_failed",
            Some("user"),
            None,
            "user login failed",
            Some(&ip),
        )
        .await;
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "invalid email or password",
        ));
    };

    state.clear_login_failures(&rate_key).await;
    sqlx::query("UPDATE users SET last_login_at=?, updated_at=? WHERE id=?")
        .bind(db::now())
        .bind(db::now())
        .bind(&id)
        .execute(&state.pool)
        .await?;
    db::audit(
        &state.pool,
        "info",
        "user.login",
        Some("user"),
        Some(&id),
        "user signed in",
        Some(&ip),
    )
    .await;
    session_response(&state, id, email)
}

pub async fn me(
    axum::extract::Extension(user): axum::extract::Extension<UserSession>,
    request: Request<Body>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(AppError::unauthorized)?;
    Ok(Json(ApiResponse::ok(json!({
        "id": user.id,
        "email": user.email,
        "csrfToken": claims.csrf
    }))))
}

pub async fn logout(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<UserSession>,
) -> AppResult<Response> {
    db::audit(
        &state.pool,
        "info",
        "user.logout",
        Some("user"),
        Some(&user.id),
        "user signed out",
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

pub async fn require_user(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    let token = cookie(request.headers(), COOKIE_NAME).ok_or_else(AppError::unauthorized)?;
    let claims = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::unauthorized())?
    .claims;
    if claims.kind != "user" {
        return Err(AppError::unauthorized());
    }
    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id=? AND email=?")
        .bind(&claims.sub)
        .bind(&claims.email)
        .fetch_one(&state.pool)
        .await?;
    if exists.0 != 1 {
        return Err(AppError::unauthorized());
    }
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| {
            crate::auth::client_ip(request.headers(), *peer, &state.config.trusted_proxy_ips)
        })
        .unwrap_or_else(|| "unknown".to_string());
    state
        .check_api_rate(&format!("user:{}:{ip}", claims.sub))
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
    request.extensions_mut().insert(UserSession {
        id: claims.sub.clone(),
        email: claims.email.clone(),
        client_ip: ip,
    });
    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

fn session_response(state: &AppState, id: String, email: String) -> AppResult<Response> {
    let csrf: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    let now = Utc::now();
    let claims = Claims {
        sub: id.clone(),
        email: email.clone(),
        kind: "user".to_string(),
        csrf: csrf.clone(),
        jti: Uuid::new_v4().to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::days(7)).timestamp() as usize,
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::internal())?;
    let mut response = Json(ApiResponse::ok(json!({
        "id": id,
        "email": email,
        "csrfToken": csrf
    })))
    .into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        cookie_value(&token, state.config.cookie_secure, false)
            .parse()
            .map_err(|_| AppError::internal())?,
    );
    Ok(response)
}

fn normalize_email(value: &str) -> AppResult<String> {
    let email = value.trim().to_ascii_lowercase();
    let valid = email.len() <= 254
        && email.len() >= 3
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
        });
    if !valid {
        return Err(AppError::bad_request("enter a valid email address"));
    }
    Ok(email)
}

fn validate_password(value: &str) -> AppResult<()> {
    if !(8..=128).contains(&value.len()) {
        return Err(AppError::bad_request(
            "password must contain between 8 and 128 characters",
        ));
    }
    Ok(())
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
    let age = if clear { 0 } else { 604_800 };
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={age}{secure}")
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bool::from(left.ct_eq(right))
}

#[cfg(test)]
mod tests {
    use super::{normalize_email, validate_password};

    #[test]
    fn normalizes_valid_email() {
        assert_eq!(
            normalize_email(" User@Example.COM ").unwrap(),
            "user@example.com"
        );
    }

    #[test]
    fn rejects_invalid_credentials_shape() {
        assert!(normalize_email("not-an-email").is_err());
        assert!(validate_password("short").is_err());
        assert!(validate_password("long-enough").is_ok());
    }
}
