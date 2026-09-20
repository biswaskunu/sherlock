use serde::Serialize;
use std::collections::VecDeque;
use sysinfo::System;
use tokio::time::{interval, Duration};

const POLL_INTERVAL_SECS: u64 = 3;
const FLUSH_EVERY_N_SAMPLES: usize = 20; // 20 * 3s = 60s

#[derive(Serialize, Clone)]
struct ProcessMetric {
    pid: u32,
    name: String,
    cpu_pct: f32,
    mem_kb: u64,
}

#[derive(Serialize, Clone)]
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

    processes.sort_by(|a, b| b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap());
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

fn flush(buffer: &mut VecDeque<Metric>) {
    // Phase 2 replaces this print with a POST to /api/metrics/batch.
    let batch: Vec<&Metric> = buffer.iter().collect();
    match serde_json::to_string(&batch) {
        Ok(json) => println!("FLUSH ({} samples): {}", batch.len(), json),
        Err(e) => eprintln!("serialize error on flush: {}", e),
    }
    buffer.clear();
}

#[tokio::main]
async fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut buffer: VecDeque<Metric> = VecDeque::with_capacity(FLUSH_EVERY_N_SAMPLES);
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));

    loop {
        ticker.tick().await;

        let metric = sample(&mut sys);
        println!(
            "tick: cpu={:.1}% mem={}/{}kb",
            metric.global_cpu_pct, metric.used_mem_kb, metric.total_mem_kb
        );
        buffer.push_back(metric);

        if buffer.len() >= FLUSH_EVERY_N_SAMPLES {
            flush(&mut buffer);
        }
    }
}
