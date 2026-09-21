//! Tauri shell: thin commands over `innards-core`. State is the last
//! `Report` and last advisor recommendations; everything else is stateless.

mod cloud;
mod history;
mod license;
mod narrate;
mod settings;

use innards_core::advisor::{self, Answers, Recommendation};
use innards_core::report::{self, Report, RenderedRecommendation, RenderedReport};
use innards_core::{i18n, probe, Level};
use license::Tier;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use tauri::{Manager, State};

#[derive(Default)]
struct AppState {
    report: Mutex<Option<Report>>,
    recs: Mutex<Vec<Recommendation>>,
    /// True while viewing a report opened from a file (Pro) rather than this machine's.
    loaded_from_file: Mutex<bool>,
}

fn parse_level(s: &str) -> Level {
    match s {
        "plain" => Level::Plain,
        "expert" => Level::Expert,
        _ => Level::Informed,
    }
}

// ------------------------------------------------------------------ analysis

#[tauri::command]
async fn analyze(state: State<'_, AppState>, elevate: bool) -> Result<RenderedReport, String> {
    let st = settings::load();
    // Probing takes ~1s and shells out; keep it off the UI thread.
    let rep = tauri::async_runtime::spawn_blocking(move || report::build(probe::collect_with(probe::Options { elevate_for_smart: elevate })))
        .await
        .map_err(|e| e.to_string())?;
    let rendered = report::render(&rep, &st.lang, parse_level(&st.level));
    history::record(&rep);
    *state.report.lock().unwrap() = Some(rep);
    *state.loaded_from_file.lock().unwrap() = false;
    if st.cloud_auto_upload && license_status().tier >= Tier::Team {
        // Fire and forget; failures are visible on the next manual upload.
        if let (Some(e), Some(t)) = (st.cloud_endpoint.clone(), st.cloud_token.clone()) {
            let rep = state.report.lock().unwrap().clone().unwrap();
            let label = st.machine_label.clone();
            tauri::async_runtime::spawn(async move {
                let en = report::render(&rep, "en", Level::Informed);
                let _ = cloud::upload(&e, &t, label.as_deref(), &rep, &en).await;
            });
        }
    }
    Ok(rendered)
}

#[tauri::command]
fn render(state: State<'_, AppState>, lang: String, level: String) -> Result<RenderedReport, String> {
    let guard = state.report.lock().unwrap();
    let rep = guard.as_ref().ok_or("no report yet")?;
    Ok(report::render(rep, &lang, parse_level(&level)))
}

#[tauri::command]
fn export_markdown(state: State<'_, AppState>, lang: String, level: String) -> Result<String, String> {
    let guard = state.report.lock().unwrap();
    let rep = guard.as_ref().ok_or("no report yet")?;
    Ok(report::to_markdown(&report::render(rep, &lang, parse_level(&level))))
}

