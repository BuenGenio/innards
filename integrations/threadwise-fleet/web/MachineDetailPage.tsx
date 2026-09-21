// /x/innards.fleet/machines/:id — one machine: summary header, health history, the
// latest rendered report grouped by severity, capability scores, and the owner.
import { useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useSearch } from '@/api/hooks';
import { PageHeader, Section } from '@/features/common/PageHeader';
import { formatDateTime, relativeTime } from '@/lib/format';
import { toast } from '@/stores/toasts';
import { Badge } from '@/ui/Badge';
import { Button } from '@/ui/Button';
import { Bars } from '@/ui/Chart';
import { Combobox, type ComboboxOption } from '@/ui/Combobox';
import { Dialog } from '@/ui/Dialog';
import { ErrorState } from '@/ui/ErrorState';
import { Input } from '@/ui/Input';
import { KnowledgeBadge } from '@/ui/KnowledgeBadge';
import { KpiTile } from '@/ui/KpiTile';
import { SkeletonRows } from '@/ui/Skeleton';
import {
  BASE,
  type MachinePatch,
  type MachineSummary,
  type RenderedFinding,
  type RenderedReport,
  useFleetMutations,
  useMachine,
  useMachineReports,
} from './api';
import {
  describeHardware,
  HealthChip,
  HealthSparkline,
  SEVERITY_LABEL,
  SEVERITY_ORDER,
  SEVERITY_TONE,
  TrashIcon,
} from './shared';

const page = 'flex min-h-0 flex-1 flex-col overflow-y-auto';

