# Innards — working agreement for every agent/session in this repo

Two Claude Code sessions work on this project: one on the ThinkPad (`~/Projects/innards`, owns the app,
core, billing, docs, `site/`) and one on rusoriz (has been building `web/`). Read this before touching
anything shared. Full contributor guide: `docs/AGENTS.md`. Architecture: `docs/ARCHITECTURE.md`.

## Decisions already made (don't reopen without Eugene)
- Product name **Innards**; domain **innards.app** (registered, zone in Cloudflare; innards.io is NOT ours).
- Desktop app: Tauri 2 + Rust core + Svelte 5, one codebase for Linux/macOS/Windows; Android/CLI via `crates/innards-cli`.
- Tiers: Free / Supporter $9 once / Pro $29 yr / Team $4 per machine per month (min 5) / Enterprise from $2,500 yr.
  Team & Enterprise are backed by Threadwise via `integrations/threadwise-fleet`.
- Billing: Stripe → Ed25519 license keys, implemented in `billing/` and mounted as Cloudflare Pages Functions at `/api/*`.

## The website: ONE canonical site
- **Canonical = `site/`** (static HTML in `site/public`, Pages Functions in `site/functions`). It is deployed as the
  Cloudflare Pages project `innards` → https://innards.pages.dev with `innards.app` + `www.innards.app` attached.
  Deploy: `cd site && pnpm dlx wrangler@4 pages deploy --project-name innards --branch main`.
- **`web/` is not a second site.** Its interactive demo (TS port of the engine + real snapshots) is the part worth
  keeping: build it as a self-contained static bundle and place it at `site/public/demo/` (linked from the home
  page), or as a `<script type="module">` island. Do not add pages to `web/`, do not deploy `web/` anywhere,
  do not point `innards.app` at a Worker. Once the demo is merged, `web/` will be deleted.
- Product claims on the site come only from `crates/innards-core/i18n/en.json` and the press kit. No fake ratings,
  testimonials or stats. No analytics, no external scripts/fonts.

## Ownership (to avoid clobbering each other)
- ThinkPad session: `crates/`, `src/`, `src-tauri/`, `billing/`, `integrations/`, `docs/`, `site/`, `tools/`, `.github/`.
- rusoriz session: `web/` only, until its demo is merged into `site/public/demo/` (coordinate in `docs/WEBSITE.md`).
- Anyone: fix typos/bugs anywhere, but say so in the commit message.

## Before you say "done"
`cargo test -p innards-core && pnpm check && cargo build -p innards` from the repo root. Regenerate fixtures after core
changes: `cargo run -p innards-core --example fixtures -- static/fixtures`.

## Never
- Commit `signing.key`, `.dev.vars`, any `sk_live_`/`tw_`/OAuth token. `.gitignore` covers the known ones; check yours.
- Add network calls to the free tier. Send serials, hostnames, MACs or process lists anywhere.
- Put prose in Rust: every user-facing string lives in `crates/innards-core/i18n/*.json` (test enforces it).
