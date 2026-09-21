<script lang="ts">
  // Supporter: this machine over time. One sparkline per metric, newest right.
  import { app } from "$lib/state.svelte";
  import { api } from "$lib/api";
  import type { HistoryEntry } from "$lib/types";
  import Paywall from "./Paywall.svelte";

  let entries = $state<HistoryEntry[] | null>(null);
  let err = $state<string | null>(null);

  $effect(() => {
    if (!app.has("supporter") || app.otherMachine) return;
    api.history(365).then((h) => (entries = h)).catch((e) => (err = String(e)));
  });

  const metrics = $derived([
    { key: "health", label: app.u("history.health"), unit: "/100", get: (e: HistoryEntry) => e.health, max: 100 },
    { key: "battery", label: app.u("history.battery"), unit: "%", get: (e: HistoryEntry) => e.battery_health, max: 100 },
    { key: "root", label: app.u("history.disk_free"), unit: "%", get: (e: HistoryEntry) => e.root_free_pct, max: 100 },
    { key: "mem", label: app.u("history.ram_free"), unit: "%", get: (e: HistoryEntry) => e.memory_available_pct, max: 100 },
    { key: "cpu", label: "CPU", unit: "°C", get: (e: HistoryEntry) => e.cpu_c, max: 100 },
  ]);

  // Each sparkline is scaled to its own range (with a little headroom) so a
  // 3-point drift is visible; the numbers beside it carry the absolute values.
  function path(vals: (number | null)[], _max: number, w = 220, h = 34): string {
    const nums = vals.filter((v): v is number => v != null);
    const lo0 = Math.min(...nums), hi0 = Math.max(...nums);
    const pad = Math.max(1, (hi0 - lo0) * 0.15);
    const lo = lo0 - pad, hi = hi0 + pad;
    const pts = vals.map((v, i) => (v == null ? null : [vals.length > 1 ? (i / (vals.length - 1)) * w : w / 2, h - ((v - lo) / (hi - lo)) * (h - 4) - 2]));
    let d = "";
    for (const p of pts) {
      if (!p) continue;
      d += (d ? " L" : "M") + p[0].toFixed(1) + " " + p[1].toFixed(1);
    }
    return d;
  }
  const last = (vals: (number | null)[]) => [...vals].reverse().find((v) => v != null);
  const first = (vals: (number | null)[]) => vals.find((v) => v != null);
  const span = $derived(entries && entries.length > 1 ? Math.round((new Date(entries[entries.length - 1].at).getTime() - new Date(entries[0].at).getTime()) / 86_400_000) : 0);
</script>

<section class="card hist">
  <div class="head">
    <h2>{app.u("history.title")}</h2>
    <span class="pill accent">Supporter</span>
  </div>
  {#if !app.has("supporter")}
    <Paywall tier="supporter" compact />
  {:else if app.otherMachine}
    <p class="muted small">{app.u("history.other_machine")}</p>
  {:else if err}
    <p class="small" style="color: var(--crit)">{err}</p>
  {:else if !entries || entries.length < 2}
    <p class="muted small">{app.u("history.empty")}</p>
  {:else}
    <p class="faint small">{entries.length} {app.u("history.scans_over")} {span} {app.u("history.days")}</p>
    <ul>
      {#each metrics as m}
        {@const vals = entries.map(m.get)}
        {#if vals.some((v) => v != null)}
          <li>
            <div class="row"><span>{m.label}</span><span class="val">{last(vals)?.toFixed(0)}{m.unit} <span class="faint">({first(vals)?.toFixed(0)}{m.unit} →)</span></span></div>
            <svg viewBox="0 0 220 34" preserveAspectRatio="none" aria-hidden="true"><path d={path(vals, m.max)} fill="none" stroke="var(--accent)" stroke-width="1.8" stroke-linejoin="round" /></svg>
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</section>

<style>
  .hist { padding: 16px 18px; display: grid; gap: 8px; }
  .head { display: flex; align-items: center; justify-content: space-between; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  li { min-width: 0; }
  .row { display: flex; justify-content: space-between; gap: 8px; font-size: 13px; min-width: 0; }
  .row > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .val { font-variant-numeric: tabular-nums; }
  svg { width: 100%; height: 34px; display: block; }
</style>