#[tauri::command]
fn report_json(state: State<'_, AppState>) -> Result<Value, String> {
    let guard = state.report.lock().unwrap();
    let rep = guard.as_ref().ok_or("no report yet")?;
    serde_json::to_value(rep).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_text(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}

/// Read a file the user picked in a dialog (Pro "Open report…"). Capped at 8 MB.
#[tauri::command]
fn read_text(path: String) -> Result<String, String> {
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > 8 * 1024 * 1024 {
        return Err("file too large".into());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ history (Supporter) & other machines (Pro)

#[tauri::command]
fn history(limit: Option<usize>) -> Result<Vec<history::Entry>, String> {
    if license_status().tier < Tier::Supporter {
        return Err("supporter".into());
    }
    Ok(history::load(limit.unwrap_or(365)))
}

#[tauri::command]
fn history_clear() -> Result<(), String> {
    history::clear()
}

/// Pro: view a report exported by another Innards (the JSON from `report_json`).
/// It replaces the current view until `analyze` runs again; nothing is recorded
/// in this machine's history.
#[tauri::command]
fn load_report_json(state: State<'_, AppState>, text: String, lang: String, level: String) -> Result<RenderedReport, String> {
    if license_status().tier < Tier::Pro {
        return Err("pro".into());
    }
    let rep: Report = serde_json::from_str(&text).map_err(|e| format!("not an Innards report: {e}"))?;
    let rendered = report::render(&rep, &lang, parse_level(&level));
    *state.report.lock().unwrap() = Some(rep);
    *state.loaded_from_file.lock().unwrap() = true;
    state.recs.lock().unwrap().clear();
    Ok(rendered)
}

#[tauri::command]
fn viewing_other_machine(state: State<'_, AppState>) -> bool {
    *state.loaded_from_file.lock().unwrap()
}

// ------------------------------------------------------------------ i18n

#[tauri::command]
fn languages() -> Vec<(String, String)> {
    i18n::available()
}

/// The UI-facing slice of a catalog (labels, advisor questions, etc.).
#[tauri::command]
fn catalog(lang: String) -> Value {
    let src = i18n::LANGS.iter().find(|(c, _, _)| *c == lang).or_else(|| i18n::LANGS.first()).map(|(_, _, s)| *s).unwrap_or("{}");
    let v: Value = serde_json::from_str(src).unwrap_or_default();
    serde_json::json!({
        "ui": v.get("ui"), "severities": v.get("severities"), "categories": v.get("categories"),
        "grades": v.get("grades"), "workloads": v.get("workloads"), "advisor": v.get("advisor"),
        "rec_kinds": v.get("rec_kinds"), "impacts": v.get("impacts"),
    })
}

// ------------------------------------------------------------------ advisor

#[tauri::command]
fn advisor_questions() -> Vec<advisor::Question> {
    advisor::questions()
}

#[tauri::command]
fn advise(state: State<'_, AppState>, answers: Answers, lang: String) -> Result<Vec<RenderedRecommendation>, String> {
    if license_status().tier < Tier::Supporter {
        return Err("supporter".into());
    }
    let guard = state.report.lock().unwrap();
    let rep = guard.as_ref().ok_or("no report yet")?;
    let recs = advisor::recommend(&rep.snapshot, &answers);
    let rendered = report::render_recs(&recs, &lang);
    *state.recs.lock().unwrap() = recs;
    Ok(rendered)
}

#[tauri::command]
fn shop_links(state: State<'_, AppState>, rec_id: String, region: String) -> Result<Vec<advisor::ShopLink>, String> {
    if license_status().tier < Tier::Pro {
        return Err("pro".into());
    }
    let recs = state.recs.lock().unwrap();
    let rec = recs.iter().find(|r| r.id == rec_id).ok_or("unknown recommendation")?;
    Ok(advisor::shop_links(rec, &region))
}

// ------------------------------------------------------------------ narration

#[tauri::command]
async fn narrate(state: State<'_, AppState>, lang: String, level: String, include_recs: bool) -> Result<narrate::Narrative, String> {
    if license_status().tier < Tier::Supporter {
        return Err("supporter".into());
    }
    let st = settings::load();
    let key = st.anthropic_api_key.filter(|k| !k.is_empty()).or_else(|| std::env::var("ANTHROPIC_API_KEY").ok()).ok_or("no_api_key")?;
    let (rendered, recs) = {
        let guard = state.report.lock().unwrap();
        let rep = guard.as_ref().ok_or("no report yet")?;
        let rendered = report::render(rep, &lang, parse_level(&level));
        let recs = if include_recs { Some(report::render_recs(&state.recs.lock().unwrap(), &lang)) } else { None };
        (rendered, recs)
    };
    narrate::narrate(&key, &rendered, recs.as_deref()).await
}

// ------------------------------------------------------------------ cloud (Team / Enterprise)

#[tauri::command]
async fn cloud_upload(state: State<'_, AppState>) -> Result<cloud::UploadResult, String> {
    if license_status().tier < Tier::Team {
        return Err("team".into());
    }
    let st = settings::load();
    let endpoint = st.cloud_endpoint.filter(|e| !e.is_empty()).ok_or("no_endpoint")?;
    let token = st.cloud_token.filter(|t| !t.is_empty()).ok_or("no_token")?;
    let (rep, rendered) = {
        let guard = state.report.lock().unwrap();
        let rep = guard.as_ref().ok_or("no report yet")?;
        // The dashboard stores English/informed prose alongside the structured report.
        (rep.clone(), report::render(rep, "en", Level::Informed))
    };
    cloud::upload(&endpoint, &token, st.machine_label.as_deref(), &rep, &rendered).await
}

// ------------------------------------------------------------------ settings & license

#[tauri::command]
fn get_settings() -> settings::Settings {
    settings::load()
}

#[tauri::command]
fn set_settings(s: settings::Settings) -> Result<(), String> {
    settings::save(&s)
}

#[tauri::command]
fn license_status() -> license::Status {
    license::status(settings::load().license_key.as_deref())
}

#[tauri::command]
fn activate_license(key: String) -> Result<license::Status, String> {
    let st = license::status(Some(&key));
    if st.tier == Tier::Free && st.source != "env" {
        return Err("invalid".into());
    }
    let mut s = settings::load();
    s.license_key = Some(key);
    settings::save(&s)?;
    Ok(st)
}

/// Renew a subscription key through the billing service and persist it.
/// The frontend calls this at startup when the key expires within 30 days.
#[tauri::command]
async fn license_refresh() -> Result<license::Status, String> {
    let mut s = settings::load();
    let key = s.license_key.clone().filter(|k| !k.is_empty()).ok_or("no_key")?;
    let fresh = license::refresh(&key).await?;
    if license::verify(&fresh).is_none() {
        return Err("invalid".into());
    }
    s.license_key = Some(fresh.clone());
    settings::save(&s)?;
    Ok(license::status(Some(&fresh)))
}

#[derive(Serialize, Deserialize)]
struct AppInfo {
    version: String,
    core_version: String,
    os: String,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION").into(), core_version: innards_core::VERSION.into(), os: std::env::consts::OS.into() }
}

/// Team/Enterprise: while the app is open, rescan and upload every
/// `cloud_interval_hours` (default 24) if auto-upload is on. Cheap to run;
/// the scan takes ~1 s.
fn start_scheduler(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let st = settings::load();
            let hours = st.cloud_interval_hours.max(1);
            tokio::time::sleep(std::time::Duration::from_secs(hours as u64 * 3600)).await;
            let st = settings::load();
            if !st.cloud_auto_upload || license_status().tier < Tier::Team {
                continue;
            }
            let (Some(e), Some(t)) = (st.cloud_endpoint.clone(), st.cloud_token.clone()) else { continue };
            let elevate = st.elevate_for_smart;
            let Ok(rep) = tauri::async_runtime::spawn_blocking(move || report::build(probe::collect_with(probe::Options { elevate_for_smart: elevate }))).await else { continue };
            history::record(&rep);
            let en = report::render(&rep, "en", Level::Informed);
            let _ = cloud::upload(&e, &t, st.machine_label.as_deref(), &rep, &en).await;
            let state = app.state::<AppState>();
            if !*state.loaded_from_file.lock().unwrap() {
                *state.report.lock().unwrap() = Some(rep);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            start_scheduler(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            analyze,
            render,
            export_markdown,
            report_json,
            save_text,
            read_text,
            languages,
            catalog,
            advisor_questions,
            advise,
            shop_links,
            narrate,
            get_settings,
            set_settings,
            license_status,
            activate_license,
            cloud_upload,
            license_refresh,
            history,
            history_clear,
            load_report_json,
            viewing_other_machine,
            app_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
