use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::Stream;
use serde_json::{json, Value};
use std::{convert::Infallible, time::Duration};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use crate::{models::LiveSample, AppState};

/// Agent pushes one tick here every ~3s. Broadcasts in-memory only (no DB write).
pub async fn publish(
    State(state): State<AppState>,
    Json(sample): Json<LiveSample>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match state.live_tx.send(sample) {
        Ok(_) => Ok(Json(json!({ "ok": true }))),
        // No dashboard connected: live is lossy by design, still ack the agent.
        Err(tokio::sync::broadcast::error::SendError(_)) => Ok(Json(json!({ "ok": true })) ),
    }
}

/// Dashboard connects here with `EventSource`. Streams live ticks as JSON events.
pub async fn sse(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.live_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        // Skip lagged (stale) ticks: live tail stays fresh under backpressure.
        Ok(sample) => Some(Ok(Event::default()
            .json_data(sample)
            .unwrap_or_else(|_| Event::default().data("{\"error\":\"encode\"}")))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
