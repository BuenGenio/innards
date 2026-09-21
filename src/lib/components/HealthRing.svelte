<script lang="ts">
  let { score }: { score: number } = $props();
  const r = 44;
  const c = 2 * Math.PI * r;
  const offset = $derived(c * (1 - score / 100));
  const color = $derived(score >= 80 ? "var(--good)" : score >= 55 ? "var(--warn)" : "var(--crit)");
</script>

<div class="ring" style="--c: {color}">
  <svg width="112" height="112" viewBox="0 0 112 112">
    <circle cx="56" cy="56" r={r} stroke="var(--line)" stroke-width="9" fill="none" />
    <circle cx="56" cy="56" r={r} stroke="var(--c)" stroke-width="9" fill="none" stroke-linecap="round"
      stroke-dasharray={c} stroke-dashoffset={offset} transform="rotate(-90 56 56)" class="arc" />
  </svg>
  <div class="num"><span>{score}</span><small>/100</small></div>
</div>

<style>
  .ring { position: relative; width: 112px; height: 112px; }
  .num { position: absolute; inset: 0; display: flex; align-items: baseline; justify-content: center; gap: 2px; padding-top: 34px; }
  .num span { font-size: 30px; font-weight: 700; letter-spacing: -0.03em; color: var(--c); }
  .num small { color: var(--ink-3); font-size: 12px; }
  .arc { transition: stroke-dashoffset 0.8s cubic-bezier(0.2, 0.7, 0.2, 1); }
</style>
