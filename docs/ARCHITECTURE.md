# Architecture

Innards is three layers with one direction of dependency:

```
crates/innards-core   →   src-tauri (Rust shell)   →   src/ (Svelte 5 UI)
  probes, rules, i18n,       Tauri commands, settings,     renders RenderedReport,
  capability, advisor,       license keys, narration,      never assembles prose
  render, Markdown           cloud upload
```

The core crate knows nothing about Tauri, the UI, licenses, or the network. The shell knows nothing about how findings are computed. The UI knows nothing about the snapshot; it only sees rendered text plus a few enums.

Everything below is true of the code as it exists in this checkout. File paths are relative to the repository root.

## 1. The pipeline

```
probe::collect_with(Options)          crates/innards-core/src/probe/mod.rs
        │
        ▼
     Snapshot                          crates/innards-core/src/snapshot.rs
        │
        ▼
   report::build(snapshot)             crates/innards-core/src/report.rs
        ├─ rules::run(&snapshot)        → Vec<Finding>        src/rules.rs
        ├─ capability::score(&snapshot) → Vec<Capability>     src/capability.rs
        └─ summarize(...)               → Summary (health score, spec line)
        │
        ▼
      Report { snapshot, findings, capabilities, summary }
        │
        ▼
   report::render(&report, lang, level)   → RenderedReport      (i18n lookup + {param} fill)
        │
        ├─▶ UI (src/lib/components/Report.svelte)
        └─▶ report::to_markdown(&rendered) → String            (Export Markdown)

   advisor::recommend(&report.snapshot, &Answers) → Vec<Recommendation>   src/advisor.rs
   report::render_recs(&recs, lang)               → Vec<RenderedRecommendation>
   advisor::shop_links(&rec, region)              → Vec<ShopLink>        (Pro)
```

`innards_core::analyze(lang, level)` (`crates/innards-core/src/lib.rs`) is the one-call convenience that runs probe → build → render.

### 1.1 Probes → `Snapshot`

`probe::collect_with(Options { elevate_for_smart })` (`probe/mod.rs`) runs, in order:

1. `common::fill` (`probe/common.rs`) — cross-platform, built on `sysinfo` 0.39 and `starship-battery` 0.11: hostname, OS, kernel, arch, uptime; CPU brand (cleaned by `clean_brand`), vendor, logical CPUs, physical cores, current MHz, launch year (`util::cpu_launch_year`); memory totals and swap totals; filesystems (skipping squashfs/overlay/`/snap/`/`/boot/efi`); temperature sensors (first "package"/"tdie"/"cpu" label → `thermal.cpu_c`, first "gpu"/"edge" → `thermal.gpu_c`); load averages; top-10 processes by RSS; the first battery.
2. The platform module, compiled in with `#[cfg(target_os = …)]`: `linux::fill`, `macos::fill` or `windows::fill`.
3. `smart::fill(&mut snap, elevate)` (`probe/smart.rs`) — SMART via `smartctl -j -a`, with an `nvme smart-log -o json` fallback on Linux.
4. `common::finalize` — infers chassis from battery presence when DMI gave nothing, marks the system disk from the root filesystem's device if no platform probe did, sorts filesystems root-first.

Every probe is best-effort: a missing data source leaves an `Option` as `None` and, where useful, pushes a line into `snapshot.probe_notes` (shown at the expert level as "Probe notes"). Probes never panic and never return errors.

The `Snapshot` (`snapshot.rs`) is plain data with `serde` derives. Fields that a platform may not be able to determine are `Option<T>` so rules can distinguish "not measured" from "zero". It also carries identifying data — `system.hostname`, `storage[].serial`, `network[].mac`, `load.top_memory` — which is why every path that leaves the machine scrubs it (see §5).

### 1.2 Rules → `Finding`

`rules::run` (`rules.rs`) calls one function per area (`memory`, `storage`, `cpu`, `battery`, `gpu`, `network`, `system`) and sorts the result by severity. Each rule is a plain function that reads the snapshot and pushes `Finding`s. Thresholds live inline in `rules.rs` so they can be tuned in one place.

