<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { app } from "$lib/state.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import Scanning from "$lib/components/Scanning.svelte";
  let { children } = $props();
  onMount(() => { app.init(); });
</script>

<div class="shell">
  <TopBar />
  <main>
    {#if app.error}
      <div class="card error fade-in">
        <strong>{app.u("error.title")}</strong>
        <code>{app.error}</code>
        <button class="btn" onclick={() => app.scan()}>{app.u("error.retry")}</button>
      </div>
    {:else if !app.report}
      <Scanning />
    {:else}
      {@render children()}
    {/if}
  </main>
</div>

<style>
  .shell { display: flex; flex-direction: column; height: 100vh; }
  main { flex: 1; overflow-y: auto; padding: 20px 28px 40px; }
  .error { padding: 20px; display: grid; gap: 10px; justify-items: start; max-width: 640px; margin: 40px auto; }
  .error code { white-space: pre-wrap; color: var(--crit); }
</style>
