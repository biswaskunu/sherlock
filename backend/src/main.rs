use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;

mod db;
mod handlers;
mod models;

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

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/metrics/batch", post(handlers::metrics::post_batch))
        .with_state(pool);

    let addr: SocketAddr = format!("{host}:{port}").parse().expect("invalid bind addr");
    tracing::info!("backend listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
