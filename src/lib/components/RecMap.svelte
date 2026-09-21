<script lang="ts">
  // Impact-vs-cost map: each recommendation is a bubble. X = cost (free
  // pinned at the left, then log scale), Y = impact. Kind sets the colour.
  // Hovering / focusing a bubble shows its card; click opens the shop links.
  import { app } from "$lib/state.svelte";
  import type { RenderedRecommendation } from "$lib/types";

  let { recs, onselect }: { recs: RenderedRecommendation[]; onselect?: (r: RenderedRecommendation) => void } = $props();

  const W = 760, H = 340, PAD = { l: 118, r: 28, t: 20, b: 44 };
  const MAXC = 3000;

  function costOf(r: RenderedRecommendation): number {
    // "$40–$120" -> 80 (midpoint); "Free"/"Gratis" -> 0
    const m = r.cost.match(/\$(\d+)[^\d]+\$(\d+)/);
    return m ? (Number(m[1]) + Number(m[2])) / 2 : 0;
  }
  function x(c: number) {
    const x0 = PAD.l + 70; // first paid tick
    if (c <= 0) return PAD.l + 14; // free column
    const t = Math.log10(Math.max(c, 10) / 10) / Math.log10(MAXC / 10);
    return x0 + t * (W - PAD.r - x0);
  }
  const impactIdx = (r: RenderedRecommendation) => (r.impact === app.t("impacts/high") ? 2 : r.impact === app.t("impacts/medium") ? 1 : 0);
  const y = (i: number) => H - PAD.b - (i + 0.5) * ((H - PAD.t - PAD.b) / 3);
  const kindTone = (r: RenderedRecommendation) =>
    r.kind === app.t("rec_kinds/software") ? "var(--good)" : r.kind === app.t("rec_kinds/replace") ? "var(--crit)" : r.kind === app.t("rec_kinds/keep") ? "var(--ink-3)" : "var(--accent)";

  // Bubbles that land in the same cell are stacked vertically (0, -26, +26, -52, …).
  const placed = $derived.by(() => {
    const cells = new Map<string, number>();
    return recs.map((r) => {
      const c = costOf(r), i = impactIdx(r);
      const key = `${Math.round(x(c) / 60)}:${i}`;
      const n = cells.get(key) ?? 0;
      cells.set(key, n + 1);
      const dy = n === 0 ? 0 : (n % 2 ? -1 : 1) * 34 * Math.ceil(n / 2);
      const cx = x(c);
      // Labels sit beside the bubble in the free column or when stacked; below it otherwise.
      const beside = cx < PAD.l + 90 || n > 0 || cells.get(key)! > 1;
      const anchor = beside ? "start" : cx > W - PAD.r - 90 ? "end" : "middle";
      const lx = beside ? cx + 20 : anchor === "end" ? cx + 16 : cx;
      const ly = beside ? y(i) + dy + 4 : y(i) + dy + 30;
      return { r, cx, cy: y(i) + dy, tone: kindTone(r), anchor, lx, ly };
    });
  });

  let hover = $state<RenderedRecommendation | null>(null);
  const ticks = [0, 25, 100, 300, 1000, 3000];
  const yLabels = $derived([app.t("impacts/low"), app.t("impacts/medium"), app.t("impacts/high")]);
</script>

