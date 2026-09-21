<script lang="ts">
  import Seo from '$lib/components/Seo.svelte';
  import ReportCard from '$lib/components/ReportCard.svelte';
  import OsSwitch from '$lib/components/OsSwitch.svelte';
  import LevelDemo from '$lib/components/LevelDemo.svelte';
  import CapabilityDemo from '$lib/components/CapabilityDemo.svelte';
  import AdvisorDemo from '$lib/components/AdvisorDemo.svelte';
  import { fit } from '$lib/actions/fit';
  import { x1 } from '$lib/data/machines';
  import { GITHUB_URL } from '$lib/site';

  const principles = [
    { t: 'No network. Ever.', b: 'The engine has no networking code. It reads /proc, /sys, DMI, battery and SMART data, renders the report locally, and exports Markdown you own.' },
    { t: 'Admin rights only when you ask', b: 'Drive health (SMART) needs smartmontools and elevated rights. innards asks before it prompts — skip it and everything else still works.' },
    { t: 'Best effort, never a crash', b: 'Every probe is allowed to fail. What it couldn’t read becomes a note in the report instead of an error — including inside a VM.' },
    { t: 'Explainable by design', b: 'Thresholds live in one file, capability weights in another. Every finding is an id plus numbers; the words are a catalog you can read and translate.' }
  ];
  const faq = [
    { q: 'Is it free?', a: 'Yes. The app and the engine are open source under the MIT license. A Pro tier is planned for live regional prices and refurbished/used comparisons on recommendations; the findings, scores and recommendations themselves stay free.' },
    { q: 'Does it send my hardware details anywhere?', a: 'No. There is no network code in the engine, and this website has no analytics or cookies either.' },
    { q: 'Why would it ask for my password?', a: 'Only if you turn on drive-health checks. Reading SMART data needs elevated rights; the prompt comes from your OS, and you can decline.' },
    { q: 'Where do the prices come from?', a: 'Rough USD bands per part class — enough to decide whether an upgrade is worth it, not a shop. Recommendations carry a search query you can paste into any retailer.' },
    { q: 'Which machines does it understand?', a: 'Laptops, desktops, mini PCs and servers on Linux, macOS and Windows. In a VM it tells you which numbers not to trust.' },
    { q: 'What languages?', a: 'English and Español today. A language is one JSON file; the report can be re-rendered in any of them after the fact.' }
  ];
</script>

<Seo title="innards" description="innards looks inside your computer and explains what it found — in plain words, with more detail, or with the raw evidence. Then it scores what the machine is still good for and tells you what's worth buying, if anything. Nothing leaves your machine. Free, open source, for Linux, macOS and Windows." />

<!-- Hero -->
<section class="hero">
  <div class="wrap copy">
    <p class="eyebrow">System diagnostics, explained</p>
    <h1 use:fit>Your computer, explained.</h1>
    <p class="lead">innards looks inside your machine and tells you what it found — in plain words, with more detail, or with the raw evidence. Then it says what the machine is still good for, and whether anything is worth buying. Nothing leaves your computer.</p>
  </div>
  <div class="wrap art" id="report">
    <ReportCard machine={x1} wide />
    <OsSwitch />
  </div>
  <div class="wrap copy after">
    <div class="ctas">
      <a href="/download" class="btn primary lg">Download — free</a>
      <a href="#capabilities" class="btn lg">See how it works</a>
    </div>
    <p class="fine muted">Linux · macOS · Windows · Open source, MIT</p>
  </div>
</section>

<!-- Capabilities -->
<section class="block" id="capabilities">
  <div class="wrap">
    <div class="head">
      <p class="eyebrow">Capability scores</p>
      <h2 use:fit>What it's still good for</h2>
      <p class="lead">Ten workloads, scored 0–100 with weights you can read in the source. Each score names what's holding it back — so "not enough memory" is a measurement, not a sales pitch.</p>
    </div>
    <CapabilityDemo />
  </div>
</section>

<!-- Advisor -->
<section class="block" id="advisor">
  <div class="wrap">
    <div class="head">
      <p class="eyebrow">Upgrade advisor</p>
      <h2 use:fit>What to buy. Or not.</h2>
      <p class="lead">A short questionnaire, combined with the snapshot, returns ranked and costed recommendations: free fixes first, then upgrades, and "keep it" when that's the honest answer. Try it on the ThinkPad.</p>
    </div>
    <AdvisorDemo />
  </div>
</section>

