# sherlock — "Why Is This Slow"

Local Rust observability agent: samples system metrics every 3s, streams live via SSE, batches 60s windows into Postgres, and correlates spikes to processes.

## Stack

| Layer | Tech |
|-------|------|
| Agent | Rust + tokio + sysinfo |
| Backend | Axum + SQLx + Postgres |
| Dashboard | SSE + Chart.js (planned) |
| Infra | Docker + Railway (backend) |

## Current Phase Status

| Phase | Status | What's done |
|-------|--------|-------------|
| 0 — Groundwork | ✅ | sysinfo polling, /proc understanding |
| 1 — Agent sampling | ✅ | Metric struct, ring buffer, 60s flush to stdout |
| 2 — Backend ingestion | 🔲 | Axum + Postgres batch POST (next) |
| 3 — Live path | 🔲 | SSE streaming + dashboard |
| 4 — History + correlation | 🔲 | Time-range queries + spike→process join |
| 5 — Polish + demo | 🔲 | Retention, README finish, demo GIF |

## Quick Start (Agent Only)

```bash
cd ~/Projects/sherlock
cargo build
cargo run
# Watch CPU/mem scroll every 3s; 60s flushes print JSON to stdout
```

## Running Full Stack (Phase 2+)

```bash
# 1. Start Postgres
docker compose up -d postgres

# 2. Apply schema
psql $DATABASE_URL -f schema.sql

# 3. Start backend (Phase 2)
cd backend && cargo run

# 4. Start agent (flush POSTs to backend instead of stdout)
cd agent && cargo run
```

## Project Structure

```
sherlock/
├── agent/              # Rust sampling binary
│   └── src/main.rs     # poll loop, buffering, flush
├── backend/            # Axum server (Phase 2+)
│   └── src/
│       ├── handlers/   # batch POST, history, correlate, SSE
│       ├── models/      # sample + process_sample structs
│       └── db/         # pool, queries
├── schema.sql          # Postgres migrations
├── docker-compose.yml  # local Postgres
├── .env.example        # env template
├── api-spec.md         # endpoint schemas
├── ARCHITECTURE.md     # design decisions
├── PHASES.md           # phase plan + exit criteria
└── PRD.md              # problem statement + scope
```

## Known Limitations (v1)

- Agent crash loses ≤60s of unsent samples (ring buffer is in-memory)
- 3s sampling may miss sub-3s spikes (documented trade-off)
- Linux-first via sysinfo; Windows/Mac not guaranteed
- No network stats collected yet (net_rx/net_tx reserved in schema)
- No alerting, no multi-user, no fleet monitoring

## Notes for AI Handoff

- Agent flush currently prints JSON to stdout — Phase 2 swaps to POST `/api/metrics/batch`
- `ARCHITECTURE.md` describes target architecture; current state is Phase 1 only
- `PHASES.md` has the canonical phase order + exit criteria — follow that for sequencing