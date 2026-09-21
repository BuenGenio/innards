// ---------------------------------------------------------------------------
// Dependency-free smoke test that runs under plain Node (>= 22.5):
//
//   node --experimental-strip-types integrations/threadwise-fleet/test/smoke.node.ts
//
// It stands in for Threadwise's PluginContext with a bun:sqlite-shaped adapter
// over node:sqlite and a tiny Hono-shaped router, then drives the real routes
// with the real fixtures. The Bun test (innards-fleet.test.ts) is the one that
// exercises the plugin inside Threadwise itself; this one exists so the SQL and
// the ingest contract can be checked without Bun installed.
// ---------------------------------------------------------------------------

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';
import plugin, {
  healthBucket,
  type PluginContext,
  scrub,
  topFinding,
  upgradeBudget,
  validateIngest,
} from '../server/index.ts';

const here = dirname(fileURLToPath(import.meta.url));
const fixtures = resolve(here, '../../../static/fixtures');
const report = JSON.parse(readFileSync(resolve(fixtures, 'report-full.json'), 'utf8'));
const rendered = JSON.parse(readFileSync(resolve(fixtures, 'report-en-informed.json'), 'utf8'));

// --- bun:sqlite shaped adapter ------------------------------------------------------------

const raw = new DatabaseSync(':memory:');
raw.exec(`
CREATE TABLE people (id text primary key, workspace_id text, display_name text);
CREATE TABLE organisations (id text primary key, workspace_id text, name text);
INSERT INTO people VALUES ('per_1', 'ws_a', 'Ann Lee');
INSERT INTO organisations VALUES ('org_1', 'ws_a', 'Acme');
`);
const sqlite = {
  query(sql: string) {
    return {
      all: (...p: unknown[]) => raw.prepare(sql).all(...(p as never[])),
      get: (...p: unknown[]) => raw.prepare(sql).get(...(p as never[])) ?? null,
      run: (...p: unknown[]) => raw.prepare(sql).run(...(p as never[])),
    };
  },
  transaction<T>(fn: () => T) {
    return () => {
      raw.exec('BEGIN');
      try {
        const r = fn();
        raw.exec('COMMIT');
        return r;
      } catch (e) {
        raw.exec('ROLLBACK');
        throw e;
      }
    };
  },
};

// --- Hono shaped router --------------------------------------------------------------------

type Handler = (c: any) => Response | Promise<Response>;
type Mw = (c: any, next: () => Promise<void>) => Promise<Response | undefined>;
const routes: Array<{ method: string; pattern: RegExp; keys: string[]; h: Handler }> = [];
const middlewares: Mw[] = [];
function add(method: string, path: string, h: Handler) {
  const keys: string[] = [];
  const pattern = new RegExp(
    `^${path.replace(/:(\w+)/g, (_m, k) => {
      keys.push(k);
      return '([^/]+)';
    })}$`,
  );
  routes.push({ method, pattern, keys, h });
}
const app = {
  use: (_p: string, mw: Mw) => middlewares.push(mw),
  get: (p: string, h: Handler) => add('GET', p, h),
  post: (p: string, h: Handler) => add('POST', p, h),
  patch: (p: string, h: Handler) => add('PATCH', p, h),
  delete: (p: string, h: Handler) => add('DELETE', p, h),
};

const settings: Record<string, Record<string, unknown>> = {};
const ctx: PluginContext = {
  pluginId: plugin.id,
  log: () => {},
  settings: (ws) => ({ retentionDays: 365, allowAnonymousMachines: true, ...settings[ws] }),
  enabled: () => true,
  routes: { mount: (build) => build(app) },
  nav: { add: () => {}, settingsPage: () => {} },
  recordPanels: { add: () => {} },
  schedules: { every: () => {} },
  db: {
    migrations: (list) => {
      for (const m of list) raw.exec(m.sql);
    },
    sqlite: () => sqlite,
  },
};

await plugin.register(ctx);