A `Finding` (`finding.rs`) carries **no prose**:

```rust
pub struct Finding {
    pub id: String,           // stable id and i18n key, e.g. "memory.no_swap"
    pub severity: Severity,   // Critical | Warning | Info | Good  (derive Ord in that order)
    pub category: Category,   // Memory | Storage | Cpu | Thermal | Battery | Gpu | Network | System
    pub params: Map<String, Value>,   // substituted into "{name}" placeholders
    pub evidence: Vec<String>,        // raw, English-only, shown only at Expert
}
```

The same id can be emitted at different severities (e.g. `memory.low_available` is Critical under 7 % available, Warning under 15 %). Some rules pick the id at the end (`battery.worn` vs `battery.ok`, built from a `battery.health` base).

### 1.3 Capability scores

`capability::score` (`capability.rs`) derives a `Facts` struct (cores, RAM GiB, fast disk, discrete GPU, VRAM, CPU age, AVX2, Apple silicon) and computes one 0–100 score per `Workload` (`Workload::ALL`, ten entries, in the display order used everywhere). Each workload is a weighted sum of `ramp(value, lo, hi)` factors. Scores bucket into `Grade`: 0–24 `Unsuitable`, 25–44 `Poor`, 45–64 `Ok`, 65–84 `Good`, 85+ `Great`. Up to three `limits` (i18n keys like `limit.ram`) name the weakest factors.

### 1.4 `Report` and `Summary`

`report::build` assembles findings, capabilities and a `Summary` (`report.rs::summarize`): the machine name (`vendor product`, or product, or vendor, or hostname, or "This machine"), one-line CPU / memory / storage / GPU / OS / battery strings, and `health_score`:

```
score = 100 − 12·critical − 5·warning + 1·good, clamped to 0..=100
```

(`Info` findings do not move it.)

### 1.5 Render → `RenderedReport`

`report::render(&report, lang, level)` looks up, for every finding, `findings/<id>/title`, `findings/<id>/<level>` and `findings/<id>/action` in the language catalog, fills `{param}` placeholders, and translates the category, workload labels, grade labels and limit strings. The verdict sentence is chosen by counts: `verdict/great` (0 critical, 0 warning), `verdict/fine` (0 critical), `verdict/attention` (exactly 1 critical), `verdict/urgent` (2+). Evidence lines and probe notes are included only when `level == Expert`.

`report::to_markdown` turns a `RenderedReport` into the Markdown export: spec bullets, verdict, findings grouped by severity (with `_What to do:_` and evidence as nested code bullets), the capability list, notes, and the `ui/generated_by` footer.

### 1.6 Advisor

`advisor::questions()` returns the five questions (`uses` multi, `pains` multi, `budget`, `horizon`, `portability`), whose option keys are i18n keys under `workloads/*` and `advisor/options/*`. `advisor::recommend(snapshot, answers)`:

1. Free software fixes first: `rec.add_swap`, `rec.free_space`, `rec.tmpfs_to_disk`.
2. Storage: `rec.ssd_replace_failing` / `rec.ssd_replace_hdd` / `rec.ssd_bigger`.
3. RAM: `rec.ram_add`, `rec.ram_add_check` (slots unknown), or `rec.ram_soldered` (a `Keep`).
4. Battery (`rec.battery_replace`), thermal (`rec.thermal_service`), GPU (`rec.gpu_add` or `rec.gpu_laptop_no_slot`).
5. Whole-machine replacement (`rec.machine_replace`) when workloads grade `Poor` or worse and the CPU is 6+ years old, RAM is the soldered ceiling, or cores fall short.
6. `rec.keep` when nothing or only software fixes came out.

Recommendations carry a USD cost band, `over_budget` (from the budget answer), `helps` (workloads) and an English `shopping_query`; they are ranked by impact then cost. `advisor::shop_links(rec, region)` builds Amazon/Newegg/eBay search URLs (plus Back Market and eBay Refurbished for `rec.machine_replace`) with a regional TLD table. The core crate does not gate anything by tier; the shell does.

