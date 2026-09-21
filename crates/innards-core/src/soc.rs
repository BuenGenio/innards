//! System-on-chip knowledge base for phones, tablets and single-board
//! computers. ARM devices don't expose a CPU brand string the way x86 does,
//! so we match the few identifiers the kernel and Android do give us
//! (`/proc/cpuinfo` "Hardware", the device tree `compatible` string, the
//! model name, `ro.board.platform`) against a static table.
//!
//! The table is deliberately opinionated about `mainline_linux`: for a
//! pocket server the question "can this run a real Linux with an upstream
//! kernel?" matters more than any benchmark. Ratings follow the
//! postmarketOS wiki's device pages as of 2025; treat them as a guide.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "snake_case")]
pub enum MainlineSupport {
    /// Nobody has upstreamed this SoC; only vendor kernels exist.
    None,
    #[default]
    Unknown,
    /// Boots upstream, but key pieces (modem, Wi-Fi, storage) are missing or unstable.
    Partial,
    /// Daily-drivable upstream kernel with most peripherals working.
    Good,
    /// Among the best-supported ARM SoCs upstream: display, Wi-Fi, USB, storage, modem.
    Excellent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SocInfo {
    pub name: String,
    pub vendor: String,
    pub launch_year: u16,
    pub big_cores: u8,
    pub little_cores: u8,
    pub max_ghz: f32,
    pub process_nm: u8,
    pub has_neon: bool,
    /// ARMv8.2 dot-product instructions (SDOT/UDOT): the main speed-up for int8 llama.cpp / onnxruntime.
    pub has_dotprod: bool,
    /// ARMv8.6 int8 matrix-multiply (I8MM): further speed-up for quantized inference.
    pub has_i8mm: bool,
    pub gpu: String,
    /// What the NPU/DSP is and, crucially, whether Linux can use it (it usually can't).
    pub npu_note: String,
    pub mainline_linux: MainlineSupport,
    pub notes: String,
}

impl SocInfo {
    pub fn total_cores(&self) -> u8 {
        self.big_cores + self.little_cores
    }
}

/// One row of the static table. `aliases` are lower-case needles matched
/// against the detection hints; the first row whose alias matches wins,
/// so put more specific aliases (e.g. "sm8150") before generic ones.
struct Row {
    aliases: &'static [&'static str],
    name: &'static str,
    vendor: &'static str,
    year: u16,
    big: u8,
    little: u8,
    ghz: f32,
    nm: u8,
    dotprod: bool,
    i8mm: bool,
    gpu: &'static str,
    npu: &'static str,
    mainline: MainlineSupport,
    notes: &'static str,
}

const HEXAGON: &str = "Hexagon DSP: usable only through Qualcomm's Android SDK; nothing on mainline Linux.";
const NO_NPU: &str = "No NPU.";

