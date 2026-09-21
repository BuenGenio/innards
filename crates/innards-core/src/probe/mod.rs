//! Probes fill a `Snapshot`. `common` uses cross-platform crates; each
//! platform module then enriches with what only it can see. Every probe is
//! best-effort: a failure adds a note, never an error.

use crate::snapshot::Snapshot;

mod common;
mod smart;
pub(crate) mod util;

// Android is a Linux kernel with a different userland: the sysfs/procfs
// probes apply, then `android` layers getprop/battery/root detection on top.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Ask the OS for elevated rights (pkexec / osascript admin) to read
    /// SMART data. Triggers a system password prompt; off by default.
    pub elevate_for_smart: bool,
}

/// Collect everything we can about this machine, without prompting.
pub fn collect() -> Snapshot {
    collect_with(Options::default())
}

pub fn collect_with(opts: Options) -> Snapshot {
    let mut snap = Snapshot {
        taken_at: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    };
    common::fill(&mut snap);

    #[cfg(any(target_os = "linux", target_os = "android"))]
    linux::fill(&mut snap);
    #[cfg(target_os = "android")]
    android::fill(&mut snap);
    #[cfg(target_os = "macos")]
    macos::fill(&mut snap);
    #[cfg(target_os = "windows")]
    windows::fill(&mut snap);

    smart::fill(&mut snap, opts.elevate_for_smart);
    common::finalize(&mut snap);
    snap
}