async function call(
  ws: string,
  method: string,
  path: string,
  body?: unknown,
  perms: string[] = ['sources:manage'],
): Promise<{ status: number; body: any }> {
  const url = new URL(`http://localhost:3001/api/ext/innards.fleet${path}`);
  const text = body === undefined ? '' : JSON.stringify(body);
  const params: Record<string, string> = {};
  const route = routes.find((r) => {
    if (r.method !== method) return false;
    const m = url.pathname.replace('/api/ext/innards.fleet', '').match(r.pattern);
    if (!m) return false;
    r.keys.forEach((k, i) => {
      params[k] = decodeURIComponent(m[i + 1] ?? '');
    });
    return true;
  });
  if (!route) return { status: 404, body: null };
  const c = {
    req: {
      method,
      url: url.toString(),
      param: (k: string) => params[k] ?? '',
      query: (k: string) => url.searchParams.get(k) ?? undefined,
      header: (k: string) =>
        k === 'content-length' ? String(new TextEncoder().encode(text).byteLength) : undefined,
      text: async () => text,
      json: async () => JSON.parse(text),
    },
    get: (k: string) => (k === 'workspaceId' ? ws : { permissions: new Set(perms) }),
    json: (b: unknown, status = 200) =>
      new Response(JSON.stringify(b), { status, headers: { 'content-type': 'application/json' } }),
    body: (_b: null, status: number) => new Response(null, { status }),
  };
  let res: Response | undefined;
  let i = 0;
  const next = async () => {
    const mw = middlewares[i++];
    if (mw) {
      const r = await mw(c, next);
      if (r) res = r;
    } else res = await route.h(c);
  };
  await next();
  const out = res as Response;
  const t = await out.text();
  return { status: out.status, body: t ? JSON.parse(t) : null };
}

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));
const machine = { id: 'mach_x1carbon_0001', label: 'Ann’s X1 Carbon' };
const body = () => ({
  machine,
  innards_version: '0.4.0',
  report: clone(report),
  rendered: clone(rendered),
});

// --- Pure helpers ---------------------------------------------------------------------------

assert.equal(healthBucket(95), 'good');
assert.equal(healthBucket(80), 'good');
assert.equal(healthBucket(70), 'ok');
assert.equal(healthBucket(58), 'warning');
assert.equal(healthBucket(10), 'critical');
assert.equal(healthBucket(null), null);
assert.deepEqual(
  upgradeBudget([
    { id: 'a', cost_usd: [40, 90] },
    { id: 'b', cost: '$100–$200' },
  ]),
  {
    low: 140,
    high: 290,
  },
);
assert.equal(upgradeBudget(undefined), null);
assert.equal(upgradeBudget([{ id: 'a', cost: 'Free' }])?.high, 0);
assert.equal(topFinding(rendered.findings)?.severity, 'critical');

// scrub removes identifiers
{
  const r = clone(report);
  const rr = clone(rendered);
  r.snapshot.system.hostname = 'my-laptop';
  r.snapshot.storage[0].serial = 'S3RIAL';
  r.snapshot.network[0].mac = 'aa:bb';
  r.summary.machine = 'my-laptop';
  scrub(r, rr);
  assert.equal(r.snapshot.system.hostname, null);
  assert.equal(r.snapshot.storage[0].serial, null);
  assert.equal(r.snapshot.network[0].mac, null);
  assert.deepEqual(r.snapshot.load.top_memory, []);
  assert.equal(r.summary.machine, 'Unnamed machine');
}

// validation
assert.throws(() => validateIngest({}), /machine is required/);
assert.throws(() => validateIngest({ ...body(), machine: { id: 'short' } }), /machine.id/);

// --- Routes ---------------------------------------------------------------------------------

// permission gate: write without sources:manage is 403; read passes
assert.equal((await call('ws_a', 'POST', '/reports', body(), ['records:read'])).status, 403);
assert.equal((await call('ws_a', 'GET', '/machines', undefined, [])).status, 200);

// ingest
const first = await call('ws_a', 'POST', '/reports', body());
assert.equal(first.status, 201, JSON.stringify(first.body));
assert.equal(first.body.machineId, machine.id);
assert.match(first.body.dashboardUrl, /\/x\/innards\.fleet\/machines\/mach_x1carbon_0001$/);

// stored blobs are scrubbed
const stored = await call('ws_a', 'GET', `/reports/${first.body.reportId}`);
assert.equal(stored.status, 200);
assert.equal(stored.body.report.snapshot.system.hostname, null);
assert.deepEqual(stored.body.report.snapshot.load.top_memory, []);
assert.ok(stored.body.report.snapshot.storage.every((d: any) => d.serial === null));
assert.equal(stored.body.rendered.lang, 'en');

// list
const list = await call('ws_a', 'GET', '/machines');
assert.equal(list.body.total, 1);
const m = list.body.items[0];
assert.equal(m.label, machine.label);
assert.equal(m.lastHealth, 65);
assert.equal(m.healthBucket, 'ok');
const count = (sev: string) => rendered.findings.filter((f: any) => f.severity === sev).length;
assert.equal(m.criticalCount, count('critical'));
assert.equal(m.warningCount, count('warning'));
assert.equal(m.topFinding.severity, 'critical');
assert.equal(m.product, 'ThinkPad X1 Carbon 5th');
assert.equal(m.ramGb, 16);
assert.equal(m.stale, false);
assert.equal(m.owner, null);

// workspace isolation
assert.equal((await call('ws_b', 'GET', '/machines')).body.total, 0);
assert.equal((await call('ws_b', 'GET', `/machines/${machine.id}`)).status, 404);

