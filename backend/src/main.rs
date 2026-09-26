use axum::{
    http::{HeaderValue, Method},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

mod db;
mod handlers;
mod models;
mod retention;

use models::LiveSample;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub live_tx: tokio::sync::broadcast::Sender<LiveSample>,
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let host = std::env::var("BACKEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("BACKEND_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set (see .env.example)");

    let pool = db::connect(&database_url)
        .await
        .expect("failed to connect to Postgres");
    tracing::info!("connected to Postgres");

    // Retention: drop raw samples older than RETENTION_HOURS (default 48h).
    // process_samples cleaned via ON DELETE CASCADE. In-memory live ticks
    // are never persisted, so nothing to retain there.
    let (retention_hours, retention_interval) = retention::config_from_env();
    retention::spawn(pool.clone(), retention_hours, retention_interval);
    tracing::info!(
        "retention enabled: keep {retention_hours}h, purge every {retention_interval}s"
    );

    // In-memory fan-out for live ticks. Cap 32 keeps memory bounded;
    // slow dashboards skip lagged ticks (see handlers::live::sse).
    let (live_tx, _) = tokio::sync::broadcast::channel::<LiveSample>(32);
    let state = AppState { pool, live_tx };

    // Separate frontend server (Vite :5173) is cross-origin, so allow it.
    let dashboard_origin = std::env::var("DASHBOARD_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".to_string());
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_origin(
            dashboard_origin
                .parse::<HeaderValue>()
                .expect("invalid DASHBOARD_ORIGIN"),
        );

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/metrics/batch", post(handlers::metrics::post_batch))
        .route(
            "/api/metrics/live/publish",
            post(handlers::live::publish),
        )
        .route("/api/metrics/live", get(handlers::live::sse))
        .route("/api/metrics/history", get(handlers::history::get_history))
        .route("/api/correlate", get(handlers::correlate::get_correlate))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse().expect("invalid bind addr");
    tracing::info!("backend listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
