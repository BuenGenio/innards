<script lang="ts">
  import { app } from "$lib/state.svelte";
  import { api } from "$lib/api";
  import { saveTextFile } from "$lib/platform";
  import type { Severity } from "$lib/types";
  import HealthRing from "./HealthRing.svelte";
  import FindingCard from "./FindingCard.svelte";
  import Capabilities from "./Capabilities.svelte";
  import Narrative from "./Narrative.svelte";
  import History from "./History.svelte";

  const order: Severity[] = ["critical", "warning", "info", "good"];
  const report = $derived(app.report!);
  const groups = $derived(order.map((s) => ({ sev: s, items: report.findings.filter((f) => f.severity === s) })).filter((g) => g.items.length));

  let exporting = $state(false);
  async function exportJson() {
    const json = JSON.stringify(await api.reportJson(), null, 2);
    await saveTextFile("innards-report.json", json, "json");
  }
  let openErr = $state<string | null>(null);
  async function openReport() {
    openErr = null;
    const { openTextFile } = await import("$lib/platform");
    const text = await openTextFile("json");
    if (!text) return;
    try {
      await app.openReport(text);
    } catch (e) {
      openErr = String(e) === "pro" ? app.u("report.requires_pro") : String(e);
    }
  }
  async function exportMd() {
    exporting = true;
    try {
      const md = await api.exportMarkdown(app.lang, app.level);
      await saveTextFile("innards-report.md", md);
    } finally {
      exporting = false;
    }
  }

  const s = $derived(report.summary);
  const specs = $derived([
    [app.t("ui/cpu"), s.cpu],
    [app.t("ui/memory"), s.memory],
    [app.t("ui/storage"), s.storage],
    [app.t("ui/gpu"), s.gpu],
    [app.t("ui/os"), s.os],
    ...(s.battery ? [[app.t("ui/battery"), s.battery]] : []),
  ].filter(([, v]) => v));
</script>

<div class="report fade-in">
  <section class="hero card">
    <HealthRing score={s.health_score} />
    <div class="hero-text">
      <p class="eyebrow faint">{app.t("ui/report_title")}</p>
      <h1>{s.machine}</h1>
      <p class="verdict">{report.verdict}</p>
      {#if app.otherMachine}
        <p class="pill warning" style="margin-bottom:8px">{app.u("report.other_machine")}</p>
      {/if}
      <div class="actions">
        {#if app.otherMachine}
          <button class="btn primary" onclick={() => app.scan()}>{app.u("report.back")}</button>
        {/if}
        <button class="btn" onclick={exportMd} disabled={exporting}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"/></svg>
          {app.u("report.export_md")}
        </button>
        <button class="btn" onclick={exportJson}>{app.u("report.export_json")}</button>
        <button class="btn ghost" onclick={openReport} title={app.has("pro") ? "" : "Pro"}>{app.u("report.open")}{#if !app.has("pro")} <span class="pill accent">Pro</span>{/if}</button>
      </div>
      {#if openErr}<p class="small" style="color: var(--crit)">{openErr}</p>{/if}
    </div>
    <dl class="specs">
      {#each specs as [k, v]}
        <div><dt>{k}</dt><dd>{v}</dd></div>
      {/each}
    </dl>
  </section>

  <div class="columns">
    <section class="findings">
      {#each groups as g}
        <h2 class="group-title"><span class="dot {g.sev}"></span>{app.t(`severities/${g.sev}`)} <span class="count faint">{g.items.length}</span></h2>
        <div class="list">
          {#each g.items as f (f.id + f.title)}
            <FindingCard finding={f} />
          {/each}
        </div>
      {/each}
      {#if report.notes.length}
        <h2 class="group-title">{app.t("ui/notes")}</h2>
        <ul class="notes small muted">
          {#each report.notes as n}<li>{n}</li>{/each}
        </ul>
      {/if}
    </section>

    <aside>
      <Capabilities caps={report.capabilities} />
      <History />
      <Narrative />
    </aside>
  </div>
</div>

<style>
  .report { max-width: 1180px; margin: 0 auto; display: grid; gap: 22px; }
  .hero { display: grid; grid-template-columns: auto 1fr auto; gap: 26px; align-items: center; padding: 22px 26px; }
  .eyebrow { text-transform: uppercase; letter-spacing: 0.08em; font-size: 11px; font-weight: 600; margin-bottom: 4px; }
  .hero h1 { font-size: 24px; margin-bottom: 6px; }
  .verdict { font-size: 15px; color: var(--ink-2); max-width: 46ch; }
  .actions { margin-top: 14px; display: flex; gap: 8px; flex-wrap: wrap; }
  .specs { margin: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 6px 22px; min-width: 300px; }
  .specs div { display: grid; }
  dt { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--ink-3); font-weight: 600; }
  dd { margin: 0; font-size: 13px; max-width: 230px; }
  .columns { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 22px; align-items: start; }
  aside { display: grid; gap: 18px; position: sticky; top: 8px; min-width: 0; }
  .group-title { display: flex; align-items: center; gap: 8px; margin: 6px 0 10px; }
  .group-title + .list { margin-bottom: 20px; }
  .count { font-weight: 500; font-size: 13px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; display: inline-block; }
  .dot.critical { background: var(--crit); } .dot.warning { background: var(--warn); } .dot.info { background: var(--info); } .dot.good { background: var(--good); }
  .list { display: grid; gap: 10px; }
  .notes { margin: 0; padding-left: 18px; }
  @media (max-width: 980px) {
    .hero { grid-template-columns: auto 1fr; }
    .specs { grid-column: 1 / -1; }
    .columns { grid-template-columns: 1fr; }
    aside { position: static; }
  }
</style>
