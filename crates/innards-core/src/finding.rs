//! A `Finding` is one thing worth telling the user. It carries no prose —
//! only an id, severity, and parameters. Text comes from the i18n catalogs
//! so every finding can be rendered at any level in any language.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something is wrong or at risk; act soon.
    Critical,
    /// Costs performance or reliability; worth fixing.
    Warning,
    /// Useful to know, no action required.
    Info,
    /// Something that is fine and worth saying so.
    Good,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Memory,
    Storage,
    Cpu,
    Thermal,
    Battery,
    Gpu,
    Network,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Stable identifier, also the i18n key: e.g. `memory.no_swap`.
    pub id: String,
    pub severity: Severity,
    pub category: Category,
    /// Values substituted into the message templates as `{name}`.
    pub params: Map<String, Value>,
    /// Raw evidence lines for the expert level (already formatted, English-only).
    pub evidence: Vec<String>,
}

impl Finding {
    pub fn new(id: &str, severity: Severity, category: Category) -> Self {
        Self {
            id: id.to_string(),
            severity,
            category,
            params: Map::new(),
            evidence: Vec::new(),
        }
    }

    pub fn param(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }

    pub fn evidence(mut self, line: impl Into<String>) -> Self {
        self.evidence.push(line.into());
        self
    }
}

/// Human-friendly byte formatting shared by rules and renderers.
pub fn fmt_bytes(b: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", b, UNITS[i])
    } else if v >= 100.0 {
        format!("{:.0} {}", v, UNITS[i])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

/// RAM in whole marketing gigabytes: the kernel reserves some, so 14.9 GiB
/// reported is a 16 GB machine.
pub fn ram_marketing_gb(bytes: u64) -> u64 {
    let gib = bytes as f64 / (1u64 << 30) as f64;
    for s in [1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0, 192.0, 256.0, 512.0] {
        if gib <= s && gib >= s * 0.88 {
            return s as u64;
        }
    }
    gib.round() as u64
}
