//! Hand-built snapshots of a rooted OnePlus 6T (Snapdragon 845, 8 GB, UFS)
//! — once on postmarketOS, once on stock Android — run through rules,
//! capability scoring and the advisor. This is the "is my old phone a
//! viable pocket server?" question, pinned down.

use innards_core::advisor::{self, Answers, Budget, Horizon, Pain};
use innards_core::capability::{self, Grade, Workload};
use innards_core::snapshot::*;
use innards_core::soc::{self, MainlineSupport};
use innards_core::{report, rules, Level};

const GIB: u64 = 1 << 30;

fn oneplus_6t_base() -> Snapshot {
    let soc = soc::by_name("sdm845").expect("SDM845 in table");
    assert_eq!(soc.mainline_linux, MainlineSupport::Excellent);
    Snapshot {
        taken_at: "2026-09-21T12:00:00Z".into(),
        system: SystemInfo {
            hostname: Some("fajita".into()),
            arch: "aarch64".into(),
            chassis: Chassis::Phone,
            device_model: Some("OnePlus 6T".into()),
            product: Some("OnePlus 6T".into()),
            vendor: Some("OnePlus".into()),
            uptime_secs: 3600,
            soc: Some(soc),
            ..Default::default()
        },
        cpu: CpuInfo {
            brand: "Snapdragon 845 (SDM845)".into(),
            vendor: "Qualcomm".into(),
            physical_cores: Some(8),
            logical_cpus: 8,
            max_mhz: Some(2803),
            current_mhz: Some(1766),
            launch_year: Some(2018),
            flags: vec!["asimd".into(), "aes".into(), "asimddp".into(), "fphp".into()],
            big_cores: Some(4),
            little_cores: Some(4),
            ..Default::default()
        },
        memory: MemoryInfo {
            total_bytes: (7.7 * GIB as f64) as u64,
            available_bytes: 5 * GIB,
            used_bytes: 2 * GIB,
            swap_total_bytes: 2 * GIB,
            swap_backends: vec![SwapBackend { name: "/dev/zram0".into(), kind: SwapKind::Zram, size_bytes: 2 * GIB, used_bytes: 0, priority: 100 }],
            upgradeable: Some(false),
            kind: Some("LPDDR4X".into()),
            ..Default::default()
        },
        storage: vec![StorageDevice {
            name: "sda".into(),
            model: Some("SAMSUNG KLUDG4U1EA-B0C1".into()),
            size_bytes: 128 * 1_000_000_000,
            kind: StorageKind::Ufs,
            bus: Bus::Unknown,
            is_system_disk: true,
            ..Default::default()
        }],
        filesystems: vec![Filesystem { mount_point: "/".into(), device: "/dev/sda17".into(), fs_type: "ext4".into(), total_bytes: 110 * 1_000_000_000, available_bytes: 80 * 1_000_000_000, is_root: true, is_removable: false }],
        gpus: vec![],
        battery: Some(BatteryInfo { present: true, design_wh: Some(14.2), full_wh: Some(12.1), health_pct: Some(85.0), cycles: Some(420), charge_pct: Some(85.0), on_ac: Some(true) }),
        thermal: ThermalInfo { cpu_c: Some(41.0), gpu_c: None, fan_rpm: vec![], sensors: vec![("cpu-1-0".into(), 41.0)], has_active_cooling: Some(false) },
        network: vec![NetworkInterface { name: "wlan0".into(), kind: NetKind::Wifi, is_up: true, link_mbps: None, mac: None }],
        load: LoadInfo { load_1: 0.3, load_5: 0.2, load_15: 0.1, io_wait_pct: Some(0.5), top_memory: vec![] },
        probe_notes: vec![],
    }
}

/// postmarketOS: a real Linux, mainline kernel.
fn oneplus_6t_pmos() -> Snapshot {
    let mut s = oneplus_6t_base();
    s.system.os_name = Some("postmarketOS".into());
    s.system.os_version = Some("v25.06".into());
    s.system.kernel_version = Some("6.12.0".into());
    s
}

/// Stock Android 11, bootloader locked, no root.
fn oneplus_6t_android_unrooted() -> Snapshot {
    let mut s = oneplus_6t_base();
    s.system.os_name = Some("Android".into());
    s.system.os_version = Some("11".into());
    s.system.android_version = Some("11".into());
    s.system.kernel_version = Some("4.9.227".into());
    s.system.is_rooted = Some(false);
    s
}

fn server_answers() -> Answers {
    Answers { uses: vec![Workload::PortableServer, Workload::EdgeAi], pains: vec![Pain::None], budget: Budget::Under300, horizon: Horizon::Now, needs_portability: true }
}

