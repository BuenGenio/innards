// Pieces shared by the fleet pages and the record panel: the health chip, severity
// tones, the health-history sparkline and the icons. Icons are inline SVG rather
// than lucide-react imports because this module lives outside the Threadwise tree
// and only the packages Vite dedupes (react, react-router, react-query) resolve
// from here.
import { Badge, type BadgeTone } from '@/ui/Badge';
import type { HealthBucket, HistoryPoint, Severity } from './api';

export const BUCKET_LABEL: Record<HealthBucket, string> = {
  critical: 'Critical',
  warning: 'Needs attention',
  ok: 'OK',
  good: 'Good',
};

export const BUCKET_TONE: Record<HealthBucket, BadgeTone> = {
  critical: 'danger',
  warning: 'warning',
  ok: 'neutral',
  good: 'success',
};

export const SEVERITY_LABEL: Record<Severity, string> = {
  critical: 'Needs attention now',
  warning: 'Worth fixing',
  info: 'Good to know',
  good: 'Working well',
};

export const SEVERITY_TONE: Record<Severity, BadgeTone> = {
  critical: 'danger',
  warning: 'warning',
  info: 'info',
  good: 'success',
};

export const SEVERITY_ORDER: Severity[] = ['critical', 'warning', 'info', 'good'];

export function bucketOf(score: number | null | undefined): HealthBucket | null {
  if (score == null) return null;
  if (score >= 90) return 'good';
  if (score >= 75) return 'ok';
  if (score >= 50) return 'warning';
  return 'critical';
}

/** Health score as a status chip: number plus a word, never colour alone. */
export function HealthChip({
  score,
  size = 'sm',
}: {
  score: number | null | undefined;
  size?: 'xs' | 'sm';
}) {
  const bucket = bucketOf(score);
  if (score == null || !bucket)
    return (
      <Badge tone="outline" size={size}>
        No report
      </Badge>
    );
  return (
    <Badge tone={BUCKET_TONE[bucket]} size={size} title={`Health ${score} of 100`}>
      <span className="tabular-nums">{score}</span>
      <span className="opacity-80">· {BUCKET_LABEL[bucket]}</span>
    </Badge>
  );
}

const iconProps = {
  xmlns: 'http://www.w3.org/2000/svg',
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 2,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
  'aria-hidden': true,
  focusable: 'false' as const,
};

/** lucide "monitor" */
export function MonitorIcon({ className }: { className?: string }) {
  return (
    <svg {...iconProps} className={className}>
      <title>Monitor</title>
      <rect width="20" height="14" x="2" y="3" rx="2" />
      <line x1="8" x2="16" y1="21" y2="21" />
      <line x1="12" x2="12" y1="17" y2="21" />
    </svg>
  );
}

/** lucide "trash-2" */
export function TrashIcon({ className }: { className?: string }) {
  return (
    <svg {...iconProps} className={className}>
      <title>Remove</title>
      <path d="M3 6h18" />
      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
      <line x1="10" x2="10" y1="11" y2="17" />
      <line x1="14" x2="14" y1="11" y2="17" />
    </svg>
  );
}

/**
 * Health over time on a fixed 0–100 scale (autoscaling would turn a 92→90 wobble into a
 * cliff). One series, so no legend: the title names it. Each point carries its own
 * tooltip via <title>; the last point is marked and the 50/75/90 bucket edges are drawn
 * as recessive guides.
 */
export function HealthSparkline({
  history,
  width = 240,
  height = 56,
  className,
}: {
  history: HistoryPoint[];
  width?: number;
  height?: number;
  className?: string;
}) {
  const pad = 4;
  const innerW = width - pad * 2;
  const innerH = height - pad * 2;
  const n = history.length;
  const x = (i: number) => pad + (n <= 1 ? innerW / 2 : (i / (n - 1)) * innerW);
  const y = (v: number) => pad + innerH - (Math.max(0, Math.min(100, v)) / 100) * innerH;
  const last = history[n - 1];
  const points = history.map((h, i) => `${x(i).toFixed(1)},${y(h.health_score).toFixed(1)}`);
  const label =
    n === 0
      ? 'No health history yet'
      : `Health over ${n} report${n === 1 ? '' : 's'}, latest ${last?.health_score ?? '—'}`;
  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className={className}
      role="img"
      aria-label={label}
    >
      <title>{label}</title>
      {[50, 75, 90].map((g) => (
        <line
          key={g}
          x1={pad}
          x2={width - pad}
          y1={y(g)}
          y2={y(g)}
          stroke="var(--color-line)"
          strokeWidth={1}
          strokeDasharray="2 3"
        />
      ))}
      {n >= 2 && (
        <polyline
          fill="none"
          stroke="var(--color-accent)"
          strokeWidth={2}
          strokeLinejoin="round"
          strokeLinecap="round"
          points={points.join(' ')}
        />
      )}
      {history.map((h, i) => (
        <circle
          key={h.reportId}
          cx={x(i)}
          cy={y(h.health_score)}
          r={i === n - 1 ? 4 : 3}
          fill={i === n - 1 ? 'var(--color-accent)' : 'var(--color-surface)'}
          stroke="var(--color-accent)"
          strokeWidth={2}
        >
          <title>
            {`${new Date(h.received_at).toLocaleString()}: health ${h.health_score}, ${h.critical_count} critical, ${h.warning_count} warning`}
          </title>
        </circle>
      ))}
    </svg>
  );
}

export function describeHardware(m: {
  vendor: string | null;
  product: string | null;
  cpu: string | null;
  ramGb: number | null;
  storage: string | null;
  os: string | null;
}): string {
  const model = [m.vendor, m.product].filter(Boolean).join(' ');
  return [model, m.cpu, m.ramGb != null ? `${m.ramGb} GB` : null, m.storage, m.os]
    .filter(Boolean)
    .join(' · ');
}
