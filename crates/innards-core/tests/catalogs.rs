//! Every finding / recommendation id referenced in the Rust sources must
//! exist in every language catalog with all required fields, and every
//! catalog must have the same key set as English. Catches the classic
//! "added a rule, forgot the Spanish string" mistake at test time.

use serde_json::Value;
use std::collections::BTreeSet;

fn ids_in_source(src: &str, prefix: &str) -> BTreeSet<String> {
    // Matches Finding::new("memory.no_swap", ...) and Recommendation::new("rec.ram_add", ...)
    // and the bare-string ids used for overrides like `id: "battery.worn".into()`.
    let mut out = BTreeSet::new();
    for cap in src.split('"').skip(1).step_by(2) {
        if cap.starts_with(prefix) && !cap.contains(' ') && !cap.contains('{') {
            out.insert(cap.to_string());
        }
    }
    out
}

fn keys(v: &Value, path: &str) -> BTreeSet<String> {
    let mut cur = v;
    for seg in path.split('/') {
        cur = cur.get(seg).unwrap_or(&Value::Null);
    }
    cur.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
}

#[test]
fn every_rule_id_is_in_every_catalog() {
    let rules = include_str!("../src/rules.rs");
    let mut finding_ids = BTreeSet::new();
    for prefix in ["memory.", "storage.", "cpu.", "thermal.", "battery.", "gpu.", "network.", "system.", "mobile."] {
        finding_ids.extend(ids_in_source(rules, prefix));
    }
    let rec_ids = ids_in_source(include_str!("../src/advisor.rs"), "rec.");
    assert!(finding_ids.len() > 20, "expected to find rule ids in rules.rs");
    assert!(rec_ids.len() > 5, "expected to find rec ids in advisor.rs");

    for (lang, _, src) in innards_core::i18n::LANGS {
        let cat: Value = serde_json::from_str(src).unwrap_or_else(|e| panic!("{lang}.json is not valid JSON: {e}"));
        for id in &finding_ids {
            let f = cat.pointer(&format!("/findings/{id}")).unwrap_or_else(|| panic!("{lang}: missing findings.{id}"));
            for field in ["title", "plain", "informed", "expert", "action"] {
                assert!(f.get(field).and_then(Value::as_str).is_some(), "{lang}: findings.{id}.{field} missing");
            }
        }
        for id in &rec_ids {
            let r = cat.pointer(&format!("/recs/{id}")).unwrap_or_else(|| panic!("{lang}: missing recs.{id}"));
            for field in ["title", "body", "why"] {
                assert!(r.get(field).and_then(Value::as_str).is_some(), "{lang}: recs.{id}.{field} missing");
            }
        }
    }
}

#[test]
fn catalogs_share_the_english_key_set() {
    let en: Value = serde_json::from_str(innards_core::i18n::LANGS[0].2).unwrap();
    for (lang, _, src) in innards_core::i18n::LANGS.iter().skip(1) {
        let cat: Value = serde_json::from_str(src).unwrap();
        for section in ["ui", "verdict", "severities", "categories", "grades", "workloads", "limits", "mainline", "rec_kinds", "impacts", "findings", "recs", "advisor", "advisor/options"] {
            let a = keys(&en, section);
            let b = keys(&cat, section);
            let missing: Vec<_> = a.difference(&b).collect();
            let extra: Vec<_> = b.difference(&a).collect();
            assert!(missing.is_empty() && extra.is_empty(), "{lang}: section {section} differs from en — missing {missing:?}, extra {extra:?}");
        }
    }
}

#[test]
fn placeholders_match_english() {
    // A translated template must use exactly the same {params} as English.
    fn params(s: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = s;
        while let Some(a) = rest.find('{') {
            if let Some(b) = rest[a..].find('}') {
                out.insert(rest[a + 1..a + b].to_string());
                rest = &rest[a + b + 1..];
            } else {
                break;
            }
        }
        out
    }
    let en: Value = serde_json::from_str(innards_core::i18n::LANGS[0].2).unwrap();
    for (lang, _, src) in innards_core::i18n::LANGS.iter().skip(1) {
        let cat: Value = serde_json::from_str(src).unwrap();
        for section in ["findings", "recs"] {
            for (id, entry) in en[section].as_object().unwrap() {
                for (field, text) in entry.as_object().unwrap() {
                    let translated = cat[section][id][field].as_str().unwrap_or("");
                    assert_eq!(params(text.as_str().unwrap()), params(translated), "{lang}: {section}.{id}.{field} placeholders differ");
                }
            }
        }
    }
}
