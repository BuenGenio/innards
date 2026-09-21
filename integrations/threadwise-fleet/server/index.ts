// ---------------------------------------------------------------------------
// innards.fleet — Threadwise plugin that receives machine health reports from
// the Innards desktop app and turns them into a fleet dashboard.
//
// This module lives in the Innards repository and is loaded by Threadwise via
// THREADWISE_PLUGINS, so it must not import anything from `@crm/*`, `hono` or
// Threadwise's source tree: Bun resolves a module's imports from the module's
// own location, and none of those packages exist under innards/node_modules.
// Everything it needs (the Hono sub-app, the request context, the sqlite
// handle, settings) arrives through the PluginContext it is registered with.
// The minimal structural types below mirror apps/api/src/plugins/types.ts and
// packages/shared/src/plugins.ts closely enough for `tsc --strict`.
// ---------------------------------------------------------------------------

export const PLUGIN_ID = 'innards.fleet';
export const DASHBOARD_BASE = `/x/${PLUGIN_ID}`;

/** Largest accepted POST /reports body. A full Report + RenderedReport is ~25 KB. */
export const MAX_BODY_BYTES = 2 * 1024 * 1024;
/** A machine with no report for this long counts as stale. */
export const STALE_AFTER_DAYS = 30;
const PRUNE_EVERY_MS = 24 * 60 * 60 * 1000;
const MACHINE_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{7,127}$/;
const MAX_LABEL = 120;

// --- Minimal structural types (no imports possible; see header) ------------------------

type Permission = string;

interface RequestActorLike {
  permissions: Set<Permission>;
}

/**
 * The slice of a Hono `Context<AppEnv>` the routes use. Typed by hand because `hono`
 * cannot be imported from outside the Threadwise tree.
 */
interface Ctx {
  req: {
    method: string;
    url: string;
    param(name: string): string;
    query(name: string): string | undefined;
    header(name: string): string | undefined;
    text(): Promise<string>;
    json(): Promise<unknown>;
  };
  get(key: 'workspaceId'): string;
  get(key: 'actor'): RequestActorLike | undefined;
  json(body: unknown, status?: number): Response;
  body(body: null, status: number): Response;
}

type Handler = (c: Ctx) => Response | Promise<Response>;
type Middleware = (c: Ctx, next: () => Promise<void>) => Promise<Response | undefined>;

/** The Hono sub-app `ctx.routes.mount` hands us, narrowed to what is used. */
interface SubApp {
  use(path: string, mw: Middleware): unknown;
  get(path: string, h: Handler): unknown;
  post(path: string, h: Handler): unknown;
  patch(path: string, h: Handler): unknown;
  delete(path: string, h: Handler): unknown;
}

/** bun:sqlite `Database`, narrowed to what is used. */
interface SqliteLike {
  query(sql: string): {
    all(...params: unknown[]): unknown[];
    get(...params: unknown[]): unknown;
    run(...params: unknown[]): unknown;
  };
  transaction<T>(fn: () => T): () => T;
}

export interface PluginSettingField {
  key: string;
  label: string;
  type: 'boolean' | 'string' | 'number' | 'select' | 'secret' | 'text';
  description?: string;
  default?: unknown;
  options?: Array<{ value: string; label: string }>;
}

export interface PluginPanelContent {
  summary?: string | null;
  items: Array<{
    label: string;
    value: string | null;
    knowledge?: 'observed' | 'calculated' | 'inferred';
    href?: string | null;
    tone?: 'neutral' | 'ok' | 'warn' | 'danger';
  }>;
  actions?: Array<{ id: string; label: string; method: 'POST'; endpoint: string }>;
}

/** Structural subset of Threadwise's PluginContext (apps/api/src/plugins/types.ts). */
export interface PluginContext {
  pluginId: string;
  log(message: string, ...rest: unknown[]): void;
  settings(workspaceId: string): Record<string, unknown>;
  enabled(workspaceId: string): boolean;
  routes: { mount(build: (app: any) => void): void };
  nav: {
    add(entry: {
      id: string;
      label: string;
      to: string;
      icon?: string;
      area: 'primary' | 'utility';
      key?: string;
    }): void;
    settingsPage(page: { label: string; to: string; icon?: string }): void;
  };
  recordPanels: {
    add(panel: {
      id: string;
      title: string;
      entityKinds: Array<'person' | 'organisation'>;
      endpoint: string;
    }): void;
  };
  schedules: { every(name: string, intervalMs: number, fn: () => void | Promise<void>): void };
  db: {
    migrations(list: Array<{ id: string; sql: string }>): void;
    sqlite(): any;
  };
}

export interface ThreadwisePlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  settingsSchema?: PluginSettingField[];
  register(ctx: PluginContext): void | Promise<void>;
}

// --- Innards data model (crates/innards-core/src/{snapshot,finding,report,capability}.rs) ---

export type Severity = 'critical' | 'warning' | 'info' | 'good';
const SEVERITY_ORDER: Severity[] = ['critical', 'warning', 'info', 'good'];

export interface Finding {
  id: string;
  severity: Severity;
  category: string;
  params: Record<string, unknown>;
  evidence: string[];
}

export interface Capability {
  workload: string;
  score: number;
  grade: string;
  limits: string[];
}

export interface Summary {
  machine: string;
  cpu: string;
  memory: string;
  storage: string;
  gpu: string;
  os: string;
  battery: string | null;
  health_score: number;
}

/** `Recommendation` from advisor.rs; only present when the client attaches the advisor result. */
export interface Recommendation {
  id: string;
  cost_usd?: [number, number];
  /** RenderedRecommendation form: "Free" or "$40–$90". */
  cost?: string;
}

export interface Report {
  snapshot: {
    taken_at?: string;
    system: Record<string, unknown> & {
      hostname?: string | null;
      vendor?: string | null;
      product?: string | null;
      chassis?: string;
      os_name?: string | null;
      os_version?: string | null;
    };
    cpu?: Record<string, unknown> & { brand?: string };
    memory?: Record<string, unknown> & { total_bytes?: number };
    storage?: Array<Record<string, unknown> & { serial?: string | null }>;
    gpus?: Array<Record<string, unknown> & { name?: string; is_discrete?: boolean }>;
    network?: Array<Record<string, unknown> & { mac?: string | null }>;
    load?: Record<string, unknown> & { top_memory?: unknown[] };
    [key: string]: unknown;
  };
  findings: Finding[];
  capabilities: Capability[];
  summary: Summary;
  recommendations?: Recommendation[];
}

