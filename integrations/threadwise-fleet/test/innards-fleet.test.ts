// ---------------------------------------------------------------------------
// Bun test for the innards.fleet plugin, written to Threadwise's own pattern
// (apps/api/src/tests/plugins.test.ts): registerPlugin + createApp, then call
// /api/ext/innards.fleet/… through the real workspace + actor middleware.
//
// Run it from the Threadwise checkout:
//   cp  <innards>/integrations/threadwise-fleet/test/innards-fleet.test.ts apps/api/src/tests/
//   INNARDS_FLEET_PLUGIN=<innards>/integrations/threadwise-fleet/server/index.ts bun test innards-fleet
// With INNARDS_FLEET_PLUGIN unset it assumes `innards` is checked out next to `threadwise`.
// ---------------------------------------------------------------------------

import { beforeAll, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { createApp } from '../app.ts';
import { registerPlugin } from '../plugins/index.ts';
import { createOrganisation, createPerson } from '../services/records.ts';
import { freshWorkspace } from './setup.ts';

const pluginPath =
  process.env.INNARDS_FLEET_PLUGIN ??
  resolve(import.meta.dir, '../../../../../innards/integrations/threadwise-fleet/server/index.ts');
const fixtures = resolve(dirname(pluginPath), '../../../static/fixtures');
const report = JSON.parse(readFileSync(resolve(fixtures, 'report-full.json'), 'utf8'));
const rendered = JSON.parse(readFileSync(resolve(fixtures, 'report-en-informed.json'), 'utf8'));

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));
const machine = { id: 'mach_x1carbon_0001', label: 'Ann’s X1 Carbon' };
const payload = (overrides: Record<string, unknown> = {}) => ({
  machine,
  innards_version: '0.4.0',
  report: clone(report),
  rendered: clone(rendered),
  ...overrides,
});

async function call(
  app: ReturnType<typeof createApp>,
  ws: string,
  path: string,
  init: { method?: string; body?: unknown } = {},
) {
  const res = await app.request(`/api/ext/innards.fleet${path}`, {
    method: init.method ?? 'GET',
    headers: { 'X-Workspace-Id': ws, 'content-type': 'application/json' },
    body: init.body === undefined ? undefined : JSON.stringify(init.body),
  });
  const text = await res.text();
  return { status: res.status, body: text ? (JSON.parse(text) as any) : null };
}

