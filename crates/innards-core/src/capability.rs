//! "What is this machine still good for?" — a score per workload, derived
//! from the snapshot with simple, explainable heuristics. Scores are 0–100
//! and bucketed into a `Grade` for display.

use crate::snapshot::*;
use crate::soc::MainlineSupport;
use serde::{Deserialize, Serialize};

const GIB: u64 = 1 << 30;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Workload {
    Everyday,
    WebDev,
    HeavyCompile,
    Containers,
    HomeServer,
    VideoEditing,
    LocalLlm,
    Gaming,
    MlTraining,
    PhotoEditing,
    /// Self-hosted services on a device you carry: phone, SBC, small laptop.
    PortableServer,
    /// On-device AI: classification, embeddings/indexing, small (1–3B) models.
    EdgeAi,
}

impl Workload {
    pub const ALL: [Workload; 12] = [
        Workload::Everyday,
        Workload::WebDev,
        Workload::Containers,
        Workload::HomeServer,
        Workload::PortableServer,
        Workload::PhotoEditing,
        Workload::HeavyCompile,
        Workload::VideoEditing,
        Workload::LocalLlm,
        Workload::EdgeAi,
        Workload::Gaming,
        Workload::MlTraining,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Workload::Everyday => "everyday",
            Workload::WebDev => "web_dev",
            Workload::HeavyCompile => "heavy_compile",
            Workload::Containers => "containers",
            Workload::HomeServer => "home_server",
            Workload::VideoEditing => "video_editing",
            Workload::LocalLlm => "local_llm",
            Workload::Gaming => "gaming",
            Workload::MlTraining => "ml_training",
            Workload::PhotoEditing => "photo_editing",
            Workload::PortableServer => "portable_server",
            Workload::EdgeAi => "edge_ai",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Unsuitable,
    Poor,
    Ok,
    Good,
    Great,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub workload: Workload,
    pub score: u8,
    pub grade: Grade,
    /// i18n keys explaining the main limiting factors (max 3).
    pub limits: Vec<String>,
}

struct Facts {
    cores: f64,
    ram_gib: f64,
    fast_disk: bool,
    discrete_gpu: bool,
    vram_gib: f64,
    age: f64,
    avx2: bool,
    apple: bool,
    // --- ARM / mobile
    arm: bool,
    /// ARM dot-product (asimddp) — the int8 inference speed-up that matters for llama.cpp / onnxruntime.
    dotprod: bool,
    i8mm: bool,
    big_cores: f64,
    max_ghz: f64,
    mobile: bool,
    android: bool,
    /// Storage that behaves like an SSD for server use: NVMe, SATA SSD, UFS.
    server_disk: f64,
    battery: bool,
    active_cooling: Option<bool>,
    ethernet: bool,
    wifi: bool,
    mainline: MainlineSupport,
}

fn facts(s: &Snapshot) -> Facts {
    let year = chrono::Utc::now().format("%Y").to_string().parse::<u16>().unwrap_or(2026);
    let gpu = s.gpus.iter().find(|g| g.is_discrete);
    let arch = s.system.arch.to_ascii_lowercase();
    let arm = arch.contains("aarch64") || arch.contains("arm") || s.system.soc.is_some();
    let soc = s.system.soc.as_ref();
    let flag = |f: &str| s.cpu.flags.iter().any(|x| x == f);
    let cores = s.cpu.physical_cores.unwrap_or(if arm { s.cpu.logical_cpus } else { s.cpu.logical_cpus / 2 }).max(1) as f64;
    let big_cores = s.cpu.big_cores.or(soc.map(|c| c.big_cores as usize)).map(|b| b as f64).unwrap_or(cores);
    let max_ghz = s.cpu.max_mhz.map(|m| m as f64 / 1000.0).or(soc.map(|c| c.max_ghz as f64)).unwrap_or(if arm { 2.0 } else { 3.0 });
    Facts {
        cores,
        ram_gib: s.memory.total_bytes as f64 / GIB as f64,
        fast_disk: s.system_disk().map(|d| matches!(d.kind, StorageKind::Nvme | StorageKind::Ssd | StorageKind::Ufs)).unwrap_or(true),
        discrete_gpu: gpu.is_some(),
        vram_gib: gpu.and_then(|g| g.vram_bytes).map(|v| v as f64 / GIB as f64).unwrap_or(0.0),
        age: s.cpu.launch_year.or(soc.map(|c| c.launch_year)).map(|y| year.saturating_sub(y) as f64).unwrap_or(4.0),
        // On ARM "no AVX2" is meaningless; treat dot-product as the equivalent.
        avx2: if arm { flag("asimddp") || soc.is_some_and(|c| c.has_dotprod) || s.cpu.flags.is_empty() } else { s.cpu.flags.is_empty() || flag("avx2") },
        apple: s.cpu.brand.to_ascii_lowercase().contains("apple"),
        arm,
        dotprod: flag("asimddp") || soc.is_some_and(|c| c.has_dotprod),
        i8mm: flag("i8mm") || soc.is_some_and(|c| c.has_i8mm),
        big_cores,
        max_ghz,
        mobile: s.is_mobile(),
        android: s.system.is_android(),
        server_disk: match s.system_disk().map(|d| d.kind) {
            Some(StorageKind::Nvme) => 1.0,
            Some(StorageKind::Ssd) | Some(StorageKind::Ufs) => 0.9,
            Some(StorageKind::Emmc) => 0.5,
            Some(StorageKind::Sd) => 0.25,
            Some(StorageKind::Hdd) => 0.5,
            Some(StorageKind::Unknown) | None => 0.7,
        },
        battery: s.battery.as_ref().is_some_and(|b| b.present),
        active_cooling: s.thermal.has_active_cooling,
        ethernet: s.network.iter().any(|n| n.kind == NetKind::Ethernet),
        wifi: s.network.iter().any(|n| n.kind == NetKind::Wifi),
        mainline: soc.map(|c| c.mainline_linux).unwrap_or(if arm { MainlineSupport::Unknown } else { MainlineSupport::Excellent }),
    }
}

/// Map a value onto 0..=1 between `lo` (→0) and `hi` (→1).
fn ramp(v: f64, lo: f64, hi: f64) -> f64 {
    ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
}

pub fn score(s: &Snapshot) -> Vec<Capability> {
    let f = facts(s);
    Workload::ALL.iter().map(|w| one(*w, &f)).collect()
}

fn one(w: Workload, f: &Facts) -> Capability {
    let mut limits: Vec<(&str, f64)> = Vec::new();
    // Each workload: weighted factors in 0..1 plus limit hints when a factor is low.
    let score = match w {
        Workload::Everyday => {
            let disk = if f.fast_disk { 1.0 } else { 0.4 };
            let ram = ramp(f.ram_gib, 2.0, 8.0);
            let age = 1.0 - ramp(f.age, 8.0, 14.0);
            if !f.fast_disk { limits.push(("limit.hdd", 0.6)); }
            if f.ram_gib < 8.0 { limits.push(("limit.ram", ram)); }
            0.4 * disk + 0.35 * ram + 0.25 * age
        }
        Workload::WebDev => {
            let cores = ramp(f.cores, 1.0, 4.0);
            let ram = ramp(f.ram_gib, 4.0, 16.0);
            let disk = if f.fast_disk { 1.0 } else { 0.3 };
            if f.ram_gib < 16.0 { limits.push(("limit.ram", ram)); }
            if f.cores < 4.0 { limits.push(("limit.cores", cores)); }
            if !f.fast_disk { limits.push(("limit.hdd", 0.3)); }
            0.3 * cores + 0.4 * ram + 0.3 * disk
        }
        Workload::HeavyCompile => {
            let cores = ramp(f.cores, 2.0, 12.0);
            let ram = ramp(f.ram_gib, 8.0, 32.0);
            let disk = if f.fast_disk { 1.0 } else { 0.3 };
            if f.cores < 8.0 { limits.push(("limit.cores", cores)); }
            if f.ram_gib < 32.0 { limits.push(("limit.ram", ram)); }
            0.55 * cores + 0.3 * ram + 0.15 * disk
        }
        Workload::Containers => {
            let ram = ramp(f.ram_gib, 4.0, 20.0);
            let cores = ramp(f.cores, 1.0, 4.0);
            if f.ram_gib < 20.0 { limits.push(("limit.ram", ram)); }
            if f.cores < 4.0 { limits.push(("limit.cores", cores)); }
            0.6 * ram + 0.4 * cores
        }
        Workload::HomeServer => {
            let ram = ramp(f.ram_gib, 4.0, 16.0);
            let disk = if f.fast_disk { 1.0 } else { 0.7 };
            let cores = ramp(f.cores, 1.0, 4.0);
            if f.ram_gib < 16.0 { limits.push(("limit.ram", ram)); }
            0.5 * ram + 0.2 * disk + 0.3 * cores
        }
        Workload::PhotoEditing => {
            let ram = ramp(f.ram_gib, 8.0, 32.0);
            let cores = ramp(f.cores, 2.0, 8.0);
            let gpu = if f.discrete_gpu || f.apple { 1.0 } else { 0.6 };
            if f.ram_gib < 16.0 { limits.push(("limit.ram", ram)); }
            if !f.discrete_gpu && !f.apple { limits.push(("limit.igpu", 0.6)); }
            0.45 * ram + 0.3 * cores + 0.25 * gpu
        }
        Workload::VideoEditing => {
            let gpu = if f.discrete_gpu { ramp(f.vram_gib, 2.0, 8.0).max(0.6) } else if f.apple { 0.9 } else { 0.35 };
            let cores = ramp(f.cores, 2.0, 12.0);
            let ram = ramp(f.ram_gib, 8.0, 32.0);
            if !f.discrete_gpu && !f.apple { limits.push(("limit.igpu", gpu)); }
            if f.cores < 8.0 { limits.push(("limit.cores", cores)); }
            if f.ram_gib < 32.0 { limits.push(("limit.ram", ram)); }
            0.4 * gpu + 0.35 * cores + 0.25 * ram
        }
        Workload::LocalLlm => {
            // Model size you can run comfortably is bounded by VRAM (or unified RAM on Apple).
            let mem = if f.apple { ramp(f.ram_gib, 8.0, 64.0) } else if f.discrete_gpu { ramp(f.vram_gib, 4.0, 24.0) } else { 0.35 * ramp(f.ram_gib, 8.0, 32.0) };
            let simd = if f.avx2 { 1.0 } else { 0.3 };
            if !f.discrete_gpu && !f.apple { limits.push(("limit.no_gpu", mem)); }
            if f.discrete_gpu && f.vram_gib < 12.0 { limits.push(("limit.vram", mem)); }
            if !f.avx2 { limits.push(("limit.no_avx2", 0.3)); }
            0.8 * mem + 0.2 * simd
        }
        Workload::Gaming => {
            let gpu = if f.discrete_gpu { ramp(f.vram_gib, 4.0, 12.0).max(0.5) } else if f.apple { 0.5 } else { 0.15 };
            let cores = ramp(f.cores, 2.0, 6.0);
            let ram = ramp(f.ram_gib, 8.0, 16.0);
            if !f.discrete_gpu { limits.push(("limit.no_gpu", gpu)); }
            if f.cores < 6.0 { limits.push(("limit.cores", cores)); }
            0.65 * gpu + 0.2 * cores + 0.15 * ram
        }
        Workload::MlTraining => {
            let gpu = if f.discrete_gpu { ramp(f.vram_gib, 6.0, 24.0) } else if f.apple { 0.4 * ramp(f.ram_gib, 16.0, 64.0) } else { 0.05 };
            if !f.discrete_gpu { limits.push(("limit.no_gpu", gpu)); }
            if f.discrete_gpu && f.vram_gib < 16.0 { limits.push(("limit.vram", gpu)); }
            0.85 * gpu + 0.15 * ramp(f.ram_gib, 16.0, 64.0)
        }
        Workload::PortableServer => {
            // Self-hosted services on the go: Nextcloud/Syncthing/Home Assistant/Pi-hole class.
            // 4 GB runs a few; 6–8 GB runs a real stack; more is comfort.
            let ram = 0.5 * ramp(f.ram_gib, 2.0, 4.0) + 0.3 * ramp(f.ram_gib, 4.0, 6.0) + 0.2 * ramp(f.ram_gib, 6.0, 8.0);
            let cores = ramp(f.cores, 2.0, 6.0);
            let disk = f.server_disk;
            // A battery is a built-in UPS: a plus, not a requirement.
            let ups = if f.battery { 1.0 } else { 0.6 };
            // Can it run a real Linux? Unknown (x86 or unrecognised) is neutral.
            let linux = match f.mainline {
                MainlineSupport::Excellent => 1.0,
                MainlineSupport::Good => 0.85,
                MainlineSupport::Unknown => if f.arm { 0.6 } else { 1.0 },
                MainlineSupport::Partial => 0.45,
                MainlineSupport::None => 0.2,
            };
            let thermal = match f.active_cooling { Some(false) => 0.7, _ => 1.0 };
            let net = if f.ethernet { 1.0 } else if f.wifi { 0.75 } else { 0.5 };
            let age = 1.0 - ramp(f.age, 6.0, 12.0);
            // 7.x GiB reported is an "8 GB" device; don't nag about it.
            if f.ram_gib < 7.0 { limits.push(("limit.ram", ram)); }
            if f.cores < 4.0 { limits.push(("limit.cores", cores)); }
            if disk < 0.9 { limits.push(("limit.slow_storage", disk)); }
            if f.active_cooling == Some(false) { limits.push(("limit.no_active_cooling", 0.7)); }
            if !f.ethernet && f.wifi { limits.push(("limit.wifi_only", 0.75)); }
            match f.mainline {
                MainlineSupport::None => limits.push(("limit.no_mainline", 0.2)),
                MainlineSupport::Partial => limits.push(("limit.no_mainline", 0.45)),
                _ => {}
            }
            let raw = 0.3 * ram + 0.1 * cores + 0.15 * disk + 0.1 * ups + 0.15 * linux + 0.1 * thermal + 0.05 * net + 0.05 * age;
            // Android without a Linux path caps at Poor: no real service stack, no reboot survival.
            if f.android && matches!(f.mainline, MainlineSupport::None | MainlineSupport::Unknown | MainlineSupport::Partial) {
                limits.push(("limit.android_only", 0.44));
                raw.min(0.44)
            } else if f.android {
                limits.push(("limit.android_only", 0.64));
                raw.min(0.64)
            } else {
                raw
            }
        }
        Workload::EdgeAi => {
            // Classification, embeddings, indexing and 1–3B quantised models on CPU
            // (llama.cpp / onnxruntime). 6 GB runs them, 8 GB comfortably, 12+ well.
            let ram = 0.5 * ramp(f.ram_gib, 3.0, 6.0) + 0.3 * ramp(f.ram_gib, 6.0, 8.0) + 0.2 * ramp(f.ram_gib, 8.0, 12.0);
            let simd = if f.arm {
                if f.i8mm { 1.0 } else if f.dotprod { 0.85 } else { 0.35 }
            } else if f.avx2 { 1.0 } else { 0.3 };
            let compute = 0.6 * ramp(f.big_cores, 1.0, 4.0) + 0.4 * ramp(f.max_ghz, 1.5, 3.0);
            let thermal = match f.active_cooling { Some(false) => 0.6, _ => 1.0 };
            let gpu = if f.discrete_gpu || f.apple { 1.0 } else { 0.7 };
            if f.ram_gib < 7.0 { limits.push(("limit.ram", ram)); }
            if f.arm && !f.dotprod { limits.push(("limit.no_dotprod", 0.35)); }
            if !f.arm && !f.avx2 { limits.push(("limit.no_avx2", 0.3)); }
            if f.big_cores < 4.0 { limits.push(("limit.cores", compute)); }
            if f.active_cooling == Some(false) { limits.push(("limit.no_active_cooling", 0.6)); }
            // NPUs/DSPs (Hexagon, APU, TPU) are not usable from Linux; they add nothing here.
            // Compute matters as much as memory: a 2-core laptop with 16 GB is "Good", not "Excellent".
            let raw = 0.3 * ram + 0.15 * simd + 0.45 * compute + 0.05 * thermal + 0.05 * gpu;
            // A fanless device throttles within minutes: "Workable" is the honest ceiling.
            if f.mobile && f.active_cooling != Some(true) { raw.min(0.64) } else { raw }
        }
    };
    let score = (score * 100.0).round().clamp(0.0, 100.0) as u8;
    let grade = match score {
        0..=24 => Grade::Unsuitable,
        25..=44 => Grade::Poor,
        45..=64 => Grade::Ok,
        65..=84 => Grade::Good,
        _ => Grade::Great,
    };
    limits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    Capability {
        workload: w,
        score,
        grade,
        limits: limits.into_iter().take(3).map(|(k, _)| k.to_string()).collect(),
    }
}
