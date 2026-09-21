//! Supporter feature: turn the structured report into a short narrative in
//! the user's language and at their level, via the Claude API. Only the
//! rendered findings and summary are sent — never serial numbers, hostnames,
//! process lists, or raw probe output.

use innards_core::report::{RenderedRecommendation, RenderedReport};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const MODEL: &str = "claude-opus-5";
const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Narrative {
    pub text: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

fn language_name(code: &str) -> String {
    innards_core::i18n::available().into_iter().find(|(c, _)| c == code).map(|(_, n)| n).unwrap_or_else(|| code.to_string())
}

fn scrub(report: &RenderedReport) -> Value {
    let mut v = scrub_inner(report);
    innards_core::report::redact_paths(&mut v);
    v
}

fn scrub_inner(report: &RenderedReport) -> Value {
    json!({
        "machine": report.summary.machine,
        "cpu": report.summary.cpu,
        "memory": report.summary.memory,
        "storage": report.summary.storage,
        "gpu": report.summary.gpu,
        "os": report.summary.os,
        "battery_health": report.summary.battery,
        "health_score": report.summary.health_score,
        "verdict": report.verdict,
        "findings": report.findings.iter().map(|f| json!({
            "severity": f.severity, "category": f.category, "title": f.title, "detail": f.body, "action": f.action
        })).collect::<Vec<_>>(),
        "capabilities": report.capabilities.iter().map(|c| json!({
            "workload": c.label, "grade": c.grade_label, "score": c.score, "limits": c.limits
        })).collect::<Vec<_>>(),
    })
}

pub async fn narrate(api_key: &str, report: &RenderedReport, recs: Option<&[RenderedRecommendation]>) -> Result<Narrative, String> {
    let level_desc = match report.level {
        innards_core::Level::Plain => "a non-technical person; no jargon, explain any term you must use",
        innards_core::Level::Informed => "a comfortable computer user; light technical vocabulary is fine",
        innards_core::Level::Expert => "a systems engineer; be precise and technical",
    };
    let lang = language_name(&report.lang);
    let system = format!(
        "You are Innards, a desktop app that inspects a computer and explains what it found. \
         Write in {lang}. The reader is {level_desc}. \
         Write a short narrative (150–300 words) in bullet points grouped under two or three bold headings: \
         what matters most, what the machine is still good for, and (if recommendations are supplied) what to do about it. \
         Be concrete and honest; do not invent facts not present in the data; do not repeat every finding — prioritize. \
         Never mention that you are an AI. Output Markdown only."
    );
    let mut user = format!("Report data (JSON):\n{}", serde_json::to_string_pretty(&scrub(report)).unwrap());
    if let Some(recs) = recs {
        let r: Vec<Value> = recs.iter().map(|r| json!({"title": r.title, "kind": r.kind, "impact": r.impact, "cost": r.cost, "why": r.why, "over_budget": r.over_budget})).collect();
        user.push_str(&format!("\n\nUpgrade recommendations (JSON):\n{}", serde_json::to_string_pretty(&r).unwrap()));
    }

    let body = json!({
        "model": MODEL,
        "max_tokens": 4000,
        "fallbacks": "default",
        "output_config": { "effort": "medium" },
        "system": system,
        "messages": [{ "role": "user", "content": user }],
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(ENDPOINT)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "server-side-fallback-2026-07-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;

    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| format!("bad response: {e}"))?;
    if !status.is_success() {
        let msg = v.pointer("/error/message").and_then(Value::as_str).unwrap_or("unknown error");
        return Err(format!("API {status}: {msg}"));
    }
    if v.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
        return Err("The model declined to write this narrative.".into());
    }
    let text = v
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| blocks.iter().filter(|b| b.get("type").and_then(Value::as_str) == Some("text")).filter_map(|b| b.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    Ok(Narrative {
        text,
        model: v.get("model").and_then(Value::as_str).unwrap_or(MODEL).to_string(),
        input_tokens: v.pointer("/usage/input_tokens").and_then(Value::as_u64).unwrap_or(0),
        output_tokens: v.pointer("/usage/output_tokens").and_then(Value::as_u64).unwrap_or(0),
    })
}
