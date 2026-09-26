# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Primary: solo developer debugging their own Linux machine ("why is it slow right now / what spiked an hour ago"). Secondary, equal weight: portfolio reviewers evaluating a live demo of the same flow. Single local user, no auth, no multi-user.

## Product Purpose

Sherlock samples system + per-process metrics every 3s, stores 60s batches in Postgres via Axum, streams live ticks over SSE, and answers "what caused the spike at time X" via timestamp correlation. Success for this surface: find the culprit process in seconds with a calm daily-driver feel — clean tool first, demo polish second.

## Positioning

Unlike `top`/Task Manager (current state only), Sherlock keeps a queryable history and joins system spikes to the exact processes running at that timestamp — live view + history view + click-spike-to-correlate in one local dashboard.

## Operating Context

Local-only stack on the monitored machine: Rust agent → Axum backend (`:8080`) → Postgres (`:5432`) → Vite dashboard (`:5173`, proxies `/api`). Rituals: watch live charts during heavy work, deliberately spike CPU, later pick a range in History and click the peak to name culprits. Backend URLs via `VITE_BACKEND_URL` / `BACKEND_URL`; retention `RETENTION_HOURS=48`.

## Capabilities and Constraints

Confirmed functionality (must preserve): Live SSE charts (CPU %, used mem) + top-processes table with connected/stalled/disconnected states; History range fetch (`from`/`to`/`limit=500`) with loading/empty/error/stale guards; click-point correlate (`timestamp`/`top_n=10`, ±5s fallback, 404 beyond tolerance); Live/History tab toggle.

Constraints: local-only agent by design (never deployed); 3s sampling may miss sub-3s spikes; agent crash loses ≤60s; Linux-first via sysinfo; net_rx/net_tx reserved but unpopulated; no alerting, fleet, or auth in v1. Stack for this surface: Vite + Chart.js (keep).

## Brand Commitments

Name: sherlock ("Why Is This Slow"). No confirmed palette, type, logo, or voice beyond the existing dark developer-console dashboard — redesign replaces the visual world. No testimonials, benchmarks, or deployment claims to preserve.

## Evidence on Hand

Real backend endpoints (`api-spec.md`): `POST /api/metrics/batch`, `POST /api/metrics/live/publish`, `GET /api/metrics/live` (SSE), `GET /api/metrics/history`, `GET /api/correlate`. Real demo asset: `docs/demo.gif` (live → spike → history → correlate, 41.7% peak). No fabricated users, pricing, or claims.

## Product Principles

1. Answer first: every spike resolves to named processes, not just a chart shape.
2. Live and history are one workflow, not two tools — switching tabs never loses context.
3. Honest states over silent gaps: stalled, empty, and error states say what to do next.
4. Local and legible: runs on localhost, reads instantly, never pretends to be fleet SaaS.
