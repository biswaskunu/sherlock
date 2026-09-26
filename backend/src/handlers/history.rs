use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::AppState;

#[derive(Deserialize)]
pub struct HistoryParams {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
struct HistorySample {
    timestamp: i64,
    global_cpu_pct: f32,
    used_mem_kb: i64,
    disk_read_bytes: i64,
    disk_write_bytes: i64,
}

pub async fn get_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (from, to) = match (params.from, params.to) {
        (Some(f), Some(t)) => (f, t),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "from and to query params are required (unix seconds)" })),
            ))
        }
    };
    if from > to {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "from must be <= to" })),
        ));
    }
    let limit = params.limit.unwrap_or(500).clamp(1, 2000);

    let samples = sqlx::query_as::<_, HistorySample>(
        "SELECT timestamp, cpu_pct AS global_cpu_pct, used_mem_kb, disk_read_bytes, disk_write_bytes \
         FROM samples WHERE timestamp BETWEEN $1 AND $2 \
         ORDER BY timestamp ASC LIMIT $3",
    )
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("history query failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "query failed" })),
        )
    })?;

    Ok(Json(json!({ "samples": samples })))
}