<!-- Levels -->
<section class="block" id="levels">
  <div class="wrap">
    <div class="head">
      <p class="eyebrow">The twist</p>
      <h2 use:fit>Same finding, three ways</h2>
      <p class="lead">A finding is facts, not prose. You choose how it's told: <strong>Plain</strong> for no jargon, <strong>Informed</strong> for the default, <strong>Expert</strong> for the numbers and the raw evidence. Switch any time.</p>
    </div>
    <LevelDemo />
  </div>
</section>

<!-- Privacy / principles -->
<section class="block" id="privacy">
  <div class="wrap">
    <div class="head">
      <p class="eyebrow">Privacy</p>
      <h2 use:fit>Nothing leaves your machine</h2>
    </div>
    <ul class="principles">
      {#each principles as p}
        <li><h3>{p.t}</h3><p>{p.b}</p></li>
      {/each}
    </ul>
  </div>
</section>

<!-- Engine -->
<section class="block" id="engine">
  <div class="wrap grid-2 eng">
    <div class="head">
      <p class="eyebrow">Under the hood</p>
      <h2 use:fit>An engine first, an app second</h2>
      <p class="lead"><code>innards-core</code> is a UI-agnostic Rust crate: probe → snapshot → findings → render. The desktop app is a thin Tauri shell around it. Embed the crate in your own tooling, or run the CLI example.</p>
      <div class="ctas"><a href={GITHUB_URL} rel="noopener" class="btn">View source on GitHub</a></div>
    </div>
    <pre><code><span class="c"># the whole pipeline, no GUI</span>
cargo run -p innards-core --example report -- en plain

<span class="c"># or pick a level and dump JSON</span>
cargo run -p innards-core --example report -- es expert --json</code></pre>
  </div>
</section>

<!-- FAQ -->
<section class="block" id="faq">
  <div class="wrap">
    <div class="head"><h2 use:fit>Questions</h2></div>
    <dl class="faq">
      {#each faq as f}
        <div><dt>{f.q}</dt><dd>{f.a}</dd></div>
      {/each}
    </dl>
  </div>
</section>

<!-- CTA -->
<section class="block cta">
  <div class="wrap">
    <h2 use:fit>See what's inside.</h2>
    <p class="lead">Free, open source, and honest about what your machine can and can't do.</p>
    <div class="ctas">
      <a href="/download" class="btn primary lg">Download innards</a>
      <a href={GITHUB_URL} rel="noopener" class="btn lg">GitHub</a>
    </div>
  </div>
</section>

<style>
  .hero { padding-top: clamp(24px, 4.5vw, 52px); }
  .hero .copy { display: grid; grid-template-columns: minmax(0, 1fr); gap: clamp(12px, 1.8vw, 20px); justify-items: start; }
  .hero .eyebrow { margin-bottom: -2px; }
  .hero .lead { max-width: 60ch; }
  .hero .art { margin-top: clamp(18px, 2.6vw, 30px); display: grid; gap: 8px; }
  .hero .after { margin-top: clamp(16px, 2.4vw, 26px); gap: 10px; }
  .ctas { display: flex; flex-wrap: wrap; gap: 10px; }
  .fine { font-size: 0.85rem; }
  .art { min-width: 0; }

  .principles { list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; }
  @media (min-width: 560px) { .principles { grid-template-columns: 1fr 1fr; } }
  @media (min-width: 1000px) { .principles { grid-template-columns: repeat(4, 1fr); } }
  .principles li { background: var(--bg-elev); border: 1px solid var(--line); border-radius: var(--radius); padding: 18px; display: grid; gap: 8px; align-content: start; }
  .principles h3 { font-size: 1.05rem; }
  .principles p { font-size: 0.93rem; color: var(--ink-2); text-wrap: pretty; }

  .eng { align-items: center; }
  .eng .head { display: grid; gap: 12px; }
  .eng pre { min-width: 0; }
  .eng .c { color: var(--ink-3); }

  .faq { margin: 0; display: grid; gap: 0 32px; }
  @media (min-width: 800px) { .faq { grid-template-columns: 1fr 1fr; } }
  .faq div { border-top: 1px solid var(--line); padding: 14px 0; display: grid; gap: 5px; }
  .faq dt { font-weight: 650; font-family: var(--font-display); font-size: 1.05rem; letter-spacing: -0.01em; }
  .faq dd { margin: 0; color: var(--ink-2); font-size: 0.95rem; text-wrap: pretty; }

  .cta .wrap { display: grid; gap: 14px; justify-items: start; }
  .cta h2 { font-size: clamp(2rem, 6.5vw, 4rem); }
</style>
