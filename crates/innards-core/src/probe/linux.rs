//! Linux: everything lives in /sys and /proc, no root needed for most of it.
//! Also compiled for Android, where the same sysfs layout mostly applies;
//! `probe/android.rs` then adds what only Android knows.

use super::util::{read_trim, read_u64};
use crate::snapshot::*;
use crate::soc;
use std::fs;
use std::path::Path;

pub fn fill(snap: &mut Snapshot) {
    dmi(snap);
    soc_and_chassis(snap);
    cpu(snap);
    big_little(snap);
    swap(snap);
    tmpfs(snap);
    block_devices(snap);
    gpus(snap);
    network(snap);
    fans_and_nvme_temps(snap);
    thermal_zones(snap);
    io_wait(snap);
    memory_modules(snap);
}

/// ARM SoCs expose temperatures as thermal zones, not hwmon. Fill the CPU
/// reading from a cpu-ish zone when nothing else did.
fn thermal_zones(snap: &mut Snapshot) {
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else { return };
    let mut cpu_max: Option<f32> = None;
    for e in entries.flatten() {
        let p = e.path();
        if !e.file_name().to_string_lossy().starts_with("thermal_zone") {
            continue;
        }
        let Some(kind) = read_trim(p.join("type")) else { continue };
        let Some(t) = read_u64(p.join("temp")).map(|m| m as f32 / 1000.0).filter(|t| *t > -40.0 && *t < 150.0) else { continue };
        let k = kind.to_ascii_lowercase();
        let cpuish = k.starts_with("cpu") || k.contains("cpuss") || k.contains("-cpu") || k.contains("cpu-") || k.contains("silver") || k.contains("gold") || k.contains("kryo") || k.contains("soc-thermal") || k.contains("cpu_thermal") || k.contains("x86_pkg_temp") || k.contains("apc") || k.contains("cluster");
        let gpuish = k.contains("gpu");
        if cpuish {
            cpu_max = Some(cpu_max.map_or(t, |m: f32| m.max(t)));
        }
        if gpuish && snap.thermal.gpu_c.is_none() {
            snap.thermal.gpu_c = Some(t);
        }
        // Phones expose dozens of zones; keep only the ones a reader can act on.
        if (cpuish || gpuish || k.contains("battery") || k.contains("skin")) && !snap.thermal.sensors.iter().any(|(n, _)| n == &kind) {
            snap.thermal.sensors.push((kind, t));
        }
    }
    if snap.thermal.cpu_c.is_none() {
        snap.thermal.cpu_c = cpu_max;
    }
}

/// Device-tree / SoC identification for ARM boards and phones. Runs after
/// `dmi` so a real DMI chassis type wins; only fills what is still unknown.
fn soc_and_chassis(snap: &mut Snapshot) {
    let dt = Path::new("/proc/device-tree");
    let dt_model = read_trim("/sys/firmware/devicetree/base/model")
        .or_else(|| read_trim(dt.join("model")))
        .map(|m| m.trim_end_matches('\0').to_string())
        .filter(|m| !m.is_empty());
    let hardware = read_trim("/proc/cpuinfo").and_then(|info| {
        info.lines().find(|l| l.starts_with("Hardware")).and_then(|l| l.split(':').nth(1)).map(|v| v.trim().to_string())
    });
    let soc0 = Path::new("/sys/devices/soc0");
    let hints = soc::Hints {
        hardware: hardware.or_else(|| read_trim(soc0.join("family"))),
        soc_id: read_trim(soc0.join("soc_id")).or_else(|| read_trim(soc0.join("machine"))),
        dt_compatible: fs::read(dt.join("compatible")).ok().map(|b| String::from_utf8_lossy(&b).into_owned()),
        dt_model: dt_model.clone(),
        cpu_brand: Some(snap.cpu.brand.clone()).filter(|b| !b.is_empty()),
        ..Default::default()
    };
    if snap.system.soc.is_none() {
        snap.system.soc = soc::detect(&hints);
    }
    if snap.system.device_model.is_none() {
        snap.system.device_model = dt_model.clone();
    }

    let has_dmi = Path::new("/sys/class/dmi/id").exists();
    if snap.system.chassis == Chassis::Unknown && !has_dmi && dt.exists() {
        snap.system.chassis = classify_dt(dt_model.as_deref().unwrap_or(""), snap.system.soc.as_ref(), Path::new("/sys/class/power_supply"));
    }
}

