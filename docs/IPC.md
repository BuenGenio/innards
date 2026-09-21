# IPC reference

Every Tauri command lives in `src-tauri/src/lib.rs` and is registered in the `tauri::generate_handler![…]` list at the bottom of that file. The frontend never calls `invoke` directly: `src/lib/api.ts` has one typed wrapper per command, and outside Tauri (`pnpm dev` in a plain browser) the same wrappers route to `mockInvoke` in `src/lib/mock.ts`, which serves `static/fixtures/*.json`.

Conventions that apply to all commands:

- **Argument names**: Tauri maps camelCase JavaScript arguments onto snake_case Rust parameters (`includeRecs` → `include_recs`, `recId` → `rec_id`). The tables below show the Rust names; `api.ts` shows the JS names.
- **Errors** are `Result<T, String>` on the Rust side. The promise rejects with the bare string, so the UI compares against short codes (`"pro"`, `"supporter"`, `"team"`, `"no_api_key"`, `"no_endpoint"`, `"no_token"`, `"invalid"`, `"no report yet"`). Anything else is a human-readable message.
- **Tier gating** uses `license::Tier`, which is ordered `free < supporter < pro < team < enterprise`; a check like `tier < Tier::Pro` lets Team and Enterprise through. The tier comes from `license_status()` on every call — there is no cached session — so activating a key takes effect immediately. `INNARDS_TIER=<tier>` (and `INNARDS_ORG=<slug>`) in the environment overrides the key for development.
- **State**: `AppState { report: Mutex<Option<Report>>, recs: Mutex<Vec<Recommendation>> }`. Commands marked *needs report* return `"no report yet"` until `analyze` has completed once.
- **Level strings** are parsed by `parse_level`: `"plain"`, `"expert"`, anything else → `informed`.
- Fixture files referenced below are regenerated with `cargo run -p innards-core --example fixtures -- static/fixtures` and reflect the developer's own ThinkPad.

Types crossing the boundary are mirrored in `src/lib/types.ts` (`RenderedReport`, `RenderedFinding`, `RenderedCapability`, `Question`, `Answers`, `RenderedRecommendation`, `ShopLink`, `Settings`, `LicenseStatus`, `UploadResult`, `Narrative`, `Catalog`).

---

## Analysis

### `analyze`

| | |
|---|---|
| Rust | `async fn analyze(state, elevate: bool) -> Result<RenderedReport, String>` |
| JS | `api.analyze(elevate)` |
| Tier | Free |
| Does | Probes the machine in a blocking thread (`probe::collect_with(Options { elevate_for_smart: elevate })`), builds the `Report`, stores it in `AppState.report`, and renders it with the **saved** `settings.lang` and `settings.level` (not with arguments). If `settings.cloud_auto_upload` is true and the tier is ≥ Team, spawns a fire-and-forget `cloud::upload` (rendered as `en`/`informed`); its errors are swallowed. |
| Errors | Only the `spawn_blocking` join error (`e.to_string()`), i.e. a panic inside a probe. Probes are designed not to panic. |
| Takes | About one second: `sysinfo` needs two CPU samples (`MINIMUM_CPU_UPDATE_INTERVAL` + 50 ms), the Linux iowait probe sleeps 300 ms, and `smartctl` runs per drive. With `elevate = true` the OS password prompt may appear (see `docs/ARCHITECTURE.md` §8). |

Return value (`RenderedReport`), abridged from `static/fixtures/report-en-informed.json`:

