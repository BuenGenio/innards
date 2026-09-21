//! The `Snapshot` is everything we learned about the machine, in one
//! serializable value. Probes fill it in; rules read it. Every field that
//! a probe might not be able to determine on a given platform is an
//! `Option` so rules can distinguish "not measured" from "zero".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub taken_at: String,
    pub system: SystemInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub storage: Vec<StorageDevice>,
    pub filesystems: Vec<Filesystem>,
    pub gpus: Vec<GpuInfo>,
    pub battery: Option<BatteryInfo>,
    pub thermal: ThermalInfo,
    pub network: Vec<NetworkInterface>,
    pub load: LoadInfo,
    /// Free-form notes from probes (e.g. "smartctl not installed").
    pub probe_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfo {
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: String,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub bios_version: Option<String>,
    pub chassis: Chassis,
    pub uptime_secs: u64,
    pub boot_count_unclean: Option<u32>,
    /// ARM system-on-chip, when recognised (phones, tablets, SBCs).
    #[serde(default)]
    pub soc: Option<crate::soc::SocInfo>,
    /// Android: whether a `su` binary / Magisk / KernelSU is present.
    /// `None` when not Android or not checked.
    #[serde(default)]
    pub is_rooted: Option<bool>,
    /// Android release ("11", "14") when running on Android.
    #[serde(default)]
    pub android_version: Option<String>,
    /// Marketing device name ("OnePlus 6T", "Raspberry Pi 5 Model B").
    #[serde(default)]
    pub device_model: Option<String>,
}

impl SystemInfo {
    /// True when the OS is Android (as opposed to a Linux distribution on the same hardware).
    pub fn is_android(&self) -> bool {
        self.android_version.is_some() || self.os_name.as_deref().is_some_and(|o| o.eq_ignore_ascii_case("android"))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Chassis {
    #[default]
    Unknown,
    Laptop,
    Desktop,
    Server,
    Convertible,
    MiniPc,
    Vm,
    Phone,
    Tablet,
    /// Single-board computer (Raspberry Pi, Rockchip boards, ...).
    Sbc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub physical_cores: Option<usize>,
    pub logical_cpus: usize,
    pub base_mhz: Option<u64>,
    pub max_mhz: Option<u64>,
    pub current_mhz: Option<u64>,
    /// Ballpark launch year, inferred from the model string when possible.
    pub launch_year: Option<u16>,
    pub flags: Vec<String>,
    pub throttle_events: Option<u64>,
    pub governor: Option<String>,
    /// big.LITTLE split on ARM, from cpufreq policies or the SoC table.
    #[serde(default)]
    pub big_cores: Option<usize>,
    #[serde(default)]
    pub little_cores: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_backends: Vec<SwapBackend>,
    /// e.g. "LPDDR3", "DDR4". Usually needs elevated rights on Linux.
    pub kind: Option<String>,
    pub speed_mts: Option<u32>,
    pub upgradeable: Option<bool>,
    pub modules: Vec<MemoryModule>,
    /// Tmpfs mounts and their usage — RAM that looks like disk.
    pub tmpfs_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryModule {
    pub size_bytes: u64,
    pub kind: Option<String>,
    pub speed_mts: Option<u32>,
    pub slot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapBackend {
    pub name: String,
    pub kind: SwapKind,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub priority: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SwapKind {
    Partition,
    File,
    Zram,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageDevice {
    pub name: String,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub size_bytes: u64,
    pub kind: StorageKind,
    pub bus: Bus,
    pub is_removable: bool,
    pub is_system_disk: bool,
    pub link_speed: Option<String>,
    pub smart: Option<SmartInfo>,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    #[default]
    Unknown,
    Nvme,
    Ssd,
    Hdd,
    Emmc,
    Sd,
    /// Universal Flash Storage: the phone-class flash that behaves like a SATA SSD.
    Ufs,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Bus {
    #[default]
    Unknown,
    Pcie,
    Sata,
    Usb,
    Thunderbolt,
    Mmc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmartInfo {
    pub healthy: Option<bool>,
    pub power_on_hours: Option<u64>,
    pub power_cycles: Option<u64>,
    pub unsafe_shutdowns: Option<u64>,
    pub percentage_used: Option<u8>,
    pub media_errors: Option<u64>,
    pub reallocated_sectors: Option<u64>,
    pub pending_sectors: Option<u64>,
    pub data_written_bytes: Option<u64>,
    pub warning_temp_minutes: Option<u64>,
    pub critical_temp_minutes: Option<u64>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Filesystem {
    pub mount_point: String,
    pub device: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub is_root: bool,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub is_discrete: bool,
    pub vram_bytes: Option<u64>,
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryInfo {
    pub present: bool,
    pub design_wh: Option<f32>,
    pub full_wh: Option<f32>,
    pub health_pct: Option<f32>,
    pub cycles: Option<u32>,
    pub charge_pct: Option<f32>,
    pub on_ac: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalInfo {
    pub cpu_c: Option<f32>,
    pub gpu_c: Option<f32>,
    pub fan_rpm: Vec<u32>,
    pub sensors: Vec<(String, f32)>,
    /// `Some(false)` when there are no fan sensors and the chassis is a
    /// phone/tablet: sustained loads will throttle. `None` = not determined.
    #[serde(default)]
    pub has_active_cooling: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInterface {
    pub name: String,
    pub kind: NetKind,
    pub is_up: bool,
    pub link_mbps: Option<u32>,
    pub mac: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetKind {
    #[default]
    Unknown,
    Ethernet,
    Wifi,
    Loopback,
    Virtual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoadInfo {
    pub load_1: f64,
    pub load_5: f64,
    pub load_15: f64,
    pub io_wait_pct: Option<f32>,
    pub top_memory: Vec<ProcessSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessSummary {
    pub name: String,
    pub rss_bytes: u64,
    pub cpu_pct: f32,
}

impl Snapshot {
    pub fn system_disk(&self) -> Option<&StorageDevice> {
        self.storage.iter().find(|d| d.is_system_disk)
    }

    pub fn root_fs(&self) -> Option<&Filesystem> {
        self.filesystems.iter().find(|f| f.is_root)
    }

    pub fn is_laptop(&self) -> bool {
        matches!(self.system.chassis, Chassis::Laptop | Chassis::Convertible)
            || (self.battery.as_ref().is_some_and(|b| b.present) && !self.is_mobile())
    }

    /// Phone or tablet: a battery-powered device with no fan and no upgrade path.
    pub fn is_mobile(&self) -> bool {
        matches!(self.system.chassis, Chassis::Phone | Chassis::Tablet)
    }

    /// Phone, tablet or single-board computer: ARM-class devices where the SoC
    /// table and mainline-Linux status matter more than DMI.
    pub fn is_arm_device(&self) -> bool {
        self.is_mobile() || self.system.chassis == Chassis::Sbc || self.system.soc.is_some()
    }
}