<div class="map card">
  <svg viewBox="0 0 {W} {H}" role="img" aria-label={app.u("recmap.aria")}>
    <!-- bands -->
    {#each [0, 1, 2] as i}
      <rect x={PAD.l - 8} y={y(i) - (H - PAD.t - PAD.b) / 6} width={W - PAD.l - PAD.r + 8} height={(H - PAD.t - PAD.b) / 3} fill={i === 2 ? "var(--good-soft)" : i === 1 ? "var(--paper-3)" : "transparent"} opacity="0.55" rx="6" />
      <text x={PAD.l - 14} y={y(i) + 4} text-anchor="end" class="axis">{yLabels[i]}</text>
    {/each}
    <!-- x axis -->
    <line x1={PAD.l - 8} y1={H - PAD.b + 6} x2={W - PAD.r} y2={H - PAD.b + 6} stroke="var(--line-2)" />
    {#each ticks as t}
      <text x={x(t)} y={H - PAD.b + 24} text-anchor="middle" class="axis">{t === 0 ? app.t("ui/free") : `$${t}`}</text>
    {/each}
    <text x={W - PAD.r} y={H - 4} text-anchor="end" class="axis faint">{app.u("recmap.cost_axis")}</text>
    <!-- budget line -->
    {#if recs.some((r) => r.over_budget)}
      {@const firstOver = placed.filter((p) => p.r.over_budget).sort((a, b) => a.cx - b.cx)[0]}
      <line x1={firstOver.cx - 26} y1={PAD.t} x2={firstOver.cx - 26} y2={H - PAD.b + 6} stroke="var(--warn)" stroke-dasharray="4 4" />
      <text x={firstOver.cx - 30} y={PAD.t + 10} text-anchor="end" class="axis" fill="var(--warn)">{app.u("recmap.budget")}</text>
    {/if}
    <!-- bubbles -->
    {#each placed as p (p.r.id)}
      <g class="bubble" class:dim={hover && hover !== p.r} role="button" tabindex="0"
         onmouseenter={() => (hover = p.r)} onmouseleave={() => (hover = null)}
         onfocus={() => (hover = p.r)} onblur={() => (hover = null)}
         onclick={() => onselect?.(p.r)} onkeydown={(e) => e.key === "Enter" && onselect?.(p.r)}>
        <circle cx={p.cx} cy={p.cy} r="15" fill={p.tone} opacity={p.r.over_budget ? 0.45 : 0.9} />
        <text x={p.cx} y={p.cy + 4} text-anchor="middle" class="num">{recs.indexOf(p.r) + 1}</text>
        <text x={p.lx} y={p.ly} text-anchor={p.anchor} class="label">{p.r.title.length > 30 ? p.r.title.slice(0, 28) + "…" : p.r.title}</text>
      </g>
    {/each}
  </svg>

  <div class="legend">
    {#each ["software", "upgrade", "service", "replace", "keep"] as k}
      <span><i style="background: {k === 'software' ? 'var(--good)' : k === 'replace' ? 'var(--crit)' : k === 'keep' ? 'var(--ink-3)' : 'var(--accent)'}"></i>{app.t(`rec_kinds/${k}`)}</span>
    {/each}
  </div>

  {#if hover}
    <div class="tip fade-in">
      <strong>{recs.indexOf(hover) + 1}. {hover.title}</strong>
      <span class="muted">{hover.body}</span>
      <span class="small faint">{hover.kind} · {hover.impact} · {hover.cost}</span>
    </div>
  {/if}
</div>

<style>
  .map { padding: 16px 18px 12px; display: grid; gap: 10px; }
  svg { width: 100%; height: auto; }
  .axis { font-size: 11px; fill: var(--ink-3); font-family: var(--font); }
  .num { font-size: 12px; font-weight: 700; fill: #fff; font-family: var(--font); pointer-events: none; }
  .label { font-size: 11px; fill: var(--ink-2); font-family: var(--font); pointer-events: none; }
  .bubble { cursor: pointer; transition: opacity 0.15s; outline: none; }
  .bubble.dim { opacity: 0.35; }
  .bubble:focus-visible circle { stroke: var(--ink); stroke-width: 2; }
  .legend { display: flex; flex-wrap: wrap; gap: 14px; font-size: 12px; color: var(--ink-2); }
  .legend i { display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 5px; vertical-align: -1px; }
  .tip { display: grid; gap: 2px; padding: 10px 12px; border-radius: var(--radius-sm); background: var(--paper-3); font-size: 13px; }
</style>
