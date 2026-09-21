<script lang="ts">
  import { x1 } from '$lib/data/machines';
  import { QUESTIONS, recommend, type Answers, type Budget, type Pain } from '$lib/engine/advisor';
  import { WORKLOADS, type Workload } from '$lib/engine/capability';
  import { t, tf } from '$lib/engine/i18n';

  let uses = $state<Workload[]>(['web_dev', 'containers']);
  let pains = $state<Pain[]>(['slow']);
  let budget = $state<Budget>('under_300');
  let portable = $state(true);
  let tab = $state<'q' | 'r'>('q'); // phones only: one panel at a time

  const answers = $derived<Answers>({ uses, pains, budget, horizon: 'now', needs_portability: portable });
  const recs = $derived(recommend(x1.snapshot, answers));
  const budgets = QUESTIONS[2].options as readonly Budget[];
  const painOpts = QUESTIONS[1].options as readonly Pain[];

  // Chip labels are shortened for space; the recommendations use the engine's full text.
  const shortNames: Record<Workload, string> = {
    everyday: 'Everyday use', web_dev: 'Web dev', containers: 'Containers', home_server: 'Home server', photo_editing: 'Photo editing',
    heavy_compile: 'Big compiles', video_editing: 'Video editing', local_llm: 'Local AI models', gaming: 'Gaming', ml_training: 'ML training'
  };
  function toggle<T>(list: T[], v: T): T[] { return list.includes(v) ? list.filter((x) => x !== v) : [...list, v]; }
  const cost = (r: (typeof recs)[number]) => (r.cost_usd[0] === 0 && r.cost_usd[1] === 0 ? t('ui/free') : `$${r.cost_usd[0]}–$${r.cost_usd[1]}`);
  const recLabel = $derived(`${recs.length} recommendation${recs.length === 1 ? '' : 's'}`);
</script>