fn grade(snap: &Snapshot, w: Workload) -> Grade {
    capability::score(snap).into_iter().find(|c| c.workload == w).map(|c| c.grade).unwrap()
}

#[test]
fn oneplus_6t_on_postmarketos_is_a_viable_pocket_server() {
    let snap = oneplus_6t_pmos();
    assert!(snap.is_mobile());
    assert!(!snap.is_laptop(), "a phone is not a laptop for advisor purposes");

    // Capabilities: good portable server; AI is workable but capped by passive cooling.
    let ps = grade(&snap, Workload::PortableServer);
    assert!(ps >= Grade::Good, "PortableServer graded {ps:?}");
    assert_eq!(grade(&snap, Workload::EdgeAi), Grade::Ok, "EdgeAi should be Workable (fanless cap)");
    let caps = capability::score(&snap);
    let ai = caps.iter().find(|c| c.workload == Workload::EdgeAi).unwrap();
    assert!(ai.limits.iter().any(|l| l == "limit.no_active_cooling"), "limits: {:?}", ai.limits);
    let srv = caps.iter().find(|c| c.workload == Workload::PortableServer).unwrap();
    assert!(!srv.limits.iter().any(|l| l == "limit.ram"), "8 GB should not be flagged as low: {:?}", srv.limits);

    // Findings.
    let ids: Vec<String> = rules::run(&snap).into_iter().map(|f| f.id).collect();
    for want in ["mobile.soc", "mobile.mainline_excellent", "battery.as_ups", "thermal.no_active_cooling", "storage.ufs", "network.wifi_only"] {
        assert!(ids.iter().any(|i| i == want), "missing finding {want}; got {ids:?}");
    }
    for unwanted in ["mobile.not_rooted", "mobile.rooted", "cpu.no_avx2", "mobile.mainline_none"] {
        assert!(!ids.iter().any(|i| i == unwanted), "unexpected finding {unwanted}");
    }

    // Advisor.
    let recs = advisor::recommend(&snap, &server_answers());
    let rec_ids: Vec<&str> = recs.iter().map(|r| r.id.as_str()).collect();
    assert!(rec_ids.contains(&"rec.usb_ethernet"), "{rec_ids:?}");
    assert!(rec_ids.contains(&"rec.cooling_case"), "{rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.phone_not_viable"), "{rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.install_linux"), "already on Linux: {rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.machine_replace"), "{rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.ram_add"), "phone RAM is soldered: {rec_ids:?}");
    assert!(recs.iter().all(|r| !r.over_budget), "everything should fit under $300");

    // Rendering in both languages must not leave any ⟨missing⟩ keys or raw @refs.
    let rep = report::build(snap);
    for lang in ["en", "es"] {
        let rendered = report::render(&rep, lang, Level::Informed);
        let md = report::to_markdown(&rendered);
        assert!(!md.contains('⟨'), "{lang}: unresolved key in\n{md}");
        assert!(!md.contains("@mainline"), "{lang}: unresolved @ref in\n{md}");
        let soc_line = rendered.findings.iter().find(|f| f.id == "mobile.soc").unwrap();
        assert!(soc_line.body.contains("845"), "{lang}: {}", soc_line.body);
        let recs_r = report::render_recs(&recs, lang);
        assert!(recs_r.iter().all(|r| !r.title.contains('⟨') && !r.body.contains('⟨')));
        assert!(rendered.summary.storage.contains("UFS"));
    }
}

#[test]
fn oneplus_6t_on_stock_android_needs_linux() {
    let snap = oneplus_6t_android_unrooted();
    assert!(snap.system.is_android());

    let ids: Vec<String> = rules::run(&snap).into_iter().map(|f| f.id).collect();
    assert!(ids.iter().any(|i| i == "mobile.not_rooted"), "{ids:?}");
    assert!(ids.iter().any(|i| i == "mobile.soc"), "{ids:?}");
    assert!(!ids.iter().any(|i| i == "mobile.rooted"));

    // Android caps the server grade until a real Linux is installed.
    assert!(grade(&snap, Workload::PortableServer) <= Grade::Ok);
    let caps = capability::score(&snap);
    let srv = caps.iter().find(|c| c.workload == Workload::PortableServer).unwrap();
    assert!(srv.limits.iter().any(|l| l == "limit.android_only"), "{:?}", srv.limits);

    let recs = advisor::recommend(&snap, &server_answers());
    let rec_ids: Vec<&str> = recs.iter().map(|r| r.id.as_str()).collect();
    assert!(rec_ids.contains(&"rec.install_linux"), "{rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.phone_not_viable"), "{rec_ids:?}");
    let linux = recs.iter().find(|r| r.id == "rec.install_linux").unwrap();
    assert_eq!(linux.cost_usd, (0, 0));
    assert_eq!(linux.params.get("rooted").and_then(|v| v.as_bool()), Some(false));
    // The free fix ranks first: highest impact, zero cost.
    assert_eq!(recs[0].id, "rec.install_linux");

    for lang in ["en", "es"] {
        let r = report::render_recs(&recs, lang);
        assert!(r.iter().all(|x| !x.body.contains('⟨')), "{lang}: {r:?}");
    }
}

