//! Cross-platform probes built on `sysinfo` and `starship-battery`.

use super::util;
use crate::snapshot::*;
use sysinfo::{Components, Disks, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

pub fn fill(snap: &mut Snapshot) {
    let mut sys = System::new_with_specifics(RefreshKind::everything());
    // CPU usage needs two samples to mean anything.
    util::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.as_millis() as u64 + 50);
    sys.refresh_cpu_all();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::everything());

    // --- system
    snap.system.hostname = System::host_name();
    snap.system.os_name = System::name();
    snap.system.os_version = System::os_version();
    snap.system.kernel_version = System::kernel_version();
    snap.system.arch = System::cpu_arch();
    snap.system.uptime_secs = System::uptime();

    // --- cpu
    if let Some(cpu) = sys.cpus().first() {
        snap.cpu.brand = clean_brand(cpu.brand());
        snap.cpu.vendor = cpu.vendor_id().to_string();
        snap.cpu.current_mhz = Some(cpu.frequency());
    }
    snap.cpu.logical_cpus = sys.cpus().len();
    snap.cpu.physical_cores = System::physical_core_count();
    snap.cpu.launch_year = util::cpu_launch_year(&snap.cpu.brand);

    // --- memory
    snap.memory.total_bytes = sys.total_memory();
    snap.memory.available_bytes = sys.available_memory();
    snap.memory.used_bytes = sys.used_memory();
    snap.memory.swap_total_bytes = sys.total_swap();
    snap.memory.swap_used_bytes = sys.used_swap();

    // --- filesystems
    let disks = Disks::new_with_refreshed_list();
    for d in disks.list() {
        let mount = d.mount_point().to_string_lossy().to_string();
        let fs_type = d.file_system().to_string_lossy().to_string();
        // Skip pseudo/snap/overlay mounts that only add noise.
        if fs_type == "squashfs" || fs_type == "overlay" || mount.starts_with("/snap/") || mount.starts_with("/boot/efi") {
            continue;
        }
        snap.filesystems.push(Filesystem {
            is_root: mount == "/" || (cfg!(windows) && mount.to_ascii_uppercase().starts_with("C:")),
            mount_point: mount,
            device: d.name().to_string_lossy().to_string(),
            fs_type,
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            is_removable: d.is_removable(),
        });
    }

    // --- thermal (component sensors)
    let comps = Components::new_with_refreshed_list();
    for c in comps.list() {
        let Some(t) = c.temperature() else { continue };
        let label = c.label().to_string();
        let l = label.to_ascii_lowercase();
        if (l.contains("package") || l.contains("tdie") || l.contains("cpu")) && snap.thermal.cpu_c.is_none() {
            snap.thermal.cpu_c = Some(t);
        }
        if (l.contains("gpu") || l.contains("edge")) && snap.thermal.gpu_c.is_none() {
            snap.thermal.gpu_c = Some(t);
        }
        snap.thermal.sensors.push((label, t));
    }

    // --- load & top processes
    let la = System::load_average();
    snap.load.load_1 = la.one;
    snap.load.load_5 = la.five;
    snap.load.load_15 = la.fifteen;
    let mut procs: Vec<ProcessSummary> = sys
        .processes()
        .values()
        .map(|p| ProcessSummary {
            name: p.name().to_string_lossy().to_string(),
            rss_bytes: p.memory(),
            cpu_pct: p.cpu_usage(),
        })
        .collect();
    procs.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
    procs.truncate(10);
    snap.load.top_memory = procs;

    // --- battery (Android reads /sys/class/power_supply in probe/android.rs)
    #[cfg(not(target_os = "android"))]
    match starship_battery::Manager::new().and_then(|m| m.batteries()) {
        Ok(iter) => {
            for b in iter.flatten() {
                use starship_battery::units::energy::watt_hour;
                use starship_battery::units::ratio::percent;
                let design = b.energy_full_design().get::<watt_hour>();
                let full = b.energy_full().get::<watt_hour>();
                snap.battery = Some(BatteryInfo {
                    present: true,
                    design_wh: Some(design),
                    full_wh: Some(full),
                    health_pct: Some(b.state_of_health().get::<percent>()),
                    cycles: b.cycle_count(),
                    charge_pct: Some(b.state_of_charge().get::<percent>()),
                    on_ac: Some(!matches!(b.state(), starship_battery::State::Discharging)),
                });
                break;
            }
        }
        Err(e) => snap.probe_notes.push(format!("battery probe unavailable: {e}")),
    }
}

/// Runs after platform probes: derive anything that depends on several sources.
pub fn finalize(snap: &mut Snapshot) {
    if snap.system.chassis == Chassis::Unknown {
        snap.system.chassis = if snap.system.is_android() {
            Chassis::Phone
        } else if snap.battery.as_ref().is_some_and(|b| b.present) {
            Chassis::Laptop
        } else {
            Chassis::Desktop
        };
    }
    // Fans reported → active cooling. No fan sensors on a phone/tablet →
    // passive only. Elsewhere an idle fan reads 0 rpm and is skipped, so we
    // can't tell "no fan" from "fan idle" and leave it undetermined.
    if snap.thermal.has_active_cooling.is_none() {
        snap.thermal.has_active_cooling = if !snap.thermal.fan_rpm.is_empty() {
            Some(true)
        } else if snap.is_mobile() {
            Some(false)
        } else {
            None
        };
    }
    // big.LITTLE split from the SoC table when cpufreq didn't tell us.
    if let Some(soc) = &snap.system.soc {
        if snap.cpu.big_cores.is_none() && snap.cpu.little_cores.is_none() && soc.total_cores() as usize == snap.cpu.logical_cpus {
            snap.cpu.big_cores = Some(soc.big_cores as usize);
            snap.cpu.little_cores = Some(soc.little_cores as usize);
        }
        if snap.cpu.launch_year.is_none() {
            snap.cpu.launch_year = Some(soc.launch_year);
        }
        if snap.cpu.brand.is_empty() || snap.cpu.brand.to_ascii_lowercase().starts_with("cortex") || snap.cpu.brand.contains("unknown") {
            snap.cpu.brand = soc.name.clone();
        }
        if snap.cpu.max_mhz.is_none() {
            snap.cpu.max_mhz = Some((soc.max_ghz * 1000.0) as u64);
        }
    }
    if snap.system.product.is_none() {
        snap.system.product = snap.system.device_model.clone();
    }
    // Mark the system disk from the root filesystem's device if no platform probe did.
    if snap.system_disk().is_none() {
        if let Some(root) = snap.root_fs().map(|f| f.device.clone()) {
            let parent = util::parent_block_device(&root);
            for d in &mut snap.storage {
                if d.name == parent || root.contains(&d.name) {
                    d.is_system_disk = true;
                    break;
                }
            }
        }
    }
    snap.filesystems.sort_by(|a, b| b.is_root.cmp(&a.is_root).then(a.mount_point.cmp(&b.mount_point)));
}

/// "Intel(R) Core(TM) i7-7500U CPU @ 2.70GHz" -> "Intel Core i7-7500U".
fn clean_brand(raw: &str) -> String {
    let mut s = raw.replace("(R)", "").replace("(TM)", "").replace("(tm)", "");
    if let Some(at) = s.find('@') {
        s.truncate(at);
    }
    // Drop filler words and the "with Radeon Graphics" tail on AMD APUs.
    s.split_whitespace()
        .take_while(|w| *w != "with")
        .filter(|w| !matches!(*w, "CPU" | "Processor"))
        .collect::<Vec<_>>()
        .join(" ")
}
