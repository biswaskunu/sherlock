# Phases: "sherlock" System Monitor

## Phase 0 — Learning / Groundwork
- Get comfortable with the `sysinfo` crate: read CPU, memory, per-process stats, print to console.
- Skim `/proc` on your own machine (`/proc/stat`, `/proc/meminfo`, `/proc/[pid]/stat`) so you understand what `sysinfo` is abstracting.
- Write a throwaway script: poll every 3s with `tokio::time::interval`, print a JSON blob to stdout. No storage, no server yet.
- **Exit criteria:** you can watch live CPU/mem numbers scroll in your terminal for 3+ minutes without crashing.

## Phase 1 — Agent: Sampling + Buffering
- Build the real `Metric` struct (system-level + per-process).
- Implement the 3s polling loop with an in-memory ring buffer.
- Implement the 60s flush → for now, just print the batch instead of POSTing it.
- **Exit criteria:** agent runs continuously, buffers correctly, flushes every 60s with the right sample count (20).

## Phase 2 — Backend: Storage Path ✅
- Set up Axum + Postgres, `handlers/metrics.rs`.
- `samples` + `process_samples` tables + migrations.
- `POST /api/metrics/batch` — accept and insert a batch.
- Point the agent's flush at this endpoint instead of printing.
- **Exit criteria:** run the agent for 10+ minutes, confirm rows landing correctly in Postgres, batched (not per-sample).
- **Status:** complete — 10-min E2E run stored 200 samples + 2000 process_samples, 3s spacing, ~1 POST/min, zero agent errors.

## Phase 3 — Live Path ✅
- SSE endpoint: **backend-mediated** (agent → Axum → dashboard), not agent-direct (see ARCHITECTURE §2.1/§4).
- Minimal dashboard page: connect to SSE, render live-updating chart.
- **Exit criteria:** open dashboard, see numbers update in near real time during CPU-heavy workload.
- **Status:** complete — agent publishes each tick to `POST /api/metrics/live/publish`, Axum fans out via `GET /api/metrics/live` SSE to Vite + Chart.js dashboard with top-process table.

## Phase 4 — History + Correlation ✅
- `GET /api/metrics/history?from=&to=` — query Postgres, return chartable data.
- `GET /api/correlate?timestamp=` — join `samples` + `process_samples`, return top processes at that time.
- Dashboard: history view with time range picker, click-a-spike-to-correlate interaction.
- **Exit criteria:** deliberately spike CPU (e.g. run a heavy build), find it later in the history view, correctly identify the culprit process.
- **Status:** complete — history range query + correlate (exact with nearest ±5s fallback) wired in Axum; dashboard History tab with range picker, Chart.js render, click-point-to-correlate table.

## Phase 5 — Polish for Resume/Demo
- Basic retention policy (drop/downsample data older than N hours).
- README with architecture diagram + setup instructions.
- Record a short demo GIF/video showing: live view → deliberate spike → history view → correlation.
- Deploy backend (Railway) if you want a live demo link; agent stays local (it's inherently a local tool) — document that clearly in the README so it's not read as a missing deployment.

## Scope Discipline Notes
- Don't add Windows/Mac support until Linux v1 is fully working end-to-end.
- Don't add network-level packet inspection — interface-level stats only, as scoped in the PRD.
- If Phase 3 (live path) turns out harder than expected, it's fine to ship Phase 2 + Phase 4 first and treat live view as a stretch goal — history + correlation is the more resume-differentiating half anyway.
