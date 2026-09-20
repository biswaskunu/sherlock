use serde::Serialize;
use std::collections::VecDeque;
use sysinfo::System;
use tokio::time::{interval, Duration};

const POLL_INTERVAL_SECS: u64 = 3;
const FLUSH_EVERY_N_SAMPLES: usize = 20; // 20 * 3s = 60s

#[derive(Serialize)]
struct ProcessMetric {
    pid: u32,
    name: String,
    cpu_pct: f32,
    mem_kb: u64,
}

#[derive(Serialize)]
struct Metric {
    timestamp: u64,
    global_cpu_pct: f32,
    total_mem_kb: u64,
    used_mem_kb: u64,
    disk_read_bytes: u64,
    disk_write_bytes: u64,
    processes: Vec<ProcessMetric>,
}

fn sample(sys: &mut System) -> Metric {
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // Sum per-process disk usage as a stand-in for system-wide disk I/O.
    // sysinfo doesn't expose global disk throughput directly; this is an
    // approximation flagged for revisit if it proves inaccurate later.
    let (mut disk_read_bytes, mut disk_write_bytes) = (0u64, 0u64);
    let mut processes: Vec<ProcessMetric> = Vec::new();

    for p in sys.processes().values() {
        let du = p.disk_usage();
        disk_read_bytes += du.read_bytes;
        disk_write_bytes += du.written_bytes;

        processes.push(ProcessMetric {
            pid: p.pid().as_u32(),
            name: p.name().to_string_lossy().to_string(),
            cpu_pct: p.cpu_usage(),
            mem_kb: p.memory(),
        });
    }

    processes.sort_by(|a, b| {
        b.cpu_pct
            .partial_cmp(&a.cpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    processes.truncate(10); // keep top 10 by CPU per sample, not every process

    Metric {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        global_cpu_pct: sys.global_cpu_usage(),
        total_mem_kb: sys.total_memory(),
        used_mem_kb: sys.used_memory(),
        disk_read_bytes,
        disk_write_bytes,
        processes,
    }
}

fn backend_url() -> String {
    if let Ok(url) = std::env::var("BACKEND_URL") {
        return url;
    }
    let host = std::env::var("BACKEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("BACKEND_PORT").unwrap_or_else(|_| "8080".to_string());
    format!("http://{host}:{port}/api/metrics/batch")
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn flush(client: &reqwest::Client, url: &str, buffer: &mut VecDeque<Metric>, max_buffered: usize) {
    let batch: Vec<&Metric> = buffer.iter().collect();
    match client.post(url).json(&batch).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("FLUSH ({} samples): posted to {url}", batch.len());
            buffer.clear();
        }
        Ok(resp) => {
            // Keep buffer for retry on next flush; cap growth while backend is unhappy.
            eprintln!("flush rejected ({}), keeping {} samples for retry", resp.status(), buffer.len());
            drop_oldest_over(buffer, max_buffered);
        }
        Err(e) => {
            eprintln!("flush POST failed ({e}), keeping {} samples for retry", buffer.len());
            drop_oldest_over(buffer, max_buffered);
        }
    }
}

/// Bound memory while the backend is unreachable: drop oldest samples past the cap.
fn drop_oldest_over(buffer: &mut VecDeque<Metric>, max_buffered: usize) {
    while buffer.len() > max_buffered {
        buffer.pop_front();
    }
}

#[tokio::main]
async fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Clamp env config: 0 poll interval panics `tokio::time::interval`,
    // 0 flush size would POST every tick and defeat batching, and absurd
    // values would preallocate huge buffers.
    let poll_secs =
        env_usize("AGENT_POLL_INTERVAL_SECS", POLL_INTERVAL_SECS as usize).clamp(1, 3600) as u64;
    let flush_every =
        env_usize("AGENT_FLUSH_EVERY_N_SAMPLES", FLUSH_EVERY_N_SAMPLES).clamp(1, 10_000);
    // Cap retained samples at 3 flush windows so a dead backend can't OOM the agent.
    let max_buffered = flush_every.saturating_mul(3);
    let url = backend_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client");
    println!("agent posting to {url} every {flush_every} samples ({poll_secs}s poll)");

    let mut buffer: VecDeque<Metric> = VecDeque::with_capacity(flush_every);
    let mut ticker = interval(Duration::from_secs(poll_secs));
    // A slow flush (up to the 10s HTTP timeout) must not cause a burst of
    // catch-up ticks that skews the 3s sampling cadence.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        let metric = sample(&mut sys);
        println!(
            "tick: cpu={:.1}% mem={}/{}kb",
            metric.global_cpu_pct, metric.used_mem_kb, metric.total_mem_kb
        );
        buffer.push_back(metric);

        if buffer.len() >= flush_every {
            flush(&client, &url, &mut buffer, max_buffered).await;
        }
    }
}
