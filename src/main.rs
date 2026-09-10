use serde::Serialize;
use sysinfo::System;
use tokio::time::{interval, Duration};

#[derive(Serialize)]
struct Snapshot {
    timestamp: u64,
    global_cpu_pct: f32,
    total_mem_kb: u64,
    used_mem_kb: u64,
    processes: Vec<ProcSnapshot>,
}

#[derive(Serialize)]
struct ProcSnapshot {
    pid: u32,
    name: String,
    cpu_pct: f32,
    mem_kb: u64,
}

#[tokio::main]
async fn main() {
    // sysinfo needs an initial refresh before CPU % numbers are meaningful,
    // and repeated refreshes to compute deltas between ticks.
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut ticker = interval(Duration::from_secs(5));

    loop {
        ticker.tick().await;

        // Refresh again so CPU usage reflects the interval since last refresh.
        sys.refresh_cpu_all();
        sys.refresh_memory();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let global_cpu_pct = sys.global_cpu_usage();
        let total_mem_kb = sys.total_memory();
        let used_mem_kb = sys.used_memory();

        // Top 5 processes by CPU, just to eyeball correlation potential early.
        let mut procs: Vec<ProcSnapshot> = sys
            .processes()
            .values()
            .map(|p| ProcSnapshot {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu_pct: p.cpu_usage(),
                mem_kb: p.memory(),
            })
            .collect();
        procs.sort_by(|a, b| b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap());
        procs.truncate(5);

        let snapshot = Snapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            global_cpu_pct,
            total_mem_kb,
            used_mem_kb,
            processes: procs,
        };

        match serde_json::to_string(&snapshot) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("serialize error: {}", e),
        }
    }
}
