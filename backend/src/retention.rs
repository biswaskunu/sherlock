use sqlx::PgPool;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Retention policy: drop raw samples older than `retention_hours`.
/// `process_samples` rows are cleaned via `ON DELETE CASCADE`.
pub async fn purge_once(pool: &PgPool, retention_hours: i64) -> Result<u64, sqlx::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let cutoff = now - retention_hours.saturating_mul(3600);
    let res = sqlx::query("DELETE FROM samples WHERE timestamp < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub fn config_from_env() -> (i64, u64) {
    let hours = std::env::var("RETENTION_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(48)
        .clamp(1, 720);
    let interval_secs = std::env::var("RETENTION_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600)
        .clamp(10, 86_400);
    (hours, interval_secs)
}

/// Run once at startup, then on an interval. Never panics the backend —
/// failures are logged and retried on the next tick.
pub fn spawn(pool: PgPool, retention_hours: i64, interval_secs: u64) {
    tokio::spawn(async move {
        // Startup purge so a restarted backend doesn't sit on stale data
        // for a full interval.
        match purge_once(&pool, retention_hours).await {
            Ok(n) => tracing::info!("retention: startup purge deleted {n} samples (> {retention_hours}h)"),
            Err(e) => tracing::warn!("retention: startup purge failed: {e}"),
        }
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        // First tick fires immediately; skip it since startup purge just ran.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match purge_once(&pool, retention_hours).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("retention: deleted {n} samples (> {retention_hours}h)"),
                Err(e) => tracing::warn!("retention: purge failed: {e}"),
            }
        }
    });
}