```json
{
  "lang": "en",
  "level": "informed",
  "summary": {
    "machine": "LENOVO ThinkPad X1 Carbon 5th",
    "cpu": "Intel Core i7-7500U · 2C/4T",
    "memory": "16 GB LPDDR3",
    "storage": "1.8 TiB NVMe",
    "gpu": "HD Graphics 620",
    "os": "Ubuntu 25.10",
    "battery": "45%",
    "health_score": 65
  },
  "verdict": "One issue needs attention soon, plus 5 smaller one(s).",
  "findings": [
    {
      "id": "storage.fs_nearly_full",
      "severity": "critical",
      "category": "Storage",
      "title": "/mnt/sdc1 is 99.5% full",
      "body": "/mnt/sdc1 is 99.5% used (3.8 GiB of 830 GiB free). Writes to a nearly full drive get slow.",
      "action": "Free space or move data elsewhere.",
      "evidence": []
    },
    {
      "id": "battery.worn",
      "severity": "warning",
      "category": "Battery",
      "title": "Battery holds 45.3% of its original capacity",
      "body": "45.3% health, 1729 cycles. Below ~60% a replacement is worth it if you use it unplugged.",
      "action": "Replace the battery if portability matters.",
      "evidence": []
    }
  ],
  "capabilities": [
    { "workload": "everyday", "label": "Everyday use (web, office, media)", "score": 96, "grade": "great", "grade_label": "Excellent", "limits": [] },
    { "workload": "web_dev", "label": "Web & app development", "score": 76, "grade": "good", "grade_label": "Good", "limits": ["too few CPU cores", "not enough memory"] }
  ],
  "notes": []
}
```

`severity` is one of `critical | warning | info | good`; `grade` is `unsuitable | poor | ok | good | great`; `category` is already translated. `evidence` and `notes` are non-empty only at `"level": "expert"` (see `report-en-expert.json`: `"evidence": ["energy_full=25.9 Wh design=57.0 Wh cycles=Some(1729)"]`, `"notes": ["some drives returned no SMART data (usually needs administrator rights)"]`). `battery` is `null` on machines without one; `summary.battery` is the health percentage as a string.

**Mock**: waits 900 ms (0 ms with `?snap=1`) and returns `fixtures/report-<lang>-<level>.json` using the mock's in-memory settings (initialised from `?lang=` and `?level=`, updated by `set_settings`). There is no probing.

### `render`

| | |
|---|---|
| Rust | `fn render(state, lang: String, level: String) -> Result<RenderedReport, String>` |
| JS | `api.render(lang, level)` |
| Tier | Free · *needs report* |
| Does | Re-renders the stored `Report` in another language/level without re-probing. This is what language and level switches call. |
| Errors | `"no report yet"` |

Args: `{ "lang": "es", "level": "plain" }`. Return: same shape as `analyze`.

**Mock**: returns `fixtures/report-<lang>-<level>.json` immediately.

### `export_markdown`

| | |
|---|---|
| Rust | `fn export_markdown(state, lang: String, level: String) -> Result<String, String>` |
| JS | `api.exportMarkdown(lang, level)` |
| Tier | Free · *needs report* |
| Does | `report::to_markdown(render(report, lang, level))`. The UI then opens a save dialog (`tauri-plugin-dialog`) and writes the string with `save_text`. |
| Errors | `"no report yet"` |

Return (excerpt):

```markdown
# Innards report — LENOVO ThinkPad X1 Carbon 5th

- **Processor:** Intel Core i7-7500U · 2C/4T
- **Memory:** 16 GB LPDDR3
- **Storage:** 1.8 TiB NVMe
- **Graphics:** HD Graphics 620
- **System:** Ubuntu 25.10
- **Battery health:** 45%

**Verdict:** One issue needs attention soon, plus 5 smaller one(s).

## Needs attention now

- **/mnt/sdc1 is 99.5% full** — /mnt/sdc1 is 99.5% used (3.8 GiB of 830 GiB free). Writes to a nearly full drive get slow. _What to do: Free space or move data elsewhere._

## What this machine is good for

- **Everyday use (web, office, media):** Excellent (96/100)
- **Web & app development:** Good (76/100) — too few CPU cores, not enough memory

_Generated by Innards — the report stays on your machine._
```

**Mock**: returns the constant `"# Innards report (mock)\n"`.

### `report_json`

| | |
|---|---|
| Rust | `fn report_json(state) -> Result<Value, String>` |
| JS | `api.reportJson()` |
| Tier | Free · *needs report* |
| Does | Serialises the whole stored `Report` (snapshot + raw findings + capabilities + summary). **Unscrubbed**: includes `snapshot.system.hostname`, `snapshot.storage[].serial`, `snapshot.network[].mac`, `snapshot.load.top_memory`. Not called by the current UI; exists for debugging and future export. |
| Errors | `"no report yet"`, serde error text |

