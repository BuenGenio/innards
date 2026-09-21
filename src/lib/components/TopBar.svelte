<script lang="ts">
  import { app } from "$lib/state.svelte";
  import type { Level } from "$lib/types";
  import Logo from "./Logo.svelte";
  import { page } from "$app/state";
  import { resolve } from "$app/paths";

  const levels: { key: Level; label: string }[] = [
    { key: "plain", label: "level.plain" },
    { key: "informed", label: "level.informed" },
    { key: "expert", label: "level.expert" },
  ];
  const screens: { href: "/" | "/advisor" | "/settings"; label: string }[] = [
    { href: "/", label: "nav.report" },
    { href: "/advisor", label: "nav.advisor" },
    { href: "/settings", label: "nav.settings" },
  ];
  const current = $derived(page.url.pathname.replace(/\/$/, "") || "/");
  const flags: Record<string, string> = { en: "🇬🇧", es: "🇪🇸", de: "🇩🇪", fr: "🇫🇷", it: "🇮🇹", pt: "🇧🇷", nl: "🇳🇱", pl: "🇵🇱", uk: "🇺🇦", ru: "🇷🇺", ja: "🇯🇵", zh: "🇨🇳" };
  const flag = (code: string) => flags[code] ?? "🌐";
  let langOpen = $state(false);
  function pick(code: string) { langOpen = false; app.setLang(code); }
</script>

<svelte:window onclick={(e) => { if (!(e.target as HTMLElement).closest(".lang")) langOpen = false; }} onkeydown={(e) => { if (e.key === "Escape") langOpen = false; }} />

<header>
  <div class="brand">
    <Logo size={26} />
    <span class="name">Innards</span>
    {#if app.tier !== "free"}<span class="pill accent">{app.tier}</span>{/if}
  </div>

  <nav class="seg" aria-label="Screens">
    {#each screens as sc}
      <a href={resolve(sc.href)} class:on={current === sc.href} aria-current={current === sc.href ? "page" : undefined}>{app.u(sc.label)}</a>
    {/each}
  </nav>

  <div class="controls">
    <div class="seg" title="How technical should the report be?">
      {#each levels as lv}
        <button class:on={app.level === lv.key} onclick={() => app.setLevel(lv.key)}>{app.u(lv.label)}</button>
      {/each}
    </div>
    <div class="lang">
      <button class="btn flag" aria-label="Language" aria-expanded={langOpen} onclick={() => (langOpen = !langOpen)}>{flag(app.lang)}</button>
      {#if langOpen}
        <ul class="menu card" role="listbox">
          {#each app.languages as [code, name]}
            <li><button role="option" aria-selected={code === app.lang} class:on={code === app.lang} onclick={() => pick(code)}><span>{flag(code)}</span> {name}</button></li>
          {/each}
        </ul>
      {/if}
    </div>
    <button class="btn" onclick={() => app.scan()} disabled={app.scanning} title="Inspect again">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class:spin={app.scanning}><path d="M21 12a9 9 0 1 1-3-6.7"/><path d="M21 3v6h-6"/></svg>
      {app.u("topbar.rescan")}
    </button>
  </div>
</header>

<style>
  header {
    display: flex; align-items: center; gap: 20px;
    padding: 12px 28px; border-bottom: 1px solid var(--line);
    background: var(--paper-2);
    position: sticky; top: 0; z-index: 5;
  }
  .brand { display: flex; align-items: center; gap: 9px; min-width: 130px; }
  .name { font-weight: 700; font-size: 16px; letter-spacing: -0.02em; }
  nav { margin-left: 4px; }
  nav a { padding: 5px 12px; border-radius: 999px; color: var(--ink-2); text-decoration: none; }
  nav a.on { background: var(--paper-2); color: var(--ink); box-shadow: 0 1px 2px rgba(0,0,0,0.08); font-weight: 600; }
  .controls { margin-left: auto; display: flex; align-items: center; gap: 10px; }
  .lang { position: relative; }
  .flag { font-size: 18px; padding: 4px 9px; line-height: 1; }
  .menu { position: absolute; right: 0; top: calc(100% + 6px); margin: 0; padding: 4px; list-style: none; min-width: 160px; z-index: 10; }
  .menu button { display: flex; gap: 8px; align-items: center; width: 100%; padding: 7px 10px; border: 0; background: transparent; border-radius: var(--radius-sm); cursor: pointer; text-align: left; }
  .menu button:hover { background: var(--paper-3); }
  .menu button.on { font-weight: 600; }
  .spin { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 860px) {
    header { flex-wrap: wrap; gap: 10px; }
    .controls { margin-left: 0; width: 100%; }
  }
</style>