/// No DMI + a device tree = ARM board or phone. Decide which from the model
/// string, the SoC vendor, and whether there is a battery.
fn classify_dt(model: &str, soc: Option<&soc::SocInfo>, power_supply: &Path) -> Chassis {
    let m = model.to_ascii_lowercase();
    let sbc_words = ["raspberry pi", "rock", "orange pi", "orangepi", "odroid", "nanopi", "nanopc", "pine64", "rockpro", "banana pi", "libre computer", "khadas", "radxa", "beagle", "jetson", "compute module", "le potato"];
    if sbc_words.iter().any(|w| m.contains(w)) {
        return Chassis::Sbc;
    }
    if let Some(s) = soc {
        if matches!(s.vendor.as_str(), "Broadcom" | "Rockchip" | "Allwinner" | "Amlogic") {
            return Chassis::Sbc;
        }
    }
    let battery = fs::read_dir(power_supply).map(|it| it.flatten().any(|e| read_trim(e.path().join("type")).as_deref() == Some("Battery"))).unwrap_or(false);
    if battery {
        let tablet = m.split(|c: char| !c.is_alphanumeric()).any(|t| t.starts_with("tab") || t.ends_with("pad") || t == "slate");
        return if tablet { Chassis::Tablet } else { Chassis::Phone };
    }
    if soc.is_some() {
        return Chassis::Sbc;
    }
    Chassis::Unknown
}

/// ARM big.LITTLE: each cpufreq policy groups cores with the same clock; the
/// policy with the highest max frequency is the "big" cluster.
fn big_little(snap: &mut Snapshot) {
    let Ok(entries) = fs::read_dir("/sys/devices/system/cpu/cpufreq") else { return };
    let mut policies: Vec<(u64, usize)> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("policy")) {
            continue;
        }
        let Some(max) = read_u64(p.join("cpuinfo_max_freq")) else { continue };
        let cpus = read_trim(p.join("related_cpus")).map(|s| s.split_whitespace().count()).unwrap_or(0);
        if cpus > 0 {
            policies.push((max, cpus));
        }
    }
    if policies.len() < 2 {
        return;
    }
    let top = policies.iter().map(|p| p.0).max().unwrap_or(0);
    let big: usize = policies.iter().filter(|p| p.0 == top).map(|p| p.1).sum();
    let little: usize = policies.iter().filter(|p| p.0 != top).map(|p| p.1).sum();
    // One policy per core at the same clock (typical x86) is not big.LITTLE.
    if little == 0 {
        return;
    }
    snap.cpu.big_cores = Some(big);
    snap.cpu.little_cores = Some(little);
    if snap.cpu.max_mhz.is_none() || snap.cpu.max_mhz < Some(top / 1000) {
        snap.cpu.max_mhz = Some(top / 1000);
    }
    // sysinfo reports logical CPUs; on ARM every core is physical.
    if snap.cpu.physical_cores.is_none() {
        snap.cpu.physical_cores = Some(big + little);
    }
}

fn dmi(snap: &mut Snapshot) {
    let base = Path::new("/sys/class/dmi/id");
    snap.system.vendor = read_trim(base.join("sys_vendor"));
    let product = read_trim(base.join("product_name"));
    let version = read_trim(base.join("product_version")).filter(|v| !v.is_empty() && v != "None");
    snap.system.product = match (product, version) {
        // Lenovo puts the marketing name in product_version ("ThinkPad X1 Carbon 5th").
        (Some(p), Some(v)) if v.len() > p.len() => Some(v),
        (Some(p), _) => Some(p),
        (None, v) => v,
    };
    snap.system.bios_version = read_trim(base.join("bios_version"));
    snap.system.chassis = match read_u64(base.join("chassis_type")).unwrap_or(0) {
        8 | 9 | 10 | 14 => Chassis::Laptop,
        31 | 32 => Chassis::Convertible,
        3 | 4 | 6 | 7 | 13 | 15 | 16 => Chassis::Desktop,
        17 | 23 | 28 => Chassis::Server,
        35 | 34 => Chassis::MiniPc,
        30 => Chassis::Tablet,
        11 => Chassis::Phone,
        _ => Chassis::Unknown,
    };
    if snap.system.vendor.as_deref().is_some_and(|v| v.contains("QEMU") || v.contains("VMware") || v.contains("innotek")) {
        snap.system.chassis = Chassis::Vm;
    }
}