Return, abridged from `static/fixtures/report-full.json` (the fixture has serials and hostname nulled):

```json
{
  "snapshot": {
    "taken_at": "2026-09-21T01:12:33.123456789+00:00",
    "system": { "hostname": null, "os_name": "Ubuntu", "os_version": "25.10", "kernel_version": "6.17.0-41-generic", "arch": "x86_64", "vendor": "LENOVO", "product": "ThinkPad X1 Carbon 5th", "bios_version": "N1MET80W (1.65 )", "chassis": "laptop", "uptime_secs": 123456, "boot_count_unclean": null },
    "cpu": { "brand": "Intel Core i7-7500U", "vendor": "GenuineIntel", "physical_cores": 2, "logical_cpus": 4, "base_mhz": 2700, "max_mhz": 3500, "current_mhz": 2898, "launch_year": 2017, "flags": ["vmx", "sse4_2", "aes", "avx", "avx2"], "throttle_events": 155050, "governor": "powersave" },
    "memory": { "total_bytes": 16589139968, "available_bytes": 3100000000, "used_bytes": 12000000000, "swap_total_bytes": 12884901888, "swap_used_bytes": 5905580032, "swap_backends": [{ "name": "/swapfile", "kind": "file", "size_bytes": 8589934592, "used_bytes": 4000000000, "priority": 5 }], "kind": "LPDDR3", "speed_mts": 1866, "upgradeable": false, "modules": [], "tmpfs_used_bytes": 0 },
    "storage": [{ "name": "nvme0n1", "model": "KINGSTON SNV2S2000G", "serial": null, "firmware": "SBM02103", "size_bytes": 2000398934016, "kind": "nvme", "bus": "pcie", "is_removable": false, "is_system_disk": true, "link_speed": "PCIe 8.0 GT/s x4", "smart": null, "temperature_c": 41.85 }],
    "filesystems": [{ "mount_point": "/", "device": "/dev/nvme0n1p2", "fs_type": "ext4", "total_bytes": 1000000000000, "available_bytes": 400000000000, "is_root": true, "is_removable": false }],
    "gpus": [{ "name": "HD Graphics 620", "vendor": "Intel", "is_discrete": false, "vram_bytes": null, "driver": "i915" }],
    "battery": { "present": true, "design_wh": 57.02, "full_wh": 25.85, "health_pct": 45.33, "cycles": 1729, "charge_pct": 98.0, "on_ac": true },
    "thermal": { "cpu_c": 62.0, "gpu_c": null, "fan_rpm": [3500], "sensors": [["Package id 0", 62.0]] },
    "network": [{ "name": "wlp4s0", "kind": "wifi", "is_up": true, "link_mbps": null, "mac": "…" }],
    "load": { "load_1": 8.1, "load_5": 8.6, "load_15": 7.9, "io_wait_pct": 38.0, "top_memory": [{ "name": "firefox", "rss_bytes": 2000000000, "cpu_pct": 3.2 }] },
    "probe_notes": ["some drives returned no SMART data (usually needs administrator rights)"]
  },
  "findings": [
    { "id": "storage.fs_nearly_full", "severity": "critical", "category": "storage", "params": { "free": "3.8 GiB", "mount": "/mnt/sdc1", "total": "830 GiB", "used_pct": 99.5 }, "evidence": ["/dev/sdc1 ntfs3 826 GiB/830 GiB used"] }
  ],
  "capabilities": [
    { "workload": "everyday", "score": 96, "grade": "great", "limits": [] },
    { "workload": "web_dev", "score": 76, "grade": "good", "limits": ["limit.cores", "limit.ram"] }
  ],
  "summary": { "machine": "LENOVO ThinkPad X1 Carbon 5th", "cpu": "Intel Core i7-7500U · 2C/4T", "memory": "16 GB LPDDR3", "storage": "1.8 TiB NVMe", "gpu": "HD Graphics 620", "os": "Ubuntu 25.10", "battery": "45%", "health_score": 65 }
}
```

