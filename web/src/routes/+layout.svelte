<script lang="ts">
  import '../app.css';
  import Header from '$lib/components/Header.svelte';
  import Footer from '$lib/components/Footer.svelte';
  let { children } = $props();
</script>

<a href="#main" class="skip">Skip to content</a>
<Header />
<main id="main" class="page">
  {@render children()}
</main>
<Footer />

<style>
  .page { padding-top: var(--header-h); } /* header is fixed, not in flow */
  .skip { position: absolute; left: -999px; top: 8px; z-index: 100; background: var(--ink); color: var(--bg); padding: 8px 12px; border-radius: 8px; }
  .skip:focus { left: 8px; }
</style>