fn cpu(snap: &mut Snapshot) {
    let c0 = Path::new("/sys/devices/system/cpu/cpu0");
    snap.cpu.max_mhz = read_u64(c0.join("cpufreq/cpuinfo_max_freq")).map(|k| k / 1000);
    snap.cpu.base_mhz = read_u64(c0.join("cpufreq/base_frequency")).map(|k| k / 1000);
    snap.cpu.governor = read_trim(c0.join("cpufreq/scaling_governor"));
    snap.cpu.throttle_events = read_u64(c0.join("thermal_throttle/package_throttle_count"));
    if let Some(info) = read_trim("/proc/cpuinfo") {
        // x86 says "flags", ARM says "Features" (asimd = NEON, asimddp = dot product, i8mm = int8 matmul).
        if let Some(line) = info.lines().find(|l| l.starts_with("flags") || l.starts_with("Features")) {
            let interesting = ["avx", "avx2", "avx512f", "sse4_2", "aes", "vmx", "svm", "sha_ni", "amx_tile", "asimd", "asimddp", "asimdhp", "i8mm", "bf16", "sve", "sve2", "fphp"];
            snap.cpu.flags = line
                .split(':')
                .nth(1)
                .unwrap_or("")
                .split_whitespace()
                .filter(|f| interesting.contains(f))
                .map(String::from)
                .collect();
        }
        // ARM kernels print no "model name"; sysinfo then reports an empty or
        // "unknown" brand. Use the part name if there is one.
        if snap.cpu.brand.is_empty() || snap.cpu.brand.eq_ignore_ascii_case("unknown") {
            if let Some(part) = info.lines().find(|l| l.starts_with("CPU part")).and_then(|l| l.split(':').nth(1)) {
                snap.cpu.brand = arm_part_name(part.trim()).to_string();
            }
        }
    }
}

/// ARM "CPU part" register value → core name, for the few cores that matter here.
fn arm_part_name(part: &str) -> &'static str {
    match part.to_ascii_lowercase().as_str() {
        "0xd03" => "Cortex-A53",
        "0xd07" => "Cortex-A57",
        "0xd08" => "Cortex-A72",
        "0xd09" => "Cortex-A73",
        "0xd0a" => "Cortex-A75",
        "0xd0b" => "Cortex-A76",
        "0xd0d" => "Cortex-A77",
        "0xd41" => "Cortex-A78",
        "0xd44" => "Cortex-X1",
        "0xd05" => "Cortex-A55",
        "0xd46" => "Cortex-A510",
        "0xd47" => "Cortex-A710",
        "0xd48" => "Cortex-X2",
        "0xd4d" => "Cortex-A715",
        "0xd4e" => "Cortex-X3",
        "0xd80" => "Cortex-A520",
        "0xd81" => "Cortex-A720",
        "0xd82" => "Cortex-X4",
        "0x802" | "0x803" | "0x804" | "0x805" => "Qualcomm Kryo",
        _ => "ARM",
    }
}

fn swap(snap: &mut Snapshot) {
    let Some(text) = read_trim("/proc/swaps") else { return };
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }
        let name = cols[0].to_string();
        let kind = if name.contains("zram") {
            SwapKind::Zram
        } else if cols[1] == "file" {
            SwapKind::File
        } else if cols[1] == "partition" {
            SwapKind::Partition
        } else {
            SwapKind::Other
        };
        snap.memory.swap_backends.push(SwapBackend {
            name,
            kind,
            size_bytes: cols[2].parse::<u64>().unwrap_or(0) * 1024,
            used_bytes: cols[3].parse::<u64>().unwrap_or(0) * 1024,
            priority: cols[4].parse().unwrap_or(0),
        });
    }
}