## 2. Crate and app boundaries

| Path | Role | May do | Must not do |
|---|---|---|---|
| `crates/innards-core/` | The engine. `probe`, `snapshot`, `rules`, `finding`, `capability`, `advisor`, `i18n`, `report`. Catalogs `i18n/{en,es}.json` are embedded with `include_str!`. | Read `/sys`, `/proc`, shell out to `smartctl`, `nvme`, `df`, `udevadm`, `system_profiler`, `sysctl`, `powershell`, `sudo -n`, `pkexec`, `osascript`. | Touch the network. Contain user-facing prose. Know about tiers or Tauri. |
| `src-tauri/` | Thin Tauri 2 shell. `lib.rs` holds every command (see `docs/IPC.md`); `settings.rs` persists one JSON file; `license.rs` verifies Ed25519 keys and can renew subscription keys; `narrate.rs` calls the Claude API; `cloud.rs` uploads to a Threadwise workspace. | Hold the last `Report` and last recommendations in `AppState` (two `Mutex`es). Gate by tier. Use `reqwest` (rustls) for the paid-only network features. | Compute findings. Render prose. |
| `billing/` | Cloudflare Worker: Stripe Checkout → signed keys, renewal endpoint (see `docs/MONETIZATION.md`). Not part of the Cargo workspace. | Hold the private signing key as a Worker secret. | Be reachable from the free tier. |
| `integrations/threadwise-fleet/` | The `innards.fleet` Threadwise plugin (server + web) that receives uploads (see `docs/CLOUD.md`). | | Import from the Threadwise tree. |
| `src/` | SvelteKit (adapter-static, `ssr = false`) SPA. `lib/api.ts` wraps `invoke`; `lib/mock.ts` serves fixtures in a browser; `lib/state.svelte.ts` is the single app state; `lib/components/*.svelte` render. | Display `RenderedReport` and `RenderedRecommendation`. Carry a handful of chrome strings (tab labels, button captions) inline. | Build report sentences from raw values. Call the network. |
| `tools/innards-license/` | CLI: `keygen`, `sign`, `verify`. Holds the only copy of the private key (`~/.config/innards/signing.key`). | | Ever be shipped or committed with a key. |
| `static/fixtures/` | Generated JSON (`cargo run -p innards-core --example fixtures -- static/fixtures`). | | Be edited by hand. |
| `docs/` | This documentation and the press kit. | | |
| `web/`, `site/` | Marketing site sources (see `docs/DESIGN.md`). Not part of the Cargo workspace or the app build. | | |

`Cargo.toml` at the root is a workspace with three members: `crates/innards-core`, `src-tauri` (package `innards`, lib `innards_lib`), `tools/innards-license`. The workspace `target/` is at the root, so a `pnpm tauri build` writes bundles under `target/release/bundle/`, not `src-tauri/target/…`.

## 3. The IPC layer

Tauri commands are the only bridge. All of them are in `src-tauri/src/lib.rs`, registered in `tauri::generate_handler![…]`. The full reference, with JSON examples and error strings, is `docs/IPC.md`. The shape:

- **State**: `AppState { report: Mutex<Option<Report>>, recs: Mutex<Vec<Recommendation>> }`. `analyze` fills `report`; `advise` fills `recs`. Every other command reads them and answers `"no report yet"` if `analyze` has not run.
- **`analyze`** is `async` and runs the probe in `tauri::async_runtime::spawn_blocking` because probing sleeps (CPU sampling, 300 ms iowait sample) and shells out. It renders with the *saved* language and level (`settings::load()`), not with arguments. If `cloud_auto_upload` is set and the tier is Team or above, it spawns a fire-and-forget upload.
- **Rendering commands** (`render`, `export_markdown`, `narrate`) take `lang` and `level` explicitly; `level` is parsed by `parse_level` (`"plain"`, `"expert"`, anything else → `Informed`).
- **Gating** happens in three commands only: `shop_links` (`Tier::Pro`), `narrate` (`Tier::Supporter`), `cloud_upload` (`Tier::Team`). `Tier` derives `Ord` (`Free < Supporter < Pro < Team < Enterprise`), so a higher key unlocks everything below it. `advise` is *not* gated server-side; the UI hides it below Supporter. `license_refresh` is not gated by tier but needs a saved key (`"no_key"`).
- **Argument naming**: Tauri converts camelCase JS arguments to the snake_case Rust parameters (`includeRecs` → `include_recs`, `recId` → `rec_id`). `set_settings` takes the whole `Settings` struct as `s`.
- **Frontend side**: `src/lib/api.ts` detects Tauri via `"__TAURI_INTERNALS__" in window`; outside Tauri every call goes to `mockInvoke` in `src/lib/mock.ts`, which serves `static/fixtures/*.json` (design mode, §8).
- **Plugins**: `tauri-plugin-opener` (open shop links in the browser) and `tauri-plugin-dialog` (the save dialog for Markdown export). `src-tauri/capabilities/default.json` grants `core:default`, `opener:default`, `dialog:default` to the main window. `src/lib/platform.ts` wraps both so they degrade to `window.open` / a Blob download in the browser.
- **CSP** (`src-tauri/tauri.conf.json`): `connect-src ipc: http://ipc.localhost` — the webview cannot reach any network host. All network traffic, when it happens, originates in Rust.

## 4. Frontend state

`src/lib/state.svelte.ts` exports one `app` instance of `AppState`, built on Svelte 5 runes:

| Field | Type | Set by |
|---|---|---|
| `settings` | `Settings` | `init()` from `get_settings`; mutated directly by Settings/TopBar; persisted with `set_settings` |
| `languages` | `[code, name][]` | `init()` from `languages` |
| `catalog` | `Catalog \| null` | `init()` and `setLang()` from `catalog(lang)` — the UI slice (`ui`, `severities`, `categories`, `grades`, `workloads`, `advisor`, `rec_kinds`, `impacts`) |
| `report` | `RenderedReport \| null` | `scan()` from `analyze`; `rerender()` from `render` |
| `license` | `LicenseStatus` | `init()`, `saveSettings()`, and `Settings.svelte` after `activate_license` |
| `scanning`, `error` | | `scan()` |

