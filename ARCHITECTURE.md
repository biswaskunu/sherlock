# Architecture: "sherlock" System Monitor

## 1. High-Level Overview

Two data paths, one sampling loop:

```
                     ┌──────────────────┐
                     │   Sampling Loop   │
                     │ (tokio::interval, │
                     │     every 5s)     │
                     └─────────┬─────────┘
                               │
                 ┌─────────────┴─────────────┐
                 ▼                           ▼
         LIVE PATH (SSE)              STORAGE PATH (batched)
         push each sample             buffer 12 samples (60s)
         immediately to               then POST batch to
         dashboard, no                Axum backend
         storage roundtrip                   │
                 │                           ▼
                 ▼                    ┌──────────────┐
         ┌──────────────┐             │   Postgres   │
         │  Dashboard    │◄───────────┤ (time-series  │
         │  (live charts)│  history   │   samples)    │
         └──────────────┘  queries    └──────────────┘
```

## 2. Components

### 2.1 Agent (Rust binary, runs locally)
- Uses `sysinfo` crate to read CPU, memory, disk I/O, and per-process stats.
- `tokio::time::interval` drives a 5s polling loop.
- Each tick:
  1. Serializes a `Metric` struct to JSON.
  2. Pushes it to an in-memory ring buffer (for batching).
  3. Sends it immediately over an SSE channel (for live view).
- Every 60s (12 ticks): flushes the buffer as a batch POST to the backend, then clears it.

### 2.2 Backend (Axum + Postgres)
- `POST /api/metrics/batch` — accepts an array of ~12 samples, writes them in a single `INSERT`.
- `GET /api/metrics/live` (SSE) — proxies/streams live samples from the agent to any connected dashboard client. (If agent and dashboard run on the same machine, agent can stream SSE directly; backend involvement here is optional for v1.)
- `GET /api/metrics/history?from=&to=` — queries Postgres for a time range, returns samples for charting.
- `GET /api/correlate?timestamp=` — given a timestamp, returns system metrics + top N processes by resource usage at that moment.

### 2.3 Storage (Postgres)
- `samples` table: timestamp, cpu_pct, mem_used, disk_read, disk_write, net_rx, net_tx.
- `process_samples` table: timestamp, pid, process_name, cpu_pct, mem_used — linked to `samples` by timestamp for correlation queries.
- Retention: raw data kept for a configurable window (e.g. 24-48h); older data can be downsampled or dropped in a later phase.

### 2.4 Dashboard (minimal frontend)
- Live view: connects to SSE endpoint, renders rolling charts (Chart.js or similar).
- History view: time-range picker, fetches from `/api/metrics/history`, renders charts + a "click spike to see processes" interaction backed by `/api/correlate`.

## 3. Folder Structure (backend)

```
src/
  handlers/
    metrics.rs      # batch POST, history GET
    correlate.rs    # correlation endpoint
    live.rs         # SSE endpoint (if backend-mediated)
  models/
    sample.rs
    process_sample.rs
  db/
    mod.rs          # pool setup, queries
  main.rs
agent/
  src/
    main.rs         # sampling loop, buffering, batch POST, SSE push
```

## 4. Key Design Decisions
- **Two data paths (live vs. storage)** rather than one: live view needs low latency and doesn't need durability; storage needs durability and can tolerate up to 60s latency. Trying to serve both from one path forces a bad trade-off on one side.
- **Batched writes over per-sample writes**: 1 insert/minute instead of 12 inserts/minute — far friendlier to Postgres and matches how real observability agents (Prometheus node_exporter, Datadog agent) behave.
- **`sysinfo` crate over raw `/proc` parsing**: cross-platform abstraction saves significant time; can drop to raw `/proc` later if a specific stat isn't exposed.
- **Correlation via timestamp join, not real-time streaming logic**: keeps v1 simple — correlation is a query over stored data, not a live streaming algorithm.

## 5. Known Limitations (v1)
- Agent crash mid-buffer loses up to 60s of unsent samples.
- Correlation granularity limited by 5s sampling interval — very short spikes may be missed.
- Linux-first (via `/proc` through `sysinfo`); Windows/Mac support not guaranteed in v1.