// second report -> history grows, label kept
const second = await call('ws_a', 'POST', '/reports', { ...body(), machine: { id: machine.id } });
assert.equal(second.status, 201);
const detail = await call('ws_a', 'GET', `/machines/${machine.id}`);
assert.equal(detail.status, 200);
assert.equal(detail.body.machine.label, machine.label);
assert.equal(detail.body.history.length, 2);
assert.equal(detail.body.latest.id, second.body.reportId);
assert.equal(detail.body.latest.rendered.findings.length, rendered.findings.length);
assert.equal((await call('ws_a', 'GET', `/machines/${machine.id}/reports`)).body.total, 2);

// owner assignment + panel + filter
assert.equal(
  (await call('ws_a', 'PATCH', `/machines/${machine.id}`, { ownerPersonId: 'per_nope' })).status,
  400,
);
const patched = await call('ws_a', 'PATCH', `/machines/${machine.id}`, { ownerPersonId: 'per_1' });
assert.equal(patched.status, 200);
assert.deepEqual(patched.body.owner, { kind: 'person', id: 'per_1', name: 'Ann Lee' });
const panel = await call('ws_a', 'GET', '/panel?kind=person&id=per_1');
assert.equal(panel.body.items.length, 1);
assert.equal(panel.body.items[0].tone, 'neutral');
assert.equal(panel.body.items[0].href, `/x/innards.fleet/machines/${machine.id}`);
assert.equal((await call('ws_a', 'GET', '/panel?kind=organisation&id=org_1')).body.items.length, 0);
assert.equal((await call('ws_a', 'GET', '/machines?ownerKind=person&ownerId=per_1')).body.total, 1);
// switching to an organisation clears the person
const org = await call('ws_a', 'PATCH', `/machines/${machine.id}`, { ownerOrgId: 'org_1' });
assert.equal(org.body.owner.kind, 'organisation');
assert.equal(org.body.ownerPersonId, null);

// analytics
const analytics = await call('ws_a', 'GET', '/analytics');
assert.equal(analytics.status, 200);
assert.equal(analytics.body.machines, 1);
assert.equal(analytics.body.avgHealth, 65);
assert.deepEqual(analytics.body.healthBuckets, { critical: 0, warning: 0, ok: 1, good: 0 });
assert.equal(analytics.body.attention, 1);
assert.equal(analytics.body.staleMachines, 0);
assert.equal(analytics.body.upgradeBudget, null);
assert.equal(analytics.body.topFindings[0].id, 'storage.fs_nearly_full');
assert.equal(analytics.body.capabilityAverages.length, 12);
assert.equal(analytics.body.capabilityAverages[0].workload, 'everyday');

// with a budget attached
const withRecs = body();
(withRecs.report as any).recommendations = [{ id: 'rec.ssd', cost_usd: [60, 120] }];
await call('ws_a', 'POST', '/reports', { ...withRecs, machine: { id: 'mach_second_0002' } });
assert.deepEqual((await call('ws_a', 'GET', '/analytics')).body.upgradeBudget, {
  low: 60,
  high: 120,
});

// size limit
const huge = body();
(huge.report as any).padding = 'x'.repeat(2 * 1024 * 1024);
assert.equal((await call('ws_a', 'POST', '/reports', huge)).status, 413);

// allowAnonymousMachines = false
settings.ws_c = { allowAnonymousMachines: false };
assert.equal(
  (await call('ws_c', 'POST', '/reports', { ...body(), machine: { id: 'mach_unknown_0003' } }))
    .status,
  403,
);
raw.exec("INSERT INTO people VALUES ('per_c', 'ws_c', 'Cy')");
const reg = await call('ws_c', 'POST', '/machines', {
  id: 'mach_unknown_0003',
  ownerPersonId: 'per_c',
});
assert.equal(reg.status, 201);
assert.equal(
  (await call('ws_c', 'POST', '/reports', { ...body(), machine: { id: 'mach_unknown_0003' } }))
    .status,
  201,
);

// delete
assert.equal((await call('ws_a', 'DELETE', `/machines/${machine.id}`)).status, 204);
assert.equal((await call('ws_a', 'GET', `/machines/${machine.id}`)).status, 404);
assert.equal((await call('ws_a', 'GET', `/reports/${first.body.reportId}`)).status, 404);

// prune keeps the latest report per machine
raw.exec("UPDATE innards_reports SET received_at = '2020-01-01T00:00:00.000Z'");
const before = raw.prepare('SELECT count(*) AS n FROM innards_reports').get() as { n: number };
// Reach the schedule through the registered callback.
let pruneFn: (() => void | Promise<void>) | null = null;
await plugin.register({ ...ctx, schedules: { every: (_n, _ms, fn) => (pruneFn = fn) } });
await pruneFn!();
const after = raw.prepare('SELECT count(*) AS n FROM innards_reports').get() as { n: number };
const machinesLeft = raw
  .prepare('SELECT count(*) AS n FROM innards_machines WHERE last_report_id IS NOT NULL')
  .get() as {
  n: number;
};
assert.ok(before.n >= after.n);
assert.equal(after.n, machinesLeft.n);

console.log('innards.fleet smoke test: ok');