static TABLE: &[Row] = &[
    // ---------------------------------------------------------------- Qualcomm
    Row { aliases: &["sdm845", "sda845", "snapdragon 845", "oneplus 6t", "oneplus 6", "oneplus6t", "oneplus6", "enchilada", "fajita", "pixel 3", "blueline", "crosshatch", "beryllium", "pocophone f1", "poco f1", "shift6mq", "axolotl", "dipper", "polaris"], name: "Snapdragon 845 (SDM845)", vendor: "Qualcomm", year: 2018, big: 4, little: 4, ghz: 2.8, nm: 10, dotprod: true, i8mm: false, gpu: "Adreno 630", npu: HEXAGON, mainline: MainlineSupport::Excellent, notes: "Best-supported phone SoC upstream (OnePlus 6/6T, Pixel 3, Poco F1, SHIFT6mq): display, Wi-Fi, USB, UFS, battery, and the modem via qmi all work on postmarketOS. GPU acceleration exists (freedreno) but a server doesn't need it. Kryo 385 = Cortex-A75/A55." },
    Row { aliases: &["msm8998", "sdm835", "snapdragon 835", "oneplus 5t", "oneplus 5", "pixel 2", "walleye", "taimen"], name: "Snapdragon 835 (MSM8998)", vendor: "Qualcomm", year: 2017, big: 4, little: 4, ghz: 2.45, nm: 10, dotprod: false, i8mm: false, gpu: "Adreno 540", npu: HEXAGON, mainline: MainlineSupport::Good, notes: "Upstream boots with display, USB and Wi-Fi on OnePlus 5/5T; modem support lags SDM845. Kryo 280 = Cortex-A73/A53, no dot-product instructions." },
    Row { aliases: &["sm8150", "snapdragon 855", "oneplus 7t", "oneplus 7", "hotdog", "guacamole", "guacamoleb", "hotdogb"], name: "Snapdragon 855 (SM8150)", vendor: "Qualcomm", year: 2019, big: 4, little: 4, ghz: 2.84, nm: 7, dotprod: true, i8mm: false, gpu: "Adreno 640", npu: HEXAGON, mainline: MainlineSupport::Good, notes: "OnePlus 7/7T and others boot upstream with display, USB, UFS and Wi-Fi; modem and some audio pieces are still work in progress. Kryo 485 = Cortex-A76/A55." },
    Row { aliases: &["sm8250", "snapdragon 865", "snapdragon 870", "oneplus 8t", "oneplus 8", "kebab", "instantnoodle", "instantnoodlep", "lemonades"], name: "Snapdragon 865 (SM8250)", vendor: "Qualcomm", year: 2020, big: 4, little: 4, ghz: 2.84, nm: 7, dotprod: true, i8mm: false, gpu: "Adreno 650", npu: HEXAGON, mainline: MainlineSupport::Good, notes: "OnePlus 8/8T/9R and Xiaomi devices boot upstream; display, USB, storage and Wi-Fi work, modem varies per device. Kryo 585 = Cortex-A77/A55." },
    Row { aliases: &["sm8350", "snapdragon 888", "oneplus 9 pro", "oneplus 9", "lemonade", "lemonadep"], name: "Snapdragon 888 (SM8350)", vendor: "Qualcomm", year: 2021, big: 4, little: 4, ghz: 2.84, nm: 5, dotprod: true, i8mm: true, gpu: "Adreno 660", npu: HEXAGON, mainline: MainlineSupport::Partial, notes: "Upstream boots on a few devices (OnePlus 9 series, Sony Xperia 1 III) with USB and storage; display and Wi-Fi are hit-and-miss. Kryo 680 = Cortex-X1/A78/A55." },
    Row { aliases: &["sm8450", "snapdragon 8 gen 1", "snapdragon 8 gen1", "sm8475", "8+ gen 1"], name: "Snapdragon 8 Gen 1 (SM8450)", vendor: "Qualcomm", year: 2022, big: 4, little: 4, ghz: 3.0, nm: 4, dotprod: true, i8mm: true, gpu: "Adreno 730", npu: HEXAGON, mainline: MainlineSupport::Partial, notes: "Upstream boots (Sony Xperia 1 IV, reference boards) with UFS and USB; most phone peripherals need vendor kernels. Cortex-X2/A710/A510." },
    Row { aliases: &["sm8550", "snapdragon 8 gen 2", "snapdragon 8 gen2"], name: "Snapdragon 8 Gen 2 (SM8550)", vendor: "Qualcomm", year: 2023, big: 5, little: 3, ghz: 3.2, nm: 4, dotprod: true, i8mm: true, gpu: "Adreno 740", npu: HEXAGON, mainline: MainlineSupport::Partial, notes: "Good SoC-level upstream support (Qualcomm reference boards), very little per-phone support. Cortex-X3/A715/A710/A510." },
    Row { aliases: &["sm8650", "snapdragon 8 gen 3", "snapdragon 8 gen3"], name: "Snapdragon 8 Gen 3 (SM8650)", vendor: "Qualcomm", year: 2024, big: 6, little: 2, ghz: 3.3, nm: 4, dotprod: true, i8mm: true, gpu: "Adreno 750", npu: HEXAGON, mainline: MainlineSupport::Partial, notes: "SoC upstreamed by Qualcomm/Linaro for reference boards; retail phones remain on vendor kernels. Cortex-X4/A720/A520." },
    Row { aliases: &["sdm660", "snapdragon 660", "sda660"], name: "Snapdragon 660 (SDM660)", vendor: "Qualcomm", year: 2017, big: 4, little: 4, ghz: 2.2, nm: 14, dotprod: false, i8mm: false, gpu: "Adreno 512", npu: HEXAGON, mainline: MainlineSupport::Good, notes: "Xiaomi Redmi Note 7 and Mi A2 boot upstream with display, USB, Wi-Fi and storage. Kryo 260 = Cortex-A73/A53." },
    Row { aliases: &["sdm670", "snapdragon 670", "sdm710", "snapdragon 710", "sdm712", "snapdragon 712", "pixel 3a", "sargo", "bonito"], name: "Snapdragon 670/710 (SDM670)", vendor: "Qualcomm", year: 2018, big: 2, little: 6, ghz: 2.2, nm: 10, dotprod: true, i8mm: false, gpu: "Adreno 615/616", npu: HEXAGON, mainline: MainlineSupport::Good, notes: "Pixel 3a (SDM670) boots upstream with display, USB, Wi-Fi and modem; 710 shares the platform. Kryo 360 = Cortex-A75/A55." },
    Row { aliases: &["sm7125", "snapdragon 720g", "sm7150", "snapdragon 730", "snapdragon 732g", "sm7225", "snapdragon 750g", "sm7250", "snapdragon 765", "snapdragon 768g"], name: "Snapdragon 7-series (SM7150/SM7250)", vendor: "Qualcomm", year: 2020, big: 2, little: 6, ghz: 2.4, nm: 8, dotprod: true, i8mm: false, gpu: "Adreno 618/620", npu: HEXAGON, mainline: MainlineSupport::Partial, notes: "Some Xiaomi/Google 7-series devices boot upstream with display and USB; Wi-Fi and modem usually need vendor firmware and are incomplete. Kryo 470/475 = Cortex-A76/A55." },
    Row { aliases: &["sm7325", "snapdragon 778g", "snapdragon 778", "sm7450", "snapdragon 7 gen 1", "sm7435", "snapdragon 7s gen 2", "sm7475", "snapdragon 7+ gen 2"], name: "Snapdragon 7-series (SM7325/SM7450)", vendor: "Qualcomm", year: 2021, big: 4, little: 4, ghz: 2.4, nm: 6, dotprod: true, i8mm: true, gpu: "Adreno 642L/644", npu: HEXAGON, mainline: MainlineSupport::Partial, notes: "Fairphone 5 (QCM6490, same family) has strong upstream work; most other 778G/7 Gen 1 phones are vendor-kernel only. Cortex-A78/A55." },
    Row { aliases: &["qcm6490", "qcs6490", "fairphone 5", "sc7280", "sc7180", "trogdor", "lazor"], name: "Snapdragon 7c / QCM6490 (SC7180/SC7280)", vendor: "Qualcomm", year: 2021, big: 4, little: 4, ghz: 2.7, nm: 6, dotprod: true, i8mm: true, gpu: "Adreno 643", npu: HEXAGON, mainline: MainlineSupport::Good, notes: "Chromebook lineage: Google upstreamed nearly everything, and Fairphone 5 rides on that. One of the better ARM Linux platforms." },
    // ---------------------------------------------------------------- Samsung
    Row { aliases: &["exynos9810", "exynos 9810", "universal9810", "starlte", "star2lte", "crownlte", "galaxy s9", "galaxy note9"], name: "Exynos 9810", vendor: "Samsung", year: 2018, big: 4, little: 4, ghz: 2.9, nm: 10, dotprod: false, i8mm: false, gpu: "Mali-G72 MP18", npu: NO_NPU, mainline: MainlineSupport::Partial, notes: "Galaxy S9/Note 9 boot upstream with display and USB; Wi-Fi, modem and audio are unfinished. Mongoose M3 + Cortex-A55." },
    Row { aliases: &["exynos9820", "exynos 9820", "universal9820", "beyond0lte", "beyond1lte", "beyond2lte", "galaxy s10"], name: "Exynos 9820", vendor: "Samsung", year: 2019, big: 4, little: 4, ghz: 2.73, nm: 8, dotprod: true, i8mm: false, gpu: "Mali-G76 MP12", npu: "Samsung NPU (Android-only).", mainline: MainlineSupport::Partial, notes: "Galaxy S10 line boots upstream (display, USB, storage); wireless and modem lag. Mongoose M4 + Cortex-A75/A55." },
    Row { aliases: &["exynos2100", "exynos 2100", "universal2100", "galaxy s21"], name: "Exynos 2100", vendor: "Samsung", year: 2021, big: 4, little: 4, ghz: 2.9, nm: 5, dotprod: true, i8mm: true, gpu: "Mali-G78 MP14", npu: "Samsung NPU (Android-only).", mainline: MainlineSupport::None, notes: "No meaningful upstream support; Android vendor kernels only. Cortex-X1/A78/A55." },
    Row { aliases: &["exynos2200", "exynos 2200", "universal2200", "galaxy s22", "s5e9925"], name: "Exynos 2200", vendor: "Samsung", year: 2022, big: 4, little: 4, ghz: 2.8, nm: 4, dotprod: true, i8mm: true, gpu: "Xclipse 920 (RDNA2)", npu: "Samsung NPU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-X2/A710/A510." },
    // ---------------------------------------------------------------- Google
    Row { aliases: &["gs101", "tensor g1", "google tensor", "oriole", "raven", "bluejay", "pixel 6"], name: "Tensor G1 (GS101)", vendor: "Google", year: 2021, big: 4, little: 4, ghz: 2.8, nm: 5, dotprod: true, i8mm: true, gpu: "Mali-G78 MP20", npu: "Google TPU (Android-only).", mainline: MainlineSupport::Partial, notes: "Exynos-derived; upstream boots on Pixel 6 with USB and storage, little else. Cortex-X1/A76/A55." },
    Row { aliases: &["gs201", "tensor g2", "cheetah", "panther", "lynx", "pixel 7"], name: "Tensor G2 (GS201)", vendor: "Google", year: 2022, big: 4, little: 4, ghz: 2.85, nm: 5, dotprod: true, i8mm: true, gpu: "Mali-G710 MP7", npu: "Google TPU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-X1/A78/A55." },
    Row { aliases: &["zuma", "tensor g3", "shiba", "husky", "pixel 8"], name: "Tensor G3 (Zuma)", vendor: "Google", year: 2023, big: 5, little: 4, ghz: 2.91, nm: 4, dotprod: true, i8mm: true, gpu: "Mali-G715 MP7", npu: "Google TPU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-X3/A715/A510." },
    Row { aliases: &["zumapro", "tensor g4", "tokay", "caiman", "komodo", "pixel 9"], name: "Tensor G4 (Zuma Pro)", vendor: "Google", year: 2024, big: 4, little: 4, ghz: 3.1, nm: 4, dotprod: true, i8mm: true, gpu: "Mali-G715 MP7", npu: "Google TPU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-X4/A720/A520." },
    // ---------------------------------------------------------------- MediaTek
    Row { aliases: &["mt6833", "dimensity 700", "dimensity700"], name: "Dimensity 700 (MT6833)", vendor: "MediaTek", year: 2020, big: 2, little: 6, ghz: 2.2, nm: 7, dotprod: true, i8mm: false, gpu: "Mali-G57 MC2", npu: "MediaTek APU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-A76/A55." },
    Row { aliases: &["mt6877", "dimensity 900", "dimensity900", "dimensity 920", "dimensity 930"], name: "Dimensity 900 (MT6877)", vendor: "MediaTek", year: 2021, big: 2, little: 6, ghz: 2.4, nm: 6, dotprod: true, i8mm: false, gpu: "Mali-G68 MC4", npu: "MediaTek APU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-A78/A55." },
    Row { aliases: &["mt6893", "dimensity 1200", "dimensity1200", "dimensity 1100", "mt6891"], name: "Dimensity 1200 (MT6893)", vendor: "MediaTek", year: 2021, big: 4, little: 4, ghz: 3.0, nm: 6, dotprod: true, i8mm: false, gpu: "Mali-G77 MC9", npu: "MediaTek APU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-A78/A55." },
    Row { aliases: &["mt6895", "dimensity 8100", "dimensity8100", "dimensity 8000", "dimensity 8200"], name: "Dimensity 8100 (MT6895)", vendor: "MediaTek", year: 2022, big: 4, little: 4, ghz: 2.85, nm: 5, dotprod: true, i8mm: true, gpu: "Mali-G610 MC6", npu: "MediaTek APU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-A78/A55." },
    Row { aliases: &["mt6985", "dimensity 9200", "dimensity9200"], name: "Dimensity 9200 (MT6985)", vendor: "MediaTek", year: 2023, big: 4, little: 4, ghz: 3.05, nm: 4, dotprod: true, i8mm: true, gpu: "Immortalis-G715 MC11", npu: "MediaTek APU (Android-only).", mainline: MainlineSupport::None, notes: "Vendor kernels only. Cortex-X3/A715/A510." },
    Row { aliases: &["mt6789", "helio g99", "helio g96", "heliog99"], name: "Helio G99 (MT6789)", vendor: "MediaTek", year: 2022, big: 2, little: 6, ghz: 2.2, nm: 6, dotprod: true, i8mm: false, gpu: "Mali-G57 MC2", npu: NO_NPU, mainline: MainlineSupport::None, notes: "Common in budget tablets and phones; vendor kernels only. Cortex-A76/A55." },
    // ---------------------------------------------------------------- Apple (for completeness; no Linux path)
    Row { aliases: &["apple a12", "t8020", "iphone xs", "iphone xr"], name: "Apple A12 Bionic", vendor: "Apple", year: 2018, big: 2, little: 4, ghz: 2.49, nm: 7, dotprod: true, i8mm: false, gpu: "Apple 4-core", npu: "Neural Engine (iOS-only).", mainline: MainlineSupport::None, notes: "No Linux for iPhones/iPads; checkra1n-era projects never reached usability." },
    Row { aliases: &["apple a13", "t8030", "iphone 11"], name: "Apple A13 Bionic", vendor: "Apple", year: 2019, big: 2, little: 4, ghz: 2.65, nm: 7, dotprod: true, i8mm: false, gpu: "Apple 4-core", npu: "Neural Engine (iOS-only).", mainline: MainlineSupport::None, notes: "No Linux path." },
    Row { aliases: &["apple a14", "t8101", "iphone 12"], name: "Apple A14 Bionic", vendor: "Apple", year: 2020, big: 2, little: 4, ghz: 3.0, nm: 5, dotprod: true, i8mm: false, gpu: "Apple 4-core", npu: "Neural Engine (iOS-only).", mainline: MainlineSupport::None, notes: "No Linux path." },
    Row { aliases: &["apple a15", "t8110", "iphone 13", "iphone 14"], name: "Apple A15 Bionic", vendor: "Apple", year: 2021, big: 2, little: 4, ghz: 3.23, nm: 5, dotprod: true, i8mm: true, gpu: "Apple 5-core", npu: "Neural Engine (iOS-only).", mainline: MainlineSupport::None, notes: "No Linux path." },
    Row { aliases: &["apple a16", "t8120", "iphone 14 pro", "iphone 15"], name: "Apple A16 Bionic", vendor: "Apple", year: 2022, big: 2, little: 4, ghz: 3.46, nm: 4, dotprod: true, i8mm: true, gpu: "Apple 5-core", npu: "Neural Engine (iOS-only).", mainline: MainlineSupport::None, notes: "No Linux path." },
    Row { aliases: &["apple a17", "t8130", "iphone 15 pro"], name: "Apple A17 Pro", vendor: "Apple", year: 2023, big: 2, little: 4, ghz: 3.78, nm: 3, dotprod: true, i8mm: true, gpu: "Apple 6-core", npu: "Neural Engine (iOS-only).", mainline: MainlineSupport::None, notes: "No Linux path." },
    // ---------------------------------------------------------------- Single-board computers
    Row { aliases: &["bcm2711", "raspberry pi 4", "raspberry pi 400", "raspberry pi compute module 4", "rpi4"], name: "BCM2711 (Raspberry Pi 4)", vendor: "Broadcom", year: 2019, big: 0, little: 4, ghz: 1.8, nm: 28, dotprod: false, i8mm: false, gpu: "VideoCore VI", npu: NO_NPU, mainline: MainlineSupport::Excellent, notes: "Fully upstream; the reference small server. Cortex-A72, LPDDR4, boots from SD or USB SSD (no PCIe slot)." },
    Row { aliases: &["bcm2712", "raspberry pi 5", "rpi5", "raspberry pi compute module 5"], name: "BCM2712 (Raspberry Pi 5)", vendor: "Broadcom", year: 2023, big: 0, little: 4, ghz: 2.4, nm: 16, dotprod: true, i8mm: false, gpu: "VideoCore VII", npu: NO_NPU, mainline: MainlineSupport::Excellent, notes: "Upstream support landed within a year of launch; PCIe 2.0 x1 for NVMe. Cortex-A76." },
    Row { aliases: &["rk3399", "rockpro64", "pinebook pro", "rock pi 4", "rockpi4", "nanopc-t4", "nanopi m4"], name: "RK3399", vendor: "Rockchip", year: 2016, big: 2, little: 4, ghz: 2.0, nm: 28, dotprod: false, i8mm: false, gpu: "Mali-T860 MP4", npu: NO_NPU, mainline: MainlineSupport::Excellent, notes: "Long-standing upstream support, PCIe x4 for NVMe (ROCKPro64). Cortex-A72/A53." },
    Row { aliases: &["rk3588", "rk3588s", "rock 5b", "rock 5a", "rock5b", "orange pi 5", "orangepi 5", "nanopc-t6", "turing rk1", "khadas edge2"], name: "RK3588", vendor: "Rockchip", year: 2022, big: 4, little: 4, ghz: 2.4, nm: 8, dotprod: true, i8mm: false, gpu: "Mali-G610 MP4", npu: "6 TOPS NPU; usable on Linux only via Rockchip's out-of-tree rknpu driver and rknn toolkit.", mainline: MainlineSupport::Good, notes: "Upstream boots with USB, PCIe/NVMe, Ethernet and display; HDMI/video codec bits arrived later. The strongest ARM SBC for a home server. Cortex-A76/A55." },
    Row { aliases: &["allwinner h6", "sun50i-h6", "sun50i h6", "orange pi 3", "orangepi 3", "orange pi one plus", "pine h64", "tanix tx6"], name: "Allwinner H6", vendor: "Allwinner", year: 2017, big: 0, little: 4, ghz: 1.8, nm: 28, dotprod: false, i8mm: false, gpu: "Mali-T720 MP2", npu: NO_NPU, mainline: MainlineSupport::Excellent, notes: "Upstream via the sunxi community: USB 3, Gigabit Ethernet, PCIe (with quirks). Cortex-A53; typically 1–3 GB RAM." },
    Row { aliases: &["s905x", "s905d", "s905w", "s905", "amlogic s905", "meson-gxl", "meson-gxbb", "le potato", "libretech", "odroid-c2", "khadas vim"], name: "Amlogic S905 family", vendor: "Amlogic", year: 2016, big: 0, little: 4, ghz: 1.5, nm: 28, dotprod: false, i8mm: false, gpu: "Mali-450 MP3", npu: NO_NPU, mainline: MainlineSupport::Excellent, notes: "TV-box SoC with excellent upstream support (Le Potato, ODROID-C2). Cortex-A53; 1–2 GB RAM is the usual limit." },
    Row { aliases: &["s922x", "amlogic s922", "meson-g12b", "odroid-n2", "odroid n2", "khadas vim3", "a311d"], name: "Amlogic S922X / A311D", vendor: "Amlogic", year: 2019, big: 4, little: 2, ghz: 2.2, nm: 12, dotprod: false, i8mm: false, gpu: "Mali-G52 MP6", npu: "A311D has a 5 TOPS NPU with an experimental upstream driver (etnaviv/VeriSilicon); S922X has none.", mainline: MainlineSupport::Excellent, notes: "ODROID-N2/N2+ and Khadas VIM3: fully upstream, up to 4 GB RAM. Cortex-A73/A53." },
];

