use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::AppState;

#[derive(Deserialize)]
pub struct CorrelateParams {
    pub timestamp: Option<String>,
    pub top_n: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct CorrelateSample {
    timestamp: i64,
    global_cpu_pct: f32,
    used_mem_kb: i64,
    disk_read_bytes: i64,
    disk_write_bytes: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct TopProcess {
    pid: i32,
    name: String,
    cpu_pct: f32,
    mem_kb: i64,
}

const NEAREST_TOLERANCE_SECS: i64 = 5;

pub async fn get_correlate(
    State(state): State<AppState>,
    Query(params): Query<CorrelateParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Parse as strings so malformed values return our 400 JSON, not Axum's 422.
    let requested: i64 = match params.timestamp {
        Some(s) => s.parse().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid timestamp (must be unix seconds)" })),
            )
        })?,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "timestamp query param is required (unix seconds)" })),
            ))
        }
    };
    let top_n: i64 = match params.top_n {
        Some(s) => s.parse().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid top_n" })),
            )
        })?,
        None => 10,
    }
    .clamp(1, 50);

    // Exact match first (cheap PK lookup).
    let mut sample = sqlx::query_as::<_, CorrelateSample>(
        "SELECT timestamp, cpu_pct AS global_cpu_pct, used_mem_kb, disk_read_bytes, disk_write_bytes \
         FROM samples WHERE timestamp = $1",
    )
    .bind(requested)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("correlate exact lookup failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "query failed" })),
        )
    })?;

    // Fall back to nearest sample within tolerance (clicks land between 3s ticks).
    // Bounded range keeps the index on samples(timestamp) usable instead of a full scan.
    if sample.is_none() {
        sample = sqlx::query_as::<_, CorrelateSample>(
            "SELECT timestamp, cpu_pct AS global_cpu_pct, used_mem_kb, disk_read_bytes, disk_write_bytes \
             FROM samples WHERE timestamp BETWEEN $1 - 5 AND $1 + 5 \
             ORDER BY ABS(timestamp - $1) ASC LIMIT 1",
        )
        .bind(requested)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("correlate nearest lookup failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "query failed" })),
            )
        })?;
        if let Some(ref s) = sample {
            if (s.timestamp - requested).abs() > NEAREST_TOLERANCE_SECS {
                sample = None;
            }
        }
    }

    let sample = match sample {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "no sample found for timestamp" })),
            ))
        }
    };

    let top_processes = sqlx::query_as::<_, TopProcess>(
        "SELECT pid, process_name AS name, cpu_pct, mem_kb FROM process_samples \
         WHERE timestamp = $1 ORDER BY cpu_pct DESC LIMIT $2",
    )
    .bind(sample.timestamp)
    .bind(top_n)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("correlate processes query failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "query failed" })),
        )
    })?;

    Ok(Json(json!({ "sample": sample, "top_processes": top_processes })))
}
