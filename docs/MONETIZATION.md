# Monetization

How the tiers are defined, where each one is enforced in code, how keys are made and delivered, and what is still missing. Prices are the product decision; everything else below is read from the code.

## 1. Tiers

| Tier | Price | Unlocks | Key |
|---|---|---|---|
| **Free** | $0 | Full report, three levels, all languages, Markdown export | none |
| **Supporter** | $9 one-time, pay what you want from $5 | Upgrade advisor, narrated summaries (Claude API, user's own key), machine history | `tier: "supporter"`, no `exp` |
| **Pro** | $29 / year | Where-to-buy links (new / refurbished / used, by region); open another machine's exported report (`load_report_json`) | `tier: "pro"`, `exp` yearly |
| **Team** | $4 per machine / month or $39 per machine / year, minimum 5 machines | Fleet dashboard, health over time, upgrade budget planning, CSV export (PDF on the roadmap), auto-upload on schedule | `tier: "team"`, `exp`, `org` |
| **Enterprise** | Custom, from $2,500 / year | Self-hosted backend, SSO/SAML, RBAC, hash-chained audit log, data residency, custom rules, SLA and priority support | `tier: "enterprise"`, `exp`, `org` |

The `Tier` enum (`src-tauri/src/license.rs`) is ordered `Free < Supporter < Pro < Team < Enterprise` and every check is `tier < Required`, so each tier includes everything below it. The frontend mirrors this with `app.has("<tier>")` (`src/lib/state.svelte.ts`).

## 2. What is gated where

| Feature | Backend check (`src-tauri/src/lib.rs`) | Frontend behaviour | Status |
|---|---|---|---|
| Report, levels, languages, rescan, Markdown export | none | always on | **Built** |
| Upgrade advisor (`advise`) | **none** — `advisor::recommend` runs for anyone who calls the command | `Advisor.svelte`: Supporter `Paywall` shown and "Show recommendations" disabled when `!app.has("supporter")`; live re-run on answer change only when the tier is not free | **Built**, UI-gated only |
| Narrated summary (`narrate`) | `tier < Tier::Supporter` → `"supporter"`; then `"no_api_key"` without a key | `Narrative.svelte`: compact Supporter `Paywall`; key prompt on `no_api_key` | **Built** |
| Machine history | `history`, `history_clear` (src-tauri/src/history.rs → `history.jsonl` next to settings.json; one line per scan) | Supporter | Built |
| Where-to-buy links (`shop_links`) | `tier < Tier::Pro` → `"pro"` | `Advisor.svelte`: "Where to buy" button when `app.has("pro")`, otherwise a grey "— Pro" hint and a compact Pro `Paywall` under the results | **Built** (search links; no live prices) |
| Open another machine's report | `load_report_json`, `viewing_other_machine`, `read_text` (Report → Open report…); Export JSON is free | Pro | Built (view one at a time; side-by-side compare is roadmap) |
| Cloud upload (`cloud_upload`, auto-upload in `analyze`) | `tier < Tier::Team` → `"team"`; `"no_endpoint"` / `"no_token"` | `Settings.svelte` "Team & Enterprise" card: inputs disabled below Team, "Upload report now", auto-upload checkbox | **Built** (manual + after-every-scan; no scheduler) |
| Fleet dashboard, health over time, upgrade budget, record panels | Threadwise plugin `integrations/threadwise-fleet/` | Threadwise web (`web/FleetPage.tsx`, `MachineDetailPage.tsx`) | **Built** in the plugin (see `docs/CLOUD.md`) |
| CSV export (`GET /api/ext/innards.fleet/machines.csv`), scheduled auto-upload (`cloud_interval_hours` in settings; `start_scheduler` in src-tauri/src/lib.rs) | plugin + app | — | Built (PDF export and alerts remain roadmap) |
| SSO/SAML, RBAC, audit log, residency, sandboxes | Threadwise itself (`AUTH_MODE=session`, roles, governance) | Threadwise Settings | **Provided by Threadwise**; nothing Innards-specific to build |
| Custom rules, SLA, priority support | — | — | Contractual; no code |

The tier pill in the top bar shows the raw tier string (`TopBar.svelte`). `Settings.svelte` shows tier, email, org, expiry, and `INNARDS_TIER` when the override is active.

## 3. License keys

### Format

```
INNARDS-<base64url(payload JSON)>.<base64url(Ed25519 signature)>
```

No padding in either part. The signature is over the raw payload bytes. Payload fields (`license.rs::Payload`, `billing/src/index.ts::Payload`):

| Field | Type | Meaning |
|---|---|---|
| `tier` | `supporter \| pro \| team \| enterprise` | |
| `email` | string | The buyer; shown in Settings, otherwise unused |
| `iat` | `YYYY-MM-DD` | Issued |
| `exp` | `YYYY-MM-DD`, optional | Verification fails from the day after; compared as strings against today's UTC date. Absent on Supporter keys. |
| `org` | slug, optional | Team/Enterprise: the organisation; surfaced as `LicenseStatus.org` |
| `sub`, `cust` | Stripe ids, optional | Set by the billing worker on Pro/Team keys; `sub` present ⇒ `renewable: true` |

Verification (`license.rs::verify`) is fully offline against `PUBLIC_KEY_B64`, a constant in the binary. Unknown payload fields are ignored, so the format can grow. There is no device binding, no activation count and no revocation list in the app: a key works on any number of machines until `exp`.

Example payload: `{"tier":"pro","email":"ana@example.com","iat":"2026-09-21","exp":"2027-09-24","sub":"sub_…","cust":"cus_…"}`.

### Developer override

`INNARDS_TIER=free|supporter|pro|team|enterprise` (plus optional `INNARDS_ORG`) makes `license::status` return that tier with `source: "env"` regardless of any key. Note that with the override active, `activate_license` accepts *any* string and saves it (the `"invalid"` check is skipped when `source == "env"`).

### Issuing by hand — `tools/innards-license`

```bash
cargo run -p innards-license -- keygen                 # once; writes ~/.config/innards/signing.key (0600), prints the public key
cargo run -p innards-license -- sign supporter ana@example.com            # no expiry
cargo run -p innards-license -- sign pro ana@example.com 366              # expires in 366 days (default for non-supporter)
cargo run -p innards-license -- sign team ops@acme.example 366 acme       # org slug required for team/enterprise
cargo run -p innards-license -- sign enterprise it@acme.example 366 acme
cargo run -p innards-license -- verify INNARDS-…                          # checks against the local private key's public half
```

`keygen` refuses to overwrite an existing key. The public key it prints must match `license.rs::PUBLIC_KEY_B64` — rotating the key means shipping a new app build and re-issuing every key (or verifying against a list of public keys, which the code does not do today). The tool's own usage line still says `sign <supporter|pro>` although it accepts all four tiers.

### Issuing automatically — `billing/` (Cloudflare Worker)

`billing/src/index.ts` is a dependency-free Worker that talks to Stripe over REST and signs keys with the same seed (`LICENSE_SIGNING_KEY` secret = the contents of `signing.key`) using WebCrypto Ed25519. Routes:

| Route | What it does |
|---|---|
| `GET /api/checkout?tier=supporter\|pro\|team[&qty=5][&org=acme]` | Creates a Stripe Checkout Session (automatic tax on, tax-id collection on, promotion codes allowed) and 303-redirects to it. Supporter = `mode: payment` on a price with `custom_unit_amount` (min $5, preset $9, max $100). Pro = yearly subscription. Team = monthly subscription with adjustable quantity, minimum `TEAM_MIN_MACHINES` (5), max 1000; `org` is slugified into session metadata. Success URL `${SITE_URL}/thanks?session_id={CHECKOUT_SESSION_ID}`, cancel URL `${SITE_URL}/pricing`. |
| `POST /api/stripe/webhook` | HMAC-verified (5-minute tolerance). `checkout.session.completed` → `issueForSession` signs a key (Team keys get `org` from metadata or `org_<last 8 of customer id>`) and stores it in KV under `session:`, `customer:` and `email:`. `invoice.paid` → re-signs the customer's key with the new period end. `customer.subscription.deleted` → writes `revoked:<sub>`. |
| `GET /api/key?session_id=cs_…` | The thank-you page polls this; falls back to issuing directly from the Stripe session if the webhook has not landed. |
| `GET /api/license/refresh?key=…` | For keys with `sub`: refuses if `revoked:` exists or the subscription is not `active`/`trialing`/`past_due` (`402 cancelled` / `402 inactive`), otherwise returns a re-signed key with `exp` = period end + `GRACE_DAYS` (3). One-time keys are returned unchanged. The desktop app calls this (`license_refresh`). |
| `GET /api/license/status?key=…` | `{ valid, tier, exp, subscription, org }` |
| `GET /api/health` | |

Setup is in `billing/README.md`: `scripts/setup-stripe.sh` creates the three products/prices idempotently by `lookup_key` (`innards_supporter`, `innards_pro`, `innards_team`; USD; `tax_behavior=exclusive`), then secrets for `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `LICENSE_SIGNING_KEY` and a KV namespace `LICENSES`. Deployment is through the website: `site/wrangler.toml` is a Cloudflare **Pages** project (`innards`, output `site/public/`) whose `[vars]` carry the same `PRICE_*` / `GRACE_DAYS` settings, and `site/functions/api/[[route]].ts` forwards every `/api/*` request to the Worker module (`import worker from "../../../billing/src/index"`), so site and billing share one deploy and one domain. `billing/wrangler.toml` still exists for running the Worker on its own (`wrangler dev`); `billing/.dev.vars` (gitignored) holds local secrets.

Pro/Team keys therefore expire at the end of each billing period plus three days; the app renews them silently at startup while the subscription is active (`state.svelte.ts::maybeRenew`, when within 30 days of `exp`). A cancelled subscription simply stops renewing and the key lapses at `exp`.

## 4. Store plan

**What the code implements: Stripe as the payment processor**, with Stripe Tax (`automatic_tax[enabled]=true`) calculating VAT/GST and `tax_id_collection` for B2B reverse charge. Under this model *the seller* is the merchant of record: Stripe calculates and can collect tax, but registering for VAT in each jurisdiction and filing returns stays with the seller (Stripe Tax can be paired with a filing service, or with a registered EU OSS return). For a solo developer selling globally this is the main operational cost of the Stripe route.

**The alternative discussed for launch: a merchant of record** (Paddle or Lemon Squeezy). The MoR is the seller on the invoice, handles VAT/GST registration, collection and remittance worldwide, and takes roughly 5 % + a fixed fee per transaction on top of card costs. Both support pay-what-you-want (Lemon Squeezy natively; Paddle via a custom price or multiple price points), subscriptions with quantity (Team seats), and webhooks. Switching would keep the key pipeline intact: replace `checkout()` and `webhook()` in `billing/src/index.ts` with the MoR's checkout link and its `order_created` / `subscription_payment_success` / `subscription_cancelled` events, keep `signKey`, `refresh` and `status` as they are.

Decision criteria: choose the MoR if the buyer base is materially outside the seller's home tax jurisdiction at launch (it will be); choose Stripe if margin matters more than filing effort or if an accountant already handles cross-border VAT.

Either way the flow is:

```
pricing page → checkout (hosted) → payment → webhook → sign key → show on thank-you page (+ email)
```

Delivery today is the thank-you page only (`site/public/thanks.html` fetches `/api/key?session_id=`). Email delivery of the key is not implemented; Stripe's own receipt email does not contain the key. Until checkout is live, `site/public/_redirects` sends `/buy` and `/buy/*` to the pricing page and the pricing page says so; the in-app `Paywall.svelte` still points at `https://innards.app/buy?tier=…` (a different domain from `innards.app`).

## 5. Team and Enterprise via Threadwise

Team and Enterprise do not add features to the desktop app beyond `cloud_upload`; the product is the fleet dashboard, which is a Threadwise plugin (`integrations/threadwise-fleet/`). `docs/CLOUD.md` has the architecture and the ingest contract. The commercial mechanics:

**Provisioning a workspace.**
1. The customer runs Threadwise (self-hosted via `docker compose`, or a hosted instance — see §6) with the plugin loaded: `THREADWISE_PLUGINS=/path/to/innards/integrations/threadwise-fleet/server` and, for the pages, `THREADWISE_WEB_PLUGINS=/path/to/innards/integrations/threadwise-fleet/web` at web build time.
2. Threadwise runs in `AUTH_MODE=session`; the customer's admin creates the organisation (= workspace) and enables the plugin in Settings → Plugins (retention days, whether unknown machines may report).
3. The admin creates a **workspace-scoped API key**: `POST /api/api-keys { name, scopes: ["sources:manage"], expiresInDays? }` (needs `settings:manage`; Settings → API keys in the UI). The key is `tw_` + 40 characters, shown once, stored as a sha256, bound to that one workspace, rate-limited at 600 requests/minute (Threadwise `docs/AUTH.md`). `sources:manage` is the permission the plugin requires for writes (`requirePermissionForWrites('sources:manage')` on every route; reads are open to any member).
4. Each machine gets the Threadwise base URL and the same `tw_` key in Innards → Settings → Team & Enterprise, plus a label. One key per workspace is the intended model; per-machine keys are possible (Threadwise allows many keys) but not required.
5. The Team license key (with `org`) is what unlocks the Settings card and the `cloud_upload` command; the app does not check that `org` matches the workspace, and the plugin does not see the license key at all. The Threadwise API key is the actual credential.

**Per-machine billing.** Stripe bills the Team subscription by quantity (`adjustable_quantity`, minimum 5). The number to bill is *distinct machine ids seen in the last 30 days*: the plugin marks a machine `stale` after `STALE_AFTER_DAYS = 30` without a report and `GET /api/ext/innards.fleet/analytics` returns `machines` and `staleMachines`, so `machines − staleMachines` is the billable count. Reconciling that count with the Stripe quantity is a manual step today (there is no job that reads a workspace and updates the subscription); the first version can be a monthly script that calls `/analytics` with the workspace key and `POST /v1/subscription_items/:id` with the new quantity, or simply trusts the customer's declared quantity and audits quarterly. Machine ids are one-way hashes (`cloud.rs::machine_id`), stable across reinstalls of the same hardware, so a machine that is wiped and re-enrolled still counts once.

**Enterprise** is the same plugin on a Threadwise deployment the customer controls, with the Threadwise features that make it enterprise-grade (SSO/SAML, roles, audit chain, residency, sandboxes — `docs/CLOUD.md` §6), an `enterprise` key issued by hand with `tools/innards-license`, and a contract for SLA, support and custom rules. Custom rules are a fork of `rules.rs` + catalogs today; there is no rule plugin mechanism.

## 6. Refund policy (suggestion)

- **Supporter** and **Pro**: 14 days, no questions asked, for the first purchase; refund through Stripe (or the MoR) and write `revoked:<sub>` / delete the KV entries so `refresh` stops. The key keeps verifying offline until `exp` (Supporter keys never expire) — accept that; it is a $9 product and the alternative is phoning home.
- **Pro renewals**: refund the renewal if asked within 14 days of the charge.
- **Team**: monthly plans can be cancelled any time (no partial-month refunds); yearly plans pro-rata refund of unused full months in the first 60 days, then none. Machines above the paid quantity are billed in arrears rather than blocked — the plugin never refuses an upload for billing reasons.
- **Enterprise**: per contract; default to the Team terms.
- Publish the policy on `/pricing` and in the Stripe Checkout terms, and state that keys are offline and not bound to a device, so "I lost my key" is answered by re-sending from KV (`email:<address>`) rather than by re-issuing.

## 7. Not built yet (honest list)

| Item | State |
|---|---|
| **Live checkout** | Worker code exists (`billing/`), the Pages Function wrapper exists (`site/functions/api/[[route]].ts`), the Stripe products script exists, but both `wrangler.toml` files have empty `PRICE_*`, nothing is deployed, `site/public/_redirects` sends `/buy` to the pricing page, and `Paywall.svelte` links to `innards.app/buy`. The site's own note says "Checkout is not live yet". |
| **Key delivery email** | Not implemented. The key is shown on `thanks.html` and stored in KV; there is no transactional email, and no "resend my key" endpoint (KV has `email:<address>` entries, so one is easy to add). |
| **Key delivery email** | Only the thank-you page shows the key (re-fetchable by session URL); no transactional email yet. |
| **Side-by-side compare** (Pro) | You can open one exported report at a time; a two-column diff is roadmap. |
| **PDF export, alerts** (Team) | Roadmap. CSV export and the in-app scheduler (rescan + upload every N hours while the app is open) exist; there is no background service when the app is closed and no alerting. |
| **Threadwise Cloud hosting** | Threadwise's own docs point at `cloud.threadwise.app` as a placeholder; there is no hosted Threadwise with the `innards.fleet` plugin. Team customers self-host today. |
| **Billing ↔ machine-count reconciliation** | Manual (see §5). |
| **Key rotation / multiple public keys** | The app verifies against a single embedded key. |
| **MoR decision** | Code assumes Stripe; a Paddle/Lemon Squeezy swap touches only `checkout()` and `webhook()` in the Worker. |
| **Enterprise custom rules** | No plugin mechanism; forks only. |
