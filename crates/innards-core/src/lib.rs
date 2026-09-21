//! innards-core: look inside a machine and explain what you see.
//!
//! Pipeline: [`probe::collect`] → [`Snapshot`] → [`report::build`] →
//! [`Report`] (findings + capability scores) → [`report::render`] →
//! [`RenderedReport`] in a language and at a level → optional
//! [`report::to_markdown`]. The [`advisor`] takes the same snapshot plus
//! questionnaire answers and returns costed recommendations.
//!
//! Nothing here touches the network. Every probe is best-effort and never
//! panics on a platform that lacks a data source.

pub mod advisor;
pub mod capability;
pub mod finding;
pub mod i18n;
pub mod probe;
pub mod report;
pub mod rules;
pub mod snapshot;
pub mod soc;

pub use finding::{Category, Finding, Severity};
pub use i18n::Level;
pub use report::{Report, RenderedReport};
pub use snapshot::Snapshot;

/// One-call convenience: probe, analyze, and render.
pub fn analyze(lang: &str, level: Level) -> RenderedReport {
    let report = report::build(probe::collect());
    report::render(&report, lang, level)
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
