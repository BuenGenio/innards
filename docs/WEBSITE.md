# Website: which one, and how the two efforts merge

**Decision (2026-09-21): `site/` is the canonical innards.app website.** It is live on Cloudflare Pages
(project `innards`, https://innards.pages.dev, custom domains `innards.app` and `www.innards.app`) and carries
the Stripe billing endpoints as Pages Functions (`site/functions/api/[[route]].ts` → `billing/src/index.ts`).

`web/` (SvelteKit, built by the rusoriz session) is **not** a second site. What it has that `site/` lacks — and
should keep — is the interactive demo: a TypeScript port of `capability.rs`/`advisor.rs` running on real
snapshots, so visitors can try the advisor in the browser. Plan:

1. In `web/`, build the demo as a static, self-contained bundle (SvelteKit static adapter with `paths.base = '/demo'`,
   or a single Vite library build producing `demo.js` + `demo.css`). No SvelteKit routes other than the demo.
2. Copy the output to `site/public/demo/` and link it from `site/public/index.html` ("Try the advisor on a real
   ThinkPad / on a OnePlus 6T"). It must work with `site/public/_headers` CSP (self-hosted assets only).
3. Keep the TS port honest: `web/README.md` already says it's a line-for-line port; add a check that
   `web/src/lib/data/en.json` equals `crates/innards-core/i18n/en.json` (CI can `diff` them).
4. Delete `web/` once the demo lives in `site/public/demo/`.

Why `site/` and not `web/`: it's already deployed with billing and SEO, has every page (pricing, press, privacy,
thanks), needs no build step, and matches the app's design tokens exactly. Why keep the demo: it's the one thing on
a marketing page that proves the product instead of describing it.

Deploy: `cd site && pnpm dlx wrangler@4 pages deploy --project-name innards --branch main --commit-dirty=true`.
Secrets live on the Pages project (`wrangler pages secret put … --project-name innards`), never in the repo.