Raw findings carry `params` and untranslated `category`; capabilities carry `limit.*` keys rather than sentences. The exact field list is `crates/innards-core/src/snapshot.rs`.

**Mock**: returns `fixtures/report-full.json`.

### `save_text`

| | |
|---|---|
| Rust | `fn save_text(path: String, contents: String) -> Result<(), String>` |
| JS | `api.saveText(path, contents)` — called by `platform.ts::saveTextFile` after the dialog plugin returns a path |
| Tier | Free |
| Does | `std::fs::write(path, contents)`. No path restriction beyond what the OS enforces; the path always comes from the native save dialog. |
| Errors | The I/O error text (e.g. `"Permission denied (os error 13)"`) |

Args: `{ "path": "/home/ana/innards-report.md", "contents": "# Innards report — …" }`. Returns `null`.

**Mock**: returns `undefined` (the browser build downloads a Blob instead and never calls this).

---

## i18n

### `languages`

| | |
|---|---|
| Rust | `fn languages() -> Vec<(String, String)>` |
| JS | `api.languages()` |
| Tier | Free |
| Does | `i18n::available()` — the `LANGS` table in `crates/innards-core/src/i18n.rs`. |

Return (`static/fixtures/languages.json`):

```json
[["en", "English"], ["es", "Español"]]
```

**Mock**: `fixtures/languages.json`.

### `catalog`

| | |
|---|---|
| Rust | `fn catalog(lang: String) -> Value` |
| JS | `api.catalog(lang)` |
| Tier | Free |
| Does | Returns the UI-facing slice of the language's catalog: the sections `ui`, `severities`, `categories`, `grades`, `workloads`, `advisor`, `rec_kinds`, `impacts`. Findings and recs are **not** included (they are rendered in Rust). An unknown code falls back to the first language (English). Never fails. |

Args: `{ "lang": "en" }`. Return (`static/fixtures/catalog-en.json`, abridged):

```json
{
  "ui": { "report_title": "Innards report", "cpu": "Processor", "memory": "Memory", "storage": "Storage", "gpu": "Graphics", "os": "System", "battery": "Battery health", "verdict": "Verdict:", "action": "What to do", "capabilities": "What this machine is good for", "notes": "Probe notes", "generated_by": "Generated by Innards — the report stays on your machine.", "free": "Free" },
  "severities": { "critical": "Needs attention now", "warning": "Worth fixing", "info": "Good to know", "good": "Working well" },
  "categories": { "memory": "Memory", "storage": "Storage", "cpu": "Processor", "thermal": "Cooling", "battery": "Battery", "gpu": "Graphics", "network": "Network", "system": "System" },
  "grades": { "unsuitable": "Not suitable", "poor": "Struggles", "ok": "Workable", "good": "Good", "great": "Excellent" },
  "workloads": { "everyday": "Everyday use (web, office, media)", "web_dev": "Web & app development", "…": "…" },
  "advisor": {
    "uses": "What do you use this machine for?", "pains": "What bothers you about it?", "budget": "What would you be willing to spend?", "horizon": "When?", "portability": "Do you need to use it away from a desk?",
    "options": { "slow": "It feels slow", "out_of_memory": "Runs out of memory", "under_300": "Under $300", "six_months": "In the next 6 months", "yes": "Yes", "no": "No", "…": "…" }
  },
  "rec_kinds": { "software": "Free fix", "upgrade": "Upgrade", "service": "Service", "replace": "Replace", "keep": "No purchase" },
  "impacts": { "low": "Small gain", "medium": "Noticeable gain", "high": "Big gain" }
}
```

The frontend reads it through `app.t("ui/cpu")`, `app.t("advisor/options/slow")`, etc.

**Mock**: `fixtures/catalog-<lang>.json`.

---

## Advisor

### `advisor_questions`

| | |
|---|---|
| Rust | `fn advisor_questions() -> Vec<advisor::Question>` |
| JS | `api.advisorQuestions()` |
| Tier | Free (the questions are shown to everyone; running the advisor is gated in the UI) |
| Does | `advisor::questions()`. Option strings are catalog keys: `uses` options are `workloads/<key>`, everything else `advisor/options/<key>`. |