fn tmpfs(snap: &mut Snapshot) {
    // Sum used space on tmpfs mounts that user data commonly lands on.
    let Some(mounts) = read_trim("/proc/mounts") else { return };
    let mut total = 0u64;
    for line in mounts.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 || cols[2] != "tmpfs" {
            continue;
        }
        let mp = cols[1];
        if mp == "/tmp" || mp == "/dev/shm" || mp.starts_with("/run/user") {
            if let Some(used) = statvfs_used(mp) {
                total += used;
            }
        }
    }
    snap.memory.tmpfs_used_bytes = total;
}

fn statvfs_used(path: &str) -> Option<u64> {
    // Without libc we approximate via `df`-free approach: walk is too slow,
    // so read from /proc/self/mountstats? Not available for tmpfs sizes.
    // Use the `statvfs` syscall through std::fs metadata is impossible; shell out
    // to `df -B1` which is always present on Linux.
    let out = super::util::run("df", &["-B1", "--output=used", path])?;
    out.lines().nth(1)?.trim().parse().ok()
}

fn block_devices(snap: &mut Snapshot) {
    let Ok(entries) = fs::read_dir("/sys/block") else { return };
    // On Android the OS lives on /data (a UFS/eMMC LUN); on a Linux phone it's "/".
    let root_dev = read_trim("/proc/mounts")
        .and_then(|m| {
            let find = |mp: &str| m.lines().find(|l| l.split_whitespace().nth(1) == Some(mp)).map(|l| l.split_whitespace().next().unwrap_or("").to_string());
            let dev = find("/");
            if cfg!(target_os = "android") { find("/data").or(dev) } else { dev }
        })
        .map(|d| super::util::parent_block_device(&d))
        .unwrap_or_default();
    let ufs_host = has_ufs_host();

    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") || name.starts_with("zram") || name.starts_with("sr") {
            continue;
        }
        let p = e.path();
        let size = read_u64(p.join("size")).unwrap_or(0) * 512;
        if size == 0 {
            continue;
        }
        let rotational = read_u64(p.join("queue/rotational")).unwrap_or(0) == 1;
        let removable = read_u64(p.join("removable")).unwrap_or(0) == 1;
        let dev_path = fs::canonicalize(p.join("device")).map(|c| c.to_string_lossy().to_string()).unwrap_or_default();
        // UFS shows up as SCSI disks (sda, sdb, … one per LUN) hanging off a
        // "ufshc" platform device; there is no ATA or USB in the path.
        let is_ufs = name.starts_with("sd") && (dev_path.contains("ufs") || (ufs_host && !dev_path.contains("/usb") && !dev_path.contains("/ata")));
        let bus = if dev_path.contains("/usb") {
            Bus::Usb
        } else if name.starts_with("nvme") {
            if dev_path.contains("thunderbolt") { Bus::Thunderbolt } else { Bus::Pcie }
        } else if dev_path.contains("/ata") {
            Bus::Sata
        } else if name.starts_with("mmcblk") {
            Bus::Mmc
        } else {
            Bus::Unknown
        };
        let kind = if name.starts_with("nvme") {
            StorageKind::Nvme
        } else if is_ufs {
            StorageKind::Ufs
        } else if name.starts_with("mmcblk") {
            // SD cards sit on an "mmc" host with type SD; eMMC says MMC.
            if read_trim(p.join("device/type")).as_deref() == Some("SD") || removable { StorageKind::Sd } else { StorageKind::Emmc }
        } else if rotational {
            StorageKind::Hdd
        } else {
            StorageKind::Ssd
        };
        let model = read_trim(p.join("device/model"))
            .or_else(|| read_trim(p.join("device/name")))
            .filter(|m| !m.is_empty())
            .map(|m| match read_trim(p.join("device/vendor")).filter(|v| !v.is_empty() && !m.starts_with(v.as_str())) {
                Some(v) if is_ufs || kind == StorageKind::Emmc => format!("{v} {m}"),
                _ => m,
            });
        let firmware = read_trim(p.join("device/firmware_rev")).or_else(|| read_trim(p.join("device/rev")));
        let serial = read_trim(p.join("device/serial"));
        let link_speed = if kind == StorageKind::Nvme {
            let pci = fs::canonicalize(p.join("device/device")).ok();
            pci.and_then(|pc| {
                let sp = read_trim(pc.join("current_link_speed"))?;
                let w = read_trim(pc.join("current_link_width"))?;
                Some(format!("PCIe {} x{}", sp.replace(" PCIe", ""), w))
            })
        } else {
            None
        };
        snap.storage.push(StorageDevice {
            is_system_disk: name == root_dev,
            name,
            model,
            serial,
            firmware,
            size_bytes: size,
            kind,
            bus,
            is_removable: removable || bus == Bus::Usb,
            link_speed,
            smart: None,
            temperature_c: None,
        });
    }
}