describe('innards.fleet', () => {
  beforeAll(async () => {
    const mod = (await import(pluginPath)) as { default: Parameters<typeof registerPlugin>[0] };
    await registerPlugin(mod.default, { origin: 'test' });
  });

  test('manifest: routes, nav, panel, schedule and settings defaults', async () => {
    const ws = freshWorkspace();
    const app = createApp();
    const res = await app.request('/api/plugins', { headers: { 'X-Workspace-Id': ws } });
    const list = (await res.json()) as any[];
    const mine = list.find((p) => p.manifest.id === 'innards.fleet');
    expect(mine).toBeDefined();
    expect(mine.errors).toEqual([]);
    expect(mine.manifest.provides.routes).toBe(true);
    expect(mine.manifest.provides.navigation[0].to).toBe('/x/innards.fleet');
    expect(mine.manifest.provides.recordPanels[0].entityKinds).toEqual(['person', 'organisation']);
    expect(mine.manifest.provides.schedules).toEqual(['prune']);
    expect(mine.settings).toEqual({ retentionDays: 365, allowAnonymousMachines: true });
  });

  test('ingests the fixture report, scrubs identifiers, lists and aggregates it', async () => {
    const ws = freshWorkspace();
    const app = createApp();

    const bad = await call(app, ws, '/reports', { method: 'POST', body: { machine: { id: 'x' } } });
    expect(bad.status).toBe(400);
    expect(bad.body.error.code).toBe('validation_failed');

    const posted = await call(app, ws, '/reports', { method: 'POST', body: payload() });
    expect(posted.status).toBe(201);
    expect(posted.body.machineId).toBe(machine.id);
    expect(posted.body.dashboardUrl).toMatch(/\/x\/innards\.fleet\/machines\/mach_x1carbon_0001$/);

    const stored = await call(app, ws, `/reports/${posted.body.reportId}`);
    expect(stored.status).toBe(200);
    expect(stored.body.report.snapshot.system.hostname).toBeNull();
    expect(stored.body.report.snapshot.load.top_memory).toEqual([]);
    expect(stored.body.report.snapshot.storage.every((d: any) => d.serial === null)).toBe(true);

    const machines = await call(app, ws, '/machines');
    expect(machines.status).toBe(200);
    expect(machines.body.total).toBe(1);
    const m = machines.body.items[0];
    expect(m.label).toBe(machine.label);
    expect(m.lastHealth).toBe(rendered.summary.health_score);
    expect(m.healthBucket).toBe('warning');
    expect(m.criticalCount).toBe(
      rendered.findings.filter((f: any) => f.severity === 'critical').length,
    );
    expect(m.topFinding.severity).toBe('critical');
    expect(m.product).toBe('ThinkPad X1 Carbon 5th');
    expect(m.ramGb).toBe(16);

    // Another workspace sees nothing.
    expect((await call(app, freshWorkspace(), '/machines')).body.total).toBe(0);

    const detail = await call(app, ws, `/machines/${machine.id}`);
    expect(detail.status).toBe(200);
    expect(detail.body.history).toHaveLength(1);
    expect(detail.body.latest.rendered.verdict).toBe(rendered.verdict);

    const analytics = await call(app, ws, '/analytics');
    expect(analytics.status).toBe(200);
    expect(analytics.body.machines).toBe(1);
    expect(analytics.body.avgHealth).toBe(rendered.summary.health_score);
    expect(analytics.body.healthBuckets).toEqual({ critical: 0, warning: 1, ok: 0, good: 0 });
    expect(analytics.body.staleMachines).toBe(0);
    expect(analytics.body.upgradeBudget).toBeNull();
    expect(analytics.body.topFindings[0].id).toBe('storage.fs_nearly_full');
    expect(analytics.body.capabilityAverages).toHaveLength(10);
  });

  test('owners: PATCH validates the record, the panel lists machines, DELETE removes history', async () => {
    const ws = freshWorkspace();
    const app = createApp();
    await call(app, ws, '/reports', { method: 'POST', body: payload() });
    const person = createPerson(ws, { firstName: 'Ann', lastName: 'Lee' });
    const org = createOrganisation(ws, { name: 'Acme' });

    const missing = await call(app, ws, `/machines/${machine.id}`, {
      method: 'PATCH',
      body: { ownerPersonId: 'per_nope' },
    });
    expect(missing.status).toBe(400);

    const patched = await call(app, ws, `/machines/${machine.id}`, {
      method: 'PATCH',
      body: { ownerPersonId: person.id, label: 'Ann’s laptop' },
    });
    expect(patched.status).toBe(200);
    expect(patched.body.owner.kind).toBe('person');
    expect(patched.body.label).toBe('Ann’s laptop');

    const panel = await call(app, ws, `/panel?kind=person&id=${person.id}`);
    expect(panel.status).toBe(200);
    expect(panel.body.items).toHaveLength(1);
    expect(panel.body.items[0].href).toBe(`/x/innards.fleet/machines/${machine.id}`);
    expect(panel.body.items[0].tone).toBe('warn');

    const moved = await call(app, ws, `/machines/${machine.id}`, {
      method: 'PATCH',
      body: { ownerOrgId: org.id },
    });
    expect(moved.body.owner.kind).toBe('organisation');
    expect(moved.body.ownerPersonId).toBeNull();
    expect((await call(app, ws, `/panel?kind=organisation&id=${org.id}`)).body.items).toHaveLength(
      1,
    );

    const gone = await call(app, ws, `/machines/${machine.id}`, { method: 'DELETE' });
    expect(gone.status).toBe(204);
    expect((await call(app, ws, `/machines/${machine.id}`)).status).toBe(404);
  });

  test('rejects oversized bodies and, when configured, unknown machines', async () => {
    const ws = freshWorkspace();
    const app = createApp();
    const huge = payload();
    (huge.report as any).padding = 'x'.repeat(2 * 1024 * 1024);
    expect((await call(app, ws, '/reports', { method: 'POST', body: huge })).status).toBe(413);

    const off = await app.request('/api/plugins/innards.fleet', {
      method: 'PATCH',
      headers: { 'X-Workspace-Id': ws, 'content-type': 'application/json' },
      body: JSON.stringify({ settings: { allowAnonymousMachines: false } }),
    });
    expect(off.status).toBe(200);
    const refused = await call(app, ws, '/reports', { method: 'POST', body: payload() });
    expect(refused.status).toBe(403);

    const person = createPerson(ws, { firstName: 'Cy', lastName: 'Ng' });
    const reg = await call(app, ws, '/machines', {
      method: 'POST',
      body: { id: machine.id, ownerPersonId: person.id },
    });
    expect(reg.status).toBe(201);
    expect((await call(app, ws, '/reports', { method: 'POST', body: payload() })).status).toBe(201);
  });
});
