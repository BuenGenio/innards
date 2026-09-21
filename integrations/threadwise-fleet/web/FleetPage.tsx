// /x/innards.fleet — every machine that has reported, with the fleet's numbers on top.
import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { PageHeader, Section } from '@/features/common/PageHeader';
import { relativeTime } from '@/lib/format';
import { Badge } from '@/ui/Badge';
import { Bars } from '@/ui/Chart';
import { EmptyState } from '@/ui/EmptyState';
import { ErrorState } from '@/ui/ErrorState';
import { Input } from '@/ui/Input';
import { KpiTile } from '@/ui/KpiTile';
import { SkeletonRows } from '@/ui/Skeleton';
import { BASE, type MachineSummary, useAnalytics, useMachines } from './api';
import { describeHardware, HealthChip, MonitorIcon, SEVERITY_TONE } from './shared';

const page = 'flex min-h-0 flex-1 flex-col overflow-y-auto';

const WORKLOAD_LABEL: Record<string, string> = {
  everyday: 'Everyday use',
  web_dev: 'Web & app development',
  containers: 'Containers',
  home_server: 'Home server',
  photo_editing: 'Photo editing',
  heavy_compile: 'Heavy compiles',
  video_editing: 'Video editing',
  local_llm: 'Local AI models',
  gaming: 'Gaming',
  ml_training: 'ML training',
};

function money(n: number): string {
  return `$${n.toLocaleString()}`;
}

