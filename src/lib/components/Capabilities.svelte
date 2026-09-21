<script lang="ts">
  import { app } from "$lib/state.svelte";
  import type { RenderedCapability } from "$lib/types";
  let { caps }: { caps: RenderedCapability[] } = $props();
  const color = (g: RenderedCapability["grade"]) =>
    g === "great" || g === "good" ? "var(--good)" : g === "ok" ? "var(--warn)" : g === "poor" ? "var(--warn)" : "var(--crit)";
</script>

<section class="card caps">
  <h2>{app.t("ui/capabilities")}</h2>
  <ul>
    {#each caps as c}
      <li title={c.limits.join(", ")}>
        <div class="row">
          <span class="label">{c.label}</span>
          <span class="grade" style="color: {color(c.grade)}">{c.grade_label}</span>
        </div>
        <div class="track"><div class="fill" style="width: {c.score}%; background: {color(c.grade)}"></div></div>
        {#if c.limits.length}<div class="limits faint small">{c.limits.join(" · ")}</div>{/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  .caps { padding: 16px 18px; }
  h2 { margin-bottom: 12px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 11px; }
  li { min-width: 0; }
  .row { display: flex; justify-content: space-between; gap: 8px; font-size: 13px; min-width: 0; }
  .label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .grade { font-weight: 600; white-space: nowrap; font-size: 12px; }
  .track { height: 5px; background: var(--paper-3); border-radius: 3px; margin-top: 4px; overflow: hidden; }
  .fill { height: 100%; border-radius: 3px; transition: width 0.6s ease-out; }
  .limits { margin-top: 2px; }
</style>
