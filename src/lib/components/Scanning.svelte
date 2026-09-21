<script lang="ts">
  import { app } from "$lib/state.svelte";
  import Logo from "./Logo.svelte";
</script>

<div class="scan fade-in">
  <div class="pulse"><Logo size={48} /></div>
  <h2>{app.u("scanning.title")}</h2>
  <p class="muted">{app.u("scanning.sub")}</p>
</div>

<style>
  .scan { display: grid; justify-items: center; gap: 8px; margin-top: 18vh; text-align: center; }
  .pulse { color: var(--ink-2); animation: pulse 1.4s ease-in-out infinite; margin-bottom: 8px; }
  @keyframes pulse { 0%, 100% { opacity: 0.45; transform: scale(0.97); } 50% { opacity: 1; transform: scale(1); } }
</style>
