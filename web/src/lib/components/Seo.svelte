<script lang="ts">
  import { page } from '$app/state';
  import { SITE_NAME, SITE_URL } from '$lib/site';
  let { title, description, image = '/og.png' }: { title: string; description: string; image?: string } = $props();
  const full = $derived(title === SITE_NAME ? `${SITE_NAME} — your computer, explained` : `${title} · ${SITE_NAME}`);
  const url = $derived(`${SITE_URL}${page.url.pathname === '/' ? '' : page.url.pathname}`);
</script>

<svelte:head>
  <title>{full}</title>
  <meta name="description" content={description} />
  <link rel="canonical" href={url || SITE_URL} />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content={SITE_NAME} />
  <meta property="og:title" content={full} />
  <meta property="og:description" content={description} />
  <meta property="og:url" content={url || SITE_URL} />
  <meta property="og:image" content="{SITE_URL}{image}" />
  <meta property="og:image:width" content="1200" />
  <meta property="og:image:height" content="630" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content={full} />
  <meta name="twitter:description" content={description} />
  <meta name="twitter:image" content="{SITE_URL}{image}" />
</svelte:head>
