# Working agreement

For AI coding agents and human contributors alike. Read `docs/ARCHITECTURE.md` first if you have not; this file is the checklist.

## Repository map

```
Cargo.toml                    workspace: crates/innards-core, src-tauri, tools/innards-license
package.json                  pnpm scripts: dev, build, check, tauri
crates/innards-core/
  src/lib.rs                  pipeline entry (analyze), re-exports
  src/probe/{mod,common,linux,macos,windows,smart,util}.rs
  src/snapshot.rs             data model of what was measured
  src/rules.rs                every finding + threshold   (no prose)
  src/finding.rs              Finding, Severity, Category, fmt_bytes, ram_marketing_gb
  src/capability.rs           ten workloads, scoring, grades
  src/advisor.rs              questionnaire, recommendations, shop links
  src/i18n.rs                 LANGS table, catalog lookup, {param} fill
  src/report.rs               Report, Summary, render, render_recs, to_markdown
  i18n/en.json, es.json       every user-facing string
  tests/catalogs.rs           catalog completeness — the test that keeps prose out of Rust
  examples/report.rs          CLI report; examples/fixtures.rs writes design-mode JSON
src-tauri/
  src/lib.rs                  every Tauri command (docs/IPC.md)
  src/settings.rs             settings.json in the OS config dir
  src/license.rs              Ed25519 key verification, Tier
  src/narrate.rs              Claude API narration (Supporter)
  src/cloud.rs                Threadwise upload (Team/Enterprise)
  tauri.conf.json             window, CSP, bundle
  capabilities/default.json   plugin permissions
billing/                      Cloudflare Worker: Stripe Checkout → license keys (docs/MONETIZATION.md)
integrations/threadwise-fleet/ Threadwise plugin `innards.fleet`: server/, web/, test/ (docs/CLOUD.md)
src/
  routes/+layout.svelte       shell: TopBar, error card, Scanning, then the page
  routes/{+page,advisor,settings}  the three views (Report at /)
  lib/api.ts                  invoke wrappers; picks mock outside Tauri
  lib/mock.ts                 fixture-backed backend for the browser
  lib/state.svelte.ts         the one AppState (runes)
  lib/types.ts                TS mirrors of the Rust IPC types
  lib/platform.ts             opener/dialog shims
  lib/components/*.svelte     UI
  app.css                     design tokens (docs/DESIGN.md)
  app.html                    ?snap=1 load-hold
static/fixtures/*.json        GENERATED — do not edit by hand
tools/innards-license/        keygen / sign / verify (never shipped)
docs/                         ARCHITECTURE, IPC, AGENTS, DESIGN, MONETIZATION, CLOUD, press-kit/
web/, site/                   marketing site sources (separate builds; see docs/DESIGN.md)
```

## Commands