export function FleetPage() {
  const [q, setQ] = useState('');
  const machines = useMachines(q.trim() ? { q: q.trim() } : {});
  const analytics = useAnalytics();
  const nav = useNavigate();
  const a = analytics.data;
  const items = machines.data?.items ?? [];

  return (
    <div className={page}>
      <PageHeader
        title="Machines"
        description="Health reports uploaded by the Innards desktop app: what each machine is, how it is doing, and who it belongs to."
      >
        <div className="flex flex-wrap items-center gap-2">
          <Input
            aria-label="Search machines"
            placeholder="Search by label, model, OS, CPU or owner…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            className="w-72 max-w-full"
          />
          {machines.data && (
            <span className="text-xs text-ink-3">
              {machines.data.total} machine{machines.data.total === 1 ? '' : 's'}
            </span>
          )}
        </div>
      </PageHeader>

      <div className="flex flex-col gap-4 p-4 md:p-6">
        {analytics.isError ? (
          <ErrorState error={analytics.error} onRetry={() => void analytics.refetch()} compact />
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <KpiTile label="Machines" value={a?.machines ?? 0} rawValue={a?.machines} />
            <KpiTile
              label="Average health"
              value={a?.avgHealth ?? '—'}
              rawValue={a?.avgHealth ?? undefined}
              unknown={!!a && a.avgHealth == null}
              knowledge="calculated"
              hint="Mean of each machine's latest health score (0–100)"
            />
            <KpiTile
              label="Need attention"
              value={a?.attention ?? 0}
              rawValue={a?.attention}
              knowledge="calculated"
              hint="Latest health below 75"
              trend={
                a && (a.healthBuckets.critical || a.healthBuckets.warning) ? (
                  <>
                    {a.healthBuckets.critical > 0 && (
                      <span className="text-danger">{a.healthBuckets.critical} critical</span>
                    )}
                    {a.healthBuckets.critical > 0 && a.healthBuckets.warning > 0 && ' · '}
                    {a.healthBuckets.warning > 0 && (
                      <span className="text-warning">{a.healthBuckets.warning} warning</span>
                    )}
                  </>
                ) : undefined
              }
            />
            <KpiTile
              label="Stale"
              value={a?.staleMachines ?? 0}
              rawValue={a?.staleMachines}
              hint="No report in the last 30 days"
            />
          </div>
        )}

        {machines.isError ? (
          <ErrorState error={machines.error} onRetry={() => void machines.refetch()} />
        ) : machines.isLoading ? (
          <SkeletonRows />
        ) : items.length === 0 && !q ? (
          <EmptyState
            icon={<MonitorIcon />}
            title="No machines yet"
            description="Point the Innards desktop app at this workspace (Settings → Fleet: this server's URL and an API key) and upload a report. It appears here within seconds."
            action={
              <Link
                to={`/settings/x/innards.fleet`}
                className="text-sm text-accent-ink hover:underline"
              >
                How to connect a machine
              </Link>
            }
          />
        ) : (
          <Section
            title="Fleet"
            description="Sorted by health, worst first. Click a machine for its full report and history."
          >
            {items.length === 0 ? (
              <p className="py-4 text-sm text-ink-3">No machines match “{q}”.</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead className="text-left text-xs text-ink-3">
                    <tr>
                      <th className="py-2 pr-3 font-medium">Machine</th>
                      <th className="py-2 pr-3 font-medium">Owner</th>
                      <th className="py-2 pr-3 font-medium">Health</th>
                      <th className="py-2 pr-3 text-right font-medium">Critical</th>
                      <th className="py-2 pr-3 text-right font-medium">Warnings</th>
                      <th className="py-2 pr-3 font-medium">Top finding</th>
                      <th className="py-2 font-medium">Last seen</th>
                    </tr>
                  </thead>
                  <tbody>
                    {items.map((m) => (
                      <MachineRow key={m.id} m={m} onOpen={() => nav(`${BASE}/machines/${m.id}`)} />
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Section>
        )}

        {a && a.machines > 0 && (
          <div className="grid gap-4 lg:grid-cols-2">
            <Section
              title="Most common problems"
              description="Critical and warning findings across each machine's latest report."
            >
              {a.topFindings.length === 0 ? (
                <p className="text-sm text-ink-3">Nothing to fix across the fleet right now.</p>
              ) : (
                <Bars
                  data={a.topFindings.map((f) => ({ label: f.title, value: f.count, tone: 'ink' }))}
                  format={(n) => `${n} machine${n === 1 ? '' : 's'}`}
                />
              )}
              {a.upgradeBudget && (
                <p className="mt-3 text-xs text-ink-3">
                  Upgrade advisor budget across the fleet:{' '}
                  <span className="text-ink">
                    {money(a.upgradeBudget.low)} – {money(a.upgradeBudget.high)}
                  </span>
                </p>
              )}
            </Section>
            <Section
              title="What the fleet is good for"
              description="Average capability score per workload, 0–100."
            >
              <Bars
                data={a.capabilityAverages.map((c) => ({
                  label: WORKLOAD_LABEL[c.workload] ?? c.workload,
                  value: c.avg,
                  tone: 'accent',
                }))}
                format={(n) => `${n}`}
              />
            </Section>
          </div>
        )}
      </div>
    </div>
  );
}

function MachineRow({ m, onOpen }: { m: MachineSummary; onOpen: () => void }) {
  return (
    <tr
      className="cursor-pointer border-t border-line hover:bg-surface-2/60"
      onClick={onOpen}
      onKeyDown={(e) => {
        if (e.key === 'Enter') onOpen();
      }}
      tabIndex={0}
    >
      <td className="py-2 pr-3">
        <Link
          to={`${BASE}/machines/${m.id}`}
          className="font-medium text-ink hover:underline"
          onClick={(e) => e.stopPropagation()}
        >
          {m.label}
        </Link>
        <div className="max-w-md truncate text-xs text-ink-3" title={describeHardware(m)}>
          {describeHardware(m) || '—'}
        </div>
      </td>
      <td className="py-2 pr-3">
        {m.owner ? (
          <Link
            to={`/relationships/${m.owner.kind === 'person' ? 'people' : 'organisations'}/${m.owner.id}`}
            className="text-ink hover:underline"
            onClick={(e) => e.stopPropagation()}
          >
            {m.owner.name}
          </Link>
        ) : (
          <span className="text-ink-3">Unassigned</span>
        )}
      </td>
      <td className="py-2 pr-3">
        <HealthChip score={m.lastHealth} />
      </td>
      <td className="py-2 pr-3 text-right tabular-nums">
        {m.criticalCount ? <span className="text-danger">{m.criticalCount}</span> : '0'}
      </td>
      <td className="py-2 pr-3 text-right tabular-nums">
        {m.warningCount ? <span className="text-warning">{m.warningCount}</span> : '0'}
      </td>
      <td className="max-w-xs py-2 pr-3">
        {m.topFinding ? (
          <span className="flex items-center gap-1.5">
            <Badge tone={SEVERITY_TONE[m.topFinding.severity]} size="xs">
              {m.topFinding.severity}
            </Badge>
            <span className="truncate text-ink-2" title={m.topFinding.title}>
              {m.topFinding.title}
            </span>
          </span>
        ) : (
          <span className="text-ink-3">—</span>
        )}
      </td>
      <td className="py-2 whitespace-nowrap text-ink-2">
        {m.lastSeen ? relativeTime(m.lastSeen) : '—'}
        {m.stale && (
          <Badge tone="outline" size="xs" className="ml-2">
            stale
          </Badge>
        )}
      </td>
    </tr>
  );
}
