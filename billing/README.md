# innards-billing (Cloudflare Worker)

Stripe Checkout → offline Ed25519 license keys. Same key format the desktop app verifies
(`src-tauri/src/license.rs`); the worker holds the private half as a secret.

## Routes
| Route | Purpose |
|---|---|
| `GET /api/checkout?tier=supporter\|pro\|team[&qty=5][&org=acme]` | 303 to Stripe Checkout |
| `POST /api/stripe/webhook` | `checkout.session.completed` → issue key; `invoice.paid` → renew; `customer.subscription.deleted` → revoke |
| `GET /api/key?session_id=cs_…` | Key for the thank-you page (falls back to Stripe if the webhook is late) |
| `GET /api/license/refresh?key=…` | New key with the current period end while the subscription is active |
| `GET /api/license/status?key=…` | `{ valid, tier, exp, subscription, org }` |

## One-time setup
```bash
cd billing && pnpm install
STRIPE_SECRET_KEY=sk_live_… ./scripts/setup-stripe.sh      # prints PRICE_* → paste into wrangler.toml
pnpm wrangler kv namespace create LICENSES                   # paste id into wrangler.toml
pnpm wrangler secret put STRIPE_SECRET_KEY
pnpm wrangler secret put LICENSE_SIGNING_KEY < ~/.config/innards/signing.key
pnpm wrangler deploy
# In Stripe → Developers → Webhooks: add https://<worker>/api/stripe/webhook with events
#   checkout.session.completed, invoice.paid, customer.subscription.deleted
pnpm wrangler secret put STRIPE_WEBHOOK_SECRET
```
Route the worker under the site: in Cloudflare, add a route `innards.app/api/*` → `innards-billing`
(or a `[routes]` block in wrangler.toml once the zone exists).

## Key payload
`{ tier, email, iat, exp?, org?, sub?, cust? }` — `sub`/`cust` are Stripe ids used only by `/api/license/refresh`.
Supporter keys have no `exp`. Pro/Team keys expire at period end + `GRACE_DAYS`; the desktop app refreshes
them automatically while the subscription is active.
