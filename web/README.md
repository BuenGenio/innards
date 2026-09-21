# innards.io

Marketing site for innards. SvelteKit (static prerender) → Cloudflare Workers static assets. No server code, no analytics, no third-party requests (fonts are self-hosted; the download page calls the GitHub releases API from the browser).

```
pnpm install
pnpm dev                          # http://localhost:5178
pnpm check                        # svelte-check
pnpm build                        # → ./build
pnpm wrangler deploy --env preview   # workers.dev preview
pnpm deploy                       # production (custom domain routes in wrangler.toml)
```

`VITE_SITE_URL` (default `https://innards.io`) is baked into canonical/OG URLs and the sitemap at build time.

## Where the demo data comes from

`src/lib/data/en.json` is a verbatim copy of `crates/innards-core/i18n/en.json`. `src/lib/engine/{capability,advisor}.ts` are line-for-line ports of the Rust modules so the interactive demos run the real logic; `src/lib/data/machines.ts` holds two real snapshots captured with `cargo run -p innards-core --example report -- en informed --json`. If the engine's weights or catalog change, copy the JSON and diff the ports.

The OG image is rendered from a small HTML page with Playwright (see git history / scratch script); `static/og.png` is checked in.
