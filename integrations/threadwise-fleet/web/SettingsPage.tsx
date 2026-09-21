// /settings/x/innards.fleet — retention and intake settings, plus what to type into the
// Innards desktop app to connect a machine. Settings are saved through the generic
// PATCH /api/plugins/:id like Settings → Plugins does.
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { usePluginMutations, usePlugins } from '@/api/plugins';
import { PageHeader, Section } from '@/features/common/PageHeader';
import { toast } from '@/stores/toasts';
import { Button } from '@/ui/Button';
import { ErrorState } from '@/ui/ErrorState';
import { Input } from '@/ui/Input';
import { SkeletonRows } from '@/ui/Skeleton';
import { Switch } from '@/ui/Switch';
import { PLUGIN_ID } from './api';

const page = 'flex min-h-0 flex-1 flex-col overflow-y-auto';

export function SettingsPage() {
  const plugins = usePlugins();
  const { patch } = usePluginMutations();
  const view = plugins.data?.find((p) => p.manifest.id === PLUGIN_ID);
  const [retention, setRetention] = useState('365');
  const [anonymous, setAnonymous] = useState(true);

  useEffect(() => {
    if (!view) return;
    setRetention(String(view.settings.retentionDays ?? 365));
    setAnonymous(view.settings.allowAnonymousMachines !== false);
  }, [view]);

  const origin = typeof window === 'undefined' ? '' : window.location.origin;
  const endpoint = `${origin}/api/ext/${PLUGIN_ID}/reports`;

  const save = () => {
    const days = Number(retention);
    if (!Number.isFinite(days) || days < 1)
      return toast.error('Retention must be a whole number of days');
    patch
      .mutateAsync({
        id: PLUGIN_ID,
        settings: { retentionDays: Math.round(days), allowAnonymousMachines: anonymous },
      })
      .then(() => toast.success('Settings saved'))
      .catch((e) => toast.error('Could not save', e instanceof Error ? e.message : undefined));
  };

  return (
    <div className={page}>
      <PageHeader
        title="Innards fleet"
        description="How long machine reports are kept and which machines may upload them."
      />
      <div className="flex max-w-3xl flex-col gap-4 p-4 md:p-6">
        {plugins.isError ? (
          <ErrorState error={plugins.error} onRetry={() => void plugins.refetch()} />
        ) : !view ? (
          <SkeletonRows />
        ) : (
          <>
            <Section
              title="Intake"
              description="Both settings apply to this workspace only."
              actions={
                <Button size="sm" onClick={save} loading={patch.isPending}>
                  Save
                </Button>
              }
            >
              <div className="flex flex-col gap-4">
                <Input
                  label="Keep reports for (days)"
                  type="number"
                  min={1}
                  value={retention}
                  onChange={(e) => setRetention(e.target.value)}
                  hint="Older reports are pruned once a day. The latest report per machine is always kept."
                  wrapperClassName="max-w-xs"
                />
                <Switch
                  label="Accept reports from unknown machines"
                  description="When off, a machine has to be registered and assigned to a person or organisation before its reports are stored."
                  checked={anonymous}
                  onChange={setAnonymous}
                />
                {!view.enabled && (
                  <p className="text-xs text-warning">
                    The plugin is disabled for this workspace; uploads are refused until it is
                    enabled under{' '}
                    <Link to="/settings/plugins" className="underline">
                      Settings → Plugins
                    </Link>
                    .
                  </p>
                )}
              </div>
            </Section>

            <Section
              title="Connect a machine"
              description="In the Innards desktop app, open Settings → Fleet and enter:"
            >
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 text-sm">
                <dt className="text-ink-3">Endpoint</dt>
                <dd className="min-w-0">
                  <code className="break-all rounded-sm bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-ink">
                    {origin || 'https://<this server>'}
                  </code>
                  <p className="mt-1 text-xs text-ink-3">
                    The app posts to <span className="font-mono">{endpoint}</span>.
                  </p>
                </dd>
                <dt className="text-ink-3">Token</dt>
                <dd className="min-w-0 text-ink-2">
                  A workspace API key (<span className="font-mono">tw_…</span>) whose role has{' '}
                  <span className="font-mono">sources:manage</span>. Create one under{' '}
                  <Link to="/settings/api-keys" className="text-accent-ink hover:underline">
                    Settings → Identity & access → API keys
                  </Link>
                  .
                </dd>
              </dl>
              <p className="mt-3 text-xs text-ink-3">
                What is stored: the structured report and its English rendering, minus hostnames,
                drive serial numbers, MAC addresses and the process list. Nothing else leaves the
                machine.
              </p>
            </Section>
          </>
        )}
      </div>
    </div>
  );
}
