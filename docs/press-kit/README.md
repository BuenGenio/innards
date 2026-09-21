# Innards — Press Kit

*Everything a journalist or reviewer needs to cover Innards. Quote freely. Screenshots are CC0.*

---

## One-liner

**Innards looks inside your computer and tells you, in plain language, what's wrong, what it's still good for, and what's actually worth buying.**

## Short description (≈50 words)

Innards is a free, open-source desktop app for Linux, macOS and Windows that inspects your machine — CPU, memory, drives, battery, thermals — and explains what it finds at the level of detail you choose, in your language. It scores what the computer is still good for and, optionally, recommends upgrades that are worth the money.

## Long description (≈180 words)

Most hardware tools show you numbers. Innards tells you what they mean.

Run it and in about a second you get a health score, a short verdict, and a bulleted report sorted by what needs attention now, what's worth fixing, what's good to know, and what's working well. Every item can be read three ways — *plain* for someone who doesn't want jargon, *informed* for a comfortable user, *expert* for people who want `percentage_used=1%` and `unsafe_shutdowns=69` with the raw evidence attached. Switch language and level at any time; the same findings re-render instantly.

Below the findings, Innards answers the question people actually have about an ageing machine: *what is it still good for?* Ten workloads, from everyday use to running AI models locally, each with a grade and the limiting factor named.

The upgrade advisor asks five questions and returns a ranked, costed list — free software fixes first, and honest when the answer is "keep it" or "this chassis can't get you there." A Pro tier adds where to buy each item new, refurbished or used.

Nothing leaves the machine. No account, no telemetry. Rust + Tauri, MIT licensed.

## For teams

Team and Enterprise put the same reports on a fleet dashboard: each machine uploads its scrubbed report (no serial numbers, hostnames, MAC addresses or process lists) to a Threadwise workspace — an open-source CRM the organisation can self-host — where an `innards.fleet` plugin shows every machine's health over time, the findings that recur across the fleet, capability averages, and which person or organisation owns each machine. Team is $4 per machine per month or $39 per machine per year, minimum 5 machines. Enterprise (from $2,500/year) adds what Threadwise already provides for regulated environments — SSO/SAML, role-based access, a hash-chained audit log, data residency controls — plus custom rules, an SLA and priority support.

## Key facts

| | |
|---|---|
| Product | Innards |
| Category | System utility / hardware diagnostics |
| Platforms | Linux (deb, AppImage, rpm), macOS (dmg, Apple silicon + Intel), Windows (msi, exe); `innards` CLI for aarch64 Linux phones/SBCs (postmarketOS) and Android/Termux |
| Price | Free core. Supporter $9 one-time (pay what you want from $5). Pro $29/year. Team $4 per machine/month or $39 per machine/year (minimum 5 machines). Enterprise custom, from $2,500/year. |
| License | MIT (source), proprietary paid features are gated by offline license keys but the code is public |
| Tech | Rust engine, Tauri 2 shell, Svelte 5 UI. ~8 MB download on Linux. |
| Languages | English, Spanish (more via a single JSON file — translations welcome) |
| Privacy | No network access in the free tier. Narrated summaries (Supporter) call the Claude API with the user's own key and send only rendered findings — never serials, hostnames or process lists. |
| Source | github.com/buengenio/innards |
| Website | https://innards.app |
| Developer | Eugene Trotsan, independent |
| Origin | Ubuntu 25.10 on a 2017 ThinkPad X1 Carbon — Innards' own first patient |

## Why it exists (the story)

Innards started as a debugging session: a ThinkPad that hung on reboot and couldn't write its own logs. Working through `journalctl`, `nvme smart-log` and `/sys` by hand surfaced a drive that had been at critical temperature for 4½ hours of its life, 69 unsafe shutdowns, a battery at 45%, and — the real culprit — no swap at all on a 16 GB machine running a home server. None of that was hard to find. All of it was hard to *read*. Innards is that afternoon, turned into a tool anyone can run.

## What makes it different

- **Three reading levels of the same report.** Not a "simple mode" that hides things — the same findings, phrased for three audiences, with the raw evidence one click away.
- **"What is it still good for?"** Capability grades per workload, with the limiting part named. Most tools tell you what you have; Innards tells you what that means for what you want to do.
- **An advisor that will tell you not to spend.** Free fixes rank first. Soldered RAM and no GPU slot are called out as the reason to change machine, not a part.
- **Rules, not a black box.** Every finding is a small, readable rule with a threshold you can check in the source. The optional AI narration only rephrases findings the rules already produced.
- **Local by default.** The report never leaves the machine unless the user exports it.

