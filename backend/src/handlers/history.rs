use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::AppState;

#[derive(Deserialize)]
pub struct HistoryParams {
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<String>,
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
    // Parse as strings (not i64) so malformed values like ?from=abc
    // return our 400 JSON instead of Axum's default 422.
    let parse_required = |name: &str, v: Option<String>| -> Result<i64, (StatusCode, Json<Value>)> {
        match v {
            Some(s) => s.parse::<i64>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("invalid {name} (must be unix seconds)") })),
                )
            }),
            None => Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "from and to query params are required (unix seconds)" })),
            )),
        }
    };
    let from = parse_required("from", params.from)?;
    let to = parse_required("to", params.to)?;
    if from > to {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "from must be <= to" })),
        ));
    }
    let limit = match params.limit {
        Some(s) => s.parse::<i64>().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid limit" })),
            )
        })?,
        None => 500,
    }
    .clamp(1, 2000);

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