Return (`static/fixtures/questions.json`):

```json
[
  { "id": "uses", "multi": true, "options": ["everyday", "web_dev", "containers", "home_server", "photo_editing", "heavy_compile", "video_editing", "local_llm", "gaming", "ml_training"] },
  { "id": "pains", "multi": true, "options": ["slow", "out_of_memory", "storage", "battery", "heat_noise", "crashes", "none"] },
  { "id": "budget", "multi": false, "options": ["under_100", "under_300", "under_800", "under_1500", "no_limit"] },
  { "id": "horizon", "multi": false, "options": ["now", "six_months", "year"] },
  { "id": "portability", "multi": false, "options": ["yes", "no"] }
]
```

**Mock**: `fixtures/questions.json`.

### `advise`

| | |
|---|---|
| Rust | `fn advise(state, answers: Answers, lang: String) -> Result<Vec<RenderedRecommendation>, String>` |
| JS | `api.advise(answers, lang)` |
| Tier | **Not gated in Rust.** `Advisor.svelte` disables the run button and shows the Supporter paywall when `!app.has("supporter")`. |
| Does | `advisor::recommend(&report.snapshot, &answers)`, stores the raw recommendations in `AppState.recs` (needed later by `shop_links`), returns them rendered in `lang`. |
| Errors | `"no report yet"`; a serde error if `answers` has an unknown enum value (Tauri reports it as an invalid-args error). |

Args — `Answers` uses the same snake_case keys as the questionnaire options:

```json
{
  "answers": {
    "uses": ["web_dev", "containers", "home_server", "local_llm"],
    "pains": ["slow", "out_of_memory"],
    "budget": "under_800",
    "horizon": "six_months",
    "needs_portability": true
  },
  "lang": "en"
}
```

Return (`static/fixtures/recs-en.json`, abridged):

```json
[
  {
    "id": "rec.ram_soldered",
    "kind": "No purchase",
    "impact": "Big gain",
    "cost": "Free",
    "over_budget": false,
    "title": "Memory is the ceiling — and it's soldered",
    "body": "You have 16 GB and your workloads want 32 GB, but this machine's memory can't be upgraded.",
    "why": "This is the strongest reason to consider a different machine, not a part.",
    "helps": ["Web & app development", "Docker / container stacks", "Home server / self-hosting", "Running AI models locally"],
    "shopping_query": null
  },
  {
    "id": "rec.battery_replace",
    "kind": "Service",
    "impact": "Big gain",
    "cost": "$40–$120",
    "over_budget": false,
    "title": "Replace the battery",
    "body": "Health is 45%. A replacement for the ThinkPad X1 Carbon 5th is usually a 10-minute job with a screwdriver.",
    "why": "You said you use it unplugged; this is the cheapest way to get hours back.",
    "helps": [],
    "shopping_query": "ThinkPad X1 Carbon 5th replacement battery"
  },
  {
    "id": "rec.machine_replace",
    "kind": "Replace",
    "impact": "Big gain",
    "cost": "$1200–$2500",
    "over_budget": true,
    "title": "Plan a replacement machine",
    "body": "This 9-year-old machine (2 cores, 16 GB) can't reach what you need. Look for at least 6 cores and 32 GB RAM (laptop).",
    "why": "Several of your workloads score poorly and the limiting parts can't be upgraded.",
    "helps": ["Running AI models locally"],
    "shopping_query": "laptop 6 cores 32GB RAM dedicated GPU"
  }
]
```

`kind`, `impact` and `cost` are already translated strings (`RecMap.svelte` compares them against `app.t("rec_kinds/…")` / `app.t("impacts/…")` and parses `$a–$b` from `cost`); `id` is stable. `shopping_query` is `null` when there is nothing to buy.

**Mock**: waits 400 ms and returns `fixtures/recs-<lang>.json` regardless of the answers.

### `shop_links`

