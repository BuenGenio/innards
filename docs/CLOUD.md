# Cloud: Team and Enterprise

Team and Enterprise add one thing to the desktop app — uploading a report — and put everything else in **Threadwise**, an open-source Bun/TypeScript CRM (`/home/buengenio/Projects/threadwise`, docs under `docs/`), through a plugin that lives in this repository: `integrations/threadwise-fleet/` (`innards.fleet`). The customer runs Threadwise; Innards ships the plugin and the uploader.

## 1. Architecture

```mermaid
sequenceDiagram
    autonumber
    participant U as User
    participant App as Innards desktop<br/>(src-tauri)
    participant TW as Threadwise API<br/>(Hono on Bun, SQLite)
    participant P as innards.fleet plugin<br/>(integrations/threadwise-fleet/server)
    participant Web as Threadwise web<br/>(/x/innards.fleet)

    U->>App: Settings → Team & Enterprise: URL, tw_ key, label; Save
    U->>App: Rescan (or "Upload report now")
    App->>App: probe → Report → render(en, informed)
    App->>App: cloud.rs: machine_id() hash, scrub() serials/hostname/MACs/processes
    App->>TW: POST /api/ext/innards.fleet/reports<br/>Authorization: Bearer tw_…<br/>{ machine, innards_version, report, rendered }
    TW->>TW: actor middleware: key → workspace, permissions
    TW->>P: routes.mount sub-app (plugin enabled for workspace?)
    P->>P: requirePermissionForWrites('sources:manage')
    P->>P: validateIngest, scrub() again, ingest()
    P->>TW: INSERT innards_reports; UPSERT innards_machines
    P-->>App: 201 { machineId, reportId, dashboardUrl }
    App-->>U: "Report uploaded" + "Open in dashboard"
    U->>Web: /x/innards.fleet
    Web->>P: GET /machines, GET /analytics, GET /machines/:id
    P-->>Web: MachineSummary[], Analytics, history
    Note over P: schedules.every('prune', 24h): delete reports older than retentionDays,<br/>always keep each machine's latest
```

Three pieces of code:

