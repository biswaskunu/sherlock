# Architecture: "sherlock" System Monitor

## Current vs Planned

> ✅ **Current state: end of Phase 4** — agent POSTs 20-sample batches to Axum (stored in Postgres) and publishes each 3s tick to Axum for SSE fan-out to the Vite dashboard (with top processes). History range queries + spike correlation are implemented; only Phase 5 polish remains.

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
- Every 60s (20 ticks): calls `flush()` — POSTs the batch to `POST /api/metrics/batch`; on error/non-2xx the buffer is kept for retry, growth capped at 3 flush windows.

### 2.2 Backend (Axum + Postgres) — Phase 2 (batch ✅), Phase 3 (SSE ✅), Phase 4 (history/correlate ✅)
- `POST /api/metrics/batch` ✅ — accepts an array of ~20 samples, writes them in one transaction (parents then children, 2 round-trips); empty batch → 400, idempotent parent retry via `ON CONFLICT DO NOTHING`.
- `POST /api/metrics/live/publish` ✅ — accepts a single tick, broadcasts via `broadcast::channel(32)`; in-memory only, no DB write. Always 202-ish ack (live is lossy).
- `GET /api/metrics/live` (SSE) ✅ — streams live samples to dashboard clients (backend-mediated, not agent-direct); lagged ticks skipped, keep-alive every 15s; CORS allows `DASHBOARD_ORIGIN` (Vite :5173).
- `GET /api/metrics/history?from=&to=&limit=` ✅ — queries Postgres for a time range, returns samples for charting. `from`/`to` required (400 otherwise, incl. malformed values); `from <= to` enforced; `limit` default 500, clamped 1..2000.
- `GET /api/correlate?timestamp=&top_n=` ✅ — given a timestamp, returns system metrics + top N processes by resource usage at that moment. Exact PK lookup first, then nearest sample within ±5s (index-bounded range); 404 beyond tolerance. `top_n` default 10, clamped 1..50.

### 2.3 Storage (Postgres) — Phase 2 ✅
- Managed via `sqlx migrate` (`backend/migrations/0001_v1_samples.sql`, ported from `schema.sql`).
- `samples` table: timestamp (PK), cpu_pct, total_mem_kb, used_mem_kb, disk_read_bytes, disk_write_bytes, net_rx_bytes (reserved), net_tx_bytes (reserved).
- `process_samples` table: id (serial PK), timestamp → samples(timestamp), pid, process_name, cpu_pct, mem_kb.
- Retention: raw data kept configurable window (24–48h default); older data downsampled or dropped in Phase 5.
- **Note**: `net_rx_bytes` / `net_tx_bytes` columns exist but agent doesn't populate them yet — `sysinfo` doesn't expose global network throughput; interface-level stats may be added later.

### 2.4 Dashboard (minimal frontend) — Phase 3 ✅ (live view) + Phase 4 ✅ (history view)
- Live view ✅: separate Vite server (`dashboard/`, Chart.js), `EventSource` to `GET /api/metrics/live`, rolling 60-point charts + top-process table.
- History view ✅: Live/History tab toggle, time-range picker (`datetime-local`, defaults to last hour), fetches from `/api/metrics/history` (limit 500), renders Chart.js line + a "click spike to see processes" interaction backed by `/api/correlate` (nearest ±5s).

## 3. Folder Structure (target)

```
sherlock/
├── agent/src/main.rs          # sampling loop, buffering, batch flush
├── backend/src/
│   ├── handlers/
│   │   ├── metrics.rs         # batch POST ✅
│   │   ├── history.rs         # history GET ✅
│   │   ├── correlate.rs       # correlation endpoint ✅
│   │   └── live.rs            # SSE endpoint ✅
│   ├── models.rs              # ingest DTOs (BatchItem, ProcessItem)
│   └── db.rs                  # PgPool setup
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
