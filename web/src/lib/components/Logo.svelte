<script lang="ts">
  // Mark: an open chassis with the first report line lit in the accent colour.
  let { size = 22, word = true }: { size?: number; word?: boolean } = $props();
</script>

<span class="logo" style:--s="{size}px">
  <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
    <rect x="1.5" y="1.5" width="21" height="21" rx="6" fill="none" stroke="currentColor" stroke-width="2" />
    <rect x="6" y="7" width="12" height="2.6" rx="1.3" fill="var(--accent)" />
    <rect x="6" y="11.7" width="9" height="2.6" rx="1.3" fill="currentColor" />
    <rect x="6" y="16.4" width="6" height="2.6" rx="1.3" fill="currentColor" />
  </svg>
  {#if word}<span class="word">innards</span>{/if}
</span>

<style>
  .logo { display: inline-flex; align-items: center; gap: 0.45em; font-family: var(--font-display); font-weight: 750; letter-spacing: -0.03em; font-size: calc(var(--s) * 0.95); line-height: 1; color: var(--ink); }
  .word { transform: translateY(-0.04em); }
</style>