| | |
|---|---|
| Rust | `fn shop_links(state, rec_id: String, region: String) -> Result<Vec<advisor::ShopLink>, String>` |
| JS | `api.shopLinks(recId, region)` |
| Tier | **Pro** (`tier < Tier::Pro` → `"pro"`) |
| Does | Finds `rec_id` in the recommendations stored by the last `advise` and builds vendor search URLs for `region` (`us` default; `uk de fr es it ca jp au` change the Amazon/eBay TLD). |
| Errors | `"pro"`, `"unknown recommendation"` (no `advise` yet, or the id is not in the last result) |

Args: `{ "recId": "rec.battery_replace", "region": "uk" }`. Return:

```json
[
  { "condition": "new", "vendor": "Amazon", "url": "https://www.amazon.co.uk/s?k=ThinkPad+X1+Carbon+5th+replacement+battery" },
  { "condition": "new", "vendor": "Newegg", "url": "https://www.newegg.com/p/pl?d=ThinkPad+X1+Carbon+5th+replacement+battery" },
  { "condition": "used", "vendor": "eBay", "url": "https://www.ebay.co.uk/sch/i.html?_nkw=ThinkPad+X1+Carbon+5th+replacement+battery&LH_ItemCondition=3000" }
]
```

`rec.machine_replace` additionally gets `Back Market` and `eBay Refurbished` entries with `"condition": "refurbished"`. A recommendation without a `shopping_query` returns `[]`.

**Mock**: throws `"pro"` unless `?tier=` is `pro`, `team` or `enterprise`; otherwise returns three fixed links (Amazon new, Back Market refurbished, eBay used) pointing at the vendors' home pages.

---

## Narration

### `narrate`

| | |
|---|---|
| Rust | `async fn narrate(state, lang: String, level: String, include_recs: bool) -> Result<narrate::Narrative, String>` |
| JS | `api.narrate(lang, level, includeRecs)` |
| Tier | **Supporter** (`tier < Tier::Supporter` → `"supporter"`) |
| Does | Reads the API key from `settings.anthropic_api_key` (non-empty) or the `ANTHROPIC_API_KEY` environment variable; renders the report in `lang`/`level`; optionally renders the last recommendations; scrubs to summary + verdict + findings + capabilities (`narrate.rs::scrub`); POSTs to `https://api.anthropic.com/v1/messages` with model `claude-opus-5`, `max_tokens` 4000, a system prompt that fixes the language, audience and 150–300-word bullet format. Returns the concatenated text blocks. The whole report is sent in one request; there is no streaming. |
| Errors | `"supporter"` · `"no_api_key"` · `"no report yet"` · `"network: <reqwest error>"` · `"bad response: <parse error>"` · `"API <status>: <message from error.message>"` (e.g. `"API 401 Unauthorized: invalid x-api-key"`) · `"The model declined to write this narrative."` (stop reason `refusal`) |

Args: `{ "lang": "en", "level": "informed", "includeRecs": false }` (`Narrative.svelte` always passes `false`). Return:

```json
{
  "text": "**What matters most**\n- The battery holds under half its original charge.\n- Two data drives are nearly full.\n\n**Still good for**\n- Everyday work, web development and home-server duty.\n\n**What to do**\n- Free the full drives; replace the battery if you travel.",
  "model": "claude-opus-5",
  "input_tokens": 1180,
  "output_tokens": 210
}
```

`text` is Markdown; the UI renders it with `marked`. `model` is what the API reported (may differ from the requested id when a server-side fallback applied).

**Mock**: throws `"supporter"` when `?tier=free` (the default); otherwise waits 1200 ms and returns the fixed text above with `"model": "mock"` and zero token counts.

---

## Cloud (Team / Enterprise)

### `cloud_upload`

| | |
|---|---|
| Rust | `async fn cloud_upload(state) -> Result<cloud::UploadResult, String>` |
| JS | `api.cloudUpload()` |
| Tier | **Team** (`tier < Tier::Team` → `"team"`) |
| Does | Reads `settings.cloud_endpoint` and `settings.cloud_token`; renders the stored report as `en`/`informed`; POSTs `cloud.rs::upload` to `<endpoint>/api/ext/innards.fleet/reports` with `Authorization: Bearer <token>`. The body is `{ machine: {…}, innards_version, report: <scrubbed Report>, rendered: <RenderedReport> }` — the full contract is in `docs/CLOUD.md`. Serials, hostname, MACs and the process list are removed before sending. |
| Errors | `"team"` · `"no_endpoint"` · `"no_token"` · `"no report yet"` · `"network: <reqwest error>"` · `"<status>: <error.message or 'upload failed'>"` (e.g. `"401 Unauthorized: unauthenticated"`, `"404 Not Found: plugin_disabled"`) |

