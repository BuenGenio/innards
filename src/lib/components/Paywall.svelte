<script lang="ts">
  import { app } from "$lib/state.svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { openExternal as openUrl } from "$lib/platform";
  let { tier, compact = false }: { tier: "supporter" | "pro"; compact?: boolean } = $props();

  // Checkout URLs are placeholders until the store is live.
  const STORE = "https://innards.app/buy";

  const copy = $derived({
    title: app.u(`paywall.${tier}.title`),
    body: app.u(`paywall.${tier}.body`),
    cta: app.u(`paywall.${tier}.cta`),
  });
</script>

<div class="pay" class:compact>
  <div>
    <h3>{copy.title}</h3>
    <p class="muted small">{copy.body}</p>
  </div>
  <div class="row">
    <button class="btn primary" onclick={() => openUrl(`${STORE}?tier=${tier}`)}>{copy.cta}</button>
    <button class="btn ghost" onclick={() => goto(resolve("/settings"))}>{app.u("paywall.have_key")}</button>
  </div>
</div>

<style>
  .pay { display: grid; gap: 12px; padding: 14px 16px; border-radius: var(--radius-sm); background: var(--accent-soft); border: 1px dashed var(--accent); width: 100%; }
  .pay.compact { padding: 12px 14px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
</style>
