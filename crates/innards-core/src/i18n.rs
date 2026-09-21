//! Message catalogs. One JSON file per language, embedded at build time.
//! Every finding/recommendation/capability id maps to templates with
//! `{param}` placeholders; findings have one body per `Level`.
//!
//! Catalog shape (see `i18n/en.json`):
//! ```json
//! { "findings": { "memory.no_swap": { "title": "...", "plain": "...", "informed": "...", "expert": "...", "action": "..." } },
//!   "recs": { "rec.ram_add": { "title": "...", "body": "...", "why": "..." } },
//!   "workloads": { "web_dev": "..." }, "limits": {...}, "grades": {...}, "ui": {...} }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// No jargon. "Your computer has no safety net when memory runs out."
    Plain,
    /// Some terms, still friendly. Default.
    Informed,
    /// Full technical detail plus raw evidence.
    Expert,
}

impl Level {
    pub fn key(self) -> &'static str {
        match self {
            Level::Plain => "plain",
            Level::Informed => "informed",
            Level::Expert => "expert",
        }
    }
}

/// Languages ship as embedded catalogs; add a file and a line here.
pub const LANGS: &[(&str, &str, &str)] = &[
    ("en", "English", include_str!("../i18n/en.json")),
    ("es", "Español", include_str!("../i18n/es.json")),
];

fn catalogs() -> &'static HashMap<&'static str, Value> {
    static C: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    C.get_or_init(|| {
        LANGS
            .iter()
            .map(|(code, _, src)| (*code, serde_json::from_str::<Value>(src).unwrap_or_else(|e| panic!("catalog {code}: {e}"))))
            .collect()
    })
}

pub fn available() -> Vec<(String, String)> {
    LANGS.iter().map(|(c, n, _)| (c.to_string(), n.to_string())).collect()
}

/// Look up a dotted path in a language catalog, falling back to English.
pub fn get(lang: &str, path: &str) -> Option<String> {
    let cats = catalogs();
    let lookup = |v: &Value| -> Option<String> {
        let mut cur = v;
        for seg in path.split('/') {
            cur = cur.get(seg)?;
        }
        cur.as_str().map(String::from)
    };
    cats.get(lang).and_then(lookup).or_else(|| cats.get("en").and_then(lookup))
}

/// Substitute `{name}` placeholders; missing params are left visible so
/// they show up in review rather than silently vanish.
pub fn fill(template: &str, params: &Map<String, Value>) -> String {
    fill_in(template, params, None)
}

/// `fill` that also resolves `@path` catalog references in param values.
pub fn fill_for(lang: &str, template: &str, params: &Map<String, Value>) -> String {
    fill_in(template, params, Some(lang))
}

fn fill_in(template: &str, params: &Map<String, Value>, lang: Option<&str>) -> String {
    let mut out = template.to_string();
    for (k, v) in params {
        let s = match v {
            // "@mainline/excellent": a catalog reference, so rules can pass a
            // localizable word without carrying prose.
            Value::String(s) if s.starts_with('@') && lang.is_some() => t(lang.unwrap(), &s[1..]),
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            other => other.to_string(),
        };
        out = out.replace(&format!("{{{k}}}"), &s);
    }
    out
}

pub fn t(lang: &str, path: &str) -> String {
    get(lang, path).unwrap_or_else(|| format!("⟨{path}⟩"))
}

pub fn tf(lang: &str, path: &str, params: &Map<String, Value>) -> String {
    fill_in(&t(lang, path), params, Some(lang))
}