export interface RenderedFinding {
  id: string;
  severity: Severity;
  category: string;
  title: string;
  body: string;
  action: string | null;
  evidence: string[];
}

export interface RenderedReport {
  lang: string;
  level: string;
  summary: Summary;
  verdict: string;
  findings: RenderedFinding[];
  capabilities: Array<{
    workload: string;
    label: string;
    score: number;
    grade: string;
    grade_label: string;
    limits: string[];
  }>;
  notes: string[];
  recommendations?: Recommendation[];
}

export interface IngestMachine {
  id: string;
  label?: string | null;
  vendor?: string | null;
  product?: string | null;
  chassis?: string | null;
  os?: string | null;
  cpu?: string | null;
  ram_gb?: number | null;
  storage?: string | null;
  gpu?: string | null;
}

export interface IngestBody {
  machine: IngestMachine;
  innards_version: string;
  report: Report;
  rendered: RenderedReport;
}

// --- API shapes (mirrored by web/api.ts) --------------------------------------------------

export type HealthBucket = 'critical' | 'warning' | 'ok' | 'good';

export interface MachineOwner {
  kind: 'person' | 'organisation';
  id: string;
  name: string;
}

export interface MachineSummary {
  id: string;
  label: string;
  vendor: string | null;
  product: string | null;
  chassis: string | null;
  os: string | null;
  cpu: string | null;
  ramGb: number | null;
  storage: string | null;
  gpu: string | null;
  owner: MachineOwner | null;
  ownerPersonId: string | null;
  ownerOrgId: string | null;
  firstSeen: string;
  lastSeen: string | null;
  lastHealth: number | null;
  healthBucket: HealthBucket | null;
  lastReportId: string | null;
  criticalCount: number;
  warningCount: number;
  topFinding: { id: string; title: string; severity: Severity } | null;
  innardsVersion: string | null;
  stale: boolean;
}

export interface HistoryPoint {
  reportId: string;
  received_at: string;
  health_score: number;
  critical_count: number;
  warning_count: number;
}

export interface ReportSummary {
  id: string;
  machineId: string;
  receivedAt: string;
  innardsVersion: string;
  healthScore: number;
  criticalCount: number;
  warningCount: number;
}

export interface Analytics {
  machines: number;
  avgHealth: number | null;
  healthBuckets: Record<HealthBucket, number>;
  topFindings: Array<{ id: string; title: string; count: number }>;
  upgradeBudget: { low: number; high: number } | null;
  capabilityAverages: Array<{ workload: string; avg: number }>;
  staleMachines: number;
  attention: number;
}

// --- Migrations ------------------------------------------------------------------------

const MIGRATIONS = [
  {
    id: '0001_fleet',
    sql: `
CREATE TABLE IF NOT EXISTS innards_machines (
  id text NOT NULL,
  workspace_id text NOT NULL,
  label text NOT NULL,
  vendor text,
  product text,
  chassis text,
  os text,
  cpu text,
  ram_gb real,
  storage text,
  gpu text,
  owner_person_id text,
  owner_org_id text,
  first_seen text NOT NULL,
  last_seen text,
  last_health integer,
  last_report_id text,
  last_critical integer NOT NULL DEFAULT 0,
  last_warning integer NOT NULL DEFAULT 0,
  last_top_finding_id text,
  last_top_finding_title text,
  last_top_finding_severity text,
  last_innards_version text,
  PRIMARY KEY (workspace_id, id)
);
CREATE INDEX IF NOT EXISTS innards_machines_owner_person ON innards_machines (workspace_id, owner_person_id);
CREATE INDEX IF NOT EXISTS innards_machines_owner_org ON innards_machines (workspace_id, owner_org_id);
CREATE TABLE IF NOT EXISTS innards_reports (
  id text PRIMARY KEY,
  workspace_id text NOT NULL,
  machine_id text NOT NULL,
  received_at text NOT NULL,
  innards_version text NOT NULL,
  health_score integer NOT NULL,
  critical_count integer NOT NULL,
  warning_count integer NOT NULL,
  findings_json text NOT NULL,
  capabilities_json text NOT NULL,
  upgrade_low integer,
  upgrade_high integer,
  report_json text NOT NULL,
  rendered_json text NOT NULL
);
CREATE INDEX IF NOT EXISTS innards_reports_machine ON innards_reports (workspace_id, machine_id, received_at);
`,
  },
];

// --- Pure helpers (exported for tests) ------------------------------------------------

// Same thresholds as the desktop app's health ring (src/lib/components/HealthRing.svelte):
// good ≥ 80, warning ≥ 55, critical below. 'ok' covers the upper warning band.
export function healthBucket(score: number | null | undefined): HealthBucket | null {
  if (score == null || Number.isNaN(score)) return null;
  if (score >= 80) return 'good';
  if (score >= 65) return 'ok';
  if (score >= 55) return 'warning';
  return 'critical';
}

