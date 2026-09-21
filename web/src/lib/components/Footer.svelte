<script lang="ts">
  import Logo from './Logo.svelte';
  import { GITHUB_URL } from '$lib/site';
</script>

<footer class="ftr">
  <div class="wrap inner">
    <div class="l">
      <Logo size={18} />
      <p class="muted tag">The report stays on your machine.</p>
    </div>
    <nav aria-label="Footer">
      <a href="/download">Download</a>
      <a href="/privacy">Privacy</a>
      <a href={GITHUB_URL} rel="noopener">GitHub</a>
      <a href="{GITHUB_URL}/blob/main/LICENSE" rel="noopener">MIT license</a>
    </nav>
    <p class="muted copy">© {new Date().getFullYear()} Eugene Trotsan</p>
  </div>
</footer>

<style>
  .ftr { margin-top: clamp(48px, 8vw, 96px); border-top: 1px solid var(--line); padding: 26px 0 max(22px, env(safe-area-inset-bottom)); }
  .inner { display: flex; flex-wrap: wrap; gap: 14px 28px; align-items: center; justify-content: space-between; }
  .l { display: flex; align-items: center; gap: 14px; }
  .tag { font-size: 0.9rem; }
  nav { display: flex; flex-wrap: wrap; gap: 6px 18px; font-size: 0.9rem; }
  nav a { text-decoration: none; color: var(--ink-2); }
  nav a:hover { color: var(--ink); }
  .copy { font-size: 0.85rem; }
</style>
