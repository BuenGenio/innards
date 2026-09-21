<script lang="ts">
  import { x1 } from '$lib/data/machines';
  import { LEVELS, t, tf, type Level } from '$lib/engine/i18n';

  const blurbs: Record<Level, string> = {
    plain: 'No jargon at all.',
    informed: 'Some terms, still friendly. The default.',
    expert: 'Full detail, plus the raw evidence.'
  };
  const findings = x1.findings.filter((f) => ['battery.worn', 'memory.tmpfs_heavy', 'memory.soldered', 'cpu.old', 'storage.system_nvme'].includes(f.id));
  let level = $state<Level>('plain');
  let idx = $state(0);
  const f = $derived(findings[idx]);
  const title = $derived(tf(`findings/${f.id}/title`, f.params));
  const body = $derived(tf(`findings/${f.id}/${level}`, f.params));
  const action = $derived(tf(`findings/${f.id}/action`, f.params));
</script>

<div class="demo card" class:expert={level === 'expert'}>
  <div class="top">
    <div class="seg" role="group" aria-label="Explanation level">
      {#each LEVELS as l}
        <button type="button" aria-pressed={level === l} onclick={() => (level = l)}>{l[0].toUpperCase() + l.slice(1)}</button>
      {/each}
    </div>
    <p class="muted blurb">{blurbs[level]}</p>
  </div>

  <div class="body">
    <ul class="list" role="tablist" aria-label="Findings">
      {#each findings as fi, i}
        <li>
          <button type="button" role="tab" aria-selected={i === idx} class="sev-{fi.severity}" onclick={() => (idx = i)}>
            <i></i><span>{tf(`findings/${fi.id}/title`, fi.params)}</span>
          </button>
        </li>
      {/each}
    </ul>

    <div class="pane" role="tabpanel">
      {#key `${f.id}-${level}`}
        <div class="in">
          <p class="sev sev-{f.severity}"><i></i>{t(`severities/${f.severity}`)} · {t(`categories/${f.category}`)}</p>
          <h3>{title}</h3>
          <p class="text">{body}</p>
          {#if action}
            <p class="action"><strong>{t('ui/action')}:</strong> {action}</p>
          {/if}
          {#if level === 'expert' && f.evidence.length}
            <ul class="evidence">
              {#each f.evidence as e}<li><code>{e}</code></li>{/each}
            </ul>
          {/if}
        </div>
      {/key}
    </div>
  </div>
</div>

<style>
  .demo { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; padding: clamp(14px, 2vw, 20px); max-height: calc(100dvh - var(--header-h) - 96px); overflow: auto; transition: background 0.3s var(--ease), border-color 0.3s var(--ease); }
  .top { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 14px; }
  .blurb { font-size: 0.9rem; }
  .body { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; min-width: 0; }
  @media (min-width: 720px) { .body { grid-template-columns: minmax(0, 2fr) minmax(0, 3fr); gap: 16px; } }

  .list { list-style: none; margin: 0; padding: 0; display: flex; gap: 6px; overflow-x: auto; scrollbar-width: none; -webkit-overflow-scrolling: touch; }
  .list::-webkit-scrollbar { display: none; }
  @media (min-width: 720px) { .list { flex-direction: column; overflow: visible; } }
  .list li { flex: none; max-width: 76%; }
  @media (min-width: 720px) { .list li { max-width: none; } }
  .list button {
    display: flex; align-items: center; gap: 9px; width: 100%; text-align: left; border: 1px solid var(--line); background: transparent;
    padding: 9px 11px; border-radius: 10px; cursor: pointer; font-size: 0.88rem; line-height: 1.25; white-space: nowrap;
    transition: background 0.15s var(--ease), border-color 0.15s var(--ease);
  }
  .list button span { overflow: hidden; text-overflow: ellipsis; }
  @media (min-width: 720px) { .list button { white-space: normal; } .list button span { overflow: visible; } }
  .list button:hover { border-color: var(--line-2); background: var(--bg); }
  .list button[aria-selected='true'] { background: var(--bg-sunk); border-color: var(--line-2); font-weight: 600; }
  .list i, .sev i { flex: none; width: 8px; height: 8px; border-radius: 50%; background: var(--c); }

  .pane { background: var(--bg); border: 1px solid var(--line); border-radius: 10px; padding: clamp(14px, 2vw, 20px); min-height: 160px; transition: background 0.3s var(--ease), color 0.3s var(--ease); }
  .in { display: grid; gap: 10px; animation: fade 0.25s var(--ease); }
  @keyframes fade { from { opacity: 0; transform: translateY(3px); } }
  .sev { display: inline-flex; align-items: center; gap: 8px; font-size: 0.78rem; font-family: var(--font-mono); letter-spacing: 0.04em; text-transform: uppercase; color: var(--ink-3); }
  h3 { font-size: 1.15rem; }
  .text { font-size: 1.02rem; line-height: 1.55; text-wrap: pretty; }
  .action { font-size: 0.92rem; color: var(--ink-2); }
  .evidence { list-style: none; margin: 4px 0 0; padding: 0; display: grid; gap: 4px; }
  .evidence code { display: block; font-size: 0.8rem; padding: 6px 9px; background: var(--bg-sunk); border-color: var(--line-2); }

  /* Expert flips the pane into terminal dress: same facts, different register. */
  .expert .pane { background: #131217; color: #e8e6df; border-color: #2a2830; --line: #2a2830; --bg-sunk: #1d1c22; --ink-2: #b8b4c0; --ink-3: #8b8794; }
  .expert .text { font-family: var(--font-mono); font-size: 0.9rem; line-height: 1.6; }
  .expert .pane h3 { font-family: var(--font-mono); font-weight: 600; font-size: 1rem; letter-spacing: 0; }
</style>
