<script lang="ts">
  import { app } from "$lib/state.svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { api } from "$lib/api";
  import { marked } from "marked";
  import Paywall from "./Paywall.svelte";

  let text = $state<string | null>(null);
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function go() {
    busy = true; err = null;
    try {
      const n = await api.narrate(app.lang, app.level, false);
      text = n.text;
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
    }
  }
  const html = $derived(text ? (marked.parse(text) as string) : "");
</script>

<section class="card nar">
  <div class="head">
    <h2>{app.u("narrative.title")}</h2>
    <span class="pill accent">Supporter</span>
  </div>
  {#if !app.has("supporter")}
    <Paywall tier="supporter" compact />
  {:else if err === "no_api_key"}
    <p class="muted small">{app.u("narrative.no_key")}</p>
    <button class="btn" onclick={() => goto(resolve("/settings"))}>{app.u("narrative.open_settings")}</button>
  {:else}
    {#if text}
      <div class="md">{@html html}</div>
    {:else}
      <p class="muted small">{app.u("narrative.intro")}</p>
    {/if}
    {#if err && err !== "no_api_key"}<p class="small" style="color: var(--crit)">{err}</p>{/if}
    <button class="btn primary" onclick={go} disabled={busy}>{busy ? app.u("narrative.writing") : text ? app.u("narrative.regenerate") : app.u("narrative.write")}</button>
  {/if}
</section>

<style>
  .nar { padding: 16px 18px; display: grid; gap: 10px; justify-items: start; }
  .head { display: flex; align-items: center; justify-content: space-between; width: 100%; }
  .md { font-size: 13px; }
</style>
