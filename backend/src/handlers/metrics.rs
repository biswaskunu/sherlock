use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::{models::BatchItem, AppState};

pub async fn post_batch(
    State(state): State<AppState>,
    Json(batch): Json<Vec<BatchItem>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if batch.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "empty batch" })),
        ));
    }

    let mut tx = state.pool.begin().await.map_err(internal_error)?;

    // Parent rows first: process_samples.timestamp FK-references samples(timestamp).
    {
        let mut qb =
            sqlx::QueryBuilder::new("INSERT INTO samples (timestamp, cpu_pct, total_mem_kb, used_mem_kb, disk_read_bytes, disk_write_bytes) ");

        qb.push_values(&batch, |mut b, s| {
            b.push_bind(s.timestamp)
                .push_bind(s.global_cpu_pct)
                .push_bind(s.total_mem_kb)
                .push_bind(s.used_mem_kb)
                .push_bind(s.disk_read_bytes)
                .push_bind(s.disk_write_bytes);
        });

        qb.push(" ON CONFLICT (timestamp) DO NOTHING");
        qb.build()
            .execute(&mut *tx)
            .await
            .map_err(internal_error)?;

    }

    // Child rows, one per (sample, process). Agent caps at top-10 by CPU,
    // so this is at most 10 rows/sample (<=200 for a 20-sample flush).
    let proc_count: usize = batch.iter().map(|s| s.processes.len()).sum();
    if proc_count > 0 {
        let mut qb = sqlx::QueryBuilder::new(
            "INSERT INTO process_samples (timestamp, pid, process_name, cpu_pct, mem_kb) ",
        );
        qb.push_values(batch.iter().flat_map(|s| {
            s.processes.iter().map(move |p| (s.timestamp, p))
        }), |mut b, (ts, p)| {
            b.push_bind(ts)
                .push_bind(p.pid as i32)
                .push_bind(&p.name)
                .push_bind(p.cpu_pct)
                .push_bind(p.mem_kb as i64);
        });
        // No unique constraint on child rows; plain insert (a retried batch
        // re-inserts child rows — acceptable for v1, noted in handler docs).
        qb.build()
            .execute(&mut *tx)
            .await
            .map_err(internal_error)?;
    }

    tx.commit().await.map_err(internal_error)?;

    Ok(Json(json!({ "inserted": batch.len() })))
    
}

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    tracing::error!("batch insert failed: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "insert failed" })),
    )
}
