//! The upgrade advisor: a short questionnaire, then ranked, costed
//! recommendations derived from the snapshot + capability scores + answers.
//! Free tier gets the logic; the app decides what to gate.

use crate::capability::{self, Grade, Workload};
use crate::snapshot::*;
use crate::soc::MainlineSupport;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const GIB: u64 = 1 << 30;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Pain {
    Slow,
    OutOfMemory,
    Storage,
    Battery,
    HeatNoise,
    Crashes,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Budget {
    // serde's snake_case gives "under100"; the questionnaire keys (and the UI) say "under_100".
    #[serde(alias = "under_100")]
    Under100,
    #[serde(alias = "under_300")]
    Under300,
    #[serde(alias = "under_800")]
    Under800,
    #[serde(alias = "under_1500")]
    Under1500,
    NoLimit,
}

impl Budget {
    fn max_usd(self) -> u32 {
        match self {
            Budget::Under100 => 100,
            Budget::Under300 => 300,
            Budget::Under800 => 800,
            Budget::Under1500 => 1500,
            Budget::NoLimit => u32::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Horizon {
    Now,
    SixMonths,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answers {
    pub uses: Vec<Workload>,
    pub pains: Vec<Pain>,
    pub budget: Budget,
    pub horizon: Horizon,
    pub needs_portability: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecKind {
    Software,
    Upgrade,
    Service,
    Replace,
    Keep,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// i18n key, e.g. `rec.ram_add`.
    pub id: String,
    pub kind: RecKind,
    pub impact: Impact,
    /// Rough USD band; the Pro tier replaces this with live regional prices.
    pub cost_usd: (u32, u32),
    pub over_budget: bool,
    pub params: Map<String, Value>,
    pub helps: Vec<Workload>,
    /// Search hints for the shopping layer: what to look for, in English.
    pub shopping_query: Option<String>,
}

impl Recommendation {
    fn new(id: &str, kind: RecKind, impact: Impact, cost: (u32, u32)) -> Self {
        Self { id: id.into(), kind, impact, cost_usd: cost, over_budget: false, params: Map::new(), helps: Vec::new(), shopping_query: None }
    }
    fn p(mut self, k: &str, v: impl Into<Value>) -> Self {
        self.params.insert(k.into(), v.into());
        self
    }
    fn helps(mut self, w: &[Workload]) -> Self {
        self.helps = w.to_vec();
        self
    }
    fn shop(mut self, q: impl Into<String>) -> Self {
        self.shopping_query = Some(q.into());
        self
    }
}

/// Questionnaire definition for the UI. Option keys are i18n keys under `advisor.*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: &'static str,
    pub multi: bool,
    pub options: Vec<&'static str>,
}

pub fn questions() -> Vec<Question> {
    vec![
        Question { id: "uses", multi: true, options: Workload::ALL.iter().map(|w| w.key()).collect() },
        Question { id: "pains", multi: true, options: vec!["slow", "out_of_memory", "storage", "battery", "heat_noise", "crashes", "none"] },
        Question { id: "budget", multi: false, options: vec!["under_100", "under_300", "under_800", "under_1500", "no_limit"] },
        Question { id: "horizon", multi: false, options: vec!["now", "six_months", "year"] },
        Question { id: "portability", multi: false, options: vec!["yes", "no"] },
    ]
}

/// RAM a set of workloads wants, in GiB, to be "Good".
fn ram_target_gib(uses: &[Workload]) -> u64 {
    uses.iter()
        .map(|w| match w {
            Workload::Everyday | Workload::PortableServer | Workload::EdgeAi => 8,
            Workload::WebDev | Workload::HomeServer | Workload::Gaming => 16,
            Workload::PhotoEditing | Workload::Containers | Workload::LocalLlm => 32,
            Workload::HeavyCompile | Workload::VideoEditing => 32,
            Workload::MlTraining => 64,
        })
        .max()
        .unwrap_or(8)
}

fn cores_target(uses: &[Workload]) -> u64 {
    uses.iter()
        .map(|w| match w {
            Workload::Everyday | Workload::HomeServer | Workload::PortableServer => 2,
            Workload::WebDev | Workload::Gaming | Workload::PhotoEditing | Workload::EdgeAi => 4,
            Workload::Containers | Workload::LocalLlm => 6,
            Workload::HeavyCompile | Workload::VideoEditing | Workload::MlTraining => 8,
        })
        .max()
        .unwrap_or(2)
}

fn wants_gpu(uses: &[Workload]) -> bool {
    uses.iter().any(|w| matches!(w, Workload::Gaming | Workload::MlTraining | Workload::LocalLlm | Workload::VideoEditing))
}

/// Uses that mean "keep this thing running services / models": the mobile
/// recommendations only make sense for these.
fn wants_server(uses: &[Workload]) -> bool {
    uses.iter().any(|w| matches!(w, Workload::PortableServer | Workload::EdgeAi | Workload::HomeServer | Workload::Containers | Workload::LocalLlm))
}

pub fn recommend(snap: &Snapshot, a: &Answers) -> Vec<Recommendation> {
    let mut recs = Vec::new();
    let caps = capability::score(snap);
    let grade_of = |w: Workload| caps.iter().find(|c| c.workload == w).map(|c| c.grade).unwrap_or(Grade::Ok);
    let laptop = snap.is_laptop();
    let ram_gib = crate::finding::ram_marketing_gb(snap.memory.total_bytes);
    let ram_want = ram_target_gib(&a.uses);
    let cores = snap.cpu.physical_cores.unwrap_or(snap.cpu.logical_cpus / 2).max(1) as u64;
    let year = chrono::Utc::now().format("%Y").to_string().parse::<u16>().unwrap_or(2026);
    let age = snap.cpu.launch_year.map(|y| year.saturating_sub(y)).unwrap_or(4) as u64;
    let has_pain = |p: Pain| a.pains.contains(&p);
    let sys_disk = snap.system_disk();

    // --- Phones, tablets and boards: the questions are "can it run Linux?" and
    // "can it stay cool?", not "which part do I swap".
    let mobile_recs = mobile(snap, a);
    let phone_not_viable = mobile_recs.iter().any(|r| r.id == "rec.phone_not_viable");
    recs.extend(mobile_recs);

    // --- Free software fixes come first: they cost nothing and often solve the pain.
    if snap.memory.swap_total_bytes == 0 {
        recs.push(Recommendation::new("rec.add_swap", RecKind::Software, Impact::High, (0, 0)).p("ram", ram_gib).helps(&a.uses));
    }
    if let Some(root) = snap.root_fs() {
        let used = 100.0 - root.available_bytes as f64 * 100.0 / root.total_bytes.max(1) as f64;
        if used >= 85.0 {
            recs.push(Recommendation::new("rec.free_space", RecKind::Software, Impact::Medium, (0, 0)).p("used_pct", used.round() as u64).p("free", crate::finding::fmt_bytes(root.available_bytes)));
        }
    }
    if snap.memory.tmpfs_used_bytes > snap.memory.total_bytes / 10 {
        recs.push(Recommendation::new("rec.tmpfs_to_disk", RecKind::Software, Impact::Medium, (0, 0)).p("tmpfs", crate::finding::fmt_bytes(snap.memory.tmpfs_used_bytes)));
    }

    // --- Storage
    if let Some(d) = sys_disk {
        let bad_health = d.smart.as_ref().is_some_and(|s| s.healthy == Some(false) || s.media_errors.unwrap_or(0) > 0 || s.reallocated_sectors.unwrap_or(0) > 0 || s.percentage_used.unwrap_or(0) >= 80);
        if d.kind == StorageKind::Hdd || bad_health || (has_pain(Pain::Storage) && d.size_bytes < 512 * GIB) {
            let target_gb = if d.size_bytes < 512 * GIB { 1000 } else { 2000 };
            let nvme = d.kind == StorageKind::Nvme || snap.storage.iter().any(|x| x.kind == StorageKind::Nvme);
            let id = if bad_health { "rec.ssd_replace_failing" } else if d.kind == StorageKind::Hdd { "rec.ssd_replace_hdd" } else { "rec.ssd_bigger" };
            let cost = if target_gb >= 2000 { (90, 160) } else { (50, 90) };
            recs.push(
                Recommendation::new(id, if bad_health { RecKind::Replace } else { RecKind::Upgrade }, if d.kind == StorageKind::Hdd || bad_health { Impact::High } else { Impact::Medium }, cost)
                    .p("model", d.model.clone().unwrap_or_else(|| d.name.clone()))
                    .p("size", crate::finding::fmt_bytes(d.size_bytes))
                    .p("target_gb", target_gb)
                    .p("interface", if nvme { "NVMe M.2" } else { "SATA 2.5\"" })
                    .helps(&[Workload::Everyday, Workload::WebDev, Workload::Containers])
                    .shop(format!("{} {}GB SSD", if nvme { "NVMe M.2 2280" } else { "2.5 inch SATA" }, target_gb)),
            );
        }
    }

    // --- RAM (a phone's RAM is soldered by definition)
    if (ram_gib < ram_want || has_pain(Pain::OutOfMemory)) && !phone_not_viable {
        let target = ram_want.max(ram_gib * 2).min(128);
        let upgradeable = if snap.is_mobile() { Some(false) } else { snap.memory.upgradeable };
        match upgradeable {
            Some(true) | None => {
                let kind = snap.memory.kind.clone().unwrap_or_else(|| "DDR4".into());
                let cost = ((target as u32 - ram_gib as u32).max(8) * 3, (target as u32 - ram_gib as u32).max(8) * 6);
                let id = if snap.memory.upgradeable.is_none() { "rec.ram_add_check" } else { "rec.ram_add" };
                recs.push(
                    Recommendation::new(id, RecKind::Upgrade, Impact::High, cost)
                        .p("have", ram_gib)
                        .p("target", target)
                        .p("kind", kind.clone())
                        .helps(&a.uses)
                        .shop(format!("{}GB {} {}", target, kind, if laptop { "SODIMM" } else { "DIMM" })),
                );
            }
            Some(false) => {
                recs.push(Recommendation::new("rec.ram_soldered", RecKind::Keep, Impact::High, (0, 0)).p("have", ram_gib).p("target", target).helps(&a.uses));
            }
        }
    }

    // --- Battery
    if laptop {
        if let Some(h) = snap.battery.as_ref().and_then(|b| b.health_pct) {
            if h < 65.0 && (a.needs_portability || has_pain(Pain::Battery)) {
                recs.push(
                    Recommendation::new("rec.battery_replace", RecKind::Service, Impact::High, (40, 120))
                        .p("health", h.round() as u64)
                        .p("model", snap.system.product.clone().unwrap_or_default())
                        .shop(format!("{} replacement battery", snap.system.product.clone().unwrap_or_else(|| "laptop".into()))),
                );
            }
        }
    }

    // --- Thermal
    let hot_disk = snap.storage.iter().any(|d| d.smart.as_ref().is_some_and(|s| s.critical_temp_minutes.unwrap_or(0) >= 30) || d.temperature_c.unwrap_or(0.0) >= 70.0);
    let hot_cpu = snap.thermal.cpu_c.unwrap_or(0.0) >= 85.0;
    if hot_cpu || hot_disk || has_pain(Pain::HeatNoise) {
        recs.push(
            Recommendation::new("rec.thermal_service", RecKind::Service, if hot_cpu { Impact::Medium } else { Impact::Low }, (0, 40))
                .p("cpu_temp", snap.thermal.cpu_c.map(|t| t.round() as i64).unwrap_or(0))
                .p("hot_disk", hot_disk)
                .shop(if laptop { "laptop thermal paste pads kit" } else { "CPU thermal paste" }),
        );
    }

    // --- GPU-bound workloads
    if wants_gpu(&a.uses) && !snap.gpus.iter().any(|g| g.is_discrete) && !snap.is_mobile() {
        let gpu_uses: Vec<Workload> = a.uses.iter().copied().filter(|w| matches!(w, Workload::Gaming | Workload::MlTraining | Workload::LocalLlm | Workload::VideoEditing)).collect();
        if laptop {
            recs.push(Recommendation::new("rec.gpu_laptop_no_slot", RecKind::Keep, Impact::High, (0, 0)).helps(&gpu_uses));
        } else {
            let vram = if gpu_uses.contains(&Workload::MlTraining) || gpu_uses.contains(&Workload::LocalLlm) { 16 } else { 8 };
            recs.push(
                Recommendation::new("rec.gpu_add", RecKind::Upgrade, Impact::High, if vram >= 16 { (450, 1000) } else { (250, 450) })
                    .p("vram", vram)
                    .helps(&gpu_uses)
                    .shop(format!("graphics card {}GB", vram)),
            );
        }
    }

    // --- Whole-machine replacement: when the ceiling, not a part, is the problem.
    let poorly_served: Vec<Workload> = a.uses.iter().copied().filter(|w| grade_of(*w) <= Grade::Poor).collect();
    let ceiling_hit = snap.memory.upgradeable == Some(false) && ram_gib < ram_want;
    let cpu_short = cores < cores_target(&a.uses);
    if !poorly_served.is_empty() && (age >= 6 || ceiling_hit || cpu_short) && !phone_not_viable && !snap.is_mobile() {
        let target_cores = cores_target(&a.uses).max(4);
        let target_ram = ram_want.max(16);
        let needs_gpu = wants_gpu(&a.uses);
        let cost = match (needs_gpu, laptop) {
            (true, true) => (1200, 2500),
            (true, false) => (900, 2000),
            (false, true) => (600, 1400),
            (false, false) => (400, 1000),
        };
        recs.push(
            Recommendation::new("rec.machine_replace", RecKind::Replace, Impact::High, cost)
                .p("age", age)
                .p("cores_now", cores)
                .p("ram_now", ram_gib)
                .p("target_cores", target_cores)
                .p("target_ram", target_ram)
                .p("needs_gpu", needs_gpu)
                .p("form", if a.needs_portability { "laptop" } else { "desktop_or_mini" })
                .helps(&poorly_served)
                .shop(format!("{} {} cores {}GB RAM{}", if a.needs_portability { "laptop" } else { "mini PC" }, target_cores, target_ram, if needs_gpu { " dedicated GPU" } else { "" })),
        );
    }

    if recs.is_empty() || recs.iter().all(|r| r.kind == RecKind::Software) {
        recs.push(Recommendation::new("rec.keep", RecKind::Keep, Impact::Low, (0, 0)).p("age", age));
    }

    // Rank: impact desc, then cheapest first; flag anything over budget.
    let max = a.budget.max_usd();
    for r in &mut recs {
        r.over_budget = r.cost_usd.0 > max;
    }
    recs.sort_by(|x, y| y.impact.cmp(&x.impact).then(x.cost_usd.0.cmp(&y.cost_usd.0)));
    recs
}

/// Recommendations specific to phones, tablets and single-board computers.
/// Returns nothing for ordinary PCs.
fn mobile(snap: &Snapshot, a: &Answers) -> Vec<Recommendation> {
    let mut recs = Vec::new();
    if !snap.is_arm_device() {
        return recs;
    }
    let server_uses: Vec<Workload> = a.uses.iter().copied().filter(|w| matches!(w, Workload::PortableServer | Workload::EdgeAi | Workload::HomeServer | Workload::Containers | Workload::LocalLlm)).collect();
    if !wants_server(&a.uses) {
        return recs;
    }
    let ram_gib = crate::finding::ram_marketing_gb(snap.memory.total_bytes);
    let soc = snap.system.soc.as_ref();
    let mainline = soc.map(|s| s.mainline_linux).unwrap_or(MainlineSupport::Unknown);
    let android = snap.system.is_android();
    let model = snap.system.device_model.clone().or_else(|| snap.system.product.clone()).unwrap_or_else(|| "this device".into());
    let soc_name = soc.map(|s| s.name.clone()).unwrap_or_else(|| snap.cpu.brand.clone());

    // Not viable: too little RAM, or no Linux path at all. Say so before anything else.
    if ram_gib < 4 || mainline == MainlineSupport::None {
        let reason = if ram_gib < 4 { "@ui/not_viable_ram" } else { "@ui/not_viable_no_mainline" };
        recs.push(
            Recommendation::new("rec.phone_not_viable", RecKind::Keep, Impact::High, (0, 0))
                .p("model", model.clone())
                .p("soc", soc_name.clone())
                .p("ram", ram_gib)
                .p("reason", reason)
                .helps(&server_uses),
        );
        return recs;
    }

    // Android with a good mainline story: the free fix is a real Linux.
    if android && matches!(mainline, MainlineSupport::Excellent | MainlineSupport::Good) {
        recs.push(
            Recommendation::new("rec.install_linux", RecKind::Software, Impact::High, (0, 0))
                .p("model", model.clone())
                .p("soc", soc_name.clone())
                .p("rooted", snap.system.is_rooted.unwrap_or(false))
                .helps(&server_uses),
        );
    }

    // Sustained load on a fanless device throttles within minutes.
    if snap.thermal.has_active_cooling == Some(false) {
        recs.push(
            Recommendation::new("rec.cooling_case", RecKind::Upgrade, Impact::Medium, (10, 40))
                .p("model", model.clone())
                .p("cpu_temp", snap.thermal.cpu_c.map(|t| t.round() as i64).unwrap_or(0))
                .helps(&server_uses)
                .shop(format!("{} cooling fan case heatsink", if snap.is_mobile() { "phone" } else { "SBC" })),
        );
    }

    // Wi-Fi only: a USB Ethernet adapter makes it a stable server.
    let ethernet = snap.network.iter().any(|n| n.kind == NetKind::Ethernet);
    let wifi = snap.network.iter().any(|n| n.kind == NetKind::Wifi);
    if !ethernet && (wifi || snap.is_mobile()) {
        recs.push(
            Recommendation::new("rec.usb_ethernet", RecKind::Upgrade, Impact::Medium, (15, 30))
                .p("model", model.clone())
                .helps(&server_uses)
                .shop("USB-C gigabit ethernet adapter"),
        );
    }

    recs
}

/// Pro tier: build vendor search links for a recommendation. Live pricing is
/// a backend concern; this gives the UI something useful offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopLink {
    pub condition: &'static str, // new | refurbished | used
    pub vendor: &'static str,
    pub url: String,
}

pub fn shop_links(rec: &Recommendation, region: &str) -> Vec<ShopLink> {
    let Some(q) = &rec.shopping_query else { return vec![] };
    let enc = urlencode(q);
    let amazon_tld = match region {
        "uk" => "co.uk", "de" => "de", "fr" => "fr", "es" => "es", "it" => "it", "ca" => "ca", "jp" => "co.jp", "au" => "com.au", _ => "com",
    };
    let ebay_tld = match region {
        "uk" => "co.uk", "de" => "de", "fr" => "fr", "es" => "es", "it" => "it", "ca" => "ca", "au" => "com.au", _ => "com",
    };
    let mut v = vec![
        ShopLink { condition: "new", vendor: "Amazon", url: format!("https://www.amazon.{amazon_tld}/s?k={enc}") },
        ShopLink { condition: "new", vendor: "Newegg", url: format!("https://www.newegg.com/p/pl?d={enc}") },
        ShopLink { condition: "used", vendor: "eBay", url: format!("https://www.ebay.{ebay_tld}/sch/i.html?_nkw={enc}&LH_ItemCondition=3000") },
    ];
    if rec.id == "rec.machine_replace" {
        v.push(ShopLink { condition: "refurbished", vendor: "Back Market", url: format!("https://www.backmarket.com/search?q={enc}") });
        v.push(ShopLink { condition: "refurbished", vendor: "eBay Refurbished", url: format!("https://www.ebay.{ebay_tld}/sch/i.html?_nkw={enc}&LH_ItemCondition=2000|2500") });
    }
    v
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