| Piece | Path | Role |
|---|---|---|
| Uploader | `src-tauri/src/cloud.rs`, `cloud_upload` and the auto-upload branch of `analyze` in `src-tauri/src/lib.rs`, the "Team & Enterprise" card in `src/lib/components/Settings.svelte` | Builds and POSTs the ingest body; gated at `Tier::Team` |
| Plugin server | `integrations/threadwise-fleet/server/index.ts` | Migrations, ingest, machine registry, analytics, record panel, prune schedule, settings schema. Self-contained: it imports nothing from Threadwise (Bun resolves imports from the module's own directory), so the `PluginContext`, Hono and `bun:sqlite` types are declared structurally in the file. |
| Plugin web | `integrations/threadwise-fleet/web/{index.tsx,FleetPage.tsx,MachineDetailPage.tsx,SettingsPage.tsx,api.ts,shared.tsx}` | Dashboard at `/x/innards.fleet`, machine detail at `/x/innards.fleet/machines/:id`, settings page at `/settings/x/innards.fleet`, the "Machines" panel on person and organisation records. Optional: without it Threadwise still shows a generic plugin page and the server-rendered panel. |

Tests: `integrations/threadwise-fleet/test/innards-fleet.test.ts` runs under `bun test` inside Threadwise (it imports `../app.ts`, `../plugins/index.ts`, i.e. it is meant to be run from `apps/api/src/tests/` with the plugin path resolved relative to the Threadwise checkout); `test/smoke.node.ts` runs under plain Node ≥ 22.5 (`node --experimental-strip-types integrations/threadwise-fleet/test/smoke.node.ts`) with a `node:sqlite` adapter and the real fixtures from `static/fixtures/`.

## 2. Ingest contract

`POST <threadwise base>/api/ext/innards.fleet/reports`

Headers: `Authorization: Bearer tw_…` (a Threadwise API key bound to one workspace; see §4), `Content-Type: application/json`. Body limit `MAX_BODY_BYTES` = 2 MiB (a real body is ~25 KB). The route requires the `sources:manage` permission for the key (the same permission Data Sources use); reads are open to any workspace member.

Body — built by `cloud.rs::upload`, validated by `server/index.ts::validateIngest`:

```json
{
  "machine": {
    "id": "m_9f3c2a71b05e4d18",
    "label": "Ana's laptop",
    "vendor": "LENOVO",
    "product": "ThinkPad X1 Carbon 5th",
    "chassis": "laptop",
    "os": "Ubuntu 25.10",
    "cpu": "Intel Core i7-7500U · 2C/4T",
    "ram_gb": 16,
    "storage": "1.8 TiB NVMe",
    "gpu": "HD Graphics 620"
  },
  "innards_version": "0.1.0",
  "report":   { "snapshot": { … }, "findings": [ … ], "capabilities": [ … ], "summary": { … } },
  "rendered": { "lang": "en", "level": "informed", "summary": { … }, "verdict": "…", "findings": [ … ], "capabilities": [ … ], "notes": [] }
}
```

| Field | Required | Rules |
|---|---|---|
| `machine.id` | yes | `^[A-Za-z0-9][A-Za-z0-9._:-]{7,127}$`. The app sends `m_` + 16 hex digits: a `DefaultHasher` (SipHash, fixed keys, deterministic) over `system.vendor`, `system.product`, the system disk's `serial` and the first interface's `mac`. Stable across reinstalls of the same hardware; not reversible; the inputs themselves are scrubbed from the body. |
| `machine.label` | no | ≤ 120 chars. `settings.machine_label`; when absent the plugin keeps the existing label, else `report.summary.machine`, else `vendor product`, else "Unnamed machine". |
| `machine.vendor/product/chassis/os/cpu/ram_gb/storage/gpu` | no | Strings from `RenderedReport.summary` plus `ram_marketing_gb`; the plugin falls back to the snapshot when missing. `chassis` ≤ 24 chars. |
| `innards_version` | yes | `CARGO_PKG_VERSION` of the shell, ≤ 40 chars. |
| `report` | yes | The `Report` struct (`crates/innards-core/src/report.rs`) after `cloud.rs::scrub`. Must have `snapshot.system` (object), `findings[]` with `id` and a valid `severity`, `capabilities[]` with `workload` and numeric `score`, `summary.health_score` (number). May carry `recommendations[]` (`cost_usd: [low, high]`) — the app does not send them today, so `upgradeBudget` is `null`. |
| `rendered` | yes | The `RenderedReport` in **English, informed** regardless of the user's settings (`lib.rs` renders it that way so the dashboard is uniform). Must have `summary`, `findings[]` with `id` and `title`, `capabilities[]`. |

Scrubbing happens twice, on purpose. The app (`cloud.rs::scrub`) nulls `snapshot.storage[].serial`, `snapshot.system.hostname`, `snapshot.network[].mac` and empties `snapshot.load.top_memory`. The plugin (`server/index.ts::scrub`) does the same again and additionally replaces `summary.machine` with "Unnamed machine" when it equals the hostname (the core's fallback when DMI vendor/product are unknown). Neither strips filesystem mount points or `probe_notes`.

Response `201`:

```json
{ "machineId": "m_9f3c2a71b05e4d18", "reportId": "rpt_…", "dashboardUrl": "https://crm.example.com/x/innards.fleet/machines/m_9f3c2a71b05e4d18" }
```

`dashboardUrl` is built from the request origin (`x-forwarded-proto`/`x-forwarded-host` honoured). The app maps it to `UploadResult { machine_id, report_id, dashboard_url }`.

Errors follow Threadwise's `{ error: { code, message, details? } }`:

| Status | `code` | When |
|---|---|---|
| 400 | `validation_failed` | Any rule above fails; `details.field` names the offender |
| 401 | `unauthenticated` | No or bad `tw_` key (Threadwise, session mode) |
| 403 | `forbidden` | Key lacks `sources:manage` (`details.missing`), or the workspace has `allowAnonymousMachines` off and this `machine.id` is not registered with an owner |
| 404 | `plugin_disabled` | Plugin turned off for the workspace |
| 413 | `payload_too_large` | Body over 2 MiB |

The app surfaces these as `"<status>: <message>"` (e.g. `"403 Forbidden: This workspace only accepts reports from machines that already have an owner…"`).

### The rest of the plugin API

All under `/api/ext/innards.fleet/`, workspace-scoped by the key or `X-Workspace-Id`, writes need `sources:manage`:

| Method | Path | Returns |
|---|---|---|
| `GET` | `/machines?q=&ownerKind=person\|organisation&ownerId=` | `{ items: MachineSummary[], total, exact: true, nextCursor: null }` — label, vendor/product/chassis/os/cpu/ramGb/storage/gpu, owner, firstSeen/lastSeen, lastHealth + `healthBucket` (`good` ≥ 90, `ok` ≥ 75, `warning` ≥ 50, else `critical`), critical/warning counts, `topFinding`, `innardsVersion`, `stale` (no report for `STALE_AFTER_DAYS` = 30) |
| `POST` | `/machines` `{ id, label?, ownerPersonId?, ownerOrgId? }` | Pre-registers a machine (needed when `allowAnonymousMachines` is off) |
| `GET` | `/machines/:id` | `{ machine, latest: { …ReportSummary, rendered }, history: HistoryPoint[] }` (up to 90 points of `received_at`, `health_score`, counts) |
| `GET` | `/machines/:id/reports` | `ReportSummary[]`, newest first |
| `PATCH` | `/machines/:id` `{ label?, ownerPersonId?, ownerOrgId? }` | Owners are validated against `people` / `organisations` in the workspace |
| `DELETE` | `/machines/:id` | 204; deletes the machine and all its reports |
| `GET` | `/reports/:id` | `ReportSummary` + full `report` and `rendered` JSON |
| `GET` | `/analytics` | `{ machines, avgHealth, healthBuckets, topFindings (top 10 critical/warning ids by count, latest report per machine), upgradeBudget, capabilityAverages, staleMachines, attention }` |
| `GET` | `/panel?kind=person\|organisation&id=` | `PluginPanelContent` for the record workspace: the machines that record owns, with health tone and a link |

Plugin settings (`PATCH /api/plugins/innards.fleet`, Settings → Plugins or the plugin's own settings page): `retentionDays` (number, default 365), `allowAnonymousMachines` (boolean, default true).

## 3. Storage, retention, prune

Two tables, created by the plugin's migration `0001_fleet` and applied to every workspace database file (main and sandboxes):

- `innards_machines` — `PRIMARY KEY (workspace_id, id)`; label, the spec columns, `owner_person_id` / `owner_org_id` (indexed), `first_seen`, `last_seen`, `last_health`, `last_report_id`, `last_critical`, `last_warning`, `last_top_finding_{id,title,severity}`, `last_innards_version`.
- `innards_reports` — `id` (`rpt_…`), `workspace_id`, `machine_id`, `received_at`, `innards_version`, `health_score`, `critical_count`, `warning_count`, `findings_json` (compact `{id, severity, title}` list), `capabilities_json` (compact `{workload, score}`), `upgrade_low/high`, `report_json`, `rendered_json` (the full scrubbed bodies). Indexed on `(workspace_id, machine_id, received_at)`.

Every upload inserts one report row and upserts the machine row inside a transaction. `ctx.schedules.every('prune', 24h)` runs `FleetService.prune`: for each workspace, delete reports with `received_at` older than the workspace's `retentionDays` (default 365 when unset or invalid) **except** each machine's `last_report_id`, so a machine that stopped reporting keeps its last known state indefinitely. Deleting a machine (`DELETE /machines/:id`) removes its reports immediately. Threadwise's own retention policies and erasure tooling do not know about these tables; a data-subject erasure that must cover a person's machines is a `DELETE /machines/:id` per machine today.

What is stored is exactly the scrubbed body, so the dashboard database never holds serials, hostnames, MACs or process names. It does hold vendor/product/OS/CPU strings, mount points, findings and capability scores — treat the SQLite file as confidential and back it up with the rest of `/data`.

## 4. Self-hosting Threadwise with the plugin

Threadwise is one process (API + web + landing) with an embedded SQLite file; `docker compose up -d` from the Threadwise checkout runs it on port 3000 (`docs/DEPLOY.md`). To add the plugin:

1. Make the plugin source reachable from the container or process. The plugin imports nothing, so a bind mount of `integrations/threadwise-fleet/` is enough.
2. Set `THREADWISE_PLUGINS` to the server module (comma- or colon-separated list; absolute path, path relative to the Threadwise repo root, or a package name resolvable from `apps/api`; a directory resolves to `index.ts`, `server/index.ts` or `src/index.ts`):

   ```yaml
   # docker-compose.override.yml next to Threadwise's docker-compose.yml
   services:
     threadwise:
       environment:
         THREADWISE_PLUGINS: /plugins/innards-fleet/server
         AUTH_MODE: session          # real sign-in; API keys need it
         AUTH_SECRET: ${AUTH_SECRET} # openssl rand -base64 48
         AUTH_BASE_URL: https://crm.example.com
         WEB_BASE_URL: https://crm.example.com
         TRUST_PROXY: "true"         # behind Caddy/nginx
       volumes:
         - /path/to/innards/integrations/threadwise-fleet:/plugins/innards-fleet:ro
   ```

   Or without Docker: `THREADWISE_PLUGINS=/path/to/innards/integrations/threadwise-fleet/server bun apps/api/src/index.ts`.
3. For the dashboard pages, build the Threadwise web bundle with the web half listed: `THREADWISE_WEB_PLUGINS=/path/to/innards/integrations/threadwise-fleet/web bun run --cwd apps/web build` (Vite turns the list into the `virtual:threadwise-plugins` module; the web module imports Threadwise's `@/…` modules and shares its React, so it must be built inside the Threadwise tree). Navigation comes from the API manifest, so a plugin disabled for a workspace disappears from the UI. The stock Docker image does not include this bundle; build your own image (`build: .` in the compose file) with the variable set, or run the API and a rebuilt `apps/web/dist`.
4. Start, sign in as the workspace owner, open Settings → Plugins, confirm `Innards fleet` is listed without a load error, enable it, set `retentionDays`.
5. Settings → API keys → create a key with the `sources:manage` scope (and nothing else) named after the fleet; copy `tw_…` once.
6. In each Innards install: Settings → Team & Enterprise → Threadwise URL (`https://crm.example.com`), API key, label → Save → "Upload report now". The response link opens the machine in the dashboard.

Operational notes from Threadwise's docs that matter here: `TRUST_PROXY=true` behind TLS termination; `THREADWISE_SECRET_KEY`/`AUTH_SECRET` must be set in session mode; backups are the `/data` volume (`deploy/backup.sh`); upgrades are `docker compose pull && docker compose up -d`; health at `/api/health`. Threadwise's egress gate (`assertEgressAllowed`) is irrelevant to the plugin because it makes no outbound calls.

Threadwise Cloud (`cloud.threadwise.app`, a placeholder in Threadwise's own docs) does not exist yet; when it does, "Team without self-hosting" is that instance with `innards.fleet` loaded.

## 5. What Enterprise adds (from Threadwise)

Everything below is Threadwise functionality (`docs/ENTERPRISE.md`, `AUTH.md`, `PRIVACY.md`, `RESIDENCY.md`, `SANDBOXES.md` in the Threadwise repo) that an Enterprise deployment gets by running Threadwise in session mode; the plugin inherits it because every plugin route runs inside the workspace + actor middleware.

- **Sign-in and SSO**: email + password (10+ chars, optional verification), OIDC (`OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`; Zitadel-tested; verified-email linking), SAML 2.0 (`@better-auth/sso`; one IdP per deployment from `SAML_IDP_METADATA_URL`/`_FILE`/inline or `SAML_IDP_ENTRY_POINT` + `SAML_IDP_CERT` + `SAML_IDP_ENTITY_ID`; `SAML_DOMAIN` lists the domains the IdP owns; Okta, Entra and Zitadel recipes are in `AUTH.md`). Invites create memberships with role, manager and teams.
- **Roles and RBAC**: `owner`, `admin`, `manager`, `member`, `viewer` mapped to `<resource>:<action>` permissions in `apps/api/src/lib/permissions.ts`. For the fleet: uploading and editing machines needs `sources:manage` (owner, admin, manager, member); viewers can read the dashboard but not upload or reassign. API keys are workspace-scoped, sha256-stored, scoped to at most the creator's permissions, rate-limited (600/min), expirable and revocable (`DELETE /api/api-keys/:id`).
- **Territories and visibility**: an access policy per workspace (`all` / `own` / `team` / `territory` per entity kind, manager roll-up, field rules) restricts which people and organisations a manager/member/viewer can see; territories are a rule-driven tree re-evaluated on every record change. The plugin's owner lookups use those records, so a machine assigned to a person outside a user's territory shows the machine but not the person's record.
- **Audit log**: every write goes through `writeAudit` with actor, IP, user agent and a hash chain (`hash = sha256(prevHash + canonical JSON)`); `GET /api/audit`, `POST /api/audit/export` (CSV/JSONL, `X-Audit-Chain-Head`), `POST /api/audit/verify` → `{ ok, brokenAt }`; `AUDIT_MIN_RETENTION_DAYS` (365) floors retention. Field history per record with actor and cause. Plugin writes to its own tables are not field-history events; the request itself is still attributed to the API key actor.
- **Sandboxes**: a copy of a production workspace in its own SQLite file (`config_only`, `anonymised` with deterministic fakes, or `full`), promote configuration back; `sandbox:manage` (owners only). Plugin migrations run in sandbox files too, so the fleet tables exist there.
- **Data residency**: `THREADWISE_REGION` (`eu`, `uk`, `us`, `ca`, `au`, `in`, `self`) declared per deployment and shown in `/api/health`; a per-workspace residency policy (`allowedProcessingRegions`, `backupTarget`, `egressAllowlist`) gates every outbound call through `assertEgressAllowed` with an `egress_log`; `BACKUP_REGION` must match for S3 backups.
- **GDPR tooling**: consent records as evidence, data-subject requests with `DSR_DUE_DAYS` (30) deadlines, `POST /api/privacy/find` across people/messages/custom records, export and erasure jobs with tombstones the importer honours, retention policies with dry runs, a data map of what exists and what leaves.
- **Deployment**: one container, `SERVE_STATIC`, Caddy for HTTPS, Fly/Render/Railway/Kubernetes/systemd manifests in `deploy/`, point-in-time backups via `deploy/backup.sh`.

What Innards adds on top for Enterprise is contractual: an `enterprise` license key (issued with `tools/innards-license`), SLA and priority support, and custom rules delivered as a maintained fork of `rules.rs` + catalogs.

## 6. Roadmap

In rough order, none of it built yet:

1. **Scheduled auto-upload.** Today `analyze` uploads only when the user scans and `cloud_auto_upload` is on. Needed: a background timer in the shell (or an OS scheduler entry: systemd user timer, launchd, Task Scheduler) that runs `probe → build → render(en, informed) → cloud::upload` daily without the window open, with jitter and a last-upload timestamp in settings.
2. **Alerts.** A plugin schedule that compares each machine's latest report with the previous one and raises Threadwise insights (`Insight` with evidence) or emails on: new critical finding, health drop ≥ 15, SMART failure, machine gone stale. Threadwise's insight/action pipeline is the natural home (`POST /insights/:id/act` never sends on its own).
3. **CSV export is built:** `GET /api/ext/innards.fleet/machines.csv` (same filters as `/machines`). Per-machine PDF of the rendered report is roadmap; Threadwise already streams audit exports, so the pattern exists.
4. **Upgrade budget.** The plugin sums `recommendations[].cost_usd` when present; the app should attach the last advisor result (`AppState.recs`) to the upload when there is one, so `analytics.upgradeBudget` stops being `null`.
5. **Billing reconciliation.** A monthly job that reads `/analytics` per workspace and updates the Stripe subscription quantity (`docs/MONETIZATION.md` §5).
6. **Erasure integration.** Register the plugin's tables with Threadwise's privacy tooling so a person's erasure request also deletes their machines' reports.
7. **Localised dashboard.** Store `rendered` in the workspace's language rather than English/informed, or re-render server-side from the structured `report` using the catalogs (a TypeScript port of `report::render` — `web/src/lib/engine/` in this repo already ports capability and advisor).
