use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_kb: u64,
}

#[derive(Deserialize)]
pub struct BatchItem {
    pub timestamp: i64,
    pub global_cpu_pct: f32,
    pub total_mem_kb: i64,
    pub used_mem_kb: i64,
    pub disk_read_bytes: i64,
    pub disk_write_bytes: i64,
    #[serde(default)]
    pub processes: Vec<ProcessItem>,
}

/// Single tick broadcast over SSE. In-memory only, never persisted.
/// Same shape as one `BatchItem`, plus `Clone` for `broadcast::Sender`.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LiveSample {
    pub timestamp: i64,
    pub global_cpu_pct: f32,
    pub total_mem_kb: i64,
    pub used_mem_kb: i64,
    pub disk_read_bytes: i64,
    pub disk_write_bytes: i64,
    #[serde(default)]
    pub processes: Vec<ProcessItem>,
}
