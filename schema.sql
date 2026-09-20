-- sherlock: Postgres schema v1 (reference copy — canonical source is
-- backend/migrations/0001_v1_samples.sql, applied via `sqlx migrate run`)
-- Legacy apply: psql $DATABASE_URL -f schema.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- System-level samples, one row per 3s tick
CREATE TABLE samples (
    timestamp     BIGINT PRIMARY KEY,
    cpu_pct       REAL NOT NULL,
    total_mem_kb  BIGINT NOT NULL,
    used_mem_kb   BIGINT NOT NULL,
    disk_read_bytes BIGINT NOT NULL DEFAULT 0,
    disk_write_bytes BIGINT NOT NULL DEFAULT 0,
    net_rx_bytes  BIGINT NOT NULL DEFAULT 0,  -- reserved, not collected by agent yet
    net_tx_bytes  BIGINT NOT NULL DEFAULT 0   -- reserved, not collected by agent yet
);

-- Per-process samples, linked to samples by timestamp
CREATE TABLE process_samples (
    id            SERIAL PRIMARY KEY,
    timestamp     BIGINT NOT NULL REFERENCES samples(timestamp) ON DELETE CASCADE,
    pid           INTEGER NOT NULL,
    process_name  TEXT NOT NULL,
    cpu_pct       REAL NOT NULL,
    mem_kb        BIGINT NOT NULL
);

-- Indexes for time-range queries and correlation joins
CREATE INDEX idx_samples_timestamp ON samples(timestamp);
CREATE INDEX idx_process_samples_timestamp ON process_samples(timestamp);
CREATE INDEX idx_process_samples_pid ON process_samples(pid);

-- Optional: retention — drop raw data older than 48h (run via cron/pg_cron)
-- DELETE FROM samples WHERE timestamp < EXTRACT(EPOCH FROM NOW() - INTERVAL '48 hours')::BIGINT;
-- DELETE FROM process_samples WHERE timestamp < EXTRACT(EPOCH FROM NOW() - INTERVAL '48 hours')::BIGINT;
