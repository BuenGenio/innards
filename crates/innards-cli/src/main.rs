//! `innards` — the engine in a terminal. Built for the cases the desktop app
//! can't reach: a phone over `adb shell`, a board over `ssh`, a headless box.
//!
//! ```text
//! innards report [--lang en|es] [--level plain|informed|expert] [--json]
//! innards advise --uses portable_server,edge_ai [--pains slow,heat_noise]
//!                [--budget under_300] [--horizon now] [--portable] [--lang es] [--json]
//! innards socs                      # list the SoC knowledge base
//! ```

use innards_core::advisor::{self, Answers, Budget, Horizon, Pain};
use innards_core::capability::Workload;
use innards_core::{i18n::Level, probe, report, soc};
use std::process::ExitCode;

const USAGE: &str = "innards — look inside your machine (PC, phone or board)

USAGE
  innards report  [--lang en|es] [--level plain|informed|expert] [--json] [--smart]
  innards advise  --uses <w1,w2,...> [--pains <p1,...>] [--budget <b>] [--horizon <h>]
                  [--portable] [--lang en|es] [--json] [--smart]
  innards socs    [--json]
  innards snapshot                  raw probe output as JSON, no analysis

WORKLOADS  everyday web_dev heavy_compile containers home_server portable_server
           video_editing local_llm edge_ai gaming ml_training photo_editing
PAINS      slow out_of_memory storage battery heat_noise crashes none
BUDGET     under_100 under_300 under_800 under_1500 no_limit   (default under_300)
HORIZON    now six_months year                                  (default now)
--smart    ask for elevated rights to read drive SMART data (may prompt)

EXAMPLES
  innards report --lang es --level plain
  innards advise --uses portable_server,edge_ai --budget under_300 --portable
";

struct Args {
    cmd: String,
    lang: String,
    level: Level,
    json: bool,
    smart: bool,
    uses: Vec<Workload>,
    pains: Vec<Pain>,
    budget: Budget,
    horizon: Horizon,
    portable: bool,
}

fn parse(argv: &[String]) -> Result<Args, String> {
    // `innards --lang es` means `innards report --lang es`; `innards --help` is help.
    let has_cmd = argv.first().is_some_and(|c| !c.starts_with('-'));
    let mut a = Args {
        cmd: if has_cmd { argv[0].clone() } else { "report".into() },
        lang: "en".into(),
        level: Level::Informed,
        json: false,
        smart: false,
        uses: vec![],
        pains: vec![],
        budget: Budget::Under300,
        horizon: Horizon::Now,
        portable: false,
    };
    let mut it = argv.iter().skip(usize::from(has_cmd));
    let need = |flag: &str, it: &mut dyn Iterator<Item = &String>| it.next().cloned().ok_or_else(|| format!("{flag} needs a value"));
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--lang" | "-l" => a.lang = need(arg, &mut it)?,
            "--level" => {
                a.level = match need(arg, &mut it)?.as_str() {
                    "plain" => Level::Plain,
                    "informed" => Level::Informed,
                    "expert" => Level::Expert,
                    other => return Err(format!("unknown level '{other}'")),
                }
            }
            "--json" => a.json = true,
            "--smart" => a.smart = true,
            "--portable" => a.portable = true,
            "--uses" => a.uses = parse_list(&need(arg, &mut it)?, parse_workload)?,
            "--pains" => a.pains = parse_list(&need(arg, &mut it)?, parse_pain)?,
            "--budget" => a.budget = parse_enum(&need(arg, &mut it)?)?,
            "--horizon" => a.horizon = parse_enum(&need(arg, &mut it)?)?,
            "-h" | "--help" | "help" => return Err(String::new()),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(a)
}

fn parse_list<T>(s: &str, f: impl Fn(&str) -> Result<T, String>) -> Result<Vec<T>, String> {
    s.split(',').map(str::trim).filter(|x| !x.is_empty()).map(f).collect()
}

/// Every snake_case enum in the core serializes as its key, so JSON round-trips as a parser.
fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|_| format!("unknown value '{s}'"))
}

fn parse_workload(s: &str) -> Result<Workload, String> {
    Workload::ALL.iter().copied().find(|w| w.key() == s).ok_or_else(|| format!("unknown workload '{s}'"))
}

fn parse_pain(s: &str) -> Result<Pain, String> {
    parse_enum(s)
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse(&argv) {
        Ok(a) => a,
        Err(e) if e.is_empty() => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let lang_known = innards_core::i18n::available().iter().any(|(c, _)| *c == args.lang);
    if !lang_known {
        eprintln!("warning: language '{}' not available, falling back to English", args.lang);
    }
    let opts = probe::Options { elevate_for_smart: args.smart };

    match args.cmd.as_str() {
        "report" => {
            let rep = report::build(probe::collect_with(opts));
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rep).unwrap());
            } else {
                let rendered = report::render(&rep, &args.lang, args.level);
                print!("{}", report::to_markdown(&rendered));
            }
        }
        "advise" => {
            if args.uses.is_empty() {
                eprintln!("error: --uses is required for advise\n\n{USAGE}");
                return ExitCode::from(2);
            }
            let answers = Answers { uses: args.uses.clone(), pains: args.pains.clone(), budget: args.budget, horizon: args.horizon, needs_portability: args.portable };
            let snap = probe::collect_with(opts);
            let recs = advisor::recommend(&snap, &answers);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "answers": answers, "recommendations": recs, "rendered": report::render_recs(&recs, &args.lang) })).unwrap());
            } else {
                print!("{}", recs_markdown(&snap, &recs, &args.lang));
            }
        }
        "snapshot" => {
            println!("{}", serde_json::to_string_pretty(&probe::collect_with(opts)).unwrap());
        }
        "socs" => {
            let all = soc::all();
            if args.json {
                println!("{}", serde_json::to_string_pretty(&all).unwrap());
            } else {
                println!("| SoC | Vendor | Year | Cores | Max GHz | dotprod | i8mm | Mainline Linux |");
                println!("|---|---|---|---|---|---|---|---|");
                for s in all {
                    println!("| {} | {} | {} | {}+{} | {:.2} | {} | {} | {:?} |", s.name, s.vendor, s.launch_year, s.big_cores, s.little_cores, s.max_ghz, s.has_dotprod, s.has_i8mm, s.mainline_linux);
                }
            }
        }
        other => {
            eprintln!("error: unknown command '{other}'\n\n{USAGE}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

/// Markdown for the advisor output: header line, then one block per recommendation.
fn recs_markdown(snap: &innards_core::Snapshot, recs: &[advisor::Recommendation], lang: &str) -> String {
    use innards_core::i18n::t;
    let mut md = String::new();
    let machine = snap.system.product.clone().or_else(|| snap.system.device_model.clone()).or_else(|| snap.system.hostname.clone()).unwrap_or_else(|| "this machine".into());
    md.push_str(&format!("# {} — {}\n\n", t(lang, "ui/report_title"), machine));
    for r in report::render_recs(recs, lang) {
        let flag = if r.over_budget { " ⚠" } else { "" };
        md.push_str(&format!("## {} — {} · {} · {}{}\n\n", r.title, r.kind, r.impact, r.cost, flag));
        md.push_str(&format!("{}\n\n_{}_\n", r.body, r.why));
        if !r.helps.is_empty() {
            md.push_str(&format!("\n- {}\n", r.helps.join(", ")));
        }
        if let Some(q) = &r.shopping_query {
            md.push_str(&format!("- `{q}`\n"));
        }
        md.push('\n');
    }
    md.push_str(&format!("_{}_\n", t(lang, "ui/generated_by")));
    md
}