/** RFC 4180 CSV of the fleet: one row per machine, for spreadsheets and finance. */
export function machinesCsv(items: MachineSummary[]): string {
  const cols = ['id', 'label', 'vendor', 'product', 'chassis', 'os', 'cpu', 'ram_gb', 'storage', 'gpu', 'owner', 'first_seen', 'last_seen', 'health', 'bucket', 'critical', 'warning', 'top_finding', 'innards_version', 'stale'];
  const esc = (v: unknown) => {
    const t = v == null ? '' : String(v);
    return /[",\n\r]/.test(t) ? `"${t.replace(/"/g, '""')}"` : t;
  };
  const rows = items.map((m) => [
    m.id, m.label, m.vendor, m.product, m.chassis, m.os, m.cpu, m.ramGb, m.storage, m.gpu,
    m.owner ? `${m.owner.kind}:${m.owner.name ?? m.owner.id}` : '', m.firstSeen, m.lastSeen, m.lastHealth, m.healthBucket,
    m.criticalCount, m.warningCount, m.topFinding?.title ?? '', m.innardsVersion, m.stale ? 'yes' : 'no',
  ].map(esc).join(','));
  return [cols.join(','), ...rows].join('\r\n') + '\r\n';
}

function nowIso(): string {
  return new Date().toISOString();
}

function newId(prefix: string): string {
  return `${prefix}_${crypto.randomUUID().replace(/-/g, '').slice(0, 20)}`;
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function optString(v: unknown, max = MAX_LABEL): string | null {
  if (v == null) return null;
  if (typeof v !== 'string') return null;
  const s = v.trim();
  return s ? s.slice(0, max) : null;
}

function optNumber(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

/**
 * Removes what the fleet must never hold: hostnames, drive serials, MAC addresses and the
 * process list. `summary.machine` falls back to the hostname when the vendor/product are
 * unknown, so that is replaced too. Mutates and returns the same objects.
 */
export function scrub(report: Report, rendered: RenderedReport): void {
  const system = report.snapshot.system ?? {};
  const hostname = typeof system.hostname === 'string' ? system.hostname : null;
  if (hostname) {
    if (report.summary?.machine === hostname) report.summary.machine = 'Unnamed machine';
    if (rendered.summary?.machine === hostname) rendered.summary.machine = 'Unnamed machine';
  }
  if ('hostname' in system) system.hostname = null;
  for (const d of report.snapshot.storage ?? []) if ('serial' in d) d.serial = null;
  for (const n of report.snapshot.network ?? []) if ('mac' in n) n.mac = null;
  if (report.snapshot.load && 'top_memory' in report.snapshot.load)
    report.snapshot.load.top_memory = [];
}

/** Highest-severity finding, in rules order within a severity. */
export function topFinding(
  findings: RenderedFinding[],
): { id: string; title: string; severity: Severity } | null {
  for (const sev of SEVERITY_ORDER) {
    const f = findings.find((x) => x.severity === sev);
    if (f) return { id: f.id, title: f.title, severity: f.severity };
  }
  return null;
}

/** Sums recommendation cost bands: structured `cost_usd: [low, high]` or rendered "$40–$90". */
export function upgradeBudget(
  recs: Recommendation[] | undefined,
): { low: number; high: number } | null {
  if (!Array.isArray(recs) || recs.length === 0) return null;
  let low = 0;
  let high = 0;
  let counted = 0;
  for (const r of recs) {
    if (Array.isArray(r.cost_usd) && r.cost_usd.length === 2) {
      low += Number(r.cost_usd[0]) || 0;
      high += Number(r.cost_usd[1]) || 0;
      counted++;
    } else if (typeof r.cost === 'string') {
      const m = r.cost.match(/\$?\s*([\d,]+)\s*[–-]\s*\$?\s*([\d,]+)/);
      if (m?.[1] && m[2]) {
        low += Number(m[1].replace(/,/g, ''));
        high += Number(m[2].replace(/,/g, ''));
      }
      counted++; // "Free" counts as a $0 recommendation
    }
  }
  return counted ? { low, high } : null;
}

/** RAM in marketing gigabytes, as report.rs::ram_marketing_gb: 14.9 GiB reported is a 16 GB machine. */
export function ramMarketingGb(bytes: number): number {
  const gib = bytes / 1024 ** 3;
  for (const s of [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 512])
    if (gib <= s && gib >= s * 0.88) return s;
  return Math.round(gib);
}

function requestOrigin(c: Ctx): string {
  const url = new URL(c.req.url);
  const proto =
    c.req.header('x-forwarded-proto')?.split(',')[0]?.trim() || url.protocol.slice(0, -1);
  const host = c.req.header('x-forwarded-host')?.split(',')[0]?.trim() || url.host;
  return `${proto}://${host}`;
}

// --- Validation --------------------------------------------------------------------------

class IngestError extends Error {
  readonly code: string;
  readonly status: number;
  readonly details: unknown;

  constructor(code: string, message: string, status: number, details?: unknown) {
    super(message);
    this.code = code;
    this.status = status;
    this.details = details;
  }
}

function fail(message: string, details?: unknown): never {
  throw new IngestError('validation_failed', message, 400, details);
}

export function validateIngest(raw: unknown): IngestBody {
  if (!isRecord(raw)) fail('Body must be a JSON object');
  const machine = raw.machine;
  if (!isRecord(machine)) fail('machine is required');
  if (typeof machine.id !== 'string' || !MACHINE_ID_PATTERN.test(machine.id))
    fail('machine.id must be 8–128 characters of [A-Za-z0-9._:-]', { field: 'machine.id' });
  if (typeof raw.innards_version !== 'string' || !raw.innards_version.trim())
    fail('innards_version is required', { field: 'innards_version' });
  const report = raw.report;
  if (!isRecord(report)) fail('report is required');
  if (!isRecord(report.snapshot) || !isRecord(report.snapshot.system))
    fail('report.snapshot.system is required', { field: 'report.snapshot' });
  if (!Array.isArray(report.findings)) fail('report.findings must be an array');
  if (!Array.isArray(report.capabilities)) fail('report.capabilities must be an array');
  if (!isRecord(report.summary) || typeof report.summary.health_score !== 'number')
    fail('report.summary.health_score is required', { field: 'report.summary' });
  for (const [i, f] of report.findings.entries()) {
    if (
      !isRecord(f) ||
      typeof f.id !== 'string' ||
      !SEVERITY_ORDER.includes(f.severity as Severity)
    )
      fail(`report.findings[${i}] needs id and severity`, { field: `report.findings.${i}` });
  }
  for (const [i, cap] of report.capabilities.entries()) {
    if (!isRecord(cap) || typeof cap.workload !== 'string' || typeof cap.score !== 'number')
      fail(`report.capabilities[${i}] needs workload and score`, {
        field: `report.capabilities.${i}`,
      });
  }
  const rendered = raw.rendered;
  if (!isRecord(rendered)) fail('rendered is required');
  if (!Array.isArray(rendered.findings) || !Array.isArray(rendered.capabilities))
    fail('rendered.findings and rendered.capabilities must be arrays', { field: 'rendered' });
  if (!isRecord(rendered.summary)) fail('rendered.summary is required', { field: 'rendered' });
  for (const [i, f] of rendered.findings.entries()) {
    if (!isRecord(f) || typeof f.id !== 'string' || typeof f.title !== 'string')
      fail(`rendered.findings[${i}] needs id and title`, { field: `rendered.findings.${i}` });
  }
  const ramGb = optNumber(machine.ram_gb);
  return {
    machine: {
      id: machine.id,
      label: optString(machine.label),
      vendor: optString(machine.vendor),
      product: optString(machine.product),
      chassis: optString(machine.chassis, 24),
      os: optString(machine.os),
      cpu: optString(machine.cpu),
      ram_gb: ramGb != null && ramGb >= 0 ? ramGb : null,
      storage: optString(machine.storage),
      gpu: optString(machine.gpu),
    },
    innards_version: raw.innards_version.trim().slice(0, 40),
    report: report as unknown as Report,
    rendered: rendered as unknown as RenderedReport,
  };
}

// --- Rows ------------------------------------------------------------------------------

interface MachineRow {
  id: string;
  workspace_id: string;
  label: string;
  vendor: string | null;
  product: string | null;
  chassis: string | null;
  os: string | null;
  cpu: string | null;
  ram_gb: number | null;
  storage: string | null;
  gpu: string | null;
  owner_person_id: string | null;
  owner_org_id: string | null;
  first_seen: string;
  last_seen: string | null;
  last_health: number | null;
  last_report_id: string | null;
  last_critical: number;
  last_warning: number;
  last_top_finding_id: string | null;
  last_top_finding_title: string | null;
  last_top_finding_severity: Severity | null;
  last_innards_version: string | null;
  owner_person_name?: string | null;
  owner_org_name?: string | null;
}

interface ReportRow {
  id: string;
  workspace_id: string;
  machine_id: string;
  received_at: string;
  innards_version: string;
  health_score: number;
  critical_count: number;
  warning_count: number;
  findings_json: string;
  capabilities_json: string;
  upgrade_low: number | null;
  upgrade_high: number | null;
  report_json: string;
  rendered_json: string;
}

const MACHINE_SELECT = `
SELECT m.*, p.display_name AS owner_person_name, o.name AS owner_org_name
FROM innards_machines m
LEFT JOIN people p ON p.id = m.owner_person_id AND p.workspace_id = m.workspace_id
LEFT JOIN organisations o ON o.id = m.owner_org_id AND o.workspace_id = m.workspace_id`;

function toSummary(row: MachineRow, now = Date.now()): MachineSummary {
  const owner: MachineOwner | null = row.owner_person_id
    ? { kind: 'person', id: row.owner_person_id, name: row.owner_person_name ?? 'Person' }
    : row.owner_org_id
      ? { kind: 'organisation', id: row.owner_org_id, name: row.owner_org_name ?? 'Organisation' }
      : null;
  const lastSeenMs = row.last_seen ? Date.parse(row.last_seen) : Number.NaN;
  return {
    id: row.id,
    label: row.label,
    vendor: row.vendor,
    product: row.product,
    chassis: row.chassis,
    os: row.os,
    cpu: row.cpu,
    ramGb: row.ram_gb,
    storage: row.storage,
    gpu: row.gpu,
    owner,
    ownerPersonId: row.owner_person_id,
    ownerOrgId: row.owner_org_id,
    firstSeen: row.first_seen,
    lastSeen: row.last_seen,
    lastHealth: row.last_health,
    healthBucket: healthBucket(row.last_health),
    lastReportId: row.last_report_id,
    criticalCount: row.last_critical,
    warningCount: row.last_warning,
    topFinding:
      row.last_top_finding_id && row.last_top_finding_title
        ? {
            id: row.last_top_finding_id,
            title: row.last_top_finding_title,
            severity: row.last_top_finding_severity ?? 'info',
          }
        : null,
    innardsVersion: row.last_innards_version,
    stale: Number.isNaN(lastSeenMs) || now - lastSeenMs > STALE_AFTER_DAYS * 86_400_000,
  };
}

function toReportSummary(r: ReportRow): ReportSummary {
  return {
    id: r.id,
    machineId: r.machine_id,
    receivedAt: r.received_at,
    innardsVersion: r.innards_version,
    healthScore: r.health_score,
    criticalCount: r.critical_count,
    warningCount: r.warning_count,
  };
}

// --- Service -----------------------------------------------------------------------------

class FleetService {
  private readonly ctx: PluginContext;

  constructor(ctx: PluginContext) {
    this.ctx = ctx;
  }

  private get db(): SqliteLike {
    return this.ctx.db.sqlite() as SqliteLike;
  }

  machine(ws: string, id: string): MachineRow | null {
    return (this.db.query(`${MACHINE_SELECT} WHERE m.workspace_id = ? AND m.id = ?`).get(ws, id) ??
      null) as MachineRow | null;
  }

  listMachines(
    ws: string,
    opts: { q?: string; owner?: { kind: 'person' | 'organisation'; id: string } } = {},
  ): MachineSummary[] {
    const where: string[] = ['m.workspace_id = ?'];
    const params: unknown[] = [ws];
    if (opts.owner?.kind === 'person') {
      where.push('m.owner_person_id = ?');
      params.push(opts.owner.id);
    } else if (opts.owner?.kind === 'organisation') {
      where.push('m.owner_org_id = ?');
      params.push(opts.owner.id);
    }
    if (opts.q) {
      where.push(
        '(m.label LIKE ? OR m.product LIKE ? OR m.vendor LIKE ? OR m.os LIKE ? OR m.cpu LIKE ? OR p.display_name LIKE ? OR o.name LIKE ?)',
      );
      const like = `%${opts.q}%`;
      params.push(like, like, like, like, like, like, like);
    }
    const rows = this.db
      .query(
        `${MACHINE_SELECT} WHERE ${where.join(' AND ')} ORDER BY COALESCE(m.last_health, 101) ASC, m.last_seen DESC`,
      )
      .all(...params) as MachineRow[];
    const now = Date.now();
    return rows.map((r) => toSummary(r, now));
  }

  history(ws: string, machineId: string, limit = 90): HistoryPoint[] {
    const rows = this.db
      .query(
        `SELECT id, received_at, health_score, critical_count, warning_count FROM innards_reports
         WHERE workspace_id = ? AND machine_id = ? ORDER BY received_at DESC LIMIT ?`,
      )
      .all(ws, machineId, limit) as Array<{
      id: string;
      received_at: string;
      health_score: number;
      critical_count: number;
      warning_count: number;
    }>;
    return rows.reverse().map((r) => ({
      reportId: r.id,
      received_at: r.received_at,
      health_score: r.health_score,
      critical_count: r.critical_count,
      warning_count: r.warning_count,
    }));
  }

  report(ws: string, id: string): ReportRow | null {
    return (this.db
      .query('SELECT * FROM innards_reports WHERE workspace_id = ? AND id = ?')
      .get(ws, id) ?? null) as ReportRow | null;
  }

  reportsFor(ws: string, machineId: string): ReportSummary[] {
    const rows = this.db
      .query(
        `SELECT id, workspace_id, machine_id, received_at, innards_version, health_score, critical_count, warning_count,
                '' AS findings_json, '' AS capabilities_json, NULL AS upgrade_low, NULL AS upgrade_high, '' AS report_json, '' AS rendered_json
         FROM innards_reports WHERE workspace_id = ? AND machine_id = ? ORDER BY received_at DESC`,
      )
      .all(ws, machineId) as ReportRow[];
    return rows.map(toReportSummary);
  }

  ingest(ws: string, body: IngestBody): { machineId: string; reportId: string; created: boolean } {
    const { machine, report, rendered } = body;
    scrub(report, rendered);
    const settings = this.ctx.settings(ws);
    const existing = this.machine(ws, machine.id);
    if (settings.allowAnonymousMachines === false) {
      const claimed = existing && (existing.owner_person_id || existing.owner_org_id);
      if (!claimed)
        throw new IngestError(
          'forbidden',
          'This workspace only accepts reports from machines that already have an owner. Register the machine (POST /machines) and assign it first.',
          403,
        );
    }

    const receivedAt = nowIso();
    const reportId = newId('rpt');
    const findings = rendered.findings;
    const critical = findings.filter((f) => f.severity === 'critical').length;
    const warning = findings.filter((f) => f.severity === 'warning').length;
    const health = Math.max(0, Math.min(100, Math.round(report.summary.health_score)));
    const top = topFinding(findings);
    const budget = upgradeBudget(report.recommendations ?? rendered.recommendations);
    const findingsJson = JSON.stringify(
      findings.map((f) => ({ id: f.id, severity: f.severity, title: f.title })),
    );
    const capabilitiesJson = JSON.stringify(
      report.capabilities.map((c) => ({ workload: c.workload, score: c.score })),
    );

    const sys = report.snapshot.system;
    const vendor = machine.vendor ?? optString(sys.vendor);
    const product = machine.product ?? optString(sys.product);
    const label =
      machine.label ??
      existing?.label ??
      optString(report.summary.machine) ??
      optString([vendor, product].filter(Boolean).join(' ')) ??
      'Unnamed machine';
    const os =
      machine.os ??
      optString(report.summary.os) ??
      optString([sys.os_name, sys.os_version].filter(Boolean).join(' '));
    const cpu = machine.cpu ?? optString(report.summary.cpu);
    const totalBytes = optNumber(report.snapshot.memory?.total_bytes);
    const ramGb = machine.ram_gb ?? (totalBytes != null ? ramMarketingGb(totalBytes) : null);
    const storage = machine.storage ?? optString(report.summary.storage);
    const gpu = machine.gpu ?? optString(report.summary.gpu);
    const chassis = machine.chassis ?? optString(sys.chassis, 24);

    const db = this.db;
    db.transaction(() => {
      db.query(
        `INSERT INTO innards_reports (id, workspace_id, machine_id, received_at, innards_version, health_score,
           critical_count, warning_count, findings_json, capabilities_json, upgrade_low, upgrade_high, report_json, rendered_json)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      ).run(
        reportId,
        ws,
        machine.id,
        receivedAt,
        body.innards_version,
        health,
        critical,
        warning,
        findingsJson,
        capabilitiesJson,
        budget?.low ?? null,
        budget?.high ?? null,
        JSON.stringify(report),
        JSON.stringify(rendered),
      );
      if (existing) {
        db.query(
          `UPDATE innards_machines SET label = ?, vendor = ?, product = ?, chassis = ?, os = ?, cpu = ?, ram_gb = ?, storage = ?, gpu = ?,
             last_seen = ?, last_health = ?, last_report_id = ?, last_critical = ?, last_warning = ?,
             last_top_finding_id = ?, last_top_finding_title = ?, last_top_finding_severity = ?, last_innards_version = ?
           WHERE workspace_id = ? AND id = ?`,
        ).run(
          label,
          vendor ?? existing.vendor,
          product ?? existing.product,
          chassis ?? existing.chassis,
          os ?? existing.os,
          cpu ?? existing.cpu,
          ramGb ?? existing.ram_gb,
          storage ?? existing.storage,
          gpu ?? existing.gpu,
          receivedAt,
          health,
          reportId,
          critical,
          warning,
          top?.id ?? null,
          top?.title ?? null,
          top?.severity ?? null,
          body.innards_version,
          ws,
          machine.id,
        );
      } else {
        db.query(
          `INSERT INTO innards_machines (id, workspace_id, label, vendor, product, chassis, os, cpu, ram_gb, storage, gpu,
             owner_person_id, owner_org_id, first_seen, last_seen, last_health, last_report_id, last_critical, last_warning,
             last_top_finding_id, last_top_finding_title, last_top_finding_severity, last_innards_version)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        ).run(
          machine.id,
          ws,
          label,
          vendor,
          product,
          chassis,
          os,
          cpu,
          ramGb,
          storage,
          gpu,
          receivedAt,
          receivedAt,
          health,
          reportId,
          critical,
          warning,
          top?.id ?? null,
          top?.title ?? null,
          top?.severity ?? null,
          body.innards_version,
        );
      }
    })();
    return { machineId: machine.id, reportId, created: !existing };
  }

  /** Pre-registers a machine (needed when allowAnonymousMachines is off). */
  register(
    ws: string,
    input: {
      id: string;
      label?: string | null;
      ownerPersonId?: string | null;
      ownerOrgId?: string | null;
    },
  ): MachineRow {
    const existing = this.machine(ws, input.id);
    if (existing) throw new IngestError('conflict', `Machine ${input.id} already exists`, 409);
    this.db
      .query(
        `INSERT INTO innards_machines (id, workspace_id, label, owner_person_id, owner_org_id, first_seen)
         VALUES (?, ?, ?, ?, ?, ?)`,
      )
      .run(
        input.id,
        ws,
        input.label ?? 'Unnamed machine',
        input.ownerPersonId ?? null,
        input.ownerOrgId ?? null,
        nowIso(),
      );
    return this.machine(ws, input.id) as MachineRow;
  }

  patch(
    ws: string,
    id: string,
    input: { label?: string | null; ownerPersonId?: string | null; ownerOrgId?: string | null },
  ): MachineRow | null {
    const existing = this.machine(ws, id);
    if (!existing) return null;
    const sets: string[] = [];
    const params: unknown[] = [];
    if (input.label !== undefined && input.label !== null) {
      sets.push('label = ?');
      params.push(input.label);
    }
    if (input.ownerPersonId !== undefined) {
      sets.push('owner_person_id = ?');
      params.push(input.ownerPersonId);
      // One owner at a time: setting a person clears the organisation, unless both are sent.
      if (input.ownerPersonId && input.ownerOrgId === undefined) {
        sets.push('owner_org_id = NULL');
      }
    }
    if (input.ownerOrgId !== undefined) {
      sets.push('owner_org_id = ?');
      params.push(input.ownerOrgId);
      if (input.ownerOrgId && input.ownerPersonId === undefined) {
        sets.push('owner_person_id = NULL');
      }
    }
    if (sets.length) {
      params.push(ws, id);
      this.db
        .query(`UPDATE innards_machines SET ${sets.join(', ')} WHERE workspace_id = ? AND id = ?`)
        .run(...params);
    }
    return this.machine(ws, id);
  }

  remove(ws: string, id: string): boolean {
    const existing = this.machine(ws, id);
    if (!existing) return false;
    const db = this.db;
    db.transaction(() => {
      db.query('DELETE FROM innards_reports WHERE workspace_id = ? AND machine_id = ?').run(ws, id);
      db.query('DELETE FROM innards_machines WHERE workspace_id = ? AND id = ?').run(ws, id);
    })();
    return true;
  }

  ownerExists(ws: string, kind: 'person' | 'organisation', id: string): boolean {
    const table = kind === 'person' ? 'people' : 'organisations';
    return (
      this.db.query(`SELECT 1 FROM ${table} WHERE workspace_id = ? AND id = ?`).get(ws, id) != null
    );
  }

  analytics(ws: string): Analytics {
    const machines = this.listMachines(ws);
    const healthBuckets: Record<HealthBucket, number> = { critical: 0, warning: 0, ok: 0, good: 0 };
    let healthSum = 0;
    let healthN = 0;
    for (const m of machines) {
      if (m.lastHealth == null || !m.healthBucket) continue;
      healthBuckets[m.healthBucket]++;
      healthSum += m.lastHealth;
      healthN++;
    }
    // Latest report per machine carries the compact findings/capabilities/budget columns.
    const latest = this.db
      .query(
        `SELECT r.findings_json, r.capabilities_json, r.upgrade_low, r.upgrade_high
         FROM innards_machines m JOIN innards_reports r ON r.id = m.last_report_id
         WHERE m.workspace_id = ?`,
      )
      .all(ws) as Array<
      Pick<ReportRow, 'findings_json' | 'capabilities_json' | 'upgrade_low' | 'upgrade_high'>
    >;
    const findingCounts = new Map<string, { title: string; count: number }>();
    const capSums = new Map<string, { sum: number; n: number }>();
    let budgetLow = 0;
    let budgetHigh = 0;
    let budgetN = 0;
    for (const r of latest) {
      const findings = parseJson<Array<{ id: string; severity: Severity; title: string }>>(
        r.findings_json,
        [],
      );
      for (const f of findings) {
        if (f.severity !== 'critical' && f.severity !== 'warning') continue;
        const cur = findingCounts.get(f.id);
        if (cur) cur.count++;
        else findingCounts.set(f.id, { title: f.title, count: 1 });
      }
      for (const c of parseJson<Array<{ workload: string; score: number }>>(
        r.capabilities_json,
        [],
      )) {
        const cur = capSums.get(c.workload) ?? { sum: 0, n: 0 };
        cur.sum += c.score;
        cur.n++;
        capSums.set(c.workload, cur);
      }
      if (r.upgrade_low != null && r.upgrade_high != null) {
        budgetLow += r.upgrade_low;
        budgetHigh += r.upgrade_high;
        budgetN++;
      }
    }
    const topFindings = [...findingCounts.entries()]
      .map(([id, v]) => ({ id, title: v.title, count: v.count }))
      .sort((a, b) => b.count - a.count || a.id.localeCompare(b.id))
      .slice(0, 10);
    const capabilityAverages = [...capSums.entries()]
      .map(([workload, v]) => ({ workload, avg: Math.round(v.sum / v.n) }))
      .sort((a, b) => b.avg - a.avg);
    return {
      machines: machines.length,
      avgHealth: healthN ? Math.round(healthSum / healthN) : null,
      healthBuckets,
      topFindings,
      upgradeBudget: budgetN ? { low: budgetLow, high: budgetHigh } : null,
      capabilityAverages,
      staleMachines: machines.filter((m) => m.stale).length,
      // Needs attention: any critical finding, or a health score in the warning/critical band.
      attention: machines.filter((m) => m.criticalCount > 0 || m.healthBucket === 'critical' || m.healthBucket === 'warning').length,
    };
  }

  /** Deletes reports older than the workspace's retention, always keeping each machine's latest. */
  prune(now = Date.now()): { workspaces: number; deleted: number } {
    const db = this.db;
    const wsRows = db.query('SELECT DISTINCT workspace_id FROM innards_reports').all() as Array<{
      workspace_id: string;
    }>;
    let deleted = 0;
    for (const { workspace_id: ws } of wsRows) {
      const raw = Number(this.ctx.settings(ws).retentionDays);
      const days = Number.isFinite(raw) && raw > 0 ? raw : 365;
      const cutoff = new Date(now - days * 86_400_000).toISOString();
      const res = db
        .query(
          `DELETE FROM innards_reports WHERE workspace_id = ? AND received_at < ?
             AND id NOT IN (SELECT last_report_id FROM innards_machines WHERE workspace_id = ? AND last_report_id IS NOT NULL)`,
        )
        .run(ws, cutoff, ws) as { changes?: number } | undefined;
      deleted += res?.changes ?? 0;
    }
    return { workspaces: wsRows.length, deleted };
  }
}

function parseJson<T>(text: string, fallback: T): T {
  try {
    return JSON.parse(text) as T;
  } catch {
    return fallback;
  }
}

// --- HTTP ----------------------------------------------------------------------------------

function errorJson(c: Ctx, code: string, message: string, status: number, details?: unknown) {
  return c.json(
    { error: { code, message, ...(details !== undefined ? { details } : {}) } },
    status,
  );
}

/**
 * Same behaviour as Threadwise's `requirePermissionForWrites` (apps/api/src/lib/actor.ts):
 * reads pass, mutating methods need every listed permission. Re-implemented here because
 * an external plugin cannot import that module (see the header comment).
 */
export function requirePermissionForWrites(...perms: Permission[]): Middleware {
  return async (c, next) => {
    const method = c.req.method;
    if (method === 'GET' || method === 'HEAD' || method === 'OPTIONS') {
      await next();
      return undefined;
    }
    const actor = c.get('actor');
    const missing = perms.filter((p) => !actor?.permissions.has(p));
    if (missing.length)
      return errorJson(c, 'forbidden', `Missing permission: ${missing.join(', ')}`, 403, {
        missing,
      });
    await next();
    return undefined;
  };
}

async function readJson(c: Ctx): Promise<unknown> {
  const declared = Number(c.req.header('content-length'));
  if (Number.isFinite(declared) && declared > MAX_BODY_BYTES)
    throw new IngestError('payload_too_large', `Body exceeds ${MAX_BODY_BYTES} bytes`, 413);
  const text = await c.req.text();
  if (new TextEncoder().encode(text).byteLength > MAX_BODY_BYTES)
    throw new IngestError('payload_too_large', `Body exceeds ${MAX_BODY_BYTES} bytes`, 413);
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new IngestError('bad_request', 'Body is not valid JSON', 400);
  }
}

function buildRoutes(svc: FleetService) {
  return (app: SubApp) => {
    const ws = (c: Ctx) => c.get('workspaceId');
    const handle = (fn: Handler): Handler => {
      return async (c) => {
        try {
          return await fn(c);
        } catch (err) {
          if (err instanceof IngestError)
            return errorJson(c, err.code, err.message, err.status, err.details);
          throw err;
        }
      };
    };

    // Ingest and fleet edits sit with the permission Data Sources uses; reads are open to members.
    app.use('/*', requirePermissionForWrites('sources:manage'));

    app.post(
      '/reports',
      handle(async (c) => {
        const body = validateIngest(await readJson(c));
        const r = svc.ingest(ws(c), body);
        return c.json(
          {
            machineId: r.machineId,
            reportId: r.reportId,
            dashboardUrl: `${requestOrigin(c)}${DASHBOARD_BASE}/machines/${encodeURIComponent(r.machineId)}`,
          },
          201,
        );
      }),
    );

    app.get(
      '/machines',
      handle((c) => {
        const q = c.req.query('q')?.trim() || undefined;
        const owner = ownerFilter(c.req.query('ownerKind'), c.req.query('ownerId'));
        const items = svc.listMachines(ws(c), { q, owner });
        return c.json({ items, total: items.length, exact: true, nextCursor: null });
      }),
    );

    // Team: CSV export of the fleet (same filters as /machines).
    app.get(
      '/machines.csv',
      handle((c) => {
        const q = c.req.query('q')?.trim() || undefined;
        const owner = ownerFilter(c.req.query('ownerKind'), c.req.query('ownerId'));
        const items = svc.listMachines(ws(c), { q, owner });
        return new Response(machinesCsv(items), {
          headers: {
            'content-type': 'text/csv; charset=utf-8',
            'content-disposition': `attachment; filename="innards-fleet-${new Date().toISOString().slice(0, 10)}.csv"`,
          },
        });
      }),
    );

    app.post(
      '/machines',
      handle(async (c) => {
        const raw = await readJson(c);
        if (!isRecord(raw) || typeof raw.id !== 'string' || !MACHINE_ID_PATTERN.test(raw.id))
          return errorJson(
            c,
            'validation_failed',
            'id must be 8–128 characters of [A-Za-z0-9._:-]',
            400,
          );
        const input = parseMachinePatch(raw);
        const bad = checkOwners(svc, ws(c), input);
        if (bad) return errorJson(c, 'validation_failed', bad, 400);
        const row = svc.register(ws(c), { id: raw.id, ...input });
        return c.json(toSummary(row), 201);
      }),
    );

    app.get(
      '/machines/:id',
      handle((c) => {
        const row = svc.machine(ws(c), c.req.param('id'));
        if (!row) return errorJson(c, 'not_found', `Machine ${c.req.param('id')} not found`, 404);
        const latest = row.last_report_id ? svc.report(ws(c), row.last_report_id) : null;
        return c.json({
          machine: toSummary(row),
          latest: latest
            ? { ...toReportSummary(latest), rendered: parseJson(latest.rendered_json, null) }
            : null,
          history: svc.history(ws(c), row.id),
        });
      }),
    );

    app.get(
      '/machines/:id/reports',
      handle((c) => {
        const row = svc.machine(ws(c), c.req.param('id'));
        if (!row) return errorJson(c, 'not_found', `Machine ${c.req.param('id')} not found`, 404);
        const items = svc.reportsFor(ws(c), row.id);
        return c.json({ items, total: items.length, exact: true, nextCursor: null });
      }),
    );

    app.get(
      '/reports/:id',
      handle((c) => {
        const r = svc.report(ws(c), c.req.param('id'));
        if (!r) return errorJson(c, 'not_found', `Report ${c.req.param('id')} not found`, 404);
        return c.json({
          ...toReportSummary(r),
          report: parseJson(r.report_json, null),
          rendered: parseJson(r.rendered_json, null),
        });
      }),
    );

    app.patch(
      '/machines/:id',
      handle(async (c) => {
        const raw = await readJson(c);
        if (!isRecord(raw)) return errorJson(c, 'validation_failed', 'Body must be an object', 400);
        const input = parseMachinePatch(raw);
        const bad = checkOwners(svc, ws(c), input);
        if (bad) return errorJson(c, 'validation_failed', bad, 400);
        const row = svc.patch(ws(c), c.req.param('id'), input);
        if (!row) return errorJson(c, 'not_found', `Machine ${c.req.param('id')} not found`, 404);
        return c.json(toSummary(row));
      }),
    );

    app.delete(
      '/machines/:id',
      handle((c) => {
        if (!svc.remove(ws(c), c.req.param('id')))
          return errorJson(c, 'not_found', `Machine ${c.req.param('id')} not found`, 404);
        return c.body(null, 204);
      }),
    );

    app.get(
      '/analytics',
      handle((c) => c.json(svc.analytics(ws(c)))),
    );

    // Server-rendered record panel: the machines a person / organisation owns.
    app.get(
      '/panel',
      handle((c) => {
        const kind = c.req.query('kind');
        const id = c.req.query('id');
        if ((kind !== 'person' && kind !== 'organisation') || !id)
          return errorJson(
            c,
            'validation_failed',
            'kind (person|organisation) and id are required',
            400,
          );
        const machines = svc.listMachines(ws(c), { owner: { kind, id } });
        const content: PluginPanelContent = {
          summary: machines.length
            ? `${machines.length} machine${machines.length === 1 ? '' : 's'} · avg health ${Math.round(
                machines.reduce((n, m) => n + (m.lastHealth ?? 0), 0) / machines.length,
              )}`
            : 'No machines reported for this record yet.',
          items: machines.map((m) => ({
            label: m.label,
            value: describeMachine(m),
            knowledge: 'observed' as const,
            tone: PANEL_TONE[m.healthBucket ?? 'ok'],
            href: `${DASHBOARD_BASE}/machines/${encodeURIComponent(m.id)}`,
          })),
        };
        return c.json(content);
      }),
    );
  };
}

const PANEL_TONE: Record<HealthBucket, NonNullable<PluginPanelContent['items'][number]['tone']>> = {
  critical: 'danger',
  warning: 'warn',
  ok: 'neutral',
  good: 'ok',
};

function describeMachine(m: MachineSummary): string {
  const parts: string[] = [];
  parts.push(m.lastHealth != null ? `health ${m.lastHealth}` : 'no report yet');
  if (m.criticalCount) parts.push(`${m.criticalCount} critical`);
  if (m.warningCount) parts.push(`${m.warningCount} warning${m.warningCount === 1 ? '' : 's'}`);
  if (m.lastSeen) parts.push(`seen ${m.lastSeen.slice(0, 10)}`);
  if (m.stale) parts.push('stale');
  return parts.join(' · ');
}

function ownerFilter(
  kind: string | undefined,
  id: string | undefined,
): { kind: 'person' | 'organisation'; id: string } | undefined {
  if (!id) return undefined;
  if (kind === 'person' || kind === 'organisation') return { kind, id };
  return undefined;
}

function parseMachinePatch(raw: Record<string, unknown>): {
  label?: string | null;
  ownerPersonId?: string | null;
  ownerOrgId?: string | null;
} {
  const out: { label?: string | null; ownerPersonId?: string | null; ownerOrgId?: string | null } =
    {};
  if ('label' in raw) out.label = optString(raw.label);
  if ('ownerPersonId' in raw)
    out.ownerPersonId = raw.ownerPersonId === null ? null : optString(raw.ownerPersonId, 64);
  if ('ownerOrgId' in raw)
    out.ownerOrgId = raw.ownerOrgId === null ? null : optString(raw.ownerOrgId, 64);
  return out;
}

function checkOwners(
  svc: FleetService,
  ws: string,
  input: { ownerPersonId?: string | null; ownerOrgId?: string | null },
): string | null {
  if (input.ownerPersonId && !svc.ownerExists(ws, 'person', input.ownerPersonId))
    return `Person ${input.ownerPersonId} not found in this workspace`;
  if (input.ownerOrgId && !svc.ownerExists(ws, 'organisation', input.ownerOrgId))
    return `Organisation ${input.ownerOrgId} not found in this workspace`;
  return null;
}

// --- Plugin ------------------------------------------------------------------------------

const plugin: ThreadwisePlugin = {
  id: PLUGIN_ID,
  name: 'Innards fleet',
  version: '0.1.0',
  description:
    'Machine health reports uploaded by the Innards desktop app: a fleet dashboard, per-machine history, and the machines each person or organisation owns.',
  homepage: 'https://github.com/buengenio/innards',
  settingsSchema: [
    {
      key: 'retentionDays',
      label: 'Keep reports for (days)',
      type: 'number',
      default: 365,
      description: 'Older reports are pruned daily; the latest report per machine is always kept.',
    },
    {
      key: 'allowAnonymousMachines',
      label: 'Accept reports from unknown machines',
      type: 'boolean',
      default: true,
      description:
        'When off, a machine must be registered and assigned to a person or organisation before its reports are stored.',
    },
  ],
  register(ctx) {
    ctx.db.migrations(MIGRATIONS);
    const svc = new FleetService(ctx);

    ctx.routes.mount(buildRoutes(svc));

    ctx.nav.add({
      id: 'fleet',
      label: 'Machines',
      to: DASHBOARD_BASE,
      icon: 'monitor',
      area: 'primary',
      key: 'm',
    });
    ctx.nav.settingsPage({
      label: 'Innards fleet',
      to: `/settings/x/${PLUGIN_ID}`,
      icon: 'monitor',
    });

    ctx.recordPanels.add({
      id: 'machines',
      title: 'Machines',
      entityKinds: ['person', 'organisation'],
      endpoint: 'panel',
    });

    ctx.schedules.every('prune', PRUNE_EVERY_MS, () => {
      const r = svc.prune();
      if (r.deleted) ctx.log('pruned %d report(s) across %d workspace(s)', r.deleted, r.workspaces);
    });
  },
};

export default plugin;
