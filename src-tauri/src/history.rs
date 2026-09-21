//! Machine history (Supporter): one line per scan in `history.jsonl` next to
//! settings.json — timestamp, health, counts, and the summary numbers that
//! drift over time (battery health, free space on root). Small on purpose:
//! a year of daily scans is ~50 KB.

use innards_core::report::Report;
use innards_core::Severity;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub at: String,
    pub health: u8,
    pub critical: u32,
    pub warning: u32,
    pub battery_health: Option<f32>,
    pub root_free_pct: Option<f32>,
    pub memory_available_pct: Option<f32>,
    pub cpu_c: Option<f32>,
}

fn path() -> PathBuf {
    crate::settings::path().with_file_name("history.jsonl")
}

pub fn record(rep: &Report) {
    let s = &rep.snapshot;
    let e = Entry {
        at: s.taken_at.clone(),
        health: rep.summary.health_score,
        critical: rep.findings.iter().filter(|f| f.severity == Severity::Critical).count() as u32,
        warning: rep.findings.iter().filter(|f| f.severity == Severity::Warning).count() as u32,
        battery_health: s.battery.as_ref().and_then(|b| b.health_pct),
        root_free_pct: s.root_fs().filter(|f| f.total_bytes > 0).map(|f| f.available_bytes as f32 * 100.0 / f.total_bytes as f32),
        memory_available_pct: (s.memory.total_bytes > 0).then(|| s.memory.available_bytes as f32 * 100.0 / s.memory.total_bytes as f32),
        cpu_c: s.thermal.cpu_c,
    };
    if let Ok(line) = serde_json::to_string(&e) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path()) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Newest last. Capped to the most recent `limit` entries.
pub fn load(limit: usize) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path()) else { return vec![] };
    let mut v: Vec<Entry> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    if v.len() > limit {
        v.drain(..v.len() - limit);
    }
    v
}

pub fn clear() -> Result<(), String> {
    match std::fs::remove_file(path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
