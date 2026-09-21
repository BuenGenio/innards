<script lang="ts">
  import { app } from "$lib/state.svelte";
  import type { RenderedFinding } from "$lib/types";
  let { finding }: { finding: RenderedFinding } = $props();
  let open = $state(false);
</script>

<article class="card f {finding.severity}">
  <div class="bar"></div>
  <div class="body">
    <div class="head">
      <h3>{finding.title}</h3>
      <span class="cat faint small">{finding.category}</span>
    </div>
    <p>{finding.body}</p>
    {#if finding.action}
      <p class="action"><strong>{app.t("ui/action")}:</strong> {finding.action}</p>
    {/if}
    {#if finding.evidence.length}
      <button class="btn ghost small ev-toggle" onclick={() => (open = !open)}>
        {open ? "▾" : "▸"} {app.u("finding.evidence")}
      </button>
      {#if open}
        <pre class="evidence">{finding.evidence.join("\n")}</pre>
      {/if}
    {/if}
  </div>
</article>

<style>
  .f { display: grid; grid-template-columns: 4px 1fr; overflow: hidden; }
  .bar { background: var(--line-2); }
  .critical .bar { background: var(--crit); } .warning .bar { background: var(--warn); }
  .info .bar { background: var(--info); } .good .bar { background: var(--good); }
  .body { padding: 12px 16px 12px 14px; display: grid; gap: 6px; }
  .head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
  .cat { white-space: nowrap; }
  .action { color: var(--ink-2); }
  .ev-toggle { justify-self: start; padding: 2px 6px; }
  .evidence { margin: 0; padding: 8px 10px; background: var(--paper-3); border-radius: var(--radius-sm); font-family: var(--mono); font-size: 11.5px; white-space: pre-wrap; color: var(--ink-2); }
</style>
