//! Windows: PowerShell + CIM for the parts sysinfo doesn't cover. No admin
//! rights needed for these classes. Untested on real hardware at the time
//! of writing; every field is optional.

use super::util::run;
use crate::snapshot::*;
use serde_json::Value;

fn cim(class: &str, props: &str) -> Option<Value> {
    let script = format!("Get-CimInstance {class} | Select-Object {props} | ConvertTo-Json -Compress -Depth 2");
    let out = run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &script])?;
    let v: Value = serde_json::from_str(out.trim()).ok()?;
    // ConvertTo-Json returns an object for a single result, array for many.
    Some(if v.is_array() { v } else { Value::Array(vec![v]) })
}

pub fn fill(snap: &mut Snapshot) {
    if let Some(cs) = cim("Win32_ComputerSystem", "Manufacturer,Model,PCSystemType").and_then(|v| v.get(0).cloned()) {
        snap.system.vendor = cs.get("Manufacturer").and_then(Value::as_str).map(String::from);
        snap.system.product = cs.get("Model").and_then(Value::as_str).map(String::from);
        snap.system.chassis = match cs.get("PCSystemType").and_then(Value::as_u64) {
            Some(2) => Chassis::Laptop,
            Some(1) | Some(3) => Chassis::Desktop,
            Some(4) | Some(5) => Chassis::Server,
            _ => Chassis::Unknown,
        };
    }
    if let Some(b) = cim("Win32_BIOS", "SMBIOSBIOSVersion").and_then(|v| v.get(0).cloned()) {
        snap.system.bios_version = b.get("SMBIOSBIOSVersion").and_then(Value::as_str).map(String::from);
    }
    if let Some(cpu) = cim("Win32_Processor", "MaxClockSpeed,NumberOfCores").and_then(|v| v.get(0).cloned()) {
        snap.cpu.max_mhz = cpu.get("MaxClockSpeed").and_then(Value::as_u64);
        if snap.cpu.physical_cores.is_none() {
            snap.cpu.physical_cores = cpu.get("NumberOfCores").and_then(Value::as_u64).map(|n| n as usize);
        }
    }

    if let Some(mods) = cim("Win32_PhysicalMemory", "Capacity,SMBIOSMemoryType,Speed,FormFactor") {
        let mut modules = Vec::new();
        for m in mods.as_array().into_iter().flatten() {
            let kind = match m.get("SMBIOSMemoryType").and_then(Value::as_u64) {
                Some(24) => Some("DDR3"), Some(26) => Some("DDR4"), Some(29) => Some("LPDDR3"),
                Some(30) => Some("LPDDR4"), Some(34) => Some("DDR5"), Some(35) => Some("LPDDR5"), _ => None,
            }
            .map(String::from);
            // FormFactor 8 = DIMM, 12 = SODIMM, 13 = Row of chips (soldered).
            let ff = m.get("FormFactor").and_then(Value::as_u64);
            modules.push(MemoryModule {
                size_bytes: m.get("Capacity").and_then(Value::as_u64).unwrap_or(0),
                kind,
                speed_mts: m.get("Speed").and_then(Value::as_u64).map(|s| s as u32),
                slot: match ff { Some(8) => Some("DIMM".into()), Some(12) => Some("SODIMM".into()), Some(13) => Some("Soldered".into()), _ => None },
            });
        }
        if !modules.is_empty() {
            snap.memory.kind = modules[0].kind.clone();
            snap.memory.speed_mts = modules[0].speed_mts;
            snap.memory.upgradeable = Some(modules.iter().any(|m| m.slot.as_deref().is_some_and(|s| s.contains("DIMM"))));
            snap.memory.modules = modules;
        }
    }

    if let Some(disks) = cim("Win32_DiskDrive", "DeviceID,Model,SerialNumber,FirmwareRevision,Size,InterfaceType,MediaType,Index") {
        for d in disks.as_array().into_iter().flatten() {
            let iface = d.get("InterfaceType").and_then(Value::as_str).unwrap_or("");
            let model = d.get("Model").and_then(Value::as_str).unwrap_or("").to_string();
            let bus = match iface { "USB" => Bus::Usb, "SCSI" | "IDE" => Bus::Sata, _ => Bus::Unknown };
            let kind = if model.to_ascii_uppercase().contains("NVME") { StorageKind::Nvme } else if model.to_ascii_uppercase().contains("SSD") { StorageKind::Ssd } else { StorageKind::Unknown };
            snap.storage.push(StorageDevice {
                is_system_disk: d.get("Index").and_then(Value::as_u64) == Some(0),
                name: d.get("DeviceID").and_then(Value::as_str).unwrap_or("").to_string(),
                model: Some(model),
                serial: d.get("SerialNumber").and_then(Value::as_str).map(|s| s.trim().to_string()),
                firmware: d.get("FirmwareRevision").and_then(Value::as_str).map(String::from),
                size_bytes: d.get("Size").and_then(Value::as_u64).unwrap_or(0),
                kind,
                bus: if kind == StorageKind::Nvme && bus == Bus::Unknown { Bus::Pcie } else { bus },
                is_removable: bus == Bus::Usb,
                link_speed: None,
                smart: None,
                temperature_c: None,
            });
        }
    }
    // MSFT_PhysicalDisk knows SSD vs HDD reliably.
    if let Some(pd) = run("powershell", &["-NoProfile", "-NonInteractive", "-Command", "Get-PhysicalDisk | Select-Object DeviceId,MediaType,BusType | ConvertTo-Json -Compress"]) {
        if let Ok(v) = serde_json::from_str::<Value>(pd.trim()) {
            let arr = if v.is_array() { v } else { Value::Array(vec![v]) };
            for p in arr.as_array().into_iter().flatten() {
                let idx = p.get("DeviceId").and_then(Value::as_str).unwrap_or("");
                let media = p.get("MediaType").and_then(Value::as_str).unwrap_or("");
                let bus = p.get("BusType").and_then(Value::as_str).unwrap_or("");
                for d in &mut snap.storage {
                    if d.name.ends_with(&format!("PHYSICALDRIVE{idx}")) {
                        if bus == "NVMe" { d.kind = StorageKind::Nvme; d.bus = Bus::Pcie; }
                        else if media == "SSD" && d.kind == StorageKind::Unknown { d.kind = StorageKind::Ssd; }
                        else if media == "HDD" { d.kind = StorageKind::Hdd; }
                        if bus == "USB" { d.bus = Bus::Usb; d.is_removable = true; }
                    }
                }
            }
        }
    }

    if let Some(gpus) = cim("Win32_VideoController", "Name,AdapterCompatibility,AdapterRAM,DriverVersion") {
        for g in gpus.as_array().into_iter().flatten() {
            let name = g.get("Name").and_then(Value::as_str).unwrap_or("GPU").to_string();
            let vendor = g.get("AdapterCompatibility").and_then(Value::as_str).unwrap_or("").to_string();
            snap.gpus.push(GpuInfo {
                is_discrete: vendor.contains("NVIDIA") || (vendor.contains("AMD") && !name.contains("Graphics")),
                name,
                vendor,
                vram_bytes: g.get("AdapterRAM").and_then(Value::as_u64),
                driver: g.get("DriverVersion").and_then(Value::as_str).map(String::from),
            });
        }
    }

    if let Some(nics) = cim("Win32_NetworkAdapter", "Name,NetEnabled,Speed,MACAddress,PhysicalAdapter,AdapterTypeId") {
        for n in nics.as_array().into_iter().flatten() {
            if n.get("PhysicalAdapter").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let name = n.get("Name").and_then(Value::as_str).unwrap_or("").to_string();
            let kind = if name.to_ascii_lowercase().contains("wi-fi") || name.to_ascii_lowercase().contains("wireless") { NetKind::Wifi } else { NetKind::Ethernet };
            snap.network.push(NetworkInterface {
                name,
                kind,
                is_up: n.get("NetEnabled").and_then(Value::as_bool).unwrap_or(false),
                link_mbps: n.get("Speed").and_then(Value::as_u64).map(|s| (s / 1_000_000) as u32).filter(|s| *s > 0),
                mac: n.get("MACAddress").and_then(Value::as_str).map(String::from),
            });
        }
    }
}
