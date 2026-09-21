// Web half of the innards.fleet plugin: the fleet dashboard under /x/innards.fleet,
// its settings page, the nav icon, and the "Machines" panel on person and
// organisation records. Listed in THREADWISE_WEB_PLUGINS at build time; without it
// the server plugin still works (server-rendered panel, generic plugin page).
import { Link } from 'react-router-dom';
import { relativeTime } from '@/lib/format';
import { defineWebPlugin, type WebRecordPanelProps } from '@/plugins/registry';
import { Badge } from '@/ui/Badge';
import { SkeletonRows } from '@/ui/Skeleton';
import { BASE, PLUGIN_ID, useMachines } from './api';
import { HealthChip, MonitorIcon, SEVERITY_TONE } from './shared';

function MachinesPanel({ kind, id }: WebRecordPanelProps) {
  const machines = useMachines({ ownerKind: kind, ownerId: id });
  if (machines.isLoading) return <SkeletonRows />;
  const items = machines.data?.items ?? [];
  if (!items.length)
    return (
      <p className="text-sm text-ink-3">
        No machines assigned.{' '}
        <Link to={BASE} className="text-accent-ink hover:underline">
          Open the fleet
        </Link>{' '}
        to assign one.
      </p>
    );
  return (
    <ul className="divide-y divide-line rounded-sm border border-line text-sm">
      {items.map((m) => (
        <li key={m.id} className="flex flex-col gap-1 px-2 py-1.5">
          <div className="flex items-center gap-2">
            <Link
              to={`${BASE}/machines/${m.id}`}
              className="min-w-0 flex-1 truncate font-medium text-ink hover:underline"
            >
              {m.label}
            </Link>
            <HealthChip score={m.lastHealth} size="xs" />
            <span className="whitespace-nowrap text-xs text-ink-3">
              {m.lastSeen ? relativeTime(m.lastSeen) : 'no report'}
            </span>
          </div>
          {m.topFinding && (
            <div className="flex items-center gap-1.5 text-xs text-ink-2">
              <Badge tone={SEVERITY_TONE[m.topFinding.severity]} size="xs">
                {m.topFinding.severity}
              </Badge>
              <span className="truncate">{m.topFinding.title}</span>
            </div>
          )}
        </li>
      ))}
    </ul>
  );
}

export default defineWebPlugin({
  id: PLUGIN_ID,
  routes: [
    { index: true, lazy: () => import('./FleetPage').then((m) => ({ Component: m.FleetPage })) },
    {
      path: 'machines/:id',
      lazy: () => import('./MachineDetailPage').then((m) => ({ Component: m.MachineDetailPage })),
      handle: { crumb: (_d: unknown, p: { id?: string }) => p.id ?? 'Machine' },
    },
  ],
  settingsRoutes: [
    {
      index: true,
      lazy: () => import('./SettingsPage').then((m) => ({ Component: m.SettingsPage })),
    },
  ],
  icons: { fleet: MonitorIcon },
  recordPanels: { machines: MachinesPanel },
});
