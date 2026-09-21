// Query hooks and API shapes for the innards.fleet plugin (/api/ext/innards.fleet).
// Types mirror server/index.ts; they are duplicated rather than imported so the web
// bundle never pulls the server module in.
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { extRequest, useExt } from '@/api/plugins';

export const PLUGIN_ID = 'innards.fleet';
export const BASE = `/x/${PLUGIN_ID}`;

export type Severity = 'critical' | 'warning' | 'info' | 'good';
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

export interface RenderedFinding {
  id: string;
  severity: Severity;
  category: string;
  title: string;
  body: string;
  action: string | null;
  evidence: string[];
}

export interface RenderedCapability {
  workload: string;
  label: string;
  score: number;
  grade: string;
  grade_label: string;
  limits: string[];
}

export interface RenderedReport {
  lang: string;
  level: string;
  summary: {
    machine: string;
    cpu: string;
    memory: string;
    storage: string;
    gpu: string;
    os: string;
    battery: string | null;
    health_score: number;
  };
  verdict: string;
  findings: RenderedFinding[];
  capabilities: RenderedCapability[];
  notes: string[];
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

export interface MachineDetail {
  machine: MachineSummary;
  latest: (ReportSummary & { rendered: RenderedReport | null }) | null;
  history: HistoryPoint[];
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

interface Page<T> {
  items: T[];
  total: number;
}

export const useMachines = (params: { q?: string; ownerKind?: string; ownerId?: string } = {}) =>
  useExt<Page<MachineSummary>>(PLUGIN_ID, 'machines', params);
export const useMachine = (id: string) =>
  useExt<MachineDetail>(PLUGIN_ID, `machines/${encodeURIComponent(id)}`, undefined, {
    enabled: Boolean(id),
  });
export const useMachineReports = (id: string) =>
  useExt<Page<ReportSummary>>(PLUGIN_ID, `machines/${encodeURIComponent(id)}/reports`, undefined, {
    enabled: Boolean(id),
  });
export const useAnalytics = () => useExt<Analytics>(PLUGIN_ID, 'analytics');

export interface MachinePatch {
  label?: string | null;
  ownerPersonId?: string | null;
  ownerOrgId?: string | null;
}

export function useFleetMutations() {
  const qc = useQueryClient();
  const invalidate = () => qc.invalidateQueries({ queryKey: ['plugins', PLUGIN_ID] });
  const run = <T>(fn: () => Promise<T>) =>
    fn().then((r) => {
      invalidate();
      return r;
    });
  return {
    patch: useMutation({
      mutationFn: (v: { id: string; patch: MachinePatch }) =>
        run(() =>
          extRequest<MachineSummary>(
            PLUGIN_ID,
            'PATCH',
            `machines/${encodeURIComponent(v.id)}`,
            v.patch,
          ),
        ),
    }),
    remove: useMutation({
      mutationFn: (id: string) =>
        run(() => extRequest<void>(PLUGIN_ID, 'DELETE', `machines/${encodeURIComponent(id)}`)),
    }),
  };
}