Prerequisites: Rust via rustup (`~/.cargo/bin` on `PATH`), Node 22 + pnpm, Tauri 2 platform deps (https://tauri.app/start/prerequisites/). On Linux, `smartmontools` and optionally `nvme-cli` for SMART.

| What | Command |
|---|---|
| Install JS deps | `pnpm install` |
| Desktop app, hot reload | `pnpm tauri dev` |
| Desktop app, fake tier | `INNARDS_TIER=pro pnpm tauri dev` (`supporter`, `pro`, `team`, `enterprise`; `INNARDS_ORG=acme` for an org) |
| UI only, in a browser | `pnpm dev` → http://localhost:1420/ (design mode; fixtures, no Rust) |
| Core tests (catalogs) | `cargo test -p innards-core` |
| Type-check the frontend | `pnpm check` (svelte-check) |
| Build the shell crate | `cargo build -p innards` |
| Installers | `pnpm tauri build` → `target/release/bundle/` |
| Report in the terminal | `cargo run -p innards-core --example report -- en expert` (`es plain`, `--json` for the raw `Report`) |
| Regenerate fixtures | `cargo run -p innards-core --example fixtures -- static/fixtures` |
| License tool | `cargo run -p innards-license -- keygen` / `sign …` / `verify …` |

## Conventions

1. **Rules carry no prose.** A `Finding` is an id, a severity, a category, params and English evidence lines. Sentences live in `crates/innards-core/i18n/*.json` and are chosen by `report::render`. If you find yourself writing a sentence in Rust that a user will read, stop and move it to the catalog. The only English strings in the core crate are evidence lines (`.evidence(...)`), `probe_notes` (diagnostic, expert-only), `shopping_query` (a search term), and vendor names.
2. **Every user-facing string is in every catalog.** `tests/catalogs.rs` checks that every `findings.*` id used in `rules.rs` and every `recs.*` id in `advisor.rs` exists in every language with every required field, that every section has the English key set, and that `{placeholders}` match. Add to `en.json` and `es.json` together. The Svelte components carry a few chrome strings inline (`{ en, es }` maps); add both languages there too.
3. **Findings have stable ids** of the form `<category>.<snake_name>`, where the category prefix is one of `memory. storage. cpu. thermal. battery. gpu. network. system.`. The id is the i18n key and what the UI keys lists on. Do not rename an id without renaming it in every catalog.
4. **Severities mean something.** `Critical` = at risk / act soon (−12 health). `Warning` = costs performance or reliability (−5). `Info` = useful, no action (0). `Good` = worth saying so (+1). Thresholds are in `rules.rs`, near the rule, and should be explainable in the `expert` string.
5. **Never store or transmit serials, hostnames, MAC addresses or process lists.** The only persistent file is `settings.json`. Anything that leaves the machine goes through a scrubber: `narrate.rs::scrub` (sends rendered text only), `cloud.rs::scrub` (nulls the four fields above), `examples/fixtures.rs` (nulls serials and hostname before committing fixtures). A new identifying snapshot field must be added to the scrubbers in the same change.
6. **The free tier makes no network calls.** `innards-core` has no HTTP dependency. `reqwest` is used in exactly three places: `narrate.rs` and `cloud.rs` (behind a `Tier` check in `lib.rs` and an explicit user action or opt-in setting) and `license.rs::refresh` (only reachable with a saved subscription key). The webview CSP (`tauri.conf.json`) allows `connect-src` to IPC only.
7. **Tier gating lives in `src-tauri/src/lib.rs`** and compares with `<` against the ordered `Tier` enum, so higher tiers include lower ones. In the UI use `app.has("supporter")`, never `app.tier === "supporter"`.
8. **The UI renders, it does not compose.** Components display `RenderedReport` / `RenderedRecommendation` fields. Do not build sentences from snapshot numbers in Svelte.
9. **Probes never fail.** Missing data → `None` and, if the user can fix it, a `probe_notes` line. No `unwrap` on OS data; no panics; no blocking longer than the existing sleeps.
10. **Keep the mock in step.** A new or changed command needs a case in `src/lib/mock.ts`, a type in `src/lib/types.ts`, a wrapper in `src/lib/api.ts`, and (if it returns report-shaped data) a fixture.
11. **Style**: Rust 2021, `cargo fmt` defaults but long one-line rule pushes are accepted in `rules.rs`; Svelte 5 runes (`$state`, `$derived`, `$props`, snippets), no stores; CSS through the tokens in `src/app.css` — no new colours.

## Before you claim a task is done

Run all three, from the repository root:

```bash
cargo test -p innards-core     # catalogs complete, placeholders match
pnpm check                     # svelte-check: 0 errors
cargo build -p innards         # the Tauri shell compiles against the core
```

Then, depending on what you touched:

- Changed a rule, recommendation, workload, catalog or snapshot field → regenerate fixtures (`cargo run -p innards-core --example fixtures -- static/fixtures`) and look at the result in design mode (`pnpm dev`, open http://localhost:1420/advisor?tier=pro&demo=1).
- Changed a probe → `cargo run -p innards-core --example report -- en expert` on a real machine and read the evidence lines and probe notes.
- Changed a command → update `docs/IPC.md` and `mock.ts`.
- Changed tokens or components → take screenshots (below) and compare against `docs/press-kit/screenshots/`.
- Changed tiers, keys or gating → update `docs/MONETIZATION.md`.

## Regenerating fixtures

`static/fixtures/` is written by `crates/innards-core/examples/fixtures.rs` from **this** machine's snapshot: `report-{lang}-{plain,informed,expert}.json`, `catalog-{lang}.json`, `recs-{lang}.json` (advisor answers fixed in the example: web dev + containers + home server + local LLM, slow + out of memory, under $800, six months, portable), `questions.json`, `languages.json`, and `report-full.json` (serials and hostname nulled). Regenerate after any change to rules, recs, catalogs, workloads or the snapshot. The fixtures are committed because design mode and the press-kit screenshots depend on them; expect the diff to include live numbers (temperatures, load) — that is fine.

## Screenshots in design mode

`?snap=1` makes a headless capture deterministic: fixtures load synchronously, animations are off, and `src/app.html` holds the page's load event for 2.5 s via `/__snap-hold?ms=2500` (a Vite middleware in `vite.config.js`) so the report has rendered when the browser fires `load`.

```bash
pnpm dev &                                        # http://localhost:1420
# Chromium / Chrome headless, light theme, 1180 px wide (the press-kit width):
chromium --headless=new --hide-scrollbars --window-size=1180,1500 \
  --screenshot=report-light-en.png \
  "http://localhost:1420/?snap=1&tier=supporter&lang=en&level=informed"
# Dark theme: emulate prefers-color-scheme with Playwright
node -e '
const { chromium } = require("playwright");
(async () => {
  const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 1180, height: 1300 }, colorScheme: "dark" });
  await p.goto("http://localhost:1420/advisor?snap=1&tier=pro&lang=es&demo=1", { waitUntil: "load" });
  await p.screenshot({ path: "advisor-dark-es.png", fullPage: true }); await b.close();
})();'
```

Useful URL combinations: `/?snap=1&level=plain`, `/?snap=1&level=expert` (evidence toggles), `/advisor?snap=1&tier=pro&demo=1&view=visual` (impact map), `/settings?snap=1&tier=team` (cloud section enabled). Check the file before committing: a Vite error overlay screenshots just as happily as the app.

## Generated files — do not edit by hand

- `static/fixtures/*.json` — from `examples/fixtures.rs`.
- `src-tauri/gen/schemas/*` — written by `tauri` on every dev/build; gitignored.
- `Cargo.lock`, `pnpm-lock.yaml` — by the package managers.
- `build/`, `.svelte-kit/`, `target/` — build output; gitignored.
- `src-tauri/icons/*` — from `pnpm tauri icon <source.png>` when the source icon changes.

## Do not

- **Do not add network calls to the free tier.** No telemetry, no update checks, no CDN fonts, no analytics — in Rust or in the webview. If a feature needs the network it is a paid, opt-in feature with a tier check in `lib.rs` and a scrubber.
- **Do not write prose in Rust.** Not in rules, not in the advisor, not in the shell. The catalogs are the single source of sentences; that is what makes three levels and N languages possible.
- **Do not bypass the catalog test.** No `#[ignore]`, no lowering the `> 20` / `> 5` id-count assertions, no adding an id to `en.json` only. If the test fails, the catalogs are incomplete.
- **Do not commit `signing.key`** or any private key. `.gitignore` excludes `signing.key`, `*.key` and `billing/.dev.vars` (which holds the same seed as `LICENSE_SIGNING_KEY` for local Worker runs); keep it that way. The public key is the only key in the repository (`license.rs::PUBLIC_KEY_B64`). Never paste a key into a doc, a test or a fixture.
- **Do not store or send serials, hostnames, MACs or process lists.** Also not "just for debugging".
- **Do not edit fixtures by hand** or hand-write a `report-*.json`; regenerate.
- **Do not gate with string equality** on the tier in the UI; use `app.has(...)`.
- **Do not add a dependency to `innards-core`** without checking it has no network or platform-specific requirement; the crate must build and run on all three OSes and stay small.
- **Do not put user-visible copy in `mock.ts`** beyond the fixed mock narrative; the mock imitates the backend, it does not design the UI.
- **Do not change the license-key format** (`INNARDS-<b64url(json)>.<b64url(sig)>`) without updating `tools/innards-license` and `docs/MONETIZATION.md` in the same change; keys already issued must keep verifying.