## Screenshots

All 1180 px wide, PNG, CC0. In `screenshots/`:

| File | Shows |
|---|---|
| `report-light-en.png` | Report view, informed level, light theme, Supporter tier |
| `report-plain-light-en.png` | Same report at the plain level |
| `report-expert-dark-en.png` | Same report at the expert level, dark theme, evidence toggles |
| `advisor-dark-es.png` | Upgrade advisor in Spanish, dark theme, Pro tier with where-to-buy |
| `advisor-visual-light-en.png` | Advisor's visual mode: impact-vs-cost map with the budget line |

The machine in every screenshot is real: a 2017 ThinkPad X1 Carbon 5th gen used as a home server.

## Suggested angles

- *For Linux outlets:* a Rust/Tauri utility that reads `/sys`, `/proc`, DMI, hwmon and SMART without root, explains them, and packages as deb/AppImage/rpm. The expert level is written for your readers.
- *For general tech:* "Is my old laptop still good for anything?" — answered per workload, with an upgrade advisor that's honest about when the answer is no.
- *For privacy-minded:* hardware diagnostics that never phone home in the free tier; every paid network feature is opt-in, documented, and sends the minimum (your own API key for narration; a scrubbed report for team dashboards).
- *For open-source:* MIT code with paid tiers gated by offline Ed25519 keys — a small experiment in sustainable indie desktop software.

## Quotes you can use

> "Every hardware tool I'd used showed me numbers. I wanted one that would tell my mother what the numbers meant, and tell me the raw values in the same window." — Eugene Trotsan, creator

> "The best upgrade advice is sometimes 'don't'. Innards will say that." — Eugene Trotsan

## FAQ

**Does it need root / administrator?** No. Everything except SMART drive health reads without elevated rights. SMART is an opt-in checkbox that triggers the normal system password prompt.

**Does it send my data anywhere?** The free app makes no network requests at all. Three paid features do, each narrowly: Supporter narration sends only the already-rendered findings to the Claude API with a key you provide; Pro/Team subscription keys contact innards.app once at startup when the key is within 30 days of expiry (only the key is sent, to renew it); Team upload sends a scrubbed report — no serial numbers, hostname, MAC addresses, process list, or usernames in paths — to your organisation's own Threadwise server.

**Why pay if it's open source?** You don't have to. The paid tiers fund development and unlock convenience features (advisor, narration, shopping links). The code for them is in the same repo.

**What does the AI feature actually do?** It rephrases the structured findings into a short narrative. It does not diagnose anything itself; the rules do.

**Will it run on my Raspberry Pi / phone / VM / Steam Deck?** Linux builds run anywhere WebKitGTK does; VMs are detected and battery/thermal findings are suppressed. ARM boards and phones use the `innards` CLI (static aarch64 binary): it knows 40 SoCs, their mainline-Linux status, and scores "portable server" and "on-device AI" honestly — a rooted OnePlus 6T on postmarketOS grades *Excellent* as a pocket server and *Workable* for on-device classification (no fan, so sustained loads throttle).

**How do I add my language?** Copy `crates/innards-core/i18n/en.json`, translate, add one line to `i18n.rs`. A test checks you didn't miss a string.

## Contact

Eugene Trotsan — eugene.trotsan@gmail.com
Review builds and license keys for reviewers on request.

---

## Pitch notes (internal — not for publication)

**Targets and what each cares about**
- **OMG! Ubuntu** (Joey Sneddon): loves polished new Linux desktop apps with a story; tips via the site's contact form or @omgubuntu. Lead with the screenshot and the ThinkPad story; mention deb + AppImage.
- **9to5Linux** (Marius Nestor): release-oriented; send a proper release announcement with a version number, changelog, and download links. Rust/Tauri detail welcome.
- **It's FOSS**: app roundups and "best tools" lists; pitch the "is my old laptop still good for anything" angle.
- **Phoronix**: only if there's a benchmarkable/technical hook — e.g. the probe layer, NVMe APST findings. Otherwise skip.
- **Hacker News / Lobsters**: "Show HN: Innards – explains what your computer is good for, in plain language" — post at 8–9am ET on a weekday; be in the comments for the first two hours.
- **r/linux, r/thinkpad, r/selfhosted**: the origin story plays well in r/thinkpad; r/selfhosted for the "laptop as home server" angle.

**Embargo / timing**: no embargo needed for v0.1. Ship builds for all three OSes *before* pitching — reviewers will try Windows and macOS first.

**Have ready**: a 30-second screen recording of a scan → level switch → language switch; the deb/AppImage/dmg/msi links; two reviewer Pro keys.
