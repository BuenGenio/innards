# innards.fleet — Threadwise plugin

A [Threadwise](../../../threadwise) plugin that turns Innards machine reports into a fleet dashboard.
The Innards desktop app (Team / Enterprise tiers) uploads the report it produced for the machine it
runs on; Threadwise stores it per workspace, shows every machine with its health, findings and
history under **Machines**, and lists the machines a person or organisation owns on their record.

```
Innards app ──POST /api/ext/innards.fleet/reports──▶ Threadwise API ──▶ innards_machines
  (Report + RenderedReport, Bearer tw_…)               (plugin routes)    innards_reports
                                                                                │
                                        /x/innards.fleet  ◀── fleet dashboard ──┘
                                        person / organisation record → "Machines" panel
```

The plugin lives in the Innards repository and is loaded by Threadwise from outside its source
tree, so the server module imports nothing from `@crm/*` or `hono` (see [Loading](#loading)).

## What it adds

| Extension point | What |
|---|---|
| Tables | `innards_machines` (one row per machine and workspace, with the latest health, counts and top finding denormalised) and `innards_reports` (every upload: health, counts, compact findings/capabilities for analytics, and the scrubbed `report_json` / `rendered_json`). Index on `(workspace_id, machine_id, received_at)`. |
| Routes | `/api/ext/innards.fleet/…` — see [API](#api). Reads are open to workspace members; writes need `sources:manage` (same permission as Data Sources). |
| Navigation | **Machines** in the primary nav (`g` `m`), `/x/innards.fleet`; **Innards fleet** under Settings. |
| Record panel | **Machines** on people and organisations (server-rendered from `GET /panel`; the web module replaces it with a richer one). |
| Schedule | `prune`, daily: deletes reports older than `retentionDays`, always keeping each machine's latest. |
| Settings | `retentionDays` (number, default 365) · `allowAnonymousMachines` (boolean, default true — when off, a machine must be registered and assigned to a person or organisation before its reports are stored). |

Machine ids are chosen by the client and scoped per workspace (`PRIMARY KEY (workspace_id, id)`),
so two workspaces receiving the same machine id never collide.

## Loading

Point Threadwise at this directory; the API loader picks `server/index.ts`, the Vite plugin picks
`web/index.tsx`:

```bash
# API (Bun)
THREADWISE_PLUGINS=/path/to/innards/integrations/threadwise-fleet \
  bun apps/api/src/index.ts

# Web bundle (build or dev)
THREADWISE_WEB_PLUGINS=/path/to/innards/integrations/threadwise-fleet/web \
  bun run --cwd apps/web build
```

Both variables accept comma- or colon-separated lists, so this plugin can sit next to others.
The server half works without the web half: `/x/innards.fleet` then shows the generic plugin page
and the record panel is rendered from the JSON `GET /panel` returns.

Once loaded it appears in **Settings → Plugins** per workspace (enable/disable, settings). A
disabled plugin answers `404 plugin_disabled` on every route, so uploads are refused too.

### Why the server module has no imports

Bun resolves a module's imports from the module's own location. From
`innards/integrations/threadwise-fleet/server/`, neither `@crm/api`, `@crm/shared` nor `hono` is
reachable (Threadwise's `apps/api` is a workspace package with no `exports`, and there is no
tsconfig alias for it). The module therefore declares minimal structural types for the
`PluginContext`, the Hono sub-app and the request context, and re-implements
`requirePermissionForWrites('sources:manage')` on top of `c.get('actor').permissions` — the same
check `apps/api/src/lib/actor.ts` does. Error responses use Threadwise's envelope
`{ error: { code, message, details? } }` directly instead of throwing `ApiError`.

If you would rather use the real helpers, symlink the directory into
`apps/api/src/plugins/external/` and switch to relative imports; nothing else changes.

The web module imports the shell's `@/…` modules and the packages Vite dedupes for external
plugins (`react`, `react-router-dom`, `@tanstack/react-query`). It deliberately does not import
`lucide-react` (not deduped): the two icons it needs are inline SVG.

## API

All routes: `/api/ext/innards.fleet/<path>`, workspace from the API key (or `X-Workspace-Id` in
dev mode). Errors: `{ error: { code, message, details? } }`.

| Method | Path | Notes |
|---|---|---|
| POST | `/reports` | Ingest one report (below). `201 { machineId, reportId, dashboardUrl }`. Bodies over 2 MB → `413 payload_too_large`. |
| GET | `/machines?q=&ownerKind=&ownerId=` | `{ items: MachineSummary[], total, exact, nextCursor }`, worst health first. `q` matches label, model, OS, CPU and owner name. |
| POST | `/machines` | `{ id, label?, ownerPersonId?, ownerOrgId? }` — pre-register a machine (needed when `allowAnonymousMachines` is off). |
| GET | `/machines/:id` | `{ machine, latest: { …ReportSummary, rendered: RenderedReport }, history: [{ reportId, received_at, health_score, critical_count, warning_count }] }` (last 90 reports, oldest first). |
| GET | `/machines/:id/reports` | Report summaries, newest first. |
| GET | `/reports/:id` | Full stored report: summary + `report` + `rendered`. |
| PATCH | `/machines/:id` | `{ label?, ownerPersonId?, ownerOrgId? }`. Owner ids are checked against the workspace; setting one kind of owner clears the other. |
| DELETE | `/machines/:id` | Removes the machine and all its reports. `204`. |
| GET | `/analytics` | `{ machines, avgHealth, healthBuckets: { critical, warning, ok, good }, attention, staleMachines, topFindings: [{ id, title, count }], upgradeBudget: { low, high } \| null, capabilityAverages: [{ workload, avg }] }` over each machine's latest report. |
| GET | `/panel?kind=person\|organisation&id=` | `PluginPanelContent` for the record panel. |

`MachineSummary`: `id, label, vendor, product, chassis, os, cpu, ramGb, storage, gpu, owner
({ kind, id, name } | null), ownerPersonId, ownerOrgId, firstSeen, lastSeen, lastHealth,
healthBucket, lastReportId, criticalCount, warningCount, topFinding ({ id, title, severity } | null),
innardsVersion, stale`.

Health buckets: `good` ≥ 90, `ok` ≥ 75, `warning` ≥ 50, `critical` below. A machine is `stale`
after 30 days without a report. `attention` = critical + warning machines.

### Ingest contract

```jsonc
POST /api/ext/innards.fleet/reports
Authorization: Bearer tw_…            // workspace API key with sources:manage
Content-Type: application/json

{
  "machine": {
    "id": "mach_9f1c…",                // stable per machine, 8–128 chars of [A-Za-z0-9._:-]; never the hostname
    "label": "Ann’s X1 Carbon",        // optional; kept across uploads once set
    "vendor": "LENOVO", "product": "ThinkPad X1 Carbon 5th", "chassis": "laptop",
    "os": "Ubuntu 25.10", "cpu": "Intel Core i7-7500U · 2C/4T", "ram_gb": 16,
    "storage": "1.8 TiB NVMe", "gpu": "HD Graphics 620"   // all optional: derived from the report when absent
  },
  "innards_version": "0.4.0",
  "report":   { /* innards_core::report::Report — static/fixtures/report-full.json */ },
  "rendered": { /* innards_core::report::RenderedReport — static/fixtures/report-en-informed.json */ }
}
```

`report` must carry `snapshot.system`, `findings[]` (`id`, `severity`), `capabilities[]`
(`workload`, `score`) and `summary.health_score`; `rendered` must carry `summary`, `findings[]`
(`id`, `title`) and `capabilities[]`. If the app also attaches the upgrade advisor's result as
`report.recommendations` (structured `cost_usd: [low, high]`) or `rendered.recommendations`
(`cost: "$40–$90"`), the cost bands are summed into `/analytics.upgradeBudget`.

Try it against the fixtures:

```bash
BASE=https://crm.example.com          # Threadwise base URL
KEY=tw_xxxxxxxxxxxxxxxxxxxxxxxx       # Settings → Identity & access → API keys
cd innards
jq -n --slurpfile r static/fixtures/report-full.json \
      --slurpfile x static/fixtures/report-en-informed.json \
      '{ machine: { id: "mach_fixture_x1carbon", label: "Fixture X1 Carbon" },
         innards_version: "0.4.0", report: $r[0], rendered: $x[0] }' |
curl -sS -X POST "$BASE/api/ext/innards.fleet/reports" \
     -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" --data-binary @-
# → {"machineId":"mach_fixture_x1carbon","reportId":"rpt_…","dashboardUrl":"https://crm.example.com/x/innards.fleet/machines/mach_fixture_x1carbon"}

curl -sS "$BASE/api/ext/innards.fleet/machines"  -H "Authorization: Bearer $KEY" | jq '.items[] | {label, lastHealth, criticalCount}'
curl -sS "$BASE/api/ext/innards.fleet/analytics" -H "Authorization: Bearer $KEY" | jq
```

In dev mode (`AUTH_MODE` unset) there are no API keys: drop the `Authorization` header and send
`X-Workspace-Id: ws_demo` instead.

## Privacy

Innards runs locally and sends nothing in the free tier; the fleet upload is an explicit,
per-workspace opt-in. What Threadwise stores is the structured report and its rendered prose,
**minus** anything that identifies the machine or its user beyond what the fleet needs:

- `snapshot.system.hostname` → `null` (and `summary.machine` is replaced if it fell back to the hostname)
- `snapshot.storage[].serial` → `null`
- `snapshot.network[].mac` → `null`
- `snapshot.load.top_memory` (the process list) → `[]`

Scrubbing happens on ingest, before anything is written, so the identifiers never reach the
database or its WAL. Reports are pruned after `retentionDays`; deleting a machine deletes all of
its reports. Everything is workspace-scoped, and disabling the plugin for a workspace stops both
uploads and reads. There is no outbound network access from the plugin.

Machine ids must be stable and non-reversible: the desktop app sends a hash, never a serial
number or hostname. The server does not validate that beyond the id syntax, so other clients must
follow the same rule.

## Configuring the Innards desktop app

In Innards, **Settings → Team & Enterprise** (the section is read-only on other tiers):

| Field | Value |
|---|---|
| Threadwise URL | The Threadwise base URL, e.g. `https://crm.example.com`; the app appends `/api/ext/innards.fleet/reports` (`src-tauri/src/cloud.rs::INGEST_PATH`) |
| API key (tw_…) | A workspace API key whose role includes `sources:manage`. Create one under **Settings → Identity & access → API keys** in Threadwise. |
| Label for this machine | Optional; shown in the fleet instead of vendor + product, and kept across uploads once set |
| Upload automatically after every scan | Otherwise use **Upload report now** |

The app scrubs the report before it leaves the machine (the same fields the server scrubs, so
the protection holds even against an old server), sends the `machine` block from the rendered
summary (`ram_gb` via `ram_marketing_gb`), and reads `machineId`, `reportId` and `dashboardUrl`
from the `201` response; on failure it shows `error.message`. The machine id is a non-reversible
hash of vendor + product + system-disk serial + first MAC (`m_` + 16 hex digits), so the same
machine keeps its history while the raw identifiers never leave it. After an upload, **Open in
dashboard** follows `dashboardUrl`. The **Innards fleet** settings page in Threadwise shows the
same values for the workspace you are in.

## Web pages

- `/x/innards.fleet` — **FleetPage**: KPI tiles (machines, average health, need attention, stale),
  the machine table (label/model, owner, health chip, critical/warning counts, top finding, last
  seen; search box), most-common problems and fleet capability averages.
- `/x/innards.fleet/machines/:id` — **MachineDetailPage**: header with health chip and hardware
  line, KPI tiles, the latest rendered report grouped by severity (title, body, action, evidence),
  health history as a fixed-scale (0–100) sparkline, capability bars, hardware summary, owner picker
  (the shell's global search over people and organisations) and label editor, report history,
  remove.
- `/settings/x/innards.fleet` — **SettingsPage**: retention and intake settings, and what to enter
  in the Innards app.
- Record panel **Machines** on people and organisations.

## Tests

`test/innards-fleet.test.ts` follows Threadwise's `apps/api/src/tests/plugins.test.ts` pattern
(`registerPlugin` + `createApp`, real middleware, in-memory SQLite). Its imports are relative to
that directory, so run it from a Threadwise checkout:

```bash
cp integrations/threadwise-fleet/test/innards-fleet.test.ts ../threadwise/apps/api/src/tests/
cd ../threadwise
INNARDS_FLEET_PLUGIN=$PWD/../innards/integrations/threadwise-fleet/server/index.ts bun test innards-fleet
```

`test/smoke.node.ts` runs the same routes under plain Node (≥ 22.5) with a `node:sqlite` adapter
standing in for `bun:sqlite` and a tiny Hono-shaped router, using the real fixtures:

```bash
node --experimental-strip-types --no-warnings integrations/threadwise-fleet/test/smoke.node.ts
```

Formatting and lint follow Threadwise's `biome.json`:

```bash
npx @biomejs/biome@2.5.8 check --config-path=../threadwise/biome.json integrations/threadwise-fleet
```
