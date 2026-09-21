//! `cargo run -p innards-core --example fixtures -- <out_dir>`
//! Writes JSON fixtures of this machine's rendered report, catalogs, advisor
//! questions and sample recommendations. The web frontend uses them as a
//! mock backend when running outside Tauri (design mode), and they double
//! as documentation of the IPC payloads.

use innards_core::advisor::{self, Answers, Budget, Horizon, Pain};
use innards_core::capability::Workload;
use innards_core::{i18n, probe, report, Level};
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "fixtures".into()));
    fs::create_dir_all(&out).unwrap();
    let rep = report::build(probe::collect());

    for (lang, _, src) in i18n::LANGS {
        for level in [Level::Plain, Level::Informed, Level::Expert] {
            let r = report::render(&rep, lang, level);
            fs::write(out.join(format!("report-{lang}-{}.json", level.key())), serde_json::to_string_pretty(&r).unwrap()).unwrap();
        }
        let v: serde_json::Value = serde_json::from_str(src).unwrap();
        let cat = serde_json::json!({
            "ui": v.get("ui"), "severities": v.get("severities"), "categories": v.get("categories"),
            "grades": v.get("grades"), "workloads": v.get("workloads"), "advisor": v.get("advisor"),
            "rec_kinds": v.get("rec_kinds"), "impacts": v.get("impacts"),
        });
        fs::write(out.join(format!("catalog-{lang}.json")), serde_json::to_string_pretty(&cat).unwrap()).unwrap();

        let answers = Answers {
            uses: vec![Workload::WebDev, Workload::Containers, Workload::HomeServer, Workload::LocalLlm],
            pains: vec![Pain::Slow, Pain::OutOfMemory],
            budget: Budget::Under800,
            horizon: Horizon::SixMonths,
            needs_portability: true,
        };
        let recs = advisor::recommend(&rep.snapshot, &answers);
        fs::write(out.join(format!("recs-{lang}.json")), serde_json::to_string_pretty(&report::render_recs(&recs, lang)).unwrap()).unwrap();
    }
    fs::write(out.join("questions.json"), serde_json::to_string_pretty(&advisor::questions()).unwrap()).unwrap();
    fs::write(out.join("languages.json"), serde_json::to_string_pretty(&i18n::available()).unwrap()).unwrap();
    // The full structured report, with serials redacted, as IPC documentation.
    let mut full = serde_json::to_value(&rep).unwrap();
    if let Some(devs) = full.pointer_mut("/snapshot/storage").and_then(|v| v.as_array_mut()) {
        for d in devs {
            d["serial"] = serde_json::Value::Null;
        }
    }
    full["snapshot"]["system"]["hostname"] = serde_json::Value::Null;
    fs::write(out.join("report-full.json"), serde_json::to_string_pretty(&full).unwrap()).unwrap();
    eprintln!("wrote fixtures to {}", out.display());
}
