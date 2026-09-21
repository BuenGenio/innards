//! `cargo run -p innards-core --example report -- [lang] [plain|informed|expert] [--json]`
//! Prints the Markdown report for this machine. Handy for testing probes
//! without the GUI.

use innards_core::{i18n::Level, probe, report};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lang = args.first().cloned().unwrap_or_else(|| "en".into());
    let level = match args.get(1).map(String::as_str) {
        Some("plain") => Level::Plain,
        Some("expert") => Level::Expert,
        _ => Level::Informed,
    };
    let json = args.iter().any(|a| a == "--json");

    let snap = probe::collect();
    let rep = report::build(snap);
    if json {
        println!("{}", serde_json::to_string_pretty(&rep).unwrap());
        return;
    }
    let rendered = report::render(&rep, &lang, level);
    println!("{}", report::to_markdown(&rendered));
}