Args: none (everything comes from settings). Return:

```json
{
  "machine_id": "m_0123456789abcdef",
  "report_id": "r_1",
  "dashboard_url": "https://crm.example.com/x/innards.fleet/machines/m_0123456789abcdef"
}
```

The fields are read from the plugin's response keys `machineId`, `reportId`, `dashboardUrl`; missing keys become `""` / `null`.

**Mock**: throws `"team"` unless `?tier=team|enterprise`; otherwise waits 600 ms and returns the example above.

---

## Settings and license

### `get_settings`

| | |
|---|---|
| Rust | `fn get_settings() -> settings::Settings` |
| JS | `api.getSettings()` |
| Tier | Free |
| Does | Reads `<config dir>/innards/settings.json` (`~/.config/innards/settings.json` on Linux); any missing or unparsable field falls back to `Settings::default()` (`#[serde(default)]`). |

Return:

```json
{
  "lang": "en",
  "level": "informed",
  "region": "us",
  "license_key": null,
  "anthropic_api_key": null,
  "elevate_for_smart": false,
  "cloud_endpoint": null,
  "cloud_token": null,
  "cloud_auto_upload": false,
  "machine_label": null
}
```

Defaults: `lang` is `"es"` when `$LANG` starts with `es`, else `"en"`; `level` `"informed"`; `region` `"us"`.

**Mock**: returns the in-memory settings object, initialised from `?lang=` / `?level=`.

### `set_settings`

| | |
|---|---|
| Rust | `fn set_settings(s: settings::Settings) -> Result<(), String>` |
| JS | `api.setSettings(s)` |
| Tier | Free |
| Does | Overwrites the whole settings file with pretty-printed JSON. It is a full replace, so callers must send every field (the frontend sends `$state.snapshot(app.settings)`). |
| Errors | serde or I/O error text |

Args: `{ "s": { …Settings… } }`. Returns `null`.

**Mock**: replaces the in-memory settings; later mock `analyze` calls use the new `lang`/`level`.

### `license_status`

| | |
|---|---|
| Rust | `fn license_status() -> license::Status` |
| JS | `api.licenseStatus()` |
| Tier | Free |
| Does | `license::status(settings.license_key)`. If `INNARDS_TIER` is set in the environment it wins (`source: "env"`, `org` from `INNARDS_ORG`). Otherwise the key is verified offline (`INNARDS-` prefix, base64url payload + Ed25519 signature over the payload bytes, `exp` date not in the past) → `source: "key"`; an absent or invalid key → `tier: "free"`, `source: "none"`. |

Return:

```json
{ "tier": "pro", "email": "ana@example.com", "expires": "2027-09-24", "org": null, "source": "key", "renewable": true }
```

`tier` ∈ `free | supporter | pro | team | enterprise`. `org` is set on Team/Enterprise keys. `source` ∈ `none | key | env` (`mock` in design mode). `renewable` is true when the key payload carries a Stripe subscription id (`sub`), i.e. it was issued by the billing worker for Pro or Team; `state.svelte.ts::maybeRenew` then calls `license_refresh` at startup when `expires` is within 30 days.

**Mock**: `tier` from `?tier=` (default `free`); `email` `"you@example.com"` for any paid tier; `expires` `"2027-09-21"` for pro/team/enterprise; `org` `"acme"` for team/enterprise; `source` `"mock"`; `renewable` for pro/team.

### `activate_license`

| | |
|---|---|
| Rust | `fn activate_license(key: String) -> Result<license::Status, String>` |
| JS | `api.activateLicense(key)` |
| Tier | Free |
| Does | Verifies `key`; if it does not yield a paid tier (and no `INNARDS_TIER` override is active) returns `"invalid"`; otherwise stores it in `settings.license_key` and returns the new status. Whitespace around the key is trimmed by `verify`. |
| Errors | `"invalid"`, settings save error text |

