<script lang="ts">
  import { app } from "$lib/state.svelte";
  import { api } from "$lib/api";
  import { openExternal as openUrl } from "$lib/platform";
  import type { Answers, Question, RenderedRecommendation, ShopLink } from "$lib/types";
  import Paywall from "./Paywall.svelte";
  import RecMap from "./RecMap.svelte";

  let questions = $state<Question[]>([]);
  let uses = $state<string[]>([]);
  let pains = $state<string[]>([]);
  let budget = $state("under_300");
  let horizon = $state("now");
  let portability = $state("no");
  let recs = $state<RenderedRecommendation[] | null>(null);
  let busy = $state(false);
  let links = $state<Record<string, ShopLink[]>>({});
  let linkErr = $state<string | null>(null);
  let view = $state<"cards" | "visual">("cards");
  let selected = $state<RenderedRecommendation | null>(null);

  $effect(() => {
    api.advisorQuestions().then((q) => {
      questions = q;
      // Design mode: ?demo=1 pre-fills answers and runs the advisor.
      if (new URLSearchParams(location.search).get("demo") === "1" && !recs) {
        uses = ["web_dev", "containers", "home_server", "local_llm"];
        pains = ["slow", "out_of_memory"];
        budget = "under_800"; horizon = "six_months"; portability = "yes";
        if (new URLSearchParams(location.search).get("view") === "visual") view = "visual";
        run();
      }
    });
  });

  function toggle(list: string[], v: string): string[] {
    return list.includes(v) ? list.filter((x) => x !== v) : [...list, v];
  }
  const optLabel = (q: Question, o: string) => (q.id === "uses" ? app.t(`workloads/${o}`) : app.t(`advisor/options/${o}`));
  const valueOf = (q: Question) => (q.id === "uses" ? uses : q.id === "pains" ? pains : q.id === "budget" ? budget : q.id === "horizon" ? horizon : portability);
  function set(q: Question, o: string) {
    if (q.id === "uses") uses = toggle(uses, o);
    else if (q.id === "pains") pains = toggle(pains, o);
    else if (q.id === "budget") budget = o;
    else if (q.id === "horizon") horizon = o;
    else portability = o;
    // Once recommendations exist, keep them in sync with the answers.
    if (recs && uses.length && app.tier !== "free") run();
  }

  async function run() {
    busy = true;
    links = {};
    const answers: Answers = { uses, pains, budget, horizon, needs_portability: portability === "yes" };
    try {
      recs = await api.advise(answers, app.lang);
    } finally {
      busy = false;
    }
  }

  async function shop(rec: RenderedRecommendation) {
    linkErr = null;
    try {
      links[rec.id] = await api.shopLinks(rec.id, app.settings.region);
    } catch (e) {
      linkErr = String(e);
    }
  }
  const condLabel = (c: ShopLink["condition"]) => app.u(`advisor.cond.${c}`);
</script>

