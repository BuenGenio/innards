<script lang="ts">
  import { onMount } from 'svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { fit } from '$lib/actions/fit';
  import { GITHUB_URL, RELEASES_API } from '$lib/site';

  interface Asset { name: string; browser_download_url: string; size: number }
  interface Release { tag_name: string; html_url: string; published_at: string; assets: Asset[] }

  const platforms = [
    { id: 'linux', name: 'Linux', note: 'AppImage · .deb · .rpm', match: (n: string) => /\.(AppImage|deb|rpm)$/i.test(n), icon: 'M4 3h16a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zm0 2v14h16V5H4zm2.3 3.3l1.4-1.4L11.4 11l-3.7 3.7-1.4-1.4L8.6 11 6.3 8.3zM12 14h6v2h-6v-2z' },
    { id: 'mac', name: 'macOS', note: 'Universal .dmg', match: (n: string) => /\.dmg$/i.test(n), icon: 'M16.4 12.7c0-2.4 2-3.5 2-3.6-1.1-1.6-2.8-1.8-3.4-1.9-1.4-.1-2.8.9-3.5.9s-1.8-.8-3-.8C6.9 7.3 5.4 8.2 4.6 9.6c-1.7 3-.4 7.4 1.2 9.8.8 1.2 1.8 2.5 3 2.4 1.2 0 1.7-.8 3.2-.8s1.9.8 3.2.8 2.1-1.2 2.9-2.4c.9-1.4 1.3-2.7 1.3-2.8-.1 0-2.5-1-2.5-3.9zM14.2 5.6c.7-.8 1.1-1.9 1-3.1-1 0-2.2.7-2.9 1.5-.6.7-1.2 1.9-1 3 1.1.1 2.2-.6 2.9-1.4z' },
    { id: 'win', name: 'Windows', note: '.msi · .exe installer', match: (n: string) => /\.(msi|exe)$/i.test(n), icon: 'M3 5.5l7.5-1v7H3v-6zm8.5-1.2L21 3v8.5h-9.5v-7.2zM3 12.5h7.5v7L3 18.5v-6zm8.5 0H21V21l-9.5-1.3v-7.2z' }
  ];

  let release = $state<Release | null>(null);
  let status = $state<'loading' | 'ready' | 'none'>('loading');

  onMount(async () => {
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
        {@const assets = release?.assets.filter((a) => p.match(a.name)) ?? []}
        <li class="card">
          <svg viewBox="0 0 24 24" width="28" height="28" aria-hidden="true"><path fill="currentColor" d={p.icon} /></svg>
          <h3>{p.name}</h3>
          <p class="muted note">{p.note}</p>
          {#if status === 'ready' && assets.length}
            <ul class="assets">
              {#each assets as a}
                <li><a href={a.browser_download_url} class="btn primary">{a.name.replace(/^innards[_-]?/i, '')}<span class="sz">{fmt(a.size)}</span></a></li>
              {/each}
            </ul>
          {:else if status === 'ready'}
            <p class="muted small">No {p.name} build in this release.</p>
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
      <p class="lead">You need <a href="https://rustup.rs" rel="noopener">Rust</a>, <a href="https://pnpm.io" rel="noopener">pnpm</a> and the <a href="https://tauri.app/start/prerequisites/" rel="noopener">Tauri prerequisites</a> for your OS. The bundle lands in <code>src-tauri/target/release/bundle</code>.</p>
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
  .plat svg { color: var(--ink-2); margin-bottom: 4px; }
  .note { font-size: 0.86rem; }
  .assets { list-style: none; margin: 8px 0 0; padding: 0; display: grid; gap: 6px; }
  .assets .btn { font-family: var(--font-mono); font-size: 0.82rem; gap: 10px; }
  .sz { opacity: 0.75; font-weight: 400; }
  .small { font-size: 0.86rem; }
  .all { margin-top: 14px; }
  .soon { margin-top: 8px; font-size: 0.82rem; font-weight: 600; color: var(--ink-3); border: 1px dashed var(--line-2); border-radius: 999px; padding: 5px 11px; }
  .ghost { margin-top: 8px; opacity: 0.4; }
  .src { align-items: center; }
  .src .head { display: grid; gap: 12px; }
  .code { min-width: 0; }
  .c { color: var(--ink-3); }
</style>
