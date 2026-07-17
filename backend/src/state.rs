use crate::{
    config::Config,
    crypto::Crypto,
    error::{AppError, AppResult},
};
use axum::http::StatusCode;
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub crypto: Crypto,
    login_attempts: Arc<Mutex<HashMap<String, RateWindow>>>,
    api_requests: Arc<Mutex<HashMap<String, RateWindow>>>,
}

#[derive(Clone, Copy)]
struct RateWindow {
    started_at: Instant,
    count: u32,
}

impl AppState {
    pub fn new(pool: SqlitePool, config: Config) -> Self {
        let crypto = Crypto::new(&config.encryption_key);
        Self {
            pool,
            config: Arc::new(config),
            crypto,
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
            api_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check_login_rate(&self, key: &str) -> AppResult<()> {
        check_rate(&self.login_attempts, key, 5, Duration::from_secs(15 * 60)).await
    }

    pub async fn record_login_failure(&self, key: &str) {
        increment_rate(&self.login_attempts, key, Duration::from_secs(15 * 60)).await;
    }

    pub async fn clear_login_failures(&self, key: &str) {
        self.login_attempts.lock().await.remove(key);
    }

    pub async fn check_api_rate(&self, key: &str) -> AppResult<()> {
        let mut windows = self.api_requests.lock().await;
        let now = Instant::now();
        let window = windows.entry(key.to_string()).or_insert(RateWindow {
            started_at: now,
            count: 0,
        });
        if now.duration_since(window.started_at) >= Duration::from_secs(60) {
            *window = RateWindow {
                started_at: now,
                count: 0,
            };
        }
        if window.count >= 240 {
            return Err(AppError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many requests",
            ));
        }
        window.count += 1;
        Ok(())
    }
}

async fn check_rate(
    map: &Mutex<HashMap<String, RateWindow>>,
    key: &str,
    limit: u32,
    duration: Duration,
) -> AppResult<()> {
    let mut windows = map.lock().await;
    let now = Instant::now();
    let window = windows.entry(key.to_string()).or_insert(RateWindow {
        started_at: now,
        count: 0,
    });
    if now.duration_since(window.started_at) >= duration {
        *window = RateWindow {
            started_at: now,
            count: 0,
        };
    }
    if window.count >= limit {
        return Err(AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "login_rate_limited",
            "too many failed login attempts; try again later",
        ));
    }
    Ok(())
}

async fn increment_rate(map: &Mutex<HashMap<String, RateWindow>>, key: &str, duration: Duration) {
    let mut windows = map.lock().await;
    let now = Instant::now();
    let window = windows.entry(key.to_string()).or_insert(RateWindow {
        started_at: now,
        count: 0,
    });
    if now.duration_since(window.started_at) >= duration {
        *window = RateWindow {
            started_at: now,
            count: 0,
        };
    }
    window.count = window.count.saturating_add(1);
}