<div class="adv fade-in">
  <section class="card intro">
    <h1>{app.u("advisor.title")}</h1>
    <p class="muted">
      {app.u("advisor.intro")}
    </p>
  </section>

  {#if !app.has("supporter")}
    <Paywall tier="supporter" />
  {/if}

  {#snippet questionCard(q: Question)}
    <div class="q card">
      <h3>{app.t(`advisor/${q.id}`)}</h3>
      <div class="opts">
        {#each q.options as o}
          {@const v = valueOf(q)}
          {@const on = Array.isArray(v) ? v.includes(o) : v === o}
          <button class="chip" class:on onclick={() => set(q, o)}>{optLabel(q, o)}</button>
        {/each}
      </div>
    </div>
  {/snippet}

  <section class="row-2">
    {#each questions.filter((q) => q.multi) as q (q.id)}
      {@render questionCard(q)}
    {/each}
  </section>

  {#if !recs}
    <button class="btn primary big" onclick={run} disabled={busy || uses.length === 0 || !app.has("supporter")}>
      {busy ? "…" : app.u("advisor.show")}
    </button>
  {/if}

  <section class="results">
    {#if recs}
      <div class="results-head">
        <h2>{app.u("advisor.recommendations")} {#if busy}<span class="faint small">…</span>{/if}</h2>
        <div class="seg" role="tablist">
          <button role="tab" aria-selected={view === "cards"} class:on={view === "cards"} onclick={() => (view = "cards")}>{app.u("advisor.view.cards")}</button>
          <button role="tab" aria-selected={view === "visual"} class:on={view === "visual"} onclick={() => (view = "visual")}>{app.u("advisor.view.visual")}</button>
        </div>
      </div>
      {#if view === "visual"}
        <RecMap {recs} onselect={(r) => (selected = selected?.id === r.id ? null : r)} />
      {/if}
      <div class="rec-grid" class:single={view === "visual"}>
        {#each (view === "visual" ? (selected ? [selected] : []) : recs) as r (r.id)}
          <article class="card rec fade-in" class:over={r.over_budget}>
            {#if view === "visual"}<span class="idx">{recs.indexOf(r) + 1}</span>{/if}
            <div class="rec-head">
              <div>
                <span class="pill {r.kind === app.t('rec_kinds/software') ? 'good' : r.kind === app.t('rec_kinds/replace') ? 'critical' : 'accent'}">{r.kind}</span>
                <span class="impact faint small">{r.impact}</span>
              </div>
              <span class="cost">{r.cost}</span>
            </div>
            <h3>{r.title}</h3>
            <p>{r.body}</p>
            <p class="why muted small">{r.why}</p>
            {#if r.over_budget}<p class="small" style="color: var(--warn)">{app.u("advisor.over_budget")}</p>{/if}
            {#if r.helps.length}<p class="faint small">{app.u("advisor.helps")} {r.helps.join(", ")}</p>{/if}
            {#if r.shopping_query}
              <div class="shop">
                {#if links[r.id]}
                  {#each links[r.id] as l}
                    <button class="btn small" onclick={() => openUrl(l.url)}>{condLabel(l.condition)} · {l.vendor} ↗</button>
                  {/each}
                {:else if app.has("pro")}
                  <button class="btn" onclick={() => shop(r)}>{app.u("advisor.where_to_buy")}</button>
                {:else}
                  <span class="faint small">{app.u("advisor.where_to_buy_pro")}</span>
                {/if}
              </div>
            {/if}
          </article>
        {/each}
      </div>
      {#if !app.has("pro")}
        <Paywall tier="pro" compact />
      {/if}
      {#if linkErr && linkErr !== "pro"}<p class="small" style="color: var(--crit)">{linkErr}</p>{/if}
    {/if}
  </section>

  <section class="refine">
    <h2 class="faint">{app.u("advisor.refine")}</h2>
    <div class="row-3">
      {#each questions.filter((q) => !q.multi) as q (q.id)}
        {@render questionCard(q)}
      {/each}
    </div>
  </section>
</div>

<style>
  .adv { max-width: 1180px; margin: 0 auto; display: grid; gap: 18px; }
  .intro { padding: 20px 24px; display: grid; gap: 6px; }
  .row-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .row-3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }
  .q { padding: 14px 16px; display: grid; gap: 10px; align-content: start; }
  .opts { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { padding: 5px 11px; border-radius: 999px; border: 1px solid var(--line-2); background: var(--paper-2); cursor: pointer; font-size: 13px; }
  .chip:hover { background: var(--paper-3); }
  .chip.on { background: var(--accent); color: var(--accent-ink); border-color: transparent; font-weight: 600; }
  .big { justify-content: center; padding: 10px; justify-self: end; min-width: 260px; }
  .results { display: grid; gap: 12px; }
  .results-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .rec-grid.single { grid-template-columns: 1fr; }
  .rec { position: relative; }
  .idx { position: absolute; top: 12px; right: 14px; width: 22px; height: 22px; border-radius: 50%; background: var(--paper-3); font-size: 12px; font-weight: 700; display: grid; place-items: center; }
  .results:empty { display: none; }
  .rec-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); gap: 12px; align-items: start; }
  .rec { padding: 14px 18px; display: grid; gap: 6px; }
  .rec.over { opacity: 0.8; }
  .rec-head { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  .impact { margin-left: 8px; }
  .cost { font-weight: 700; font-variant-numeric: tabular-nums; }
  .why { font-style: italic; }
  .shop { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .refine { display: grid; gap: 10px; }
  .refine h2 { font-size: 12px; text-transform: uppercase; letter-spacing: 0.08em; }
  @media (max-width: 900px) { .row-2, .row-3 { grid-template-columns: 1fr; } .big { justify-self: stretch; } }
</style>