Args: `{ "key": "INNARDS-eyJ0aWVyIjoicHJvIi….MEUCIQ…" }`. Return: same shape as `license_status`.

**Mock**: always throws `"invalid"` (there is no public key in the browser build).

### `license_refresh`

| | |
|---|---|
| Rust | `async fn license_refresh() -> Result<license::Status, String>` |
| JS | `api.licenseRefresh()` |
| Tier | Any saved key (no tier check); only useful for keys with a Stripe `sub` |
| Does | Reads `settings.license_key`, calls `GET <INNARDS_BILLING_URL or https://innards.app>/api/license/refresh?key=<urlencoded key>` (`license.rs::refresh`), verifies the returned key offline, saves it, returns the new status. This is the only network call the free-to-Pro path can make and it sends nothing but the key. The frontend calls it silently at startup when `renewable` is true and `expires` is within 30 days; there is no button. |
| Errors | `"no_key"` (nothing saved) · `"network: <reqwest error>"` · the worker's `error.code` on a non-2xx (`"invalid"`, `"cancelled"`, `"inactive"`, `"not_found"`, `"internal"`, or `"refresh_failed"` when absent) · `"no key in response"` · `"invalid"` (the returned key does not verify against the embedded public key) · settings save error |

Args: none. Return: same shape as `license_status`, with the renewed `expires`.

**Mock**: throws `"no_key"`.

### `app_info`

| | |
|---|---|
| Rust | `fn app_info() -> AppInfo` |
| JS | `api.appInfo()` |
| Tier | Free |
| Does | Versions and OS for the Settings footer. |

Return: `{ "version": "0.1.0", "core_version": "0.1.0", "os": "linux" }` (`os` is `std::env::consts::OS`: `linux`, `macos`, `windows`).

**Mock**: `{ "version": "0.1.0-mock", "core_version": "0.1.0", "os": "browser" }`.

---

## Mock summary

`src/lib/mock.ts` is selected whenever `window.__TAURI_INTERNALS__` is absent. It:

- Reads `?tier`, `?lang`, `?level` once at load. `?snap=1` switches fixture loading to synchronous `XMLHttpRequest`, removes all artificial delays, and adds the `no-anim` class to `<html>`.
- Throws plain strings for gated commands so the UI's error handling is exercised exactly as in Tauri: `"pro"`, `"supporter"`, `"team"`, `"invalid"`, `"no_key"`.
- Throws `Error("mock: unknown command <name>")` for anything not listed above — add a case when you add a command.
- Never touches the network; every response comes from `static/fixtures/` or a literal.

When a command's return type changes, update in this order: Rust struct → `src/lib/types.ts` → `mock.ts` case → regenerate fixtures → this file.

## Added 2026-09-21 (after the first draft)

| Command | Args | Returns | Gate | Errors |
|---|---|---|---|---|
| `history` | `limit?: number` (default 365) | `HistoryEntry[]` — `{ at, health, critical, warning, battery_health, root_free_pct, memory_available_pct, cpu_c }`, oldest first | Supporter | `"supporter"` |
| `history_clear` | — | `void` | — | io error text |
| `read_text` | `path: string` | file contents (≤ 8 MB) | — | `"file too large"`, io error |
| `load_report_json` | `text: string, lang, level` | `RenderedReport` of the loaded report; replaces the current view until the next `analyze` | Pro | `"pro"`, `"not an Innards report: …"` |
| `viewing_other_machine` | — | `boolean` | — | — |

`analyze` now also appends a line to `history.jsonl` and, for Team keys with `cloud_auto_upload`, uploads in the background.
A scheduler (`start_scheduler` in `src-tauri/src/lib.rs`) rescans and uploads every `settings.cloud_interval_hours` (default 24)
while the app is open. `advise` is now gated to Supporter in Rust (`"supporter"`), not only in the UI.

Mock behaviour: `history` returns 30 synthetic days when `?tier` is not free; `load_report_json` returns the fixture report;
`viewing_other_machine` is `false`.
