<script lang="ts">
  // Prerendered to build/404.html — Cloudflare serves it (status 404) for unknown paths.
  import { fit } from '$lib/actions/fit';
</script>

<svelte:head><title>Not found · innards</title><meta name="robots" content="noindex" /></svelte:head>
<section class="block">
  <div class="wrap err">
    <p class="eyebrow">404</p>
    <h1 use:fit>Nothing here</h1>
    <p class="lead">That page doesn’t exist. The report, however, is still on your machine.</p>
    <a href="/" class="btn primary">Back to innards</a>
  </div>
</section>

<style>
  .err { display: grid; gap: 14px; justify-items: start; min-height: 40vh; align-content: center; }
</style>
