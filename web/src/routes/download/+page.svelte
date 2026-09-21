<script lang="ts">
  import { onMount } from 'svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { fit } from '$lib/actions/fit';
  import { GITHUB_URL, RELEASES_API } from '$lib/site';

  interface Asset { name: string; browser_download_url: string; size: number }
  interface Release { tag_name: string; html_url: string; published_at: string; assets: Asset[] }

  // Assets are matched by extension and shown in this order; `os` matches <html data-os> for the highlight.
  const platforms = [
    { id: 'linux', os: 'linux', name: 'Linux', note: 'x86_64 · needs WebKitGTK', kinds: [
        { re: /\.deb$/i, label: '.deb', sub: 'Debian, Ubuntu, Mint' },
        { re: /\.AppImage$/i, label: 'AppImage', sub: 'any distribution' },
        { re: /\.rpm$/i, label: '.rpm', sub: 'Fedora, openSUSE' }
      ], icon: 'M4 3h16a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zm0 2v14h16V5H4zm2.3 3.3l1.4-1.4L11.4 11l-3.7 3.7-1.4-1.4L8.6 11 6.3 8.3zM12 14h6v2h-6v-2z' },
    { id: 'mac', os: 'mac', name: 'macOS', note: 'Apple silicon and Intel', kinds: [
        { re: /aarch64.*\.dmg$|arm64.*\.dmg$/i, label: '.dmg', sub: 'Apple silicon' },
        { re: /x64.*\.dmg$|x86_64.*\.dmg$|intel.*\.dmg$/i, label: '.dmg', sub: 'Intel' },
        { re: /\.dmg$/i, label: '.dmg', sub: 'universal' }
      ], icon: 'M16.4 12.7c0-2.4 2-3.5 2-3.6-1.1-1.6-2.8-1.8-3.4-1.9-1.4-.1-2.8.9-3.5.9s-1.8-.8-3-.8C6.9 7.3 5.4 8.2 4.6 9.6c-1.7 3-.4 7.4 1.2 9.8.8 1.2 1.8 2.5 3 2.4 1.2 0 1.7-.8 3.2-.8s1.9.8 3.2.8 2.1-1.2 2.9-2.4c.9-1.4 1.3-2.7 1.3-2.8-.1 0-2.5-1-2.5-3.9zM14.2 5.6c.7-.8 1.1-1.9 1-3.1-1 0-2.2.7-2.9 1.5-.6.7-1.2 1.9-1 3 1.1.1 2.2-.6 2.9-1.4z' },
    { id: 'win', os: 'windows', name: 'Windows', note: 'Windows 10 or later, x86_64', kinds: [
        { re: /\.msi$/i, label: '.msi', sub: 'installer' },
        { re: /\.exe$/i, label: '.exe', sub: 'setup' }
      ], icon: 'M3 5.5l7.5-1v7H3v-6zm8.5-1.2L21 3v8.5h-9.5v-7.2zM3 12.5h7.5v7L3 18.5v-6zm8.5 0H21V21l-9.5-1.3v-7.2z' }
  ];
  // Each asset is claimed by the first matching kind, so "universal .dmg" only shows when no arch-specific one exists.
  function assetsFor(p: (typeof platforms)[number], all: Asset[]) {
    const taken = new Set<string>();
    return p.kinds.flatMap((k) => {
      const a = all.find((x) => !taken.has(x.name) && k.re.test(x.name));
      if (!a) return [];
      taken.add(a.name);
      return [{ ...k, asset: a }];
    });
  }
  let yourOs = $state<string | null>(null);

  let release = $state<Release | null>(null);
  let status = $state<'loading' | 'ready' | 'none'>('loading');

  onMount(async () => {
    yourOs = document.documentElement.dataset.os ?? null;
    try {
      const r = await fetch(RELEASES_API, { headers: { Accept: 'application/vnd.github+json' } });
      if (!r.ok) throw new Error(String(r.status));
      const j = (await r.json()) as Release;
      if (!j.assets?.length) throw new Error('no assets');
      release = j;
      status = 'ready';
    } catch {
      status = 'none';
    }
  });

  const fmt = (n: number) => (n >= 1 << 20 ? `${(n / (1 << 20)).toFixed(0)} MB` : `${(n / 1024).toFixed(0)} KB`);
</script>

<Seo title="Download" description="Download innards for Linux, macOS and Windows — or build it from source. Free and open source." />