/// Everything a probe managed to read that might name the SoC. All fields
/// optional; matching is case-insensitive substring search.
#[derive(Debug, Clone, Default)]
pub struct Hints {
    /// `/proc/cpuinfo` "Hardware :" line (vendor kernels) or `/sys/devices/soc0/family`.
    pub hardware: Option<String>,
    /// `/sys/devices/soc0/soc_id` or `/sys/devices/soc0/machine`.
    pub soc_id: Option<String>,
    /// `/proc/device-tree/compatible` — NUL-separated, e.g. "oneplus,fajita\0qcom,sdm845".
    pub dt_compatible: Option<String>,
    /// `/sys/firmware/devicetree/base/model` or `/proc/device-tree/model`.
    pub dt_model: Option<String>,
    /// Android `ro.board.platform` ("sdm845").
    pub board_platform: Option<String>,
    /// Android `ro.hardware` ("qcom") — only useful for the vendor.
    pub ro_hardware: Option<String>,
    /// Android `ro.product.model` ("ONEPLUS A6013") / `ro.product.device` ("OnePlus6T").
    pub product_model: Option<String>,
    /// Free-form: the CPU brand string sysinfo produced, if any.
    pub cpu_brand: Option<String>,
}

impl Hints {
    fn strings(&self) -> Vec<String> {
        // Order matters: platform identifiers first, marketing names last, so
        // "sdm845" wins over a device name that could map to several SoCs.
        [&self.board_platform, &self.soc_id, &self.hardware, &self.dt_compatible, &self.cpu_brand, &self.dt_model, &self.product_model, &self.ro_hardware]
            .into_iter()
            .flatten()
            .map(|s| s.replace('\0', " ").to_ascii_lowercase())
            .filter(|s| !s.trim().is_empty())
            .collect()
    }
}

