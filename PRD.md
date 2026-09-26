# PRD: "sherlock" System Monitor

## 1. Problem Statement
Developers notice their machine slowing down (fan spinning, UI lagging) but have no easy way to answer "what caused that, and when?" after the fact. Built-in tools (Task Manager, `top`) only show the *current* state — there's no historical, queryable record correlating system-wide resource spikes with the specific process responsible.

## 2. Goal
Build a local Rust-based agent that continuously samples system metrics (CPU, memory, disk I/O, per-process stats), stores them durably, and surfaces both a live view and a historical "what happened at time X" view — with automatic correlation between system-level spikes and the process that caused them.

## 3. Non-Goals (v1)
- Cross-machine / fleet monitoring (single local machine only)
- Alerting/notifications
- Network-level packet inspection (interface-level stats only)
- Mobile or cloud agent versions
- Authentication/multi-user support (single local user)

## 4. Users
Primarily a personal tool / portfolio piece. Secondary audience: anyone debugging "why is my machine slow" on their own dev machine.

## 5. Core Features

### 5.1 Sampling Agent
- Polls system + process-level metrics every 3s (configurable) via `sysinfo` / `/proc`.
- Buffers samples in memory.

### 5.2 Live Path
- Agent streams each 3s sample to a local dashboard in real time via SSE.
- Dashboard shows live CPU/memory/disk graphs with near-zero latency.

### 5.3 Durable Storage Path
- Agent batches 20 samples (1 minute) and POSTs the batch to the Axum backend.
- Backend writes batch to Postgres in a single insert.
- Historical data retained with a basic retention policy (raw data for N hours, then deleted; v1 does delete-only, no downsampled aggregates — see ARCHITECTURE.md §2.3).

### 5.4 Spike Correlation
- Given a system-level spike (CPU/memory/disk) at timestamp T, identify which process(es) had elevated usage at that same timestamp.
- Exposed via API endpoint + visualized on the dashboard (click a spike → see top processes at that moment).

### 5.5 Dashboard (minimal)
- Live view: real-time charts (Chart.js or similar) fed by SSE.
- History view: time range picker → charts + correlated process list, fed by Postgres via REST.

## 6. Success Criteria (v1 "done")
- Agent runs locally, samples reliably for hours without crashing or leaking memory.
- Live dashboard updates smoothly with <1s perceived latency.
- Can pick an arbitrary past spike and get a correct "here's what was running" answer.
- Deployed/runnable end-to-end (agent + backend + dashboard) with a short demo GIF/video for the resume.

## 7. Risks / Open Questions
- Cross-platform support (Linux `/proc` vs Windows/Mac APIs) — may scope to Linux-only for v1 via `sysinfo` abstraction.
- Agent crash mid-buffer loses up to 60s of unsent samples — acceptable for v1, noted as a known limitation.
- Correlation accuracy depends on sampling granularity (3s may miss very short spikes) — acceptable trade-off, documented.
