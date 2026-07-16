mod auth;
mod config;
mod crypto;
mod db;
mod error;
mod gateway;
mod handlers;
mod kiro;
mod models;
mod state;

use axum::{
    extract::DefaultBodyLimit,
    http::{
        header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE, COOKIE},
        Method,
    },
    middleware,
    routing::{get, post, put},
    Router,
};
use config::Config;
use state::AppState;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_tracing();
    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("startup configuration error: {error}");
        std::process::exit(2);
    });
    let pool = db::connect(&config.database_path)
        .await
        .unwrap_or_else(|error| {
            eprintln!("database startup error: {error}");
            std::process::exit(2);
        });
    db::seed_admin(&pool, &config.admin_username, &config.admin_password)
        .await
        .unwrap_or_else(|error| {
            eprintln!("administrator startup error: {error}");
            std::process::exit(2);
        });
    let bind_addr = config.bind_addr;
    let allowed_origin = config.allowed_origin.clone();
    let state = AppState::new(pool, config);
    let app = router(state, allowed_origin);
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .unwrap_or_else(|error| {
            eprintln!("failed to bind {bind_addr}: {error}");
            std::process::exit(2);
        });
    tracing::info!(address = %bind_addr, "Kiro web backend started");
    if let Err(error) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        tracing::error!(error = %error, "HTTP server stopped unexpectedly");
        std::process::exit(1);
    }
}

fn router(state: AppState, allowed_origin: Option<String>) -> Router {
    let protected = Router::new()
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/dashboard", get(handlers::dashboard))
        .route(
            "/api/accounts",
            get(handlers::list_accounts).post(handlers::create_account),
        )
        .route("/api/accounts/import", post(handlers::import_accounts))
        .route("/api/accounts/export", get(handlers::export_accounts))
        .route(
            "/api/accounts/{id}",
            get(handlers::get_account)
                .put(handlers::update_account)
                .delete(handlers::delete_account),
        )
        .route(
            "/api/accounts/{id}/refresh",
            post(handlers::refresh_account),
        )
        .route("/api/accounts/{id}/check", post(handlers::check_account))
        .route("/api/accounts/{id}/usage", get(handlers::account_usage))
        .route("/api/accounts/{id}/models", get(handlers::account_models))
        .route(
            "/api/groups",
            get(handlers::groups).post(handlers::create_group),
        )
        .route(
            "/api/groups/{id}",
            put(handlers::update_group).delete(handlers::delete_group),
        )
        .route("/api/tags", get(handlers::tags).post(handlers::create_tag))
        .route(
            "/api/tags/{id}",
            put(handlers::update_tag).delete(handlers::delete_tag),
        )
        .route(
            "/api/settings",
            get(handlers::settings).put(handlers::update_settings),
        )
        .route("/api/logs", get(handlers::logs))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_admin,
        ));

    let gateway = Router::new()
        .route("/v1/models", get(gateway::list_models))
        .route("/v1/messages", post(gateway::messages))
        .route("/v1/chat/completions", post(gateway::chat_completions))
        .route("/v1/responses", post(gateway::responses));

    let request_id = HeaderName::from_static("x-request-id");
    let common_layers = ServiceBuilder::new()
        .layer(SetSensitiveRequestHeadersLayer::new([
            AUTHORIZATION,
            COOKIE,
        ]))
        .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'; base-uri 'none'"),
        ))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024));

    let mut app = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/auth/login", post(auth::login))
        .merge(protected)
        .merge(gateway)
        .with_state(state)
        .layer(common_layers);

    if let Some(origin) = allowed_origin {
        let origin = origin.parse::<HeaderValue>().unwrap_or_else(|_| {
            eprintln!("startup configuration error: ALLOWED_ORIGIN is not a valid origin");
            std::process::exit(2);
        });
        app = app.layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_credentials(true)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    CONTENT_TYPE,
                    AUTHORIZATION,
                    HeaderName::from_static("x-csrf-token"),
                ]),
        );
    }
    app
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kiro_web_backend=info,tower_http=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_target(false)
                .with_current_span(false),
        )
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutdown signal received");
}
