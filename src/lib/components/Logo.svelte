<script lang="ts">
  // A chip outline with a "heartbeat" trace through it: hardware + health.
  let { size = 24 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 32 32" fill="none" aria-hidden="true">
  <rect x="6" y="6" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2.2" />
  <path d="M6 12H3M6 16H3M6 20H3M29 12h-3M29 16h-3M29 20h-3M12 6V3M16 6V3M20 6V3M12 29v-3M16 29v-3M20 29v-3" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
  <path d="M9.5 16.5h3l1.8-4.5 2.6 8 2-3.5h3.6" stroke="var(--accent)" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
</svg>