Helpers: `t(path)` looks up a `/`-separated path in the catalog and returns the path itself when missing (so untranslated keys are visible in the UI, matching the core's `⟨path⟩` behaviour); `has(tier)` compares against the ordered list `["free","supporter","pro","team","enterprise"]` so components gate with `app.has("supporter")` rather than equality.

Navigation is SvelteKit routing, not state: `src/routes/+page.svelte` (Report), `src/routes/advisor/+page.svelte`, `src/routes/settings/+page.svelte`. `src/routes/+layout.svelte` owns the shell (`TopBar`, the error card, `Scanning` until a report exists, then the page) and calls `app.init()` on mount. Components navigate with `goto(resolve("/settings"))` (`Narrative.svelte`, `Paywall.svelte`); the top bar's tabs are `<a>` links with `aria-current`.

Lifecycle: `app.init()` → `get_settings`, `languages`, `license_status` in parallel → `maybeRenew()` (silent `license_refresh` for renewable keys within 30 days of expiry) → `catalog` → `scan()`. Changing language (`setLang`) refetches the catalog, re-renders the existing report through `render` (no re-probe) and saves settings; changing level (`setLevel`) re-renders and saves. `Rescan` calls `scan()` again with the saved `elevate_for_smart` flag.

Component map (`src/lib/components/`):

- `TopBar.svelte` — brand + tier pill, route links styled as a segmented control, level segmented control, language flag dropdown, Rescan.
- `Report.svelte` — hero card (`HealthRing`, machine name, verdict, Export Markdown, spec list), findings grouped by severity as `FindingCard`s, aside with `Capabilities` and `Narrative`.
- `Advisor.svelte` — questions as chip groups, `Paywall` below Supporter, results as cards or the `RecMap` impact-vs-cost view, where-to-buy buttons at Pro, refine row.
- `Settings.svelte` — license activation, Anthropic key, SMART elevation + region, Team & Enterprise (Threadwise URL, `tw_` key, machine label, auto-upload, "Upload report now"), Save + version line.
- `Scanning.svelte`, `Paywall.svelte`, `Logo.svelte`.

## 5. Privacy model

What exists in code, path by path:

| Path | Network? | What leaves the machine |
|---|---|---|
| Free tier: scan, render, export | **None.** `innards-core` has no HTTP dependency; the webview CSP forbids outbound connections; `reqwest` is only called from `narrate.rs`, `cloud.rs` and `license.rs::refresh`, all three unreachable without a paid key. | Nothing. The Markdown export is written to a path the user picks. |
| Settings | None | `~/.config/innards/settings.json` (via `dirs::config_dir()`; `~/Library/Application Support/innards/` on macOS, `%APPDATA%\innards\` on Windows) holds language, level, region, the license key, the Anthropic key, the SMART flag, and the cloud endpoint/token/label. Plain JSON, mode as created by `std::fs::write`. |
| License check | None. Ed25519 verification against the embedded public key (`license.rs::PUBLIC_KEY_B64`). | Nothing. No phone-home, no activation server. |
| License renewal (Pro/Team keys only) | `GET https://innards.app/api/license/refresh?key=…` (`license.rs::refresh`, base overridable with `INNARDS_BILLING_URL`). Triggered by `state.svelte.ts::maybeRenew` at startup only when the key carries a Stripe `sub` id (`renewable: true`) **and** expires within 30 days; also by the `license_refresh` command. | The license key itself (tier, email, dates, Stripe ids). Nothing about the machine. Supporter keys and the free tier never call it. |
| Narration (Supporter+) | `POST https://api.anthropic.com/v1/messages` with the user's own key (`settings.anthropic_api_key` or `ANTHROPIC_API_KEY`). | `narrate.rs::scrub`: summary strings (machine, cpu, memory, storage, gpu, os, battery, health_score), the verdict, and per finding `severity/category/title/detail/action`, per capability `workload/grade/score/limits`; optionally rendered recommendations. **Never** the snapshot, serials, MACs, process lists, evidence lines or probe notes. Note that `summary.machine` falls back to the hostname only when DMI vendor *and* product are unknown. |
| Cloud upload (Team+) | `POST <endpoint>/api/ext/innards.fleet/reports`, `Authorization: Bearer tw_…`. | `cloud.rs::scrub` nulls `storage[].serial`, `system.hostname`, `network[].mac` and empties `load.top_memory`, then sends the scrubbed `Report` plus the English/informed `RenderedReport` and a `machine` header block. The `machine.id` is a `DefaultHasher` digest of vendor + product + system-disk serial + first MAC — a one-way hash; the raw values are not sent. Filesystem mount points (which may contain a username, e.g. `/media/<user>/…`) and `probe_notes` are **not** scrubbed. |
| Fixtures | None (local file write) | `examples/fixtures.rs` nulls serials and the hostname in `report-full.json` before writing, because fixtures are committed. |

Rules of the house, enforced by review rather than code: no network in the free tier; serials, hostnames, MACs and process lists never leave the machine in clear; nothing is stored beyond `settings.json`.

## 6. How to add things

### 6.1 A rule (finding)

1. Pick an id `<category>.<name>` (e.g. `storage.trim_off`). The prefix must be one of `memory.`, `storage.`, `cpu.`, `thermal.`, `battery.`, `gpu.`, `network.`, `system.` — that list is hard-coded in `crates/innards-core/tests/catalogs.rs::every_rule_id_is_in_every_catalog`, which scrapes string literals from `rules.rs`. A new prefix means adding it there too.
2. In `crates/innards-core/src/rules.rs`, push a `Finding::new(id, severity, category).param(..).evidence(..)` from the relevant area function. No prose: only params (numbers, formatted byte strings, model names) and English evidence lines.
3. Add the entry to **every** catalog: `crates/innards-core/i18n/en.json` and `es.json` under `findings.<id>` with all five fields `title`, `plain`, `informed`, `expert`, `action` (use `""` for no action; the renderer drops empty actions). Placeholders must match across languages.
4. `cargo test -p innards-core`. The three tests check: every id in `rules.rs` exists with all fields in every catalog; every catalog has exactly the English key set per section; `{placeholders}` match English.
5. If the finding should move the health score differently, it does so through its `Severity` (§1.4).
6. Regenerate fixtures if you want design mode to show it: `cargo run -p innards-core --example fixtures -- static/fixtures` (only fires if the rule triggers on your machine).

### 6.2 A language

1. Copy `crates/innards-core/i18n/en.json` to `i18n/<code>.json` and translate every string. Keep every key and every `{placeholder}`.
2. Add one line to `LANGS` in `crates/innards-core/src/i18n.rs`: `("<code>", "<Native name>", include_str!("../i18n/<code>.json"))`.
3. `cargo test -p innards-core` — `catalogs_share_the_english_key_set` and `placeholders_match_english` fail on any drift.
4. Regenerate fixtures (`report-<code>-*.json`, `catalog-<code>.json`, `recs-<code>.json`, `languages.json`) so design mode has data for it.
5. UI chrome strings are *not* in the catalog: `TopBar.svelte` (tab and level labels, "Rescan"), `Advisor.svelte`, `Narrative.svelte`, `Paywall.svelte`, `Settings.svelte`, `Scanning.svelte`, `FindingCard.svelte` ("Evidence") and `Report.svelte` ("Export Markdown") carry inline `{ en, es }` maps or `es ? … : …` ternaries that fall back to English. Extend them, and the `flags` map in `TopBar.svelte`, for a complete translation. The `narrate.rs` prompt uses the catalog's native language name, so narration works for any listed language without changes.
6. `settings.rs::Default` only auto-selects `es` from `$LANG`; any other locale starts in English until the user picks.

### 6.3 A probe (new snapshot field)

1. Add the field to the right struct in `crates/innards-core/src/snapshot.rs`, as `Option<T>` (or `Vec`) with `Default`, so platforms that cannot measure it stay honest.
2. Fill it in `probe/common.rs` if `sysinfo`/`starship-battery` expose it; otherwise in `probe/linux.rs` (sysfs/procfs, read with `util::read_trim`/`read_u64`, or `util::run` for a tool), and best-effort in `probe/macos.rs` (`system_profiler -json`, `sysctl`) and `probe/windows.rs` (`Get-CimInstance … | ConvertTo-Json`). Never panic; push a `probe_notes` line when a source is missing and the user could fix it.
3. Consume it in a rule (§6.1) or in `capability::facts`. A snapshot field nobody reads is noise.
4. If it is identifying (serials, addresses, names), add it to both scrubbers: `src-tauri/src/cloud.rs::scrub` and `crates/innards-core/examples/fixtures.rs`, and make sure `narrate.rs::scrub` does not pick it up (it only reads the rendered report).
5. Regenerate fixtures; `report-full.json` documents the snapshot shape.

### 6.4 A workload

1. Add a variant to `Workload` in `crates/innards-core/src/capability.rs`, its snake_case key in `Workload::key`, and its position in `Workload::ALL` (this is the display order).
2. Add a scoring arm in `capability::one` with weighted factors and `limits` hints.
3. Add its RAM and core targets in `advisor.rs::ram_target_gib` / `cores_target` (both `match` exhaustively — the compiler will insist) and, if it is GPU-bound, to `wants_gpu` and the GPU filter in `recommend`.
4. Add `workloads.<key>` to every catalog. The key-set test enforces parity; nothing checks that `Workload::key()` values exist in the catalog, so a typo would render as `⟨workloads/…⟩` — check the fixture output.
5. `advisor::questions()` derives the `uses` options from `Workload::ALL`, so the questionnaire updates itself.

### 6.5 A recommendation

1. Choose `rec.<name>`; the catalog test scrapes `advisor.rs` for string literals starting with `rec.`.
2. In `advisor::recommend`, push `Recommendation::new(id, RecKind, Impact, (low_usd, high_usd)).p(..).helps(&[..]).shop("english search query")`. `(0, 0)` renders as `ui/free`. Give `.shop()` only if a shopping link makes sense; `shop_links` returns an empty list otherwise.
3. Add `recs.<id>` with `title`, `body`, `why` to every catalog.
4. `cargo test -p innards-core`; regenerate `recs-*.json` fixtures (the fixture answers are fixed in `examples/fixtures.rs`).

### 6.6 A limit (capability explanation)

Add `limits.<key>` to every catalog and push `("limit.<key>", weight)` in `capability::one`; the renderer strips the `limit.` prefix when looking up `limits/<key>`.

## 7. Platform notes

| | Linux | macOS | Windows |
|---|---|---|---|
| Status | **Tested** (developed on Ubuntu 25.10, ThinkPad X1 Carbon 5th). | Best-effort, **untested on hardware** (comment at the top of `probe/macos.rs`). | Best-effort, **untested on hardware** (comment at the top of `probe/windows.rs`). |
| Module | `probe/linux.rs` | `probe/macos.rs` | `probe/windows.rs` |
| DMI / chassis | `/sys/class/dmi/id` (`sys_vendor`, `product_name`, `product_version` — Lenovo puts the marketing name there — `bios_version`, `chassis_type` mapped to `Chassis`); QEMU/VMware/innotek vendors → `Vm` | `sysctl hw.model`, `system_profiler SPHardwareDataType` (`machine_name` containing "Book" → Laptop, "mini" → MiniPc, else Desktop) | `Win32_ComputerSystem` (`PCSystemType` 2 → Laptop, 1/3 → Desktop, 4/5 → Server), `Win32_BIOS` |
| CPU extras | `cpufreq` max/base/governor, `thermal_throttle/package_throttle_count`, `/proc/cpuinfo` flags filtered to `avx avx2 avx512f sse4_2 aes vmx svm sha_ni amx_tile` | `hw.physicalcpu`; `chip_type` for brand/launch year | `Win32_Processor` MaxClockSpeed, NumberOfCores |
| Memory modules | `udevadm info -q property -p /sys/devices/virtual/dmi/id` (`MEMORY_DEVICE_N_*`); "DIMM" in the form factor → upgradeable; otherwise a probe note | `SPMemoryDataType` (`dimm_type`; Apple silicon reported soldered) | `Win32_PhysicalMemory` (SMBIOSMemoryType → DDR3/DDR4/LPDDR3/LPDDR4/DDR5/LPDDR5; FormFactor 8/12 = DIMM/SODIMM, 13 = soldered) |
| Swap / tmpfs | `/proc/swaps` (zram/file/partition), `/proc/mounts` + `df -B1 --output=used` for `/tmp`, `/dev/shm`, `/run/user/*` | `sysinfo` totals only | `sysinfo` totals only |
| Block devices | `/sys/block/*` (skips loop/ram/dm-/zram/sr): size, rotational, removable, bus from the canonical device path (usb/ata/thunderbolt), NVMe PCIe link speed from `current_link_speed`/`current_link_width`, model/firmware/serial from `device/` | `SPNVMeDataType`, `SPSerialATADataType`; boot disk from `SPStorageDataType` mount `/` | `Win32_DiskDrive` + `Get-PhysicalDisk` (MediaType/BusType); index 0 assumed system disk |
| GPUs | `/sys/bus/pci/devices/*` class `0x03…`, names from `pci.ids` (`/usr/share/misc`, `/usr/share/hwdata`, `/usr/share`); NVIDIA or AMD-without-"Graphics" → discrete; VRAM unknown | `SPDisplaysDataType` (`spdisplays_vram`); non-Apple, non-Intel → discrete | `Win32_VideoController` (`AdapterRAM`) |
| Network | `/sys/class/net/*` (`wireless`/`phy80211` → Wi-Fi; `device` → Ethernet; virtual/loopback skipped), `operstate`, `speed`, `address` | none beyond `sysinfo` | `Win32_NetworkAdapter` physical adapters; "wi-fi"/"wireless" in the name → Wi-Fi |
| Thermal / fans | hwmon `fan*_input`; NVMe composite temperature from the `nvme` hwmon | `sysinfo` components | `sysinfo` components |
| I/O wait | two `/proc/stat` samples 300 ms apart | not measured | not measured |
| SMART | `smartctl -j -a /dev/<name>`, fallback `nvme smart-log -o json` | `smartctl -j -a /dev/<bsd_name>` | `smartctl -j -a <DeviceID>` (`\\.\PHYSICALDRIVEn`) |

`util::cpu_launch_year` (`probe/util.rs`) maps Intel Core generations, Core Ultra, AMD Ryzen series and Apple M-series to a rough launch year; unknown brands give `None`, and `capability::facts` then assumes an age of 4.

**ARM boards, phones and Android — in progress.** At the time of writing the core crate is being extended (and does not compile mid-change): `crates/innards-core/src/soc.rs` is a static SoC knowledge base (`SocInfo` with vendor, launch year, big/little cores, NEON/dot-product flags, a `MainlineSupport` rating for "can this run upstream Linux?", `detect(&Hints)` from `/proc/cpuinfo` "Hardware", the device-tree `compatible`/`model` strings and `/sys/devices/soc0`); `Chassis` gains `Phone`, `Tablet`, `Sbc`; `SystemInfo` gains `soc`, `device_model`, `android_version`; `CpuInfo` gains `big_cores`/`little_cores`; `probe/linux.rs` adds `soc_and_chassis` (device-tree classification when there is no DMI) and `big_little` (cpufreq policies); `Snapshot` gains `is_mobile()` and `is_arm_device()`; `util::parent_block_device` understands `/dev/block/…` (Android). A `probe/android.rs` is referenced from the `linux.rs` header comment but does not exist yet, and no rule, capability or catalog string consumes the new fields so far. Treat this paragraph as a pointer, not a spec, until the build is green and the catalog test passes again.

## 8. Elevation for SMART

SMART data normally needs root/administrator. `probe::Options::elevate_for_smart` (the "Ask for administrator rights to read drive health" checkbox in Settings, persisted as `settings.elevate_for_smart` and passed to `analyze` on every scan) enables `smart.rs::run_maybe_elevated`:

1. Try the tool unprivileged. If it succeeds, done.
2. If elevation was requested, on Linux and macOS try `sudo -n <cmd>` first — silent, so passwordless sudo setups never see a dialog.
3. Then the platform GUI prompt: **Linux** `pkexec smartctl …` (PolicyKit dialog); **macOS** `osascript -e 'do shell script "…" with administrator privileges'` (the system password sheet). Each device is a separate call, so a machine with several drives may prompt more than once per scan.
4. **Windows** returns `None`: elevation there means relaunching the whole process under UAC, which is the app's job, not the probe's. Unprivileged `smartctl` is still tried.

When no tool is installed, `probe_notes` gets "smartmontools not installed…"; when a tool ran but returned nothing for some device, "some drives returned no SMART data (usually needs administrator rights)". `rules::storage` emits `storage.smart_unavailable` when no device has SMART data.

## 9. Design mode

`pnpm dev` serves the SPA at `http://localhost:1420` with `mock.ts` in place of Tauri. The view is the path (`/`, `/advisor`, `/settings`); query parameters are read by `mock.ts` and `Advisor.svelte` on any path: `?tier=free|supporter|pro|team|enterprise`, `?lang=en|es`, `?level=plain|informed|expert`, `?demo=1` (on `/advisor`: pre-fills and runs the advisor; add `&view=visual` for the map), `?snap=1` (loads fixtures synchronously, disables animations via the `no-anim` class, and `src/app.html` requests `/__snap-hold?ms=2500` as an image so a headless browser's load event waits for the data — the endpoint is a Vite middleware in `vite.config.js`). Example: `http://localhost:1420/advisor?tier=pro&lang=es&demo=1`. Details and a screenshot recipe are in `docs/AGENTS.md` and `docs/DESIGN.md`.