fn row_to_info(r: &Row) -> SocInfo {
    SocInfo {
        name: r.name.into(),
        vendor: r.vendor.into(),
        launch_year: r.year,
        big_cores: r.big,
        little_cores: r.little,
        max_ghz: r.ghz,
        process_nm: r.nm,
        has_neon: true,
        has_dotprod: r.dotprod,
        has_i8mm: r.i8mm,
        gpu: r.gpu.into(),
        npu_note: r.npu.into(),
        mainline_linux: r.mainline,
        notes: r.notes.into(),
    }
}

/// Fuzzy lookup: every hint string is searched for every alias; the best
/// (longest) alias match wins so "snapdragon 8 gen 1" beats "snapdragon 8".
pub fn detect(hints: &Hints) -> Option<SocInfo> {
    let strings = hints.strings();
    if strings.is_empty() {
        return None;
    }
    let mut best: Option<(usize, usize, &Row)> = None; // (hint index, alias length, row)
    for (i, s) in strings.iter().enumerate() {
        let compact = s.replace(['-', '_'], " ");
        for r in TABLE {
            for a in r.aliases {
                if s.contains(a) || compact.contains(a) {
                    let better = match best {
                        None => true,
                        Some((bi, bl, _)) => i < bi || (i == bi && a.len() > bl),
                    };
                    if better {
                        best = Some((i, a.len(), r));
                    }
                }
            }
        }
    }
    best.map(|(_, _, r)| row_to_info(r))
}

