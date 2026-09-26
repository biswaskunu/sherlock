# API Spec: sherlock Backend

Base URL: `http://localhost:8080`

## POST /api/metrics/batch

Ingest a batch of ~20 samples (60s window) from the agent.

**Request:**
```json
[
  {
    "timestamp": 1718900000,
    "global_cpu_pct": 23.5,
    "total_mem_kb": 16384000,
    "used_mem_kb": 8192000,
    "disk_read_bytes": 1048576,
    "disk_write_bytes": 524288,
    "processes": [
      { "pid": 1234, "name": "code", "cpu_pct": 12.3, "mem_kb": 524288 },
      { "pid": 5678, "name": "firefox", "cpu_pct": 8.1, "mem_kb": 409600 }
    ]
  }
]
```

**Response (200):**
```json
{ "inserted": 12 }
```

**Response (400):**
```json
{ "error": "empty batch" }
```

## POST /api/metrics/live/publish

Agent pushes one tick every ~3s. Broadcast in-memory only, never persisted.

**Request:** single `LiveSample` (same shape as one batch item, incl. top-10 `processes`).

**Response (202):** `{ "ok": true }` (also 200-OK shape when no dashboards connected — live is lossy).

## GET /api/metrics/live (SSE)

Streams live samples as they're published by the agent (backend-mediated fan-out,
`broadcast::channel(32)`, lagged ticks skipped, `keep-alive` every 15s).

```
Content-Type: text/event-stream

data: {"timestamp":1718900000,"global_cpu_pct":23.5,...,"processes":[...]}
data: {"timestamp":1718900003,"global_cpu_pct":24.1,...,"processes":[...]}
```

## GET /api/metrics/history

Query historical samples for a time range.

**Query params:**
- `from` (required): Unix timestamp seconds
- `to` (required): Unix timestamp seconds
- `limit` (optional): max samples back, default 500

**Response (200):**
```json
{
  "samples": [
    { "timestamp": 1718900000, "global_cpu_pct": 23.5, "used_mem_kb": 8192000, "disk_read_bytes": 1048576, "disk_write_bytes": 524288 }
  ]
}
```

## GET /api/correlate

Given a timestamp, return the system sample + top processes at that moment.

**Query params:**
- `timestamp` (required): Unix timestamp seconds
- `top_n` (optional): number of processes to return, default 10

**Response (200):**
```json
{
  "sample": { "timestamp": 1718900000, "global_cpu_pct": 95.2, "used_mem_kb": 15000000, "disk_read_bytes": 5000000, "disk_write_bytes": 2000000 },
  "top_processes": [
    { "pid": 1234, "name": "code", "cpu_pct": 45.0, "mem_kb": 1048576 },
    { "pid": 5678, "name": "firefox", "cpu_pct": 30.2, "mem_kb": 819200 }
  ]
}
```

**Response (404):**
```json
{ "error": "no sample found for timestamp" }
```