<section class="block top">
  <div class="wrap">
    <div class="head">
      <p class="eyebrow">Download</p>
      <h1 use:fit>Get innards</h1>
      <p class="lead">
        {#if status === 'ready' && release}
          Version <span class="mono">{release.tag_name}</span>, published {new Date(release.published_at).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' })}. Free, open source, no account.
        {:else if status === 'none'}
          Builds for Linux, macOS and Windows are on the way. Until the first release lands, you can build it from source in a few minutes.
        {:else}
          Checking the latest release…
        {/if}
      </p>
    </div>

    <ul class="plat">
      {#each platforms as p}
        {@const found = release ? assetsFor(p, release.assets) : []}
        <li class="card" class:yours={yourOs === p.os}>
          <div class="ph">
            <svg viewBox="0 0 24 24" width="28" height="28" aria-hidden="true"><path fill="currentColor" d={p.icon} /></svg>
            {#if yourOs === p.os}<span class="pill">Your system</span>{/if}
          </div>
          <h3>{p.name}</h3>
          <p class="muted note">{p.note}</p>
          {#if status === 'ready' && found.length}
            <ul class="assets">
              {#each found as k, i}
                <li><a href={k.asset.browser_download_url} class="btn" class:primary={i === 0}><span>{k.label}</span><small>{k.sub} · {fmt(k.asset.size)}</small></a></li>
              {/each}
            </ul>
          {:else if status === 'ready'}
            <span class="soon">Coming soon — not in {release?.tag_name} yet</span>
          {:else if status === 'loading'}
            <span class="btn ghost" aria-hidden="true">…</span>
          {:else}
            <span class="soon">Coming soon</span>
          {/if}
        </li>
      {/each}
    </ul>

    {#if status === 'ready' && release}
      <p class="muted small all">All files, checksums and notes: <a href={release.html_url} rel="noopener">release {release.tag_name} on GitHub</a>.</p>
    {/if}
  </div>
</section>

<section class="block">
  <div class="wrap grid-2 src">
    <div class="head">
      <p class="eyebrow">From source</p>
      <h2 use:fit>Build it yourself</h2>
      <p class="lead">You need <a href="https://rustup.rs" rel="noopener">Rust</a>, <a href="https://pnpm.io" rel="noopener">pnpm</a> and the <a href="https://tauri.app/start/prerequisites/" rel="noopener">Tauri prerequisites</a> for your OS. Bundles land in <code>target/release/bundle</code> (the workspace target dir).</p>
      <p class="muted small">Just want the report? The engine runs on its own from a terminal — no GUI, no install.</p>
    </div>
    <div class="code">
      <pre><code><span class="c"># desktop app</span>
git clone {GITHUB_URL.replace('https://', 'https://')}
cd innards
pnpm install
pnpm tauri build

<span class="c"># engine only — prints the Markdown report for this machine</span>
cargo run -p innards-core --example report -- en plain</code></pre>
    </div>
  </div>
</section>

<style>
  .top { padding-top: clamp(28px, 5vw, 56px); }
  .plat { list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; }
  @media (min-width: 680px) { .plat { grid-template-columns: repeat(3, 1fr); } }
  .plat li.card { padding: 20px; display: grid; gap: 6px; align-content: start; justify-items: start; }
  .plat li.yours { border-color: var(--accent); box-shadow: var(--shadow), 0 0 0 3px var(--accent-soft); }
  .ph { display: flex; align-items: center; justify-content: space-between; width: 100%; margin-bottom: 4px; }
  .plat svg { color: var(--ink-2); }
  .pill { font-size: 0.72rem; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; color: var(--accent); background: var(--accent-soft); border-radius: 999px; padding: 3px 9px; }
  .note { font-size: 0.86rem; }
  .assets { list-style: none; margin: 8px 0 0; padding: 0; display: grid; gap: 6px; }
  .assets .btn { justify-content: flex-start; gap: 10px; padding: 8px 14px; }
  .assets .btn small { font-weight: 400; opacity: 0.8; font-size: 0.8rem; }
  .small { font-size: 0.86rem; }
  .all { margin-top: 14px; }
  .soon { margin-top: 8px; font-size: 0.82rem; font-weight: 600; color: var(--ink-3); border: 1px dashed var(--line-2); border-radius: 999px; padding: 5px 11px; text-align: center; }
  .ghost { margin-top: 8px; opacity: 0.4; }
  .src { align-items: center; }
  .src .head { display: grid; gap: 12px; }
  .code { min-width: 0; }
  .c { color: var(--ink-3); }
</style>
