<script lang="ts">
  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import Logo from './Logo.svelte';
  import { GITHUB_URL } from '$lib/site';
  const links = [
    { href: '/#report', label: 'Report' },
    { href: '/#advisor', label: 'Advisor' },
    { href: '/privacy', label: 'Privacy' }
  ];

  /* The header rolls away as the page scrolls down and returns on the first
     scroll back up, so it stops taxing the viewport while people read.
     (Pattern from intelimaris-web NavigationBar.vue.) */
  let hidden = $state(false);
  let lastY = 0;
  let graceUntil = 0;
  const onScroll = () => {
    const y = window.scrollY;
    const delta = y - lastY;
    lastY = y;
    if (y < 96 || performance.now() < graceUntil) { hidden = false; return; }
    if (delta > 6) hidden = true;
    else if (delta < -6) hidden = false;
  };
  onMount(() => {
    lastY = window.scrollY;
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  });
  afterNavigate(() => { hidden = false; graceUntil = performance.now() + 900; });
</script>

<header class="hdr" class:is-hidden={hidden} onfocusin={() => (hidden = false)}>
  <div class="nav-glass" aria-hidden="true"></div>
  <div class="wrap bar">
    <a href="/" class="home" aria-label="innards home"><Logo /></a>
    <nav aria-label="Primary">
      {#each links as l}
        <a href={l.href} class:active={page.url.pathname === l.href}>{l.label}</a>
      {/each}
      <a href={GITHUB_URL} rel="noopener" class="gh" aria-label="GitHub">
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.3.8-.6v-2c-3.2.7-3.9-1.4-3.9-1.4-.5-1.3-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.7-1.6-2.6-.3-5.3-1.3-5.3-5.7 0-1.3.4-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.2 1.2a11 11 0 0 1 5.8 0c2.2-1.5 3.2-1.2 3.2-1.2.6 1.6.2 2.8.1 3.1.8.8 1.2 1.8 1.2 3.1 0 4.4-2.7 5.4-5.3 5.7.4.4.8 1.1.8 2.2v3.2c0 .3.2.7.8.6 4.6-1.5 7.9-5.8 7.9-10.9C23.5 5.7 18.3.5 12 .5z"/></svg>
      </a>
      <a href="/download" class="btn primary sm">Download</a>
    </nav>
  </div>
</header>

<style>
  /* Fixed + isolated; the blur lives on a child layer, never on the transformed header itself. */
  .hdr { position: fixed; inset: 0 0 auto; z-index: 50; height: var(--header-h); isolation: isolate; border-bottom: 1px solid color-mix(in srgb, var(--line) 70%, transparent); transition: transform 280ms var(--ease); will-change: transform; }
  .hdr.is-hidden { transform: translateY(-100%); }
  .nav-glass { position: absolute; inset: 0; z-index: -1; pointer-events: none; background: var(--bg); }
  @supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .nav-glass { background: color-mix(in srgb, var(--bg) 70%, transparent); -webkit-backdrop-filter: blur(20px) saturate(145%); backdrop-filter: blur(20px) saturate(145%); }
  }
  @media (prefers-reduced-transparency: reduce) { .nav-glass { background: var(--bg); backdrop-filter: none; -webkit-backdrop-filter: none; } }
  @media (prefers-reduced-motion: reduce) { .hdr { transition: none; } }

  .bar { height: 100%; display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .home { text-decoration: none; color: inherit; }
  .home:hover { color: inherit; }
  nav { display: flex; align-items: center; gap: clamp(10px, 2.2vw, 22px); }
  nav a { text-decoration: none; font-weight: 500; font-size: 0.93rem; color: var(--ink-2); }
  nav a:hover, nav a.active { color: var(--ink); }
  nav a.gh { display: inline-flex; color: var(--ink-2); }
  .btn.sm { padding: 7px 14px; font-size: 0.88rem; }
  @media (max-width: 560px) { nav a:not(.gh):not(.btn) { display: none; } }
</style>
