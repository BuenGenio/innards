//! macOS: `system_profiler -json` and `sysctl` cover what sysinfo can't.
//! Untested on real hardware at the time of writing — every step is
//! guarded so a JSON shape change degrades to "unknown", not a crash.

use super::util::run;
use crate::snapshot::*;
use serde_json::Value;

pub fn fill(snap: &mut Snapshot) {
    snap.system.vendor = Some("Apple".into());
    if let Some(model) = run("sysctl", &["-n", "hw.model"]) {
        snap.system.product = Some(model.trim().to_string());
    }
    if let Some(cores) = run("sysctl", &["-n", "hw.physicalcpu"]).and_then(|s| s.trim().parse().ok()) {
        snap.cpu.physical_cores = Some(cores);
    }

    let Some(out) = run(
        "system_profiler",
        &["-json", "SPHardwareDataType", "SPMemoryDataType", "SPDisplaysDataType", "SPNVMeDataType", "SPSerialATADataType", "SPStorageDataType"],
    ) else {
        snap.probe_notes.push("system_profiler unavailable".into());
        return;
    };
    let Ok(v) = serde_json::from_str::<Value>(&out) else { return };

    if let Some(hw) = v.pointer("/SPHardwareDataType/0") {
        if let Some(name) = hw.get("machine_name").and_then(Value::as_str) {
            snap.system.product = Some(name.to_string());
        }
        snap.system.chassis = if hw.get("machine_name").and_then(Value::as_str).is_some_and(|n| n.contains("Book")) {
            Chassis::Laptop
        } else if hw.get("machine_name").and_then(Value::as_str).is_some_and(|n| n.contains("mini")) {
            Chassis::MiniPc
        } else {
            Chassis::Desktop
        };
        snap.system.bios_version = hw.get("boot_rom_version").and_then(Value::as_str).map(String::from);
        if let Some(chip) = hw.get("chip_type").and_then(Value::as_str) {
            if snap.cpu.brand.is_empty() {
                snap.cpu.brand = chip.to_string();
            }
            snap.cpu.launch_year = super::util::cpu_launch_year(chip);
        }
    }

    // Memory: Apple silicon is unified and soldered; report as such.
    if let Some(mem) = v.pointer("/SPMemoryDataType/0") {
        snap.memory.kind = mem.get("dimm_type").and_then(Value::as_str).map(String::from);
        snap.memory.upgradeable = Some(mem.get("SPMemoryDataType").and_then(Value::as_array).is_some_and(|a| !a.is_empty()));
    } else {
        snap.memory.upgradeable = Some(false);
    }

    for g in v.get("SPDisplaysDataType").and_then(Value::as_array).into_iter().flatten() {
        let name = g.get("sppci_model").and_then(Value::as_str).unwrap_or("GPU").to_string();
        let vendor = g.get("spdisplays_vendor").and_then(Value::as_str).unwrap_or("").to_string();
        let vram = g
            .get("spdisplays_vram")
            .or_else(|| g.get("spdisplays_vram_shared"))
            .and_then(Value::as_str)
            .and_then(parse_size);
        snap.gpus.push(GpuInfo {
            is_discrete: !name.contains("Apple") && !vendor.contains("Intel"),
            name,
            vendor,
            vram_bytes: vram,
            driver: None,
        });
    }

    for (key, kind, bus) in [("SPNVMeDataType", StorageKind::Nvme, Bus::Pcie), ("SPSerialATADataType", StorageKind::Unknown, Bus::Sata)] {
        for ctrl in v.get(key).and_then(Value::as_array).into_iter().flatten() {
            for d in ctrl.get("_items").and_then(Value::as_array).into_iter().flatten() {
                let name = d.get("bsd_name").and_then(Value::as_str).unwrap_or("disk").to_string();
                let size = d.get("size_in_bytes").and_then(Value::as_u64).unwrap_or(0);
                let is_ssd = d.get("spnvme_ssd").or_else(|| d.get("spsata_medium_type")).and_then(Value::as_str).is_some_and(|s| s.contains("SSD") || s == "Yes");
                snap.storage.push(StorageDevice {
                    is_system_disk: false,
                    name,
                    model: d.get("_name").and_then(Value::as_str).map(String::from),
                    serial: d.get("device_serial").and_then(Value::as_str).map(String::from),
                    firmware: d.get("device_revision").and_then(Value::as_str).map(String::from),
                    size_bytes: size,
                    kind: if kind == StorageKind::Unknown { if is_ssd { StorageKind::Ssd } else { StorageKind::Hdd } } else { kind },
                    bus,
                    is_removable: d.get("removable_media").and_then(Value::as_str) == Some("yes"),
                    link_speed: d.get("spnvme_linkspeed").and_then(Value::as_str).map(String::from),
                    smart: None,
                    temperature_c: None,
                });
            }
        }
    }
    // Mark the boot disk.
    if let Some(root) = v.get("SPStorageDataType").and_then(Value::as_array).and_then(|a| a.iter().find(|s| s.get("mount_point").and_then(Value::as_str) == Some("/"))) {
        let bsd = root.pointer("/physical_drive/device_name").and_then(Value::as_str).unwrap_or("");
        for d in &mut snap.storage {
            if !bsd.is_empty() && bsd.starts_with(&d.name) {
                d.is_system_disk = true;
            }
        }
    }
}

fn parse_size(s: &str) -> Option<u64> {
    let mut parts = s.split_whitespace();
    let n: f64 = parts.next()?.parse().ok()?;
    let mult = match parts.next()?.to_ascii_uppercase().as_str() {
        "MB" => 1u64 << 20,
        "GB" => 1u64 << 30,
        "TB" => 1u64 << 40,
        _ => 1,
    };
    Some((n * mult as f64) as u64)
}
