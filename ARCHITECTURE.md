# Architecture: "sherlock" System Monitor

## Current vs Planned

> ⚠️ **Current state: Phase 1 only** — agent sampling loop with ring buffer, flush prints JSON to stdout. Everything below the "Target" lines is aspirational until implemented.

## 1. High-Level Overview

**Target** (two data paths, one sampling loop):

```
                     ┌──────────────────┐
                     │   Sampling Loop   │
                     │ (tokio::interval, │
                     │     every 3s)     │
                     └─────────┬─────────┘
                               │
                 ┌─────────────┴─────────────┐
                 ▼                           ▼
         LIVE PATH (SSE)              STORAGE PATH (batched)
          push each sample             buffer 20 samples (60s)
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

### 2.1 Agent (Rust binary, runs locally) — Phase 1 ✅

- Uses `sysinfo` crate to read CPU, memory, disk I/O, and per-process stats.
- `tokio::time::interval` drives a 3s polling loop.
- Each tick:
  1. Builds a `Metric` struct (see `api-spec.md` for schema).
  2. Pushes it to an in-memory ring buffer (`VecDeque`, capacity 20).
  3. Prints live tick to stdout (`cpu={:.1}% mem={}/{}kb`).
- Every 60s (20 ticks): calls `flush()` — currently prints batch JSON to stdout. Phase 2 swaps this to `POST /api/metrics/batch`.

### 2.2 Backend (Axum + Postgres) — Phase 2 🔲
- `POST /api/metrics/batch` — accepts an array of ~20 samples, writes them in a single `INSERT`.
- `GET /api/metrics/live` (SSE) — streams live samples to dashboard clients (backend-mediated, not agent-direct).
- `GET /api/metrics/history?from=&to=` — queries Postgres for a time range, returns samples for charting.
- `GET /api/correlate?timestamp=` — given a timestamp, returns system metrics + top N processes by resource usage at that moment.

### 2.3 Storage (Postgres) — Phase 2 🔲
- `samples` table: timestamp (PK), cpu_pct, total_mem_kb, used_mem_kb, disk_read_bytes, disk_write_bytes, net_rx_bytes (reserved), net_tx_bytes (reserved).
- `process_samples` table: id (serial PK), timestamp → samples(timestamp), pid, process_name, cpu_pct, mem_kb.
- Retention: raw data kept configurable window (24–48h default); older data downsampled or dropped in Phase 5.
- **Note**: `net_rx_bytes` / `net_tx_bytes` columns exist but agent doesn't populate them yet — `sysinfo` doesn't expose global network throughput; interface-level stats may be added later.

### 2.4 Dashboard (minimal frontend)
- Live view: connects to SSE endpoint, renders rolling charts (Chart.js or similar).
- History view: time-range picker, fetches from `/api/metrics/history`, renders charts + a "click spike to see processes" interaction backed by `/api/correlate`.

## 3. Folder Structure (target)

```
sherlock/
├── agent/src/main.rs          # sampling loop, buffering, batch flush
├── backend/src/
│   ├── handlers/
│   │   ├── metrics.rs         # batch POST, history GET
│   │   ├── correlate.rs       # correlation endpoint
│   │   └── live.rs            # SSE endpoint
│   ├── models/
│   │   ├── sample.rs
│   │   └── process_sample.rs
│   └── db/
│       └── mod.rs             # pool setup, queries
├── schema.sql
├── docker-compose.yml
├── .env.example
├── api-spec.md
├── ARCHITECTURE.md
├── PHASES.md
└── PRD.md
```

## 4. Key Design Decisions
- **Two data paths (live vs. storage)** rather than one: live view needs low latency and doesn't need durability; storage needs durability and can tolerate up to 60s latency. Trying to serve both from one path forces a bad trade-off on one side.
- **Batched writes over per-sample writes**: 1 insert/minute instead of 12 inserts/minute — far friendlier to Postgres and matches how real observability agents (Prometheus node_exporter, Datadog agent) behave.
- **`sysinfo` crate over raw `/proc` parsing**: cross-platform abstraction saves significant time; can drop to raw `/proc` later if a specific stat isn't exposed.
- **Backend-mediated SSE** (not agent-direct): single connection point for dashboard, easier auth/rate-limiting later.
- **Correlation via timestamp join, not streaming logic**: keeps v1 simple — correlation is a query over stored data, not a live streaming algorithm.

## 5. Known Limitations (v1)
- Agent crash mid-buffer loses up to 60s of unsent samples.
- Correlation granularity limited by 3s sampling interval — very short spikes may be missed.
- Linux-first via sysinfo; Windows/Mac support not guaranteed in v1.
- No network stats collected yet (net_rx/net_tx reserved in schema).
