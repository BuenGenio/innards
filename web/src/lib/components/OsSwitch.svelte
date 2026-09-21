<script lang="ts">
  // Lets a visitor preview the demo window in the other platforms' chrome.
  // Persisted so the inline detector in app.html honours it on reload.
  import { onMount } from 'svelte';
  type Os = 'mac' | 'windows' | 'linux';
  const options: [Os, string][] = [['mac', 'macOS'], ['windows', 'Windows'], ['linux', 'Linux']];
  let current = $state<Os | null>(null);
  let manual = $state(false);
  onMount(() => {
    current = (document.documentElement.dataset.os as Os) ?? null;
    try { manual = !!localStorage.getItem('innards-os'); } catch {}
  });
  function pick(os: Os) {
    current = os; manual = true;
    document.documentElement.dataset.os = os;
    try { localStorage.setItem('innards-os', os); } catch {}
  }
  function auto() {
    manual = false;
    try { localStorage.removeItem('innards-os'); } catch {}
    const p = (navigator as unknown as { userAgentData?: { platform?: string } }).userAgentData?.platform || navigator.platform || '';
    const u = navigator.userAgent;
    current = /win/i.test(p) || /Windows/i.test(u) ? 'windows' : /mac|iphone|ipad|ipod/i.test(p) || /Mac OS|iPhone|iPad/i.test(u) ? 'mac' : 'linux';
    document.documentElement.dataset.os = current;
  }
</script>

<p class="os muted" aria-label="Window style">
  <span>Window style:</span>
  {#each options as [os, label]}
    <button type="button" aria-pressed={current === os} onclick={() => pick(os)}>{label}</button>
  {/each}
  {#if manual}<button type="button" class="auto" onclick={auto}>auto</button>{/if}
</p>

<style>
  .os { display: flex; flex-wrap: wrap; align-items: center; gap: 2px 8px; font-size: 0.78rem; justify-content: flex-end; }
  button { border: 0; background: none; padding: 2px 4px; color: var(--ink-3); cursor: pointer; font-size: inherit; border-radius: 4px; }
  button:hover { color: var(--ink); }
  button[aria-pressed='true'] { color: var(--ink); font-weight: 600; text-decoration: underline; text-underline-offset: 3px; }
  .auto { font-style: italic; }
</style>
