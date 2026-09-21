//! Rules turn a `Snapshot` into `Finding`s. Each rule is a plain function;
//! keep them small, independent, and free of prose. Thresholds live here so
//! they can be tuned in one place.

use crate::finding::{fmt_bytes, Category, Finding, Severity};
use crate::snapshot::*;
use crate::soc::MainlineSupport;

pub fn run(snap: &Snapshot) -> Vec<Finding> {
    let mut out = Vec::new();
    memory(snap, &mut out);
    storage(snap, &mut out);
    cpu(snap, &mut out);
    battery(snap, &mut out);
    gpu(snap, &mut out);
    network(snap, &mut out);
    system(snap, &mut out);
    mobile(snap, &mut out);
    out.sort_by_key(|f| f.severity);
    out
}

fn pct(part: u64, whole: u64) -> f64 {
    if whole == 0 { 0.0 } else { part as f64 * 100.0 / whole as f64 }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn memory(s: &Snapshot, out: &mut Vec<Finding>) {
    let m = &s.memory;
    let avail_pct = pct(m.available_bytes, m.total_bytes);
    let total_gib = crate::finding::ram_marketing_gb(m.total_bytes);

    if m.swap_total_bytes == 0 {
        let sev = if avail_pct < 15.0 { Severity::Critical } else { Severity::Warning };
        out.push(
            Finding::new("memory.no_swap", sev, Category::Memory)
                .param("total", fmt_bytes(m.total_bytes))
                .param("available_pct", round1(avail_pct))
                .evidence(format!("swap_total=0, available={} ({:.0}%)", fmt_bytes(m.available_bytes), avail_pct)),
        );
    } else {
        let kinds: Vec<String> = m.swap_backends.iter().map(|b| format!("{} ({:?}, prio {})", fmt_bytes(b.size_bytes), b.kind, b.priority)).collect();
        out.push(
            Finding::new("memory.swap_ok", Severity::Good, Category::Memory)
                .param("swap", fmt_bytes(m.swap_total_bytes))
                .param("backends", kinds.join(", "))
                .evidence(format!("swap_total={}, used={}", fmt_bytes(m.swap_total_bytes), fmt_bytes(m.swap_used_bytes))),
        );
    }

    if avail_pct < 7.0 {
        out.push(Finding::new("memory.low_available", Severity::Critical, Category::Memory).param("available", fmt_bytes(m.available_bytes)).param("available_pct", round1(avail_pct)).evidence(format!("available={} of {}", fmt_bytes(m.available_bytes), fmt_bytes(m.total_bytes))));
    } else if avail_pct < 15.0 {
        out.push(Finding::new("memory.low_available", Severity::Warning, Category::Memory).param("available", fmt_bytes(m.available_bytes)).param("available_pct", round1(avail_pct)).evidence(format!("available={} of {}", fmt_bytes(m.available_bytes), fmt_bytes(m.total_bytes))));
    }

    if m.tmpfs_used_bytes > m.total_bytes / 10 {
        out.push(Finding::new("memory.tmpfs_heavy", Severity::Warning, Category::Memory).param("tmpfs", fmt_bytes(m.tmpfs_used_bytes)).param("pct", round1(pct(m.tmpfs_used_bytes, m.total_bytes))).evidence(format!("tmpfs used={}", fmt_bytes(m.tmpfs_used_bytes))));
    }

    if let Some(p) = s.load.top_memory.first() {
        if p.rss_bytes > m.total_bytes / 5 {
            out.push(Finding::new("memory.hog", Severity::Info, Category::Memory).param("process", p.name.clone()).param("rss", fmt_bytes(p.rss_bytes)).param("pct", round1(pct(p.rss_bytes, m.total_bytes))));
        }
    }

    // Marketing size: 7.7 GiB reported is an 8 GB machine, not a "below 8 GB" one.
    if total_gib < 4 {
        out.push(Finding::new("memory.small_total", Severity::Warning, Category::Memory).param("total", total_gib));
    } else if total_gib < 8 {
        out.push(Finding::new("memory.small_total", Severity::Info, Category::Memory).param("total", total_gib));
    }

    match m.upgradeable {
        Some(false) => out.push(Finding::new("memory.soldered", Severity::Info, Category::Memory).param("total", total_gib).param("kind", m.kind.clone().unwrap_or_else(|| "RAM".into()))),
        Some(true) => out.push(Finding::new("memory.upgradeable", Severity::Good, Category::Memory).param("total", total_gib).param("modules", m.modules.len() as u64)),
        None => {}
    }
}

fn storage(s: &Snapshot, out: &mut Vec<Finding>) {
    for fs in &s.filesystems {
        let used_pct = 100.0 - pct(fs.available_bytes, fs.total_bytes);
        let (warn, crit) = if fs.is_root { (85.0, 95.0) } else { (90.0, 98.0) };
        if used_pct >= crit || used_pct >= warn {
            let sev = if used_pct >= crit { Severity::Critical } else { Severity::Warning };
            let id = if fs.is_root { "storage.root_nearly_full" } else { "storage.fs_nearly_full" };
            out.push(
                Finding::new(id, sev, Category::Storage)
                    .param("mount", fs.mount_point.clone())
                    .param("used_pct", round1(used_pct))
                    .param("free", fmt_bytes(fs.available_bytes))
                    .param("total", fmt_bytes(fs.total_bytes))
                    .evidence(format!("{} {} {}/{} used", fs.device, fs.fs_type, fmt_bytes(fs.total_bytes - fs.available_bytes), fmt_bytes(fs.total_bytes))),
            );
        }
    }

    for d in &s.storage {
        let label = d.model.clone().unwrap_or_else(|| d.name.clone());
        if d.is_system_disk {
            match d.kind {
                StorageKind::Hdd => out.push(Finding::new("storage.system_hdd", Severity::Warning, Category::Storage).param("model", label.clone()).param("size", fmt_bytes(d.size_bytes))),
                StorageKind::Nvme => out.push(Finding::new("storage.system_nvme", Severity::Good, Category::Storage).param("model", label.clone()).param("size", fmt_bytes(d.size_bytes)).param("link", d.link_speed.clone().unwrap_or_default())),
                StorageKind::Ssd => out.push(Finding::new("storage.system_ssd", Severity::Good, Category::Storage).param("model", label.clone()).param("size", fmt_bytes(d.size_bytes))),
                StorageKind::Emmc => out.push(Finding::new("storage.system_emmc", Severity::Info, Category::Storage).param("model", label.clone()).param("size", fmt_bytes(d.size_bytes))),
                StorageKind::Ufs => out.push(Finding::new("storage.ufs", Severity::Good, Category::Storage).param("model", label.clone()).param("size", fmt_bytes(d.size_bytes)).evidence(format!("{}: UFS host controller, {}", d.name, fmt_bytes(d.size_bytes)))),
                _ => {}
            }
        }
        if let Some(t) = d.temperature_c {
            if t >= 70.0 {
                out.push(Finding::new("storage.hot_now", Severity::Warning, Category::Storage).param("model", label.clone()).param("temp", round1(t as f64)));
            }
        }
        let Some(sm) = &d.smart else { continue };
        if sm.healthy == Some(false) {
            out.push(Finding::new("storage.smart_failing", Severity::Critical, Category::Storage).param("model", label.clone()).evidence(format!("{}: SMART overall status FAILED", d.name)));
        }
        if sm.media_errors.unwrap_or(0) > 0 || sm.reallocated_sectors.unwrap_or(0) > 0 || sm.pending_sectors.unwrap_or(0) > 0 {
            out.push(
                Finding::new("storage.media_errors", Severity::Critical, Category::Storage)
                    .param("model", label.clone())
                    .param("count", sm.media_errors.unwrap_or(0) + sm.reallocated_sectors.unwrap_or(0) + sm.pending_sectors.unwrap_or(0))
                    .evidence(format!("media_errors={:?} reallocated={:?} pending={:?}", sm.media_errors, sm.reallocated_sectors, sm.pending_sectors)),
            );
        }
        if let (Some(u), Some(c)) = (sm.unsafe_shutdowns, sm.power_cycles) {
            if u >= 10 && c > 0 && (u as f64 / c as f64) > 0.15 {
                out.push(Finding::new("storage.unsafe_shutdowns", Severity::Warning, Category::Storage).param("model", label.clone()).param("unsafe", u).param("cycles", c).param("pct", round1(u as f64 * 100.0 / c as f64)).evidence(format!("{}: unsafe_shutdowns={u} power_cycles={c}", d.name)));
            }
        }
        if let Some(crit) = sm.critical_temp_minutes {
            if crit >= 30 {
                out.push(Finding::new("storage.critical_temp_history", Severity::Warning, Category::Storage).param("model", label.clone()).param("minutes", crit).param("warning_minutes", sm.warning_temp_minutes.unwrap_or(0)).evidence(format!("{}: critical_comp_time={crit}m warning_temp_time={:?}m", d.name, sm.warning_temp_minutes)));
            }
        }
        if let Some(w) = sm.percentage_used {
            if w >= 80 {
                out.push(Finding::new("storage.wear_high", Severity::Warning, Category::Storage).param("model", label.clone()).param("used_pct", w as u64).evidence(format!("{}: percentage_used={w}%", d.name)));
            } else if w >= 50 {
                out.push(Finding::new("storage.wear_high", Severity::Info, Category::Storage).param("model", label.clone()).param("used_pct", w as u64));
            } else if d.is_system_disk {
                out.push(Finding::new("storage.wear_low", Severity::Good, Category::Storage).param("model", label.clone()).param("used_pct", w as u64).param("written", sm.data_written_bytes.map(fmt_bytes).unwrap_or_default()).param("hours", sm.power_on_hours.unwrap_or(0)));
            }
        }
    }

    if s.storage.iter().all(|d| d.smart.is_none()) && !s.storage.is_empty() {
        out.push(Finding::new("storage.smart_unavailable", Severity::Info, Category::Storage));
    }
}

fn cpu(s: &Snapshot, out: &mut Vec<Finding>) {
    let c = &s.cpu;
    let now_year = chrono::Utc::now().format("%Y").to_string().parse::<u16>().unwrap_or(2026);
    if let Some(y) = c.launch_year {
        let age = now_year.saturating_sub(y);
        if age >= 9 {
            out.push(Finding::new("cpu.old", Severity::Warning, Category::Cpu).param("brand", c.brand.clone()).param("age", age as u64).param("year", y as u64));
        } else if age >= 6 {
            out.push(Finding::new("cpu.old", Severity::Info, Category::Cpu).param("brand", c.brand.clone()).param("age", age as u64).param("year", y as u64));
        } else {
            out.push(Finding::new("cpu.modern", Severity::Good, Category::Cpu).param("brand", c.brand.clone()).param("year", y as u64));
        }
    }
    if let Some(cores) = c.physical_cores {
        if cores <= 2 {
            out.push(Finding::new("cpu.few_cores", Severity::Info, Category::Cpu).param("cores", cores as u64).param("threads", c.logical_cpus as u64));
        }
    }
    if let Some(t) = s.thermal.cpu_c {
        if t >= 95.0 {
            out.push(Finding::new("cpu.hot", Severity::Critical, Category::Thermal).param("temp", round1(t as f64)).param("throttle_events", c.throttle_events.unwrap_or(0)));
        } else if t >= 85.0 {
            out.push(Finding::new("cpu.hot", Severity::Warning, Category::Thermal).param("temp", round1(t as f64)).param("throttle_events", c.throttle_events.unwrap_or(0)));
        }
    }
    let arm = s.system.arch.to_ascii_lowercase().contains("arm") || s.system.arch.to_ascii_lowercase().contains("aarch64") || s.system.soc.is_some();
    if !arm && !c.flags.is_empty() && !c.flags.iter().any(|f| f == "avx2") {
        out.push(Finding::new("cpu.no_avx2", Severity::Info, Category::Cpu).param("brand", c.brand.clone()));
    }
    if s.thermal.has_active_cooling == Some(false) {
        out.push(
            Finding::new("thermal.no_active_cooling", Severity::Info, Category::Thermal)
                .param("temp", s.thermal.cpu_c.map(|t| round1(t as f64)).unwrap_or(0.0))
                .evidence(format!("no fan sensors; chassis={:?}; cpu={:?}°C", s.system.chassis, s.thermal.cpu_c)),
        );
    }
    if s.thermal.fan_rpm.iter().any(|r| *r >= 5000) && s.load.load_1 < s.cpu.logical_cpus as f64 * 0.5 {
        out.push(Finding::new("thermal.fan_high_at_idle", Severity::Info, Category::Thermal).param("rpm", *s.thermal.fan_rpm.iter().max().unwrap() as u64).param("temp", s.thermal.cpu_c.map(|t| round1(t as f64)).unwrap_or(0.0)));
    }
}

fn battery(s: &Snapshot, out: &mut Vec<Finding>) {
    let Some(b) = &s.battery else { return };
    if !b.present {
        return;
    }
    if s.is_mobile() || s.system.chassis == Chassis::Sbc {
        out.push(
            Finding::new("battery.as_ups", Severity::Good, Category::Battery)
                .param("charge", b.charge_pct.map(|c| c.round() as i64).unwrap_or(0))
                .param("health", b.health_pct.map(|h| h.round() as i64).unwrap_or(0))
                .evidence(format!("battery present, charge={:?}% health={:?}% on_ac={:?}", b.charge_pct, b.health_pct, b.on_ac)),
        );
    }
    let Some(h) = b.health_pct else { return };
    let f = Finding::new("battery.health", Severity::Info, Category::Battery)
        .param("health", round1(h as f64))
        .param("design_wh", round1(b.design_wh.unwrap_or(0.0) as f64))
        .param("full_wh", round1(b.full_wh.unwrap_or(0.0) as f64))
        .param("cycles", b.cycles.unwrap_or(0) as u64)
        .evidence(format!("energy_full={:.1} Wh design={:.1} Wh cycles={:?}", b.full_wh.unwrap_or(0.0), b.design_wh.unwrap_or(0.0), b.cycles));
    let f = if h < 40.0 {
        Finding { id: "battery.worn".into(), severity: Severity::Critical, ..f }
    } else if h < 65.0 {
        Finding { id: "battery.worn".into(), severity: Severity::Warning, ..f }
    } else {
        Finding { id: "battery.ok".into(), severity: Severity::Good, ..f }
    };
    out.push(f);
}

fn gpu(s: &Snapshot, out: &mut Vec<Finding>) {
    if s.gpus.is_empty() {
        return;
    }
    if let Some(d) = s.gpus.iter().find(|g| g.is_discrete) {
        out.push(Finding::new("gpu.discrete", Severity::Good, Category::Gpu).param("name", d.name.clone()).param("vram", d.vram_bytes.map(fmt_bytes).unwrap_or_default()));
    } else {
        let g = &s.gpus[0];
        out.push(Finding::new("gpu.integrated_only", Severity::Info, Category::Gpu).param("name", g.name.clone()));
    }
}

fn network(s: &Snapshot, out: &mut Vec<Finding>) {
    let eth: Vec<_> = s.network.iter().filter(|n| n.kind == NetKind::Ethernet).collect();
    let wifi = s.network.iter().any(|n| n.kind == NetKind::Wifi);
    if eth.is_empty() && wifi {
        out.push(Finding::new("network.wifi_only", Severity::Info, Category::Network));
    }
    for e in eth {
        if e.is_up {
            if let Some(mbps) = e.link_mbps {
                if mbps < 1000 {
                    out.push(Finding::new("network.ethernet_slow", Severity::Info, Category::Network).param("iface", e.name.clone()).param("mbps", mbps as u64));
                }
            }
        }
    }
}

fn system(s: &Snapshot, out: &mut Vec<Finding>) {
    let cpus = s.cpu.logical_cpus.max(1) as f64;
    if s.load.load_5 > cpus {
        out.push(Finding::new("system.high_load", Severity::Warning, Category::System).param("load", round1(s.load.load_5)).param("cpus", cpus));
    }
    if let Some(w) = s.load.io_wait_pct {
        if w >= 20.0 {
            out.push(Finding::new("system.io_wait_high", Severity::Warning, Category::System).param("io_wait", round1(w as f64)));
        }
    }
    if s.system.chassis == Chassis::Vm {
        out.push(Finding::new("system.vm", Severity::Info, Category::System));
    }
    if s.system.uptime_secs > 30 * 86_400 {
        out.push(Finding::new("system.uptime_long", Severity::Info, Category::System).param("days", s.system.uptime_secs / 86_400));
    }
}

/// Phones, tablets and single-board computers: what the SoC is, whether a
/// real Linux runs on it, and what root / no-root means for server use.
fn mobile(s: &Snapshot, out: &mut Vec<Finding>) {
    let android = s.system.is_android();
    if let Some(soc) = &s.system.soc {
        let mainline_key = match soc.mainline_linux {
            MainlineSupport::Excellent => "@mainline/excellent",
            MainlineSupport::Good => "@mainline/good",
            MainlineSupport::Partial => "@mainline/partial",
            MainlineSupport::None => "@mainline/none",
            MainlineSupport::Unknown => "@mainline/unknown",
        };
        let sev = if matches!(soc.mainline_linux, MainlineSupport::Excellent | MainlineSupport::Good) { Severity::Good } else { Severity::Info };
        out.push(
            Finding::new("mobile.soc", sev, Category::System)
                .param("soc", soc.name.clone())
                .param("vendor", soc.vendor.clone())
                .param("year", soc.launch_year as u64)
                .param("big", soc.big_cores as u64)
                .param("little", soc.little_cores as u64)
                .param("cores", soc.total_cores() as u64)
                .param("ghz", round1(soc.max_ghz as f64))
                .param("nm", soc.process_nm as u64)
                .param("gpu", soc.gpu.clone())
                .param("mainline", mainline_key)
                .param("device", s.system.device_model.clone().unwrap_or_default())
                .evidence(format!("soc={} vendor={} year={} cores={}+{} max={} GHz process={} nm dotprod={} i8mm={} mainline={:?}", soc.name, soc.vendor, soc.launch_year, soc.big_cores, soc.little_cores, soc.max_ghz, soc.process_nm, soc.has_dotprod, soc.has_i8mm, soc.mainline_linux))
                .evidence(format!("npu: {}", soc.npu_note))
                .evidence(soc.notes.clone()),
        );
        match soc.mainline_linux {
            MainlineSupport::Excellent if s.is_mobile() => out.push(Finding::new("mobile.mainline_excellent", Severity::Good, Category::System).param("soc", soc.name.clone()).param("device", s.system.device_model.clone().unwrap_or_default())),
            MainlineSupport::None if s.is_arm_device() => out.push(Finding::new("mobile.mainline_none", Severity::Warning, Category::System).param("soc", soc.name.clone()).param("device", s.system.device_model.clone().unwrap_or_default())),
            _ => {}
        }
    }
    if android {
        match s.system.is_rooted {
            Some(true) => out.push(Finding::new("mobile.rooted", Severity::Info, Category::System).param("android", s.system.android_version.clone().unwrap_or_default()).evidence("su binary or Magisk/KernelSU directory present")),
            Some(false) => out.push(Finding::new("mobile.not_rooted", Severity::Info, Category::System).param("android", s.system.android_version.clone().unwrap_or_default()).evidence("no su binary, no /data/adb/magisk or /data/adb/ksu")),
            None => {}
        }
    }
}