/// A bound UFS host controller driver (`ufshcd-pltfrm`, `ufshcd-qcom`, `ufs-mediatek`, …).
fn has_ufs_host() -> bool {
    let has = |dir: &str| fs::read_dir(dir).map(|it| it.flatten().any(|e| e.file_name().to_string_lossy().starts_with("ufs"))).unwrap_or(false);
    has("/sys/bus/platform/drivers") || has("/sys/bus/pci/drivers")
}

fn gpus(snap: &mut Snapshot) {
    let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") else { return };
    let ids = pci_ids();
    for e in entries.flatten() {
        let p = e.path();
        let class = read_trim(p.join("class")).unwrap_or_default();
        if !class.starts_with("0x03") {
            continue;
        }
        let vendor_id = read_trim(p.join("vendor")).unwrap_or_default();
        let device_id = read_trim(p.join("device")).unwrap_or_default();
        let vendor = match vendor_id.as_str() {
            "0x8086" => "Intel",
            "0x10de" => "NVIDIA",
            "0x1002" => "AMD",
            _ => "Unknown",
        }
        .to_string();
        let name = ids
            .as_ref()
            .and_then(|t| pci_lookup(t, &vendor_id, &device_id))
            .unwrap_or_else(|| format!("{vendor} GPU {device_id}"));
        let driver = fs::read_link(p.join("driver")).ok().and_then(|l| l.file_name().map(|f| f.to_string_lossy().to_string()));
        let is_discrete = vendor == "NVIDIA" || (vendor == "AMD" && !name.to_ascii_lowercase().contains("graphics"));
        snap.gpus.push(GpuInfo { name, vendor, is_discrete, vram_bytes: None, driver });
    }
}

fn pci_ids() -> Option<String> {
    ["/usr/share/misc/pci.ids", "/usr/share/hwdata/pci.ids", "/usr/share/pci.ids"].iter().find_map(|p| fs::read_to_string(p).ok())
}

fn pci_lookup(table: &str, vendor: &str, device: &str) -> Option<String> {
    let v = vendor.trim_start_matches("0x").to_ascii_lowercase();
    let d = device.trim_start_matches("0x").to_ascii_lowercase();
    let mut in_vendor = false;
    for line in table.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            in_vendor = line.starts_with(&v);
            continue;
        }
        if in_vendor && line.starts_with('\t') && !line.starts_with("\t\t") {
            let l = line.trim();
            if l.starts_with(&d) {
                let name = l[d.len()..].trim();
                // Strip the "[Marketing Name]" bracket into the primary name when present.
                return Some(match (name.find('['), name.rfind(']')) {
                    (Some(a), Some(b)) if b > a => name[a + 1..b].to_string(),
                    _ => name.to_string(),
                });
            }
        }
    }
    None
}