/// Lookup by exact table name or any alias; handy for tests and fixtures.
pub fn by_name(needle: &str) -> Option<SocInfo> {
    let n = needle.to_ascii_lowercase();
    TABLE.iter().find(|r| r.name.to_ascii_lowercase() == n || r.aliases.contains(&n.as_str())).map(row_to_info)
}

/// All known SoCs, for documentation or a picker UI.
pub fn all() -> Vec<SocInfo> {
    TABLE.iter().map(row_to_info).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdm845_from_every_source() {
        for h in [
            Hints { hardware: Some("Qualcomm Technologies, Inc SDM845".into()), ..Default::default() },
            Hints { board_platform: Some("sdm845".into()), ..Default::default() },
            Hints { dt_compatible: Some("oneplus,fajita\0qcom,sdm845".into()), ..Default::default() },
            Hints { dt_model: Some("OnePlus 6T".into()), ..Default::default() },
            Hints { product_model: Some("ONEPLUS A6013".into()), ro_hardware: Some("qcom".into()), dt_model: Some("OnePlus6T".into()), ..Default::default() },
        ] {
            let s = detect(&h).unwrap_or_else(|| panic!("no match for {h:?}"));
            assert!(s.name.contains("845"), "{h:?} -> {}", s.name);
            assert_eq!(s.mainline_linux, MainlineSupport::Excellent);
        }
    }

    #[test]
    fn longest_alias_wins_within_a_hint() {
        let h = Hints { hardware: Some("Snapdragon 8 Gen 1".into()), ..Default::default() };
        assert!(detect(&h).unwrap().name.contains("SM8450"));
        let h = Hints { dt_model: Some("Raspberry Pi 5 Model B Rev 1.0".into()), ..Default::default() };
        assert!(detect(&h).unwrap().name.contains("BCM2712"));
        let h = Hints { dt_model: Some("Raspberry Pi 4 Model B".into()), ..Default::default() };
        assert!(detect(&h).unwrap().name.contains("BCM2711"));
    }

    #[test]
    fn platform_hint_beats_model_hint() {
        // A device tree saying sm8150 with a confusing model string still yields the 855.
        let h = Hints { dt_compatible: Some("qcom,sm8150".into()), dt_model: Some("OnePlus 6".into()), ..Default::default() };
        assert!(detect(&h).unwrap().name.contains("855"));
    }

    #[test]
    fn unknown_yields_none() {
        assert!(detect(&Hints::default()).is_none());
        assert!(detect(&Hints { cpu_brand: Some("Intel Core i7-7500U".into()), ..Default::default() }).is_none());
    }

    #[test]
    fn table_is_sane() {
        for s in all() {
            assert!(s.total_cores() > 0, "{}", s.name);
            assert!(s.max_ghz > 0.5 && s.max_ghz < 5.0, "{}", s.name);
            assert!(s.launch_year >= 2015, "{}", s.name);
        }
        assert!(by_name("sdm845").is_some());
    }
}
