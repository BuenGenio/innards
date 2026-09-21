//! `Report` is the structured result; `render` turns it into text for a
//! given language and level. Frontends should render from `RenderedReport`
//! and never assemble prose themselves.

use crate::advisor::Recommendation;
use crate::capability::{Capability, Grade};
use crate::finding::{fmt_bytes, Finding, Severity};
use crate::i18n::{self, Level};
use crate::snapshot::Snapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub snapshot: Snapshot,
    pub findings: Vec<Finding>,
    pub capabilities: Vec<Capability>,
    pub summary: Summary,
}

/// The at-a-glance header: what this machine is.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Summary {
    pub machine: String,
    pub cpu: String,
    pub memory: String,
    pub storage: String,
    pub gpu: String,
    pub os: String,
    pub battery: Option<String>,
    /// 0–100: weighted by severity of findings. Purely for the big number.
    pub health_score: u8,
}

/// RAM as the marketing size ("16 GB"): the kernel reserves a little, so
/// 14.9 GiB reported means a 16 GB machine.
pub fn fmt_ram(bytes: u64) -> String {
    format!("{} GB", crate::finding::ram_marketing_gb(bytes))
}

pub fn build(snapshot: Snapshot) -> Report {
    let findings = crate::rules::run(&snapshot);
    let capabilities = crate::capability::score(&snapshot);
    let summary = summarize(&snapshot, &findings);
    Report { snapshot, findings, capabilities, summary }
}

fn summarize(s: &Snapshot, findings: &[Finding]) -> Summary {
    let machine = match (&s.system.vendor, &s.system.product) {
        (Some(v), Some(p)) if !p.starts_with(v) => format!("{v} {p}"),
        (_, Some(p)) => p.clone(),
        (Some(v), None) => v.clone(),
        // Never the hostname: the summary travels (narration, cloud); the hostname must not.
        _ => "This machine".into(),
    };
    let cpu = format!(
        "{} · {}C/{}T",
        s.cpu.brand,
        s.cpu.physical_cores.unwrap_or(s.cpu.logical_cpus / 2),
        s.cpu.logical_cpus
    );
    let memory = match &s.memory.kind {
        Some(k) => format!("{} {}", fmt_ram(s.memory.total_bytes), k),
        None => fmt_ram(s.memory.total_bytes),
    };
    let storage = s
        .system_disk()
        .map(|d| format!("{} {}", fmt_bytes(d.size_bytes), match d.kind { crate::snapshot::StorageKind::Nvme => "NVMe", crate::snapshot::StorageKind::Ssd => "SSD", crate::snapshot::StorageKind::Hdd => "HDD", crate::snapshot::StorageKind::Ufs => "UFS", crate::snapshot::StorageKind::Emmc => "eMMC", _ => "" }))
        .unwrap_or_default();
    let gpu = s.gpus.iter().find(|g| g.is_discrete).or(s.gpus.first()).map(|g| g.name.clone()).unwrap_or_default();
    let os = format!("{} {}", s.system.os_name.clone().unwrap_or_default(), s.system.os_version.clone().unwrap_or_default()).trim().to_string();
    let battery = s.battery.as_ref().filter(|b| b.present).and_then(|b| b.health_pct).map(|h| format!("{h:.0}%"));

    let mut score: i32 = 100;
    for f in findings {
        score -= match f.severity {
            Severity::Critical => 12,
            Severity::Warning => 5,
            Severity::Info => 0,
            Severity::Good => -1,
        };
    }
    Summary { machine, cpu, memory, storage, gpu, os, battery, health_score: score.clamp(0, 100) as u8 }
}