fn network(snap: &mut Snapshot) {
    let Ok(entries) = fs::read_dir("/sys/class/net") else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let p = e.path();
        let kind = if name == "lo" {
            NetKind::Loopback
        } else if p.join("wireless").exists() || p.join("phy80211").exists() {
            NetKind::Wifi
        } else if p.join("device").exists() {
            NetKind::Ethernet
        } else {
            NetKind::Virtual
        };
        if matches!(kind, NetKind::Virtual | NetKind::Loopback) {
            continue;
        }
        let is_up = read_trim(p.join("operstate")).as_deref() == Some("up");
        let link_mbps = read_trim(p.join("speed")).and_then(|s| s.parse::<i64>().ok()).filter(|s| *s > 0).map(|s| s as u32);
        snap.network.push(NetworkInterface { name, kind, is_up, link_mbps, mac: read_trim(p.join("address")) });
    }
}

fn fans_and_nvme_temps(snap: &mut Snapshot) {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = read_trim(p.join("name")).unwrap_or_default();
        if let Ok(files) = fs::read_dir(&p) {
            for f in files.flatten() {
                let fname = f.file_name().to_string_lossy().to_string();
                if fname.starts_with("fan") && fname.ends_with("_input") {
                    if let Some(rpm) = read_u64(f.path()) {
                        if rpm > 0 {
                            snap.thermal.fan_rpm.push(rpm as u32);
                        }
                    }
                }
            }
        }
        if name == "nvme" {
            // hwmon/device -> /sys/class/nvme/nvmeX ; the block device is nvmeXn1.
            let ctrl = fs::canonicalize(p.join("device")).ok().and_then(|c| c.file_name().map(|f| f.to_string_lossy().to_string()));
            let temp = read_u64(p.join("temp1_input")).map(|m| m as f32 / 1000.0);
            if let (Some(ctrl), Some(t)) = (ctrl, temp) {
                for d in &mut snap.storage {
                    if d.name.starts_with(&ctrl) {
                        d.temperature_c = Some(t);
                    }
                }
            }
        }
    }
}

fn io_wait(snap: &mut Snapshot) {
    fn sample() -> Option<(u64, u64)> {
        let s = read_trim("/proc/stat")?;
        let cpu = s.lines().next()?;
        let v: Vec<u64> = cpu.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
        if v.len() < 5 {
            return None;
        }
        Some((v.iter().sum(), v[4]))
    }
    let Some((t0, w0)) = sample() else { return };
    super::util::sleep(300);
    let Some((t1, w1)) = sample() else { return };
    if t1 > t0 {
        snap.load.io_wait_pct = Some(((w1 - w0) as f32 / (t1 - t0) as f32) * 100.0);
    }
}

fn memory_modules(snap: &mut Snapshot) {
    // DMI memory tables need root on Linux; try udev's cached properties first.
    // Fallback: note it so the report can say the RAM type is unknown.
    if let Some(out) = super::util::run("udevadm", &["info", "-q", "property", "-p", "/sys/devices/virtual/dmi/id"]) {
        let mut modules = Vec::new();
        let mut i = 0;
        loop {
            let size = out.lines().find_map(|l| l.strip_prefix(&format!("MEMORY_DEVICE_{i}_SIZE=")));
            let Some(size) = size else { break };
            let kind = out.lines().find_map(|l| l.strip_prefix(&format!("MEMORY_DEVICE_{i}_TYPE="))).map(String::from);
            let speed = out.lines().find_map(|l| l.strip_prefix(&format!("MEMORY_DEVICE_{i}_CONFIGURED_SPEED_MTS="))).and_then(|s| s.parse().ok());
            let form = out.lines().find_map(|l| l.strip_prefix(&format!("MEMORY_DEVICE_{i}_FORM_FACTOR="))).map(String::from);
            modules.push(MemoryModule { size_bytes: size.parse().unwrap_or(0), kind, speed_mts: speed, slot: form });
            i += 1;
        }
        if !modules.is_empty() {
            snap.memory.kind = modules[0].kind.clone();
            snap.memory.speed_mts = modules[0].speed_mts;
            // Soldered RAM shows up as "Row Of Chips"; anything DIMM/SODIMM-shaped is upgradeable.
            snap.memory.upgradeable = Some(modules.iter().any(|m| m.slot.as_deref().is_some_and(|f| f.contains("DIMM"))));
            snap.memory.modules = modules;
            return;
        }
    }
    snap.probe_notes.push("memory module details unavailable without elevated rights".into());
}
