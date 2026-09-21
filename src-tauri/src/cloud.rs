//! Team / Enterprise: upload the structured report to a Threadwise workspace
//! running the `innards.fleet` plugin (see integrations/threadwise-fleet).
//! The payload is scrubbed before it leaves: no serial numbers, hostname,
//! or process list. The machine id is a stable hash so the dashboard can
//! track a machine across reports without learning anything identifying.

use innards_core::report::{Report, RenderedReport};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const INGEST_PATH: &str = "/api/ext/innards.fleet/reports";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub machine_id: String,
    pub report_id: String,
    pub dashboard_url: Option<String>,
}

/// Stable, non-reversible machine id: hash of vendor + product + the system
/// disk's serial (if known) + MAC of the first physical interface. Never sent
/// raw; only the hash leaves the machine.
pub fn machine_id(report: &Report) -> String {
    let s = &report.snapshot;
    let mut h = DefaultHasher::new();
    s.system.vendor.hash(&mut h);
    s.system.product.hash(&mut h);
    s.system_disk().and_then(|d| d.serial.clone()).hash(&mut h);
    s.network.iter().find_map(|n| n.mac.clone()).hash(&mut h);
    format!("m_{:016x}", h.finish())
}

fn scrub(report: &Report) -> Value {
    let mut v = serde_json::to_value(report).unwrap_or(Value::Null);
    innards_core::report::redact(&mut v);
    v
}

pub async fn upload(endpoint: &str, token: &str, label: Option<&str>, report: &Report, rendered: &RenderedReport) -> Result<UploadResult, String> {
    let base = endpoint.trim_end_matches('/');
    let url = format!("{base}{INGEST_PATH}");
    let s = &report.snapshot;
    let mut rendered_v = serde_json::to_value(rendered).unwrap_or(Value::Null);
    innards_core::report::redact_paths(&mut rendered_v);
    let body = json!({
        "machine": {
            "id": machine_id(report),
            "label": label,
            "vendor": s.system.vendor,
            "product": s.system.product,
            "chassis": s.system.chassis,
            "os": rendered.summary.os,
            "cpu": rendered.summary.cpu,
            "ram_gb": innards_core::finding::ram_marketing_gb(s.memory.total_bytes),
            "storage": rendered.summary.storage,
            "gpu": rendered.summary.gpu,
        },
        "innards_version": env!("CARGO_PKG_VERSION"),
        "report": scrub(report),
        "rendered": rendered_v,
    });

    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(token)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;
    let status = resp.status();
    let v: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = v.pointer("/error/message").and_then(Value::as_str).unwrap_or("upload failed");
        return Err(format!("{status}: {msg}"));
    }
    Ok(UploadResult {
        machine_id: v.get("machineId").and_then(Value::as_str).unwrap_or_default().to_string(),
        report_id: v.get("reportId").and_then(Value::as_str).unwrap_or_default().to_string(),
        dashboard_url: v.get("dashboardUrl").and_then(Value::as_str).map(String::from),
    })
}
