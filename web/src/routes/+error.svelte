<script lang="ts">
  import { page } from '$app/state';
  import { fit } from '$lib/actions/fit';
</script>

<svelte:head><title>{page.status} · innards</title></svelte:head>
<section class="block">
  <div class="wrap err">
    <p class="eyebrow">{page.status}</p>
    <h1 use:fit>{page.status === 404 ? 'Nothing here' : 'Something broke'}</h1>
    <p class="lead">{page.status === 404 ? 'That page doesn’t exist. The report, however, is still on your machine.' : page.error?.message}</p>
    <a href="/" class="btn primary">Back to innards</a>
  </div>
</section>

<style>
  .err { display: grid; gap: 14px; justify-items: start; min-height: 40vh; align-content: center; }
</style>