#[test]
fn rooted_android_is_reported() {
    let mut snap = oneplus_6t_android_unrooted();
    snap.system.is_rooted = Some(true);
    let ids: Vec<String> = rules::run(&snap).into_iter().map(|f| f.id).collect();
    assert!(ids.iter().any(|i| i == "mobile.rooted"), "{ids:?}");
    assert!(!ids.iter().any(|i| i == "mobile.not_rooted"));
}

#[test]
fn low_ram_or_no_mainline_phone_is_not_viable() {
    // 3 GB Dimensity 700 phone on Android: both ceilings at once.
    let mut snap = oneplus_6t_android_unrooted();
    snap.system.soc = soc::by_name("dimensity 700");
    snap.memory.total_bytes = 3 * GIB;
    let ids: Vec<String> = rules::run(&snap).into_iter().map(|f| f.id).collect();
    assert!(ids.iter().any(|i| i == "mobile.mainline_none"), "{ids:?}");
    assert!(grade(&snap, Workload::PortableServer) <= Grade::Poor);

    let recs = advisor::recommend(&snap, &server_answers());
    let rec_ids: Vec<&str> = recs.iter().map(|r| r.id.as_str()).collect();
    assert!(rec_ids.contains(&"rec.phone_not_viable"), "{rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.install_linux"), "{rec_ids:?}");
    assert!(!rec_ids.contains(&"rec.machine_replace"), "{rec_ids:?}");
    let en = report::render_recs(&recs, "en");
    let nv = en.iter().find(|r| r.id == "rec.phone_not_viable").unwrap();
    assert!(nv.body.contains("less than 4 GB"), "{}", nv.body);
    let es = report::render_recs(&recs, "es");
    let nv = es.iter().find(|r| r.id == "rec.phone_not_viable").unwrap();
    assert!(nv.body.contains("menos de 4 GB"), "{}", nv.body);
}

#[test]
fn non_server_uses_get_no_mobile_recs_and_pcs_are_untouched() {
    let snap = oneplus_6t_pmos();
    let a = Answers { uses: vec![Workload::Everyday], pains: vec![], budget: Budget::Under100, horizon: Horizon::Now, needs_portability: true };
    let recs = advisor::recommend(&snap, &a);
    assert!(recs.iter().all(|r| !r.id.starts_with("rec.usb_ethernet") && r.id != "rec.cooling_case" && r.id != "rec.install_linux"), "{:?}", recs.iter().map(|r| &r.id).collect::<Vec<_>>());

    // A plain x86 desktop must not pick up any mobile findings or recs.
    let pc = Snapshot {
        system: SystemInfo { arch: "x86_64".into(), chassis: Chassis::Desktop, ..Default::default() },
        cpu: CpuInfo { brand: "Intel Core i7-8700".into(), physical_cores: Some(6), logical_cpus: 12, launch_year: Some(2018), flags: vec!["avx2".into()], ..Default::default() },
        memory: MemoryInfo { total_bytes: 16 * GIB, available_bytes: 8 * GIB, swap_total_bytes: 4 * GIB, ..Default::default() },
        storage: vec![StorageDevice { name: "nvme0n1".into(), size_bytes: 512 * GIB, kind: StorageKind::Nvme, is_system_disk: true, ..Default::default() }],
        network: vec![NetworkInterface { name: "eth0".into(), kind: NetKind::Ethernet, is_up: true, link_mbps: Some(1000), mac: None }],
        ..Default::default()
    };
    let ids: Vec<String> = rules::run(&pc).into_iter().map(|f| f.id).collect();
    assert!(ids.iter().all(|i| !i.starts_with("mobile.") && i != "battery.as_ups" && i != "thermal.no_active_cooling"), "{ids:?}");
    let recs = advisor::recommend(&pc, &server_answers());
    assert!(recs.iter().all(|r| !matches!(r.id.as_str(), "rec.install_linux" | "rec.cooling_case" | "rec.usb_ethernet" | "rec.phone_not_viable")));
    assert!(grade(&pc, Workload::PortableServer) >= Grade::Good);
    assert!(grade(&pc, Workload::EdgeAi) >= Grade::Good);
}