// ---------------------------------------------------------------- rendering

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedFinding {
    pub id: String,
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub body: String,
    pub action: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedCapability {
    pub workload: String,
    pub label: String,
    pub score: u8,
    pub grade: Grade,
    pub grade_label: String,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedReport {
    pub lang: String,
    pub level: Level,
    pub summary: Summary,
    pub verdict: String,
    pub findings: Vec<RenderedFinding>,
    pub capabilities: Vec<RenderedCapability>,
    pub notes: Vec<String>,
}

pub fn render(report: &Report, lang: &str, level: Level) -> RenderedReport {
    let findings = report
        .findings
        .iter()
        .map(|f| {
            let base = format!("findings/{}", f.id);
            let action = i18n::get(lang, &format!("{base}/action")).map(|a| i18n::fill_for(lang, &a, &f.params)).filter(|a| !a.is_empty());
            RenderedFinding {
                id: f.id.clone(),
                severity: f.severity,
                category: i18n::t(lang, &format!("categories/{}", serde_json::to_value(f.category).unwrap().as_str().unwrap())),
                title: i18n::tf(lang, &format!("{base}/title"), &f.params),
                body: i18n::tf(lang, &format!("{base}/{}", level.key()), &f.params),
                action,
                evidence: if level == Level::Expert { f.evidence.clone() } else { vec![] },
            }
        })
        .collect();

    let capabilities = report
        .capabilities
        .iter()
        .map(|c| RenderedCapability {
            workload: c.workload.key().into(),
            label: i18n::t(lang, &format!("workloads/{}", c.workload.key())),
            score: c.score,
            grade: c.grade,
            grade_label: i18n::t(lang, &format!("grades/{}", serde_json::to_value(c.grade).unwrap().as_str().unwrap())),
            limits: c.limits.iter().map(|l| i18n::t(lang, &format!("limits/{}", l.trim_start_matches("limit.")))).collect(),
        })
        .collect();

    let crit = report.findings.iter().filter(|f| f.severity == Severity::Critical).count();
    let warn = report.findings.iter().filter(|f| f.severity == Severity::Warning).count();
    let verdict_key = match (crit, warn) {
        (0, 0) => "verdict/great",
        (0, _) => "verdict/fine",
        (1, _) => "verdict/attention",
        _ => "verdict/urgent",
    };
    let mut p = serde_json::Map::new();
    p.insert("critical".into(), crit.into());
    p.insert("warnings".into(), warn.into());
    let verdict = i18n::tf(lang, verdict_key, &p);

    RenderedReport {
        lang: lang.into(),
        level,
        summary: report.summary.clone(),
        verdict,
        findings,
        capabilities,
        notes: if level == Level::Expert { report.snapshot.probe_notes.clone() } else { vec![] },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedRecommendation {
    pub id: String,
    pub kind: String,
    pub impact: String,
    pub cost: String,
    pub over_budget: bool,
    pub title: String,
    pub body: String,
    pub why: String,
    pub helps: Vec<String>,
    pub shopping_query: Option<String>,
}

pub fn render_recs(recs: &[Recommendation], lang: &str) -> Vec<RenderedRecommendation> {
    recs.iter()
        .map(|r| {
            let base = format!("recs/{}", r.id);
            let cost = if r.cost_usd == (0, 0) {
                i18n::t(lang, "ui/free")
            } else {
                format!("${}–${}", r.cost_usd.0, r.cost_usd.1)
            };
            RenderedRecommendation {
                id: r.id.clone(),
                kind: i18n::t(lang, &format!("rec_kinds/{}", serde_json::to_value(r.kind).unwrap().as_str().unwrap())),
                impact: i18n::t(lang, &format!("impacts/{}", serde_json::to_value(r.impact).unwrap().as_str().unwrap())),
                cost,
                over_budget: r.over_budget,
                title: i18n::tf(lang, &format!("{base}/title"), &r.params),
                body: i18n::tf(lang, &format!("{base}/body"), &r.params),
                why: i18n::tf(lang, &format!("{base}/why"), &r.params),
                helps: r.helps.iter().map(|w| i18n::t(lang, &format!("workloads/{}", w.key()))).collect(),
                shopping_query: r.shopping_query.clone(),
            }
        })
        .collect()
}

/// Markdown export of a rendered report — the "bullet points" the user asked for.
pub fn to_markdown(r: &RenderedReport) -> String {
    let t = |k: &str| i18n::t(&r.lang, k);
    let mut md = String::new();
    md.push_str(&format!("# {} — {}\n\n", t("ui/report_title"), r.summary.machine));
    md.push_str(&format!("- **{}:** {}\n", t("ui/cpu"), r.summary.cpu));
    md.push_str(&format!("- **{}:** {}\n", t("ui/memory"), r.summary.memory));
    md.push_str(&format!("- **{}:** {}\n", t("ui/storage"), r.summary.storage));
    if !r.summary.gpu.is_empty() {
        md.push_str(&format!("- **{}:** {}\n", t("ui/gpu"), r.summary.gpu));
    }
    md.push_str(&format!("- **{}:** {}\n", t("ui/os"), r.summary.os));
    if let Some(b) = &r.summary.battery {
        md.push_str(&format!("- **{}:** {}\n", t("ui/battery"), b));
    }
    md.push_str(&format!("\n**{}** {}\n\n", t("ui/verdict"), r.verdict));

    for sev in [Severity::Critical, Severity::Warning, Severity::Info, Severity::Good] {
        let group: Vec<_> = r.findings.iter().filter(|f| f.severity == sev).collect();
        if group.is_empty() {
            continue;
        }
        let key = serde_json::to_value(sev).unwrap().as_str().unwrap().to_string();
        md.push_str(&format!("## {}\n\n", t(&format!("severities/{key}"))));
        for f in group {
            md.push_str(&format!("- **{}** — {}", f.title, f.body));
            if let Some(a) = &f.action {
                md.push_str(&format!(" _{}: {}_", t("ui/action"), a));
            }
            md.push('\n');
            for e in &f.evidence {
                md.push_str(&format!("  - `{e}`\n"));
            }
        }
        md.push('\n');
    }

    md.push_str(&format!("## {}\n\n", t("ui/capabilities")));
    for c in &r.capabilities {
        md.push_str(&format!("- **{}:** {} ({}/100)", c.label, c.grade_label, c.score));
        if !c.limits.is_empty() {
            md.push_str(&format!(" — {}", c.limits.join(", ")));
        }
        md.push('\n');
    }
    if !r.notes.is_empty() {
        md.push_str(&format!("\n## {}\n\n", t("ui/notes")));
        for n in &r.notes {
            md.push_str(&format!("- {n}\n"));
        }
    }
    md.push_str(&format!("\n_{}_\n", t("ui/generated_by")));
    md
}


/// Remove everything identifying from a serialized `Report` before it leaves the
/// machine: drive serials, hostname, MAC addresses, the process list, probe
/// notes, and usernames inside paths (`/home/ana/…`, `/media/ana/…`, `/Users/ana/…`,
/// `C:\\Users\\ana\\…`). Used by cloud upload and narration.
pub fn redact(value: &mut serde_json::Value) {
    use serde_json::Value;
    if let Some(devs) = value.pointer_mut("/snapshot/storage").and_then(Value::as_array_mut) {
        for d in devs {
            d["serial"] = Value::Null;
        }
    }
    if let Some(sys) = value.pointer_mut("/snapshot/system") {
        sys["hostname"] = Value::Null;
    }
    if let Some(load) = value.pointer_mut("/snapshot/load") {
        load["top_memory"] = serde_json::json!([]);
    }
    if let Some(net) = value.pointer_mut("/snapshot/network").and_then(Value::as_array_mut) {
        for n in net {
            n["mac"] = Value::Null;
        }
    }
    if let Some(s) = value.pointer_mut("/snapshot") {
        s["probe_notes"] = serde_json::json!([]);
    }
    redact_paths(value);
}

/// Replace the user segment after /home, /media, /Users, C:\Users with `~`.
pub fn redact_paths(value: &mut serde_json::Value) {
    use serde_json::Value;
    match value {
        Value::String(s) => {
            if s.contains("/home/") || s.contains("/media/") || s.contains("/Users/") || s.contains("\\Users\\") {
                *s = redact_path_str(s);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(redact_paths),
        Value::Object(o) => o.values_mut().for_each(redact_paths),
        _ => {}
    }
}

fn redact_path_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let hit = ["/home/", "/media/", "/Users/", "\\Users\\"].iter().filter_map(|p| rest.find(p).map(|i| (i, *p))).min_by_key(|(i, _)| *i);
        let Some((i, prefix)) = hit else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..i + prefix.len()]);
        let after = &rest[i + prefix.len()..];
        let end = after.find(|c: char| c == '/' || c == '\\' || c.is_whitespace() || c == '"' || c == '\'').unwrap_or(after.len());
        if end > 0 {
            out.push('~');
        }
        rest = &after[end..];
    }
}

#[cfg(test)]
mod redact_tests {
    #[test]
    fn paths_lose_usernames() {
        assert_eq!(super::redact_path_str("/media/ana/sda4 is 90% full"), "/media/~/sda4 is 90% full");
        assert_eq!(super::redact_path_str("/home/ana and /home/bob/x"), "/home/~ and /home/~/x");
        assert_eq!(super::redact_path_str("C:\\Users\\ana\\Documents"), "C:\\Users\\~\\Documents");
        assert_eq!(super::redact_path_str("/mnt/data"), "/mnt/data");
    }
}