export function MachineDetailPage() {
  const { id = '' } = useParams();
  const detail = useMachine(id);
  const reports = useMachineReports(id);
  const { remove } = useFleetMutations();
  const nav = useNavigate();
  const [confirmDelete, setConfirmDelete] = useState(false);

  if (detail.isError)
    return (
      <div className={page}>
        <PageHeader title="Machine" />
        <div className="p-4 md:p-6">
          <ErrorState error={detail.error} onRetry={() => void detail.refetch()} />
        </div>
      </div>
    );
  if (!detail.data)
    return (
      <div className={page}>
        <PageHeader title="Machine" />
        <SkeletonRows className="p-6" />
      </div>
    );

  const { machine, latest, history } = detail.data;
  const rendered = latest?.rendered ?? null;

  const doDelete = () =>
    remove
      .mutateAsync(machine.id)
      .then(() => {
        toast.success('Machine removed', 'All of its reports were deleted.');
        nav(BASE);
      })
      .catch((e) => toast.error('Could not remove', e instanceof Error ? e.message : undefined));

  return (
    <div className={page}>
      <PageHeader
        title={machine.label}
        description={describeHardware(machine) || 'Hardware details arrive with the first report.'}
        actions={
          <Button
            variant="ghost"
            size="sm"
            icon={<TrashIcon className="size-3.5" />}
            onClick={() => setConfirmDelete(true)}
          >
            Remove
          </Button>
        }
      >
        <div className="flex flex-wrap items-center gap-2 text-xs text-ink-3">
          <HealthChip score={machine.lastHealth} />
          {machine.chassis && (
            <Badge tone="outline" size="xs">
              {machine.chassis.replace('_', ' ')}
            </Badge>
          )}
          {machine.gpu && <span>GPU {machine.gpu}</span>}
          <span>
            Last report {machine.lastSeen ? relativeTime(machine.lastSeen) : 'never'}
            {machine.innardsVersion ? ` · Innards ${machine.innardsVersion}` : ''}
          </span>
          {machine.stale && (
            <Badge tone="outline" size="xs">
              stale
            </Badge>
          )}
          <span className="font-mono">{machine.id}</span>
        </div>
      </PageHeader>

      <div className="flex flex-col gap-4 p-4 md:p-6">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <KpiTile
            label="Health"
            value={machine.lastHealth ?? '—'}
            rawValue={machine.lastHealth ?? undefined}
            unknown={machine.lastHealth == null}
            knowledge="calculated"
            hint="Innards weighs every finding by severity; 100 is nothing to report."
          />
          <KpiTile
            label="Critical"
            value={machine.criticalCount}
            rawValue={machine.criticalCount}
            hint="Something is wrong or at risk; act soon."
          />
          <KpiTile
            label="Warnings"
            value={machine.warningCount}
            rawValue={machine.warningCount}
            hint="Costs performance or reliability; worth fixing."
          />
          <KpiTile
            label="Reports"
            value={reports.data?.total ?? history.length}
            rawValue={reports.data?.total ?? history.length}
            trend={`first seen ${relativeTime(machine.firstSeen)}`}
          />
        </div>

        <div className="grid gap-4 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
          <div className="flex flex-col gap-4">
            {rendered ? (
              <Section
                title="Latest report"
                description={
                  <>
                    {latest ? formatDateTime(latest.receivedAt) : ''} · {rendered.lang} ·{' '}
                    {rendered.level}
                  </>
                }
              >
                <p className="mb-3 text-sm text-ink">{rendered.verdict}</p>
                <Findings findings={rendered.findings} />
              </Section>
            ) : (
              <Section title="Latest report">
                <p className="text-sm text-ink-3">
                  No report yet. Once the Innards app on this machine uploads one, it shows here.
                </p>
              </Section>
            )}
          </div>

          <div className="flex flex-col gap-4">
            <Section
              title="Health over time"
              description={
                history.length >= 2
                  ? `${history.length} reports, ${formatDateTime(history[0]?.received_at)} → now`
                  : 'One report so far; the trend starts with the second.'
              }
            >
              <HealthSparkline history={history} width={320} height={64} className="max-w-full" />
              <p className="mt-2 text-xs text-ink-3">
                Guides at 50 / 75 / 90: critical · needs attention · OK · good.
              </p>
            </Section>

            <OwnerSection machine={machine} />

            {rendered && rendered.capabilities.length > 0 && (
              <Section
                title="What it is still good for"
                description="Capability score per workload, 0–100."
              >
                <Bars
                  data={rendered.capabilities.map((c) => ({
                    label: c.label,
                    value: c.score,
                    tone: 'accent',
                  }))}
                  format={(n) => `${n}`}
                />
                <ul className="mt-3 flex flex-col gap-1 text-xs text-ink-3">
                  {rendered.capabilities
                    .filter((c) => c.limits.length)
                    .slice(0, 4)
                    .map((c) => (
                      <li key={c.workload}>
                        <span className="text-ink-2">{c.label}:</span> {c.grade_label} —{' '}
                        {c.limits.join(', ')}
                      </li>
                    ))}
                </ul>
              </Section>
            )}

            <SummarySection rendered={rendered} machine={machine} />
          </div>
        </div>

        {reports.data && reports.data.items.length > 1 && (
          <Section title="Report history" description="Newest first. The latest is always kept.">
            <table className="w-full text-sm">
              <thead className="text-left text-xs text-ink-3">
                <tr>
                  <th className="py-2 pr-3 font-medium">Received</th>
                  <th className="py-2 pr-3 font-medium">Health</th>
                  <th className="py-2 pr-3 text-right font-medium">Critical</th>
                  <th className="py-2 pr-3 text-right font-medium">Warnings</th>
                  <th className="py-2 font-medium">Innards</th>
                </tr>
              </thead>
              <tbody>
                {reports.data.items.map((r) => (
                  <tr key={r.id} className="border-t border-line">
                    <td className="py-2 pr-3 text-ink-2">{formatDateTime(r.receivedAt)}</td>
                    <td className="py-2 pr-3">
                      <HealthChip score={r.healthScore} size="xs" />
                    </td>
                    <td className="py-2 pr-3 text-right tabular-nums">{r.criticalCount}</td>
                    <td className="py-2 pr-3 text-right tabular-nums">{r.warningCount}</td>
                    <td className="py-2 font-mono text-xs text-ink-3">{r.innardsVersion}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Section>
        )}
      </div>

      <Dialog
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        title="Remove this machine?"
        description="Every report it uploaded is deleted. The Innards app can re-register it by uploading again."
        tone="danger"
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              Keep
            </Button>
            <Button variant="danger" onClick={() => void doDelete()} loading={remove.isPending}>
              Remove
            </Button>
          </>
        }
      />
    </div>
  );
}

function Findings({ findings }: { findings: RenderedFinding[] }) {
  const groups = SEVERITY_ORDER.map((sev) => ({
    sev,
    items: findings.filter((f) => f.severity === sev),
  })).filter((g) => g.items.length);
  if (!groups.length) return <p className="text-sm text-ink-3">Nothing to report.</p>;
  return (
    <div className="flex flex-col gap-4">
      {groups.map(({ sev, items }) => (
        <div key={sev}>
          <div className="mb-1.5 flex items-center gap-2">
            <Badge tone={SEVERITY_TONE[sev]} size="xs">
              {SEVERITY_LABEL[sev]}
            </Badge>
            <span className="text-xs text-ink-3">{items.length}</span>
          </div>
          <ul className="divide-y divide-line rounded-sm border border-line">
            {items.map((f, i) => (
              <li key={`${f.id}-${i}`} className="flex flex-col gap-0.5 px-3 py-2">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-ink">{f.title}</span>
                  <span className="text-xs text-ink-3">{f.category}</span>
                </div>
                <p className="text-sm text-ink-2">{f.body}</p>
                {f.action && (
                  <p className="text-sm text-ink">
                    <span className="text-ink-3">Do: </span>
                    {f.action}
                  </p>
                )}
                {f.evidence.length > 0 && (
                  <ul className="mt-1 flex flex-col gap-0.5">
                    {f.evidence.map((e) => (
                      <li key={e} className="font-mono text-xs text-ink-3">
                        {e}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

function SummarySection({
  rendered,
  machine,
}: {
  rendered: RenderedReport | null;
  machine: MachineSummary;
}) {
  const rows: Array<[string, string | null]> = rendered
    ? [
        ['Machine', rendered.summary.machine],
        ['CPU', rendered.summary.cpu],
        ['Memory', rendered.summary.memory],
        ['Storage', rendered.summary.storage],
        ['GPU', rendered.summary.gpu],
        ['OS', rendered.summary.os],
        ['Battery', rendered.summary.battery],
      ]
    : [
        ['CPU', machine.cpu],
        ['Memory', machine.ramGb != null ? `${machine.ramGb} GB` : null],
        ['Storage', machine.storage],
        ['GPU', machine.gpu],
        ['OS', machine.os],
      ];
  const shown = rows.filter(([, v]) => v);
  if (!shown.length) return null;
  return (
    <Section
      title="Hardware"
      actions={<KnowledgeBadge knowledge="observed" size="xs" />}
      description="As the machine reported it. Hostnames, serial numbers, MAC addresses and process lists are never stored."
    >
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
        {shown.map(([k, v]) => (
          <div key={k} className="contents">
            <dt className="text-ink-3">{k}</dt>
            <dd className="min-w-0 break-words text-ink">{v}</dd>
          </div>
        ))}
      </dl>
      {rendered && rendered.notes.length > 0 && (
        <ul className="mt-3 flex flex-col gap-0.5 text-xs text-ink-3">
          {rendered.notes.map((n) => (
            <li key={n}>{n}</li>
          ))}
        </ul>
      )}
    </Section>
  );
}

/** Owner picker on the shell's global search (people and organisations), plus the label. */
function OwnerSection({ machine }: { machine: MachineSummary }) {
  const { patch } = useFleetMutations();
  const [q, setQ] = useState('');
  const [label, setLabel] = useState(machine.label);
  const search = useSearch(q);
  const options = useMemo<ComboboxOption[]>(
    () =>
      (search.data ?? [])
        .filter((h) => h.kind === 'person' || h.kind === 'organisation')
        .slice(0, 12)
        .map((h) => ({
          value: `${h.kind}:${h.id}`,
          label: h.title,
          description: h.subtitle ?? (h.kind === 'person' ? 'Person' : 'Organisation'),
        })),
    [search.data],
  );
  const current = machine.owner ? `${machine.owner.kind}:${machine.owner.id}` : null;

  const save = (p: MachinePatch, done: string) =>
    patch
      .mutateAsync({ id: machine.id, patch: p })
      .then(() => toast.success(done))
      .catch((e) => toast.error('Could not save', e instanceof Error ? e.message : undefined));

  const setOwner = (value: string | null) => {
    if (!value) return void save({ ownerPersonId: null, ownerOrgId: null }, 'Owner cleared');
    const [kind, id] = value.split(':') as ['person' | 'organisation', string];
    return void save(
      kind === 'person' ? { ownerPersonId: id } : { ownerOrgId: id },
      'Owner assigned',
    );
  };

  return (
    <Section
      title="Owner"
      description="Who this machine belongs to. It then shows on their record under Machines."
    >
      <div className="flex flex-col gap-3">
        <Combobox
          label="Person or organisation"
          value={current}
          valueLabel={machine.owner?.name ?? null}
          options={options}
          onQueryChange={setQ}
          loading={search.isFetching}
          onChange={(v) => setOwner(v)}
          placeholder="Search people and organisations…"
          emptyText={q ? 'No matches' : 'Type to search'}
          clearable
          size="sm"
          disabled={patch.isPending}
        />
        {machine.owner && (
          <Link
            to={`/relationships/${machine.owner.kind === 'person' ? 'people' : 'organisations'}/${machine.owner.id}`}
            className="text-xs text-accent-ink hover:underline"
          >
            Open {machine.owner.name}
          </Link>
        )}
        <form
          className="flex items-end gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            const next = label.trim();
            if (next && next !== machine.label) void save({ label: next }, 'Label saved');
          }}
        >
          <Input
            label="Label"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            hint="Shown in the fleet list instead of the model name."
            wrapperClassName="flex-1"
          />
          <Button
            type="submit"
            variant="secondary"
            size="sm"
            disabled={!label.trim() || label.trim() === machine.label}
            loading={patch.isPending}
          >
            Save
          </Button>
        </form>
      </div>
    </Section>
  );
}
