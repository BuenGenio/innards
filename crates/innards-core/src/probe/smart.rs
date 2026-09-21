//! SMART health via `smartctl -j` (smartmontools) on every platform, with an
//! `nvme smart-log -o json` fallback on Linux. Both usually need elevated
//! rights; when they fail we simply note it.

use super::util::{run, which};
use crate::snapshot::*;
use serde_json::Value;

pub fn fill(snap: &mut Snapshot, elevate: bool) {
    let have_smartctl = which("smartctl");
    let have_nvme = cfg!(target_os = "linux") && which("nvme");
    if !have_smartctl && !have_nvme {
        snap.probe_notes.push("smartmontools not installed: drive health (SMART) not available".into());
        return;
    }
    let mut any_denied = false;
    for dev in &mut snap.storage {
        let path = device_path(&dev.name);
        let mut result = None;
        if have_smartctl {
            result = run_maybe_elevated(elevate, "smartctl", &["-j", "-a", &path]).and_then(|s| parse_smartctl(&s));
        }
        if result.is_none() && have_nvme && dev.kind == StorageKind::Nvme {
            result = run_maybe_elevated(elevate, "nvme", &["smart-log", "-o", "json", &path]).and_then(|s| parse_nvme_cli(&s));
        }
        match result {
            Some((info, temp)) => {
                if dev.temperature_c.is_none() {
                    dev.temperature_c = temp;
                }
                dev.smart = Some(info);
            }
            None => any_denied = true,
        }
    }
    if any_denied {
        snap.probe_notes.push("some drives returned no SMART data (usually needs administrator rights)".into());
    }
}

/// Try unprivileged first; if that yields nothing and elevation was
/// requested, go through the platform's GUI auth prompt. `sudo -n` is
/// tried silently first on Unix so passwordless setups never see a dialog.
fn run_maybe_elevated(elevate: bool, cmd: &str, args: &[&str]) -> Option<String> {
    if let Some(out) = run(cmd, args) {
        return Some(out);
    }
    if !elevate {
        return None;
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let mut sudo_args = vec!["-n", cmd];
        sudo_args.extend_from_slice(args);
        if let Some(out) = run("sudo", &sudo_args) {
            return Some(out);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let mut a = vec![cmd];
        a.extend_from_slice(args);
        return run("pkexec", &a);
    }
    #[cfg(target_os = "macos")]
    {
        let joined = std::iter::once(cmd).chain(args.iter().copied()).map(|s| format!("'{}'", s.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" ");
        let script = format!("do shell script \"{}\" with administrator privileges", joined.replace('"', "\\\""));
        return run("osascript", &["-e", &script]);
    }
    #[cfg(target_os = "android")]
    {
        // Rooted Android: Magisk/KernelSU provide `su -c`; the phone shows its own prompt.
        let joined = std::iter::once(cmd).chain(args.iter().copied()).map(|s| format!("'{}'", s.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" ");
        return run("su", &["-c", &joined]);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "android")))]
    {
        // Elevation on Windows means relaunching the whole process with UAC;
        // that is the app's job, not the probe's. Other platforms: nothing to try.
        None
    }
}

fn device_path(name: &str) -> String {
    if cfg!(windows) {
        name.to_string()
    } else if name.starts_with('/') {
        name.to_string()
    } else {
        format!("/dev/{name}")
    }
}

fn parse_smartctl(json: &str) -> Option<(SmartInfo, Option<f32>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    // smartctl exits non-zero for many benign reasons; `run` already filtered
    // hard failures, but check the device was actually opened.
    if v.get("device").is_none() {
        return None;
    }
    let temp = v.pointer("/temperature/current").and_then(Value::as_f64).map(|t| t as f32);
    let mut s = SmartInfo { source: "smartctl".into(), ..Default::default() };
    s.healthy = v.pointer("/smart_status/passed").and_then(Value::as_bool);
    s.power_on_hours = v.pointer("/power_on_time/hours").and_then(Value::as_u64);
    s.power_cycles = v.get("power_cycle_count").and_then(Value::as_u64);
    if let Some(n) = v.get("nvme_smart_health_information_log") {
        s.unsafe_shutdowns = n.get("unsafe_shutdowns").and_then(Value::as_u64);
        s.percentage_used = n.get("percentage_used").and_then(Value::as_u64).map(|p| p.min(255) as u8);
        s.media_errors = n.get("media_errors").and_then(Value::as_u64);
        s.data_written_bytes = n.get("data_units_written").and_then(Value::as_u64).map(|u| u * 512_000);
        s.warning_temp_minutes = n.get("warning_temp_time").and_then(Value::as_u64);
        s.critical_temp_minutes = n.get("critical_comp_time").and_then(Value::as_u64);
    }
    if let Some(table) = v.pointer("/ata_smart_attributes/table").and_then(Value::as_array) {
        for a in table {
            let id = a.get("id").and_then(Value::as_u64).unwrap_or(0);
            let raw = a.pointer("/raw/value").and_then(Value::as_u64);
            match id {
                5 => s.reallocated_sectors = raw,
                197 => s.pending_sectors = raw,
                241 => s.data_written_bytes = raw.map(|r| r * 512), // LBAs written; approximate
                _ => {}
            }
        }
    }
    Some((s, temp))
}

fn parse_nvme_cli(json: &str) -> Option<(SmartInfo, Option<f32>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let temp = v.get("temperature").and_then(Value::as_f64).map(|k| (k - 273.15) as f32);
    let info = SmartInfo {
        source: "nvme-cli".into(),
        healthy: v.get("critical_warning").and_then(Value::as_u64).map(|w| w == 0),
        power_on_hours: v.get("power_on_hours").and_then(Value::as_u64),
        power_cycles: v.get("power_cycles").and_then(Value::as_u64),
        unsafe_shutdowns: v.get("unsafe_shutdowns").and_then(Value::as_u64),
        percentage_used: v.get("percent_used").and_then(Value::as_u64).map(|p| p.min(255) as u8),
        media_errors: v.get("media_errors").and_then(Value::as_u64),
        reallocated_sectors: None,
        pending_sectors: None,
        data_written_bytes: v.get("data_units_written").and_then(Value::as_u64).map(|u| u * 512_000),
        warning_temp_minutes: v.get("warning_temp_time").and_then(Value::as_u64),
        critical_temp_minutes: v.get("critical_comp_time").and_then(Value::as_u64),
    };
    Some((info, temp))
}
