<script lang="ts">
  import Seo from '$lib/components/Seo.svelte';
  import { fit } from '$lib/actions/fit';
  import { GITHUB_URL } from '$lib/site';
</script>

<Seo title="Privacy" description="innards has no network code: the report never leaves your machine. This website sets no cookies and runs no analytics." />

<section class="block top">
  <div class="wrap narrow">
    <div class="head">
      <p class="eyebrow">Privacy</p>
      <h1 use:fit>Short, because there's little to say</h1>
      <p class="lead">innards is built so that there is nothing to collect. This page covers the app and this website.</p>
    </div>

    <h2 use:fit>The app</h2>
    <ul>
      <li><strong>No network access.</strong> The engine (<code>innards-core</code>) contains no networking code. It reads hardware and OS information from your own machine and writes the report to your screen or to a Markdown file you choose.</li>
      <li><strong>What it reads:</strong> CPU, memory, storage and filesystem usage, GPU, battery, temperatures and fan speeds, network interface types, load averages, the names and memory use of your largest processes, and — only if you enable it — drive health (SMART) data.</li>
      <li><strong>Elevated rights</strong> are requested only for SMART data, through your operating system's own prompt. Declining disables that one probe and nothing else.</li>
      <li><strong>No accounts, no telemetry, no crash reporting.</strong> If a probe fails, that becomes a line in your report, not a message to us.</li>
      <li><strong>Shopping links</strong> in recommendations are plain search URLs to third-party retailers, built locally. Nothing is sent until you choose to open one, and what that retailer does then is governed by its own policy.</li>
    </ul>

    <h2 use:fit>This website</h2>
    <ul>
      <li><strong>No cookies, no analytics, no third-party scripts or fonts.</strong> Everything is served from this domain.</li>
      <li>The site is static and hosted on <a href="https://www.cloudflare.com/" rel="noopener">Cloudflare</a>, which keeps standard, short-lived request logs (IP address, user agent, page requested) to operate and protect the service.</li>
      <li>The download page asks the GitHub API for the latest release so it can show current files; that request goes to GitHub and is subject to <a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement" rel="noopener">GitHub's privacy statement</a>.</li>
    </ul>

    <h2 use:fit>Questions</h2>
    <p>Open an issue on <a href={GITHUB_URL} rel="noopener">GitHub</a>. The source is public, so you can also check any of the above yourself.</p>
    <p class="muted small">Last updated 21 September 2026.</p>
  </div>
</section>

<style>
  .top { padding-top: clamp(28px, 5vw, 56px); }
  .narrow { max-width: 760px; display: grid; gap: 14px; }
  h2 { margin-top: 18px; font-size: clamp(1.3rem, 3vw, 1.7rem); }
  ul { margin: 0; padding-left: 1.2em; display: grid; gap: 8px; color: var(--ink-2); }
  li strong { color: var(--ink); }
  .small { font-size: 0.86rem; }
</style>
