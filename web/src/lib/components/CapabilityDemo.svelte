<script lang="ts">
  import { machines } from '$lib/data/machines';
  import { score } from '$lib/engine/capability';
  import { t } from '$lib/engine/i18n';
  let which = $state(0);
  const m = $derived(machines[which]);
  const caps = $derived(score(m.snapshot));
</script>

<div class="demo card">
  <div class="top">
    <div class="seg" role="group" aria-label="Machine">
      {#each machines as mm, i}
        <button type="button" aria-pressed={which === i} onclick={() => (which = i)}>{mm.label}</button>
      {/each}
    </div>
    <p class="muted spec">{m.summary.cpu.split(' · ')[1]} · {m.summary.memory} · {m.summary.gpu || 'no dedicated GPU'}</p>
  </div>

  <ol class="rows">
    {#each caps as c (c.workload)}
      <li class="grade-{c.grade}">
        <div class="l">
          <span class="name">{t(`workloads/${c.workload}`)}</span>
          {#if c.limits.length}<span class="lim muted">{c.limits.map((k) => t(`limits/${k}`)).join(', ')}</span>{/if}
        </div>
        <div class="bar" aria-hidden="true"><i style:width="{c.score}%"></i></div>
        <span class="n mono">{c.score}</span>
        <span class="g">{t(`grades/${c.grade}`)}</span>
      </li>
    {/each}
  </ol>
</div>

<style>
  .demo { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; padding: clamp(14px, 2vw, 20px); max-height: calc(100dvh - var(--header-h) - 96px); overflow: auto; }
  .top { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 14px; min-width: 0; }
  .seg { flex-wrap: wrap; }
  .spec { font-size: 0.85rem; }
  .rows { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: minmax(0, 1fr); gap: 4px 22px; min-width: 0; }
  @media (min-width: 880px) { .rows { grid-template-columns: 1fr 1fr; } }
  li { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; grid-template-areas: 'l n g' 'bar bar bar'; align-items: center; gap: 2px 10px; padding: 4px 0; border-bottom: 1px solid var(--line); min-width: 0; }
  li:last-child { border-bottom: 0; }
  @media (min-width: 880px) { li:nth-last-child(2) { border-bottom: 0; } }
  .l { grid-area: l; display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .name { font-weight: 600; font-size: 0.9rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lim { font-size: 0.76rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar { grid-area: bar; height: 5px; background: var(--bg-sunk); border-radius: 3px; overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--c); border-radius: 3px; transition: width 0.5s var(--ease); }
  .n { grid-area: n; font-weight: 600; font-size: 0.85rem; color: var(--ink-2); }
  .g { grid-area: g; font-size: 0.78rem; font-weight: 600; color: var(--c); padding: 2px 8px; border-radius: 999px; background: color-mix(in srgb, var(--c) 12%, transparent); white-space: nowrap; }
</style>