<div class="demo card" class:show-r={tab === 'r'}>
  <div class="tabs seg" role="tablist" aria-label="Advisor panels">
    <button type="button" role="tab" aria-selected={tab === 'q'} onclick={() => (tab = 'q')}>Your answers</button>
    <button type="button" role="tab" aria-selected={tab === 'r'} onclick={() => (tab = 'r')}>{recLabel}</button>
  </div>

  <form class="q panel" onsubmit={(e) => e.preventDefault()}>
    <p class="who muted">Answers for the <strong>{x1.label}</strong> above.</p>
    <div class="pair">
      <fieldset>
        <legend>{t('advisor/uses')}</legend>
        <div class="chips">
          {#each WORKLOADS as w}
            <button type="button" class="chip" aria-pressed={uses.includes(w)} onclick={() => (uses = toggle(uses, w))} title={t(`workloads/${w}`)}>{shortNames[w]}</button>
          {/each}
        </div>
      </fieldset>
      <fieldset>
        <legend>{t('advisor/pains')}</legend>
        <div class="chips">
          {#each painOpts as p}
            <button type="button" class="chip" aria-pressed={pains.includes(p)} onclick={() => (pains = toggle(pains, p))}>{t(`advisor/options/${p}`)}</button>
          {/each}
        </div>
      </fieldset>
    </div>
    <div class="row2">
      <fieldset>
        <legend>{t('advisor/budget')}</legend>
        <div class="chips" role="radiogroup">
          {#each budgets as b}
            <button type="button" class="chip" role="radio" aria-checked={budget === b} onclick={() => (budget = b)}>{t(`advisor/options/${b}`)}</button>
          {/each}
        </div>
      </fieldset>
      <fieldset>
        <legend>{t('advisor/portability')}</legend>
        <div class="seg" role="group">
          <button type="button" aria-pressed={portable} onclick={() => (portable = true)}>{t('advisor/options/yes')}</button>
          <button type="button" aria-pressed={!portable} onclick={() => (portable = false)}>{t('advisor/options/no')}</button>
        </div>
      </fieldset>
    </div>
    <button type="button" class="btn primary see" onclick={() => (tab = 'r')}>See {recLabel} →</button>
  </form>

  <div class="out panel" aria-live="polite">
    <p class="eyebrow">{recLabel}, best first</p>
    <ol class="recs">
      {#each recs as r (r.id)}
        <li class="rec kind-{r.kind}" class:over={r.over_budget}>
          <div class="meta">
            <span class="kind">{t(`rec_kinds/${r.kind}`)}</span>
            <span class="impact">{t(`impacts/${r.impact}`)}</span>
            <span class="cost mono">{cost(r)}{#if r.over_budget} · over budget{/if}</span>
          </div>
          <h4>{tf(`recs/${r.id}/title`, r.params)}</h4>
          <p class="body">{tf(`recs/${r.id}/body`, r.params)}</p>
          <p class="why muted">{tf(`recs/${r.id}/why`, r.params)}</p>
        </li>
      {/each}
    </ol>
    <button type="button" class="btn back" onclick={() => (tab = 'q')}>← Change answers</button>
  </div>
</div>

<style>
  .demo { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; padding: clamp(14px, 2vw, 20px); max-height: calc(100dvh - var(--header-h) - 96px); overflow: auto; }
  .panel { min-width: 0; }
  .tabs { justify-self: start; }
  .tabs button { padding: 5px 12px; font-size: 0.85rem; }
  .tabs button[aria-selected='true'] { background: var(--bg-elev); color: var(--ink); box-shadow: 0 1px 2px rgb(0 0 0 / 0.12); }
  /* Phones: one panel at a time. */
  @media (max-width: 879px) {
    .demo:not(.show-r) .out { display: none; }
    .demo.show-r .q { display: none; }
  }
  @media (min-width: 880px) {
    .demo { gap: 16px; }
    .tabs, .see, .back { display: none; }
    .out { border-top: 1px solid var(--line); padding-top: 14px; }
  }
  .pair { display: grid; grid-template-columns: minmax(0, 1fr); gap: 10px; }
  @media (min-width: 720px) { .pair { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 12px 22px; } }
  .q { display: grid; gap: 10px; align-content: start; }
  .who { font-size: 0.85rem; }
  fieldset { border: 0; margin: 0; padding: 0; display: grid; gap: 6px; min-width: 0; }
  legend { padding: 0; font-weight: 600; font-size: 0.9rem; margin-bottom: 2px; }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; }
  .chip { font-size: 0.84rem; padding: 4px 10px; }
  .row2 { display: grid; grid-template-columns: minmax(0, 1fr); gap: 10px; }
  @media (min-width: 640px) { .row2 { grid-template-columns: auto auto; justify-content: start; gap: 12px 32px; } }
  fieldset > .seg { justify-self: start; }
  .seg button { padding: 5px 13px; font-size: 0.84rem; }
  .chip[role='radio'][aria-checked='true'] { background: var(--ink); color: var(--bg); border-color: var(--ink); }
  .see { justify-self: start; margin-top: 4px; }
  .back { justify-self: start; }

  .out { display: grid; gap: 10px; align-content: start; }
  .recs { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: minmax(0, 1fr); gap: 8px; }
  @media (min-width: 880px) { .recs { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 10px; } }
  .rec { border: 1px solid var(--line); border-left: 3px solid var(--c, var(--line-2)); border-radius: 10px; padding: 8px 11px; display: grid; gap: 3px; background: var(--bg); transition: opacity 0.2s var(--ease); min-width: 0; }
  .rec.over { opacity: 0.6; }
  .kind-software { --c: var(--good); } .kind-upgrade { --c: var(--info); } .kind-service { --c: var(--warn); } .kind-replace { --c: var(--crit); } .kind-keep { --c: var(--ink-3); }
  .meta { display: flex; flex-wrap: wrap; gap: 4px 10px; font-size: 0.76rem; align-items: center; }
  .kind { color: var(--c); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; font-family: var(--font-mono); }
  .impact, .cost { color: var(--ink-2); }
  h4 { font-size: 0.98rem; line-height: 1.25; }
  .body { font-size: 0.88rem; line-height: 1.42; text-wrap: pretty; }
  .why { font-size: 0.82rem; font-style: italic; }
</style>
