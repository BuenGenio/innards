//! Android (Termux, adb shell): the kernel is Linux, so `probe/linux.rs`
//! already ran; this adds what only Android knows — system properties, the
//! battery (no upower), the /data partition, and whether the device is
//! rooted. Every read is guarded: SELinux hides a lot from unprivileged
//! apps, and each OEM lays out /sys a little differently.

use super::util::{read_trim, read_u64, run, which};
use crate::snapshot::*;
use crate::soc;
use std::path::Path;

pub fn fill(snap: &mut Snapshot) {
    props(snap);
    root(snap);
    battery(snap);
    storage(snap);
    if snap.system.chassis == Chassis::Unknown {
        let tablet = snap.system.device_model.as_deref().map(|m| m.to_ascii_lowercase()).is_some_and(|m| m.contains("tab") || m.contains("pad"));
        snap.system.chassis = if tablet { Chassis::Tablet } else { Chassis::Phone };
    }
}

fn getprop(key: &str) -> Option<String> {
    let bin = if Path::new("/system/bin/getprop").exists() { "/system/bin/getprop" } else { "getprop" };
    run(bin, &[key]).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn props(snap: &mut Snapshot) {
    let release = getprop("ro.build.version.release");
    let model = getprop("ro.product.model");
    let device = getprop("ro.product.device");
    let brand = getprop("ro.product.manufacturer").or_else(|| getprop("ro.product.brand"));
    let platform = getprop("ro.board.platform");
    let hardware = getprop("ro.hardware");

    snap.system.os_name = Some("Android".into());
    if let Some(r) = &release {
        snap.system.os_version = Some(r.clone());
    }
    snap.system.android_version = release;
    if snap.system.vendor.is_none() {
        snap.system.vendor = brand.clone();
    }
    // "ONEPLUS A6013" is the SKU; prefer the marketing name when the codename is readable.
    let pretty = getprop("ro.product.marketname").or_else(|| getprop("ro.product.vendor.marketname")).or_else(|| model.clone()).or_else(|| device.clone());
    if snap.system.device_model.is_none() {
        snap.system.device_model = pretty.clone();
    }
    if snap.system.product.is_none() {
        snap.system.product = pretty;
    }

    if snap.system.soc.is_none() {
        let hints = soc::Hints {
            board_platform: platform,
            ro_hardware: hardware,
            product_model: model.into_iter().chain(device).collect::<Vec<_>>().join(" ").into(),
            dt_model: snap.system.device_model.clone(),
            ..Default::default()
        };
        snap.system.soc = soc::detect(&hints);
    }
}

/// Root = any of the usual su binaries or a Magisk/KernelSU data dir. We
/// don't execute `su` (that would pop a permission prompt on the phone).
fn root(snap: &mut Snapshot) {
    let markers = ["/system/bin/su", "/system/xbin/su", "/sbin/su", "/data/adb/magisk", "/data/adb/ksu", "/data/adb/ap"];
    let rooted = markers.iter().any(|m| Path::new(m).exists()) || which("su");
    snap.system.is_rooted = Some(rooted);
}

fn battery(snap: &mut Snapshot) {
    let base = Path::new("/sys/class/power_supply/battery");
    if !base.exists() {
        snap.probe_notes.push("android: /sys/class/power_supply/battery not present".into());
        return;
    }
    let capacity = read_u64(base.join("capacity")).map(|c| c as f32);
    let cycles = read_u64(base.join("cycle_count")).map(|c| c as u32);
    let health_str = read_trim(base.join("health"));
    // Kernels report µAh (charge_*) or µWh (energy_*); either ratio gives health.
    let full = read_u64(base.join("charge_full")).or_else(|| read_u64(base.join("energy_full")));
    let design = read_u64(base.join("charge_full_design")).or_else(|| read_u64(base.join("energy_full_design")));
    let voltage_v = read_u64(base.join("voltage_now")).map(|uv| uv as f32 / 1_000_000.0).filter(|v| *v > 2.0 && *v < 6.0).unwrap_or(3.85);
    let to_wh = |uah: u64| uah as f32 / 1_000_000.0 * voltage_v;
    let health_pct = match (full, design) {
        (Some(f), Some(d)) if d > 0 => Some((f as f32 * 100.0 / d as f32).min(100.0)),
        _ => None,
    };
    let status = read_trim(base.join("status")).unwrap_or_default();
    let temp_c = read_u64(base.join("temp")).map(|t| t as f32 / 10.0);
    if let Some(t) = temp_c {
        snap.thermal.sensors.push(("battery".into(), t));
    }
    if let Some(h) = health_str {
        snap.probe_notes.push(format!("android battery health={h}"));
    }
    snap.battery = Some(BatteryInfo {
        present: true,
        design_wh: design.map(to_wh),
        full_wh: full.map(to_wh),
        health_pct,
        cycles,
        charge_pct: capacity,
        on_ac: Some(status != "Discharging"),
    });
}

/// `/data` is where the user's storage lives; the root filesystem is a
/// read-only system image and says nothing useful about free space.
fn storage(snap: &mut Snapshot) {
    let has_data = snap.filesystems.iter().any(|f| f.mount_point == "/data");
    if !has_data {
        if let Some(out) = run("df", &["-k", "/data"]) {
            // Filesystem 1K-blocks Used Available Use% Mounted on
            if let Some(line) = out.lines().nth(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 4 {
                    let total = cols[1].parse::<u64>().unwrap_or(0) * 1024;
                    let avail = cols[3].parse::<u64>().unwrap_or(0) * 1024;
                    if total > 0 {
                        snap.filesystems.push(Filesystem { mount_point: "/data".into(), device: cols[0].into(), fs_type: "f2fs/ext4".into(), total_bytes: total, available_bytes: avail, is_root: false, is_removable: false });
                    }
                }
            }
        }
    }
    // Treat /data as the "root" for the nearly-full rule: that's the disk users fill up.
    if !snap.filesystems.iter().any(|f| f.is_root) {
        if let Some(f) = snap.filesystems.iter_mut().find(|f| f.mount_point == "/data") {
            f.is_root = true;
        }
    }
    // No dm-* mapping to follow without root: the biggest non-removable flash device is the system disk.
    if snap.system_disk().is_none() {
        if let Some(d) = snap.storage.iter_mut().filter(|d| !d.is_removable && matches!(d.kind, StorageKind::Ufs | StorageKind::Emmc | StorageKind::Ssd | StorageKind::Unknown)).max_by_key(|d| d.size_bytes) {
            d.is_system_disk = true;
        }
    }
}
