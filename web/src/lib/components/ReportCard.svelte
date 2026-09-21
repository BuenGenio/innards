<script lang="ts">
  import { healthScore, verdictKey, type Machine } from '$lib/data/machines';
  import { t, tf } from '$lib/engine/i18n';
  let { machine, compact = false, wide = false }: { machine: Machine; compact?: boolean; wide?: boolean } = $props();
  const health = $derived(healthScore(machine.findings));
  const v = $derived(verdictKey(machine.findings));
  const verdict = $derived(tf(v.key, { critical: v.critical, warnings: v.warnings }));
  const specs = $derived([
    ['ui/cpu', machine.summary.cpu],
    ['ui/memory', machine.summary.memory],
    ['ui/storage', machine.summary.storage],
    ['ui/gpu', machine.summary.gpu],
    ['ui/os', machine.summary.os],
    ['ui/battery', machine.summary.battery ?? '']
  ].filter(([, v]) => v));
  const R = 22, C = 2 * Math.PI * R;
  const tone = $derived(health >= 80 ? 'good' : health >= 55 ? 'warn' : 'crit');
</script>

<!-- A native-looking app window. Chrome variant comes from <html data-os>, set before paint in app.html. -->
<article class="window" class:wide aria-label="Example innards report">
  <div class="titlebar" aria-hidden="true">
    <span class="lights"><i class="r"></i><i class="y"></i><i class="g"></i></span>
    <span class="title">
      <svg viewBox="0 0 24 24" width="14" height="14"><rect x="1.5" y="1.5" width="21" height="21" rx="6" fill="none" stroke="currentColor" stroke-width="2.4" /><rect x="6" y="7" width="12" height="2.6" rx="1.3" fill="var(--accent)" /><rect x="6" y="11.7" width="9" height="2.6" rx="1.3" fill="currentColor" /><rect x="6" y="16.4" width="6" height="2.6" rx="1.3" fill="currentColor" /></svg>
      <span>innards</span><span class="sep">—</span><span class="doc">{t('ui/report_title')}</span>
    </span>
    <span class="caption">
      <i class="min"><svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 5h10" stroke="currentColor" stroke-width="1" /></svg></i>
      <i class="max"><svg viewBox="0 0 10 10" width="10" height="10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1" /></svg></i>
      <i class="close"><svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1" /></svg></i>
    </span>
    <span class="gnome-close"><svg viewBox="0 0 10 10" width="9" height="9"><path d="M1 1l8 8M9 1L1 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg></span>
  </div>

  <div class="report">
    <header>
      <h3>{machine.summary.machine}</h3>
      <div class="score tone-{tone}" title="Health score">
        <svg viewBox="0 0 52 52" width="52" height="52" aria-hidden="true">
          <circle cx="26" cy="26" r={R} fill="none" stroke="var(--line)" stroke-width="4.5" />
          <circle cx="26" cy="26" r={R} fill="none" stroke="var(--c)" stroke-width="4.5" stroke-linecap="round"
            stroke-dasharray="{C}" stroke-dashoffset="{C * (1 - health / 100)}" transform="rotate(-90 26 26)" />
        </svg>
        <span class="n">{health}</span>
      </div>
    </header>

    <dl class="specs">
      {#each specs as [k, val]}
        <div><dt>{t(k)}</dt><dd>{val}</dd></div>
      {/each}
    </dl>

    <p class="verdict"><strong>{t('ui/verdict')}</strong> {verdict}</p>

    {#if !compact}
      <ul class="findings">
        {#each machine.findings as f}
          <li class="sev-{f.severity}"><i></i><span>{tf(`findings/${f.id}/title`, f.params)}</span></li>
        {/each}
      </ul>
    {/if}
    <p class="cap muted">{machine.caption}</p>
  </div>
</article>

<style>
  /* ---- window shell ---- */
  .window { --chrome: var(--bg-sunk); --chrome-ink: var(--ink-2); --chrome-h: 38px; --win-radius: 12px;
    background: var(--bg-elev); border: 1px solid var(--line-2); border-radius: var(--win-radius); box-shadow: var(--shadow), 0 24px 60px -24px rgb(0 0 0 / 0.35); overflow: hidden; }
  .titlebar { position: relative; height: var(--chrome-h); display: flex; align-items: center; background: var(--chrome); color: var(--chrome-ink); border-bottom: 1px solid var(--line); user-select: none; font-size: 0.8rem; }
  .title { display: inline-flex; align-items: center; gap: 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .title .sep { opacity: 0.5; }
  .lights, .caption, .gnome-close { display: none; }

  /* macOS: traffic lights left, centred title */
  .lights { position: absolute; left: 12px; top: 0; bottom: 0; align-items: center; gap: 8px; }
  .lights i { width: 12px; height: 12px; border-radius: 50%; border: 1px solid rgb(0 0 0 / 0.12); }
  .lights .r { background: #ff5f57; } .lights .y { background: #febc2e; } .lights .g { background: #28c840; }
  :global(html:not([data-os])) .lights, :global(html[data-os='mac']) .lights { display: flex; }
  :global(html:not([data-os])) .title, :global(html[data-os='mac']) .title { margin: 0 auto; padding: 0 80px; font-weight: 600; font-size: 0.8rem; }
  :global(html:not([data-os])) .title svg, :global(html[data-os='mac']) .title svg { display: none; }

  /* Windows 11: icon + title left, caption buttons right, squarer corners */
  :global(html[data-os='windows']) .window { --win-radius: 8px; --chrome-h: 34px; --chrome: var(--bg-elev); border-color: var(--line-2); box-shadow: var(--shadow), 0 20px 50px -20px rgb(0 0 0 / 0.4); }
  :global(html[data-os='windows']) .titlebar { border-bottom-color: var(--line); }
  :global(html[data-os='windows']) .title { padding-left: 12px; gap: 8px; font-size: 0.76rem; }
  :global(html[data-os='windows']) .title .sep, :global(html[data-os='windows']) .title .doc { display: none; }
  :global(html[data-os='windows']) .caption { display: flex; margin-left: auto; height: 100%; }
  .caption i { display: grid; place-items: center; width: 46px; height: 100%; color: var(--chrome-ink); }
  .caption i.close:hover { background: #c42b1c; color: #fff; }
  .caption i:not(.close):hover { background: rgb(128 128 128 / 0.15); }

  /* GNOME / Adwaita: taller rounded header bar, bold centred title, circular close */
  :global(html[data-os='linux']) .window { --chrome-h: 46px; --win-radius: 12px; --chrome: var(--bg-sunk); }
  :global(html[data-os='linux']) .titlebar { box-shadow: inset 0 -1px 0 var(--line); }
  :global(html[data-os='linux']) .title { margin: 0 auto; padding: 0 56px; font-weight: 700; font-size: 0.86rem; color: var(--ink); }
  :global(html[data-os='linux']) .title svg { display: none; }
  :global(html[data-os='linux']) .gnome-close { display: grid; place-items: center; position: absolute; right: 10px; top: 50%; transform: translateY(-50%); width: 24px; height: 24px; border-radius: 50%; background: rgb(128 128 128 / 0.18); color: var(--ink); }
  :global(html[data-os='linux']) .gnome-close:hover { background: rgb(128 128 128 / 0.3); }

  /* ---- report body ---- */
  .report { padding: clamp(16px, 2.4vw, 24px); display: grid; grid-template-columns: minmax(0, 1fr); gap: 14px; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; }
  header h3 { font-size: 1.2rem; align-self: center; text-wrap: balance; }
  .score { position: relative; flex: none; width: 52px; height: 52px; display: grid; place-items: center; }
  .score svg { position: absolute; inset: 0; }
  .score .n { font-family: var(--font-display); font-weight: 800; font-size: 1.05rem; letter-spacing: -0.03em; }
  .tone-good { --c: var(--good); } .tone-warn { --c: var(--warn); } .tone-crit { --c: var(--crit); }
  .specs { margin: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 8px 16px; font-size: 0.88rem; }
  @media (min-width: 720px) { .wide .specs { grid-template-columns: repeat(3, 1fr); } }
  @media (min-width: 1000px) { .wide .specs { grid-template-columns: 2.3fr repeat(5, 1fr); } }
  .specs div { min-width: 0; }
  dt { color: var(--ink-3); font-size: 0.74rem; text-transform: uppercase; letter-spacing: 0.06em; font-family: var(--font-mono); }
  dd { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .verdict { font-size: 0.95rem; border-top: 1px solid var(--line); padding-top: 12px; text-wrap: pretty; }
  @media (min-width: 720px) { .wide .verdict { font-size: 1.02rem; } }
  .findings { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: minmax(0, 1fr); gap: 5px 14px; font-size: 0.86rem; }
  @media (min-width: 480px) { .findings { grid-template-columns: 1fr 1fr; } }
  @media (min-width: 960px) { .wide .findings { grid-template-columns: repeat(4, 1fr); } }
  .findings li { display: flex; gap: 8px; align-items: baseline; min-width: 0; }
  .findings li span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .findings i { flex: none; width: 8px; height: 8px; border-radius: 50%; background: var(--c); transform: translateY(-1px); }
  .cap { font-size: 0.76rem; }
</style>
