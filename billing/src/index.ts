// Innards billing worker: Stripe Checkout → Ed25519 license keys.
//
//   GET  /api/checkout?tier=supporter|pro|team[&qty=N]   → 303 to Stripe Checkout
//   POST /api/stripe/webhook                             → issues / renews keys
//   GET  /api/key?session_id=cs_…                        → { key, tier, email, exp } for the thank-you page
//   GET  /api/license/refresh?key=INNARDS-…              → a renewed key while the subscription is active
//   GET  /api/license/status?key=INNARDS-…               → { valid, tier, exp, subscription }
//   GET  /api/health
//
// Keys are the same format the desktop app verifies offline (src-tauri/src/license.rs):
// INNARDS-<b64url(json payload)>.<b64url(ed25519 signature)>. Subscription keys carry
// `sub` (Stripe subscription id) so the app can ask for a renewal without a login.
// No Stripe SDK: the REST API is small enough to call with fetch, and it keeps the
// worker dependency-free.

export interface Env {
  STRIPE_SECRET_KEY: string;
  STRIPE_WEBHOOK_SECRET: string;
  LICENSE_SIGNING_KEY: string;
  SITE_URL: string;
  PRICE_SUPPORTER: string;
  PRICE_PRO: string;
  PRICE_TEAM: string;
  SUPPORTER_MIN_CENTS: string;
  SUPPORTER_PRESET_CENTS: string;
  TEAM_MIN_MACHINES: string;
  GRACE_DAYS: string;
  LICENSES: KVNamespace;
}

type Tier = "supporter" | "pro" | "team";

interface Payload {
  tier: Tier | "enterprise";
  email: string;
  iat: string;
  exp?: string;
  org?: string;
  /** Stripe subscription id, present on pro/team keys. */
  sub?: string;
  /** Stripe customer id. */
  cust?: string;
}

// ------------------------------------------------------------------ helpers

const json = (data: unknown, status = 200, headers: Record<string, string> = {}) =>
  new Response(JSON.stringify(data), { status, headers: { "content-type": "application/json", "cache-control": "no-store", ...headers } });

const err = (code: string, message: string, status = 400) => json({ error: { code, message } }, status);

const b64url = {
  encode(bytes: Uint8Array): string {
    let s = "";
    for (const b of bytes) s += String.fromCharCode(b);
    return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  },
  decode(s: string): Uint8Array {
    const pad = s.length % 4 === 0 ? "" : "=".repeat(4 - (s.length % 4));
    const bin = atob(s.replace(/-/g, "+").replace(/_/g, "/") + pad);
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  },
};

const isoDate = (d: Date) => d.toISOString().slice(0, 10);

// Ed25519 seed (32 bytes) → PKCS#8 DER so WebCrypto can import it.
async function signingKey(env: Env): Promise<CryptoKey> {
  const seed = b64url.decode(env.LICENSE_SIGNING_KEY);
  if (seed.length !== 32) throw new Error("LICENSE_SIGNING_KEY must be a 32-byte seed");
  const prefix = Uint8Array.from([0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20]);
  const pkcs8 = new Uint8Array(prefix.length + seed.length);
  pkcs8.set(prefix);
  pkcs8.set(seed, prefix.length);
  return crypto.subtle.importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"]);
}

async function signKey(env: Env, payload: Payload): Promise<string> {
  const bytes = new TextEncoder().encode(JSON.stringify(payload));
  const sig = new Uint8Array(await crypto.subtle.sign("Ed25519", await signingKey(env), bytes));
  return `INNARDS-${b64url.encode(bytes)}.${b64url.encode(sig)}`;
}

/** Parse a key's payload. Signature is checked by re-signing (the worker holds the private key). */
async function parseKey(env: Env, key: string): Promise<Payload | null> {
  const rest = key.trim().replace(/^INNARDS-/, "");
  const [p] = rest.split(".");
  if (!p) return null;
  try {
    const payload = JSON.parse(new TextDecoder().decode(b64url.decode(p))) as Payload;
    const expected = await signKey(env, payload);
    return expected === key.trim() ? payload : null;
  } catch {
    return null;
  }
}

// ------------------------------------------------------------------ stripe (REST)

async function stripe<T = any>(env: Env, method: "GET" | "POST", path: string, form?: Record<string, string>): Promise<T> {
  const res = await fetch(`https://api.stripe.com/v1${path}`, {
    method,
    headers: {
      authorization: `Bearer ${env.STRIPE_SECRET_KEY}`,
      "content-type": "application/x-www-form-urlencoded",
      "stripe-version": "2025-08-27.basil",
    },
    body: form ? new URLSearchParams(form).toString() : undefined,
  });
  const data = await res.json<any>();
  if (!res.ok) throw new Error(`stripe ${res.status}: ${data?.error?.message ?? "unknown"}`);
  return data as T;
}

async function verifyStripeSignature(env: Env, body: string, header: string | null): Promise<boolean> {
  if (!header) return false;
  const parts = Object.fromEntries(header.split(",").map((kv) => kv.split("=") as [string, string]));
  const t = parts.t;
  const v1 = parts.v1;
  if (!t || !v1) return false;
  if (Math.abs(Date.now() / 1000 - Number(t)) > 300) return false; // 5-minute tolerance
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(env.STRIPE_WEBHOOK_SECRET), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(`${t}.${body}`)));
  const hex = [...mac].map((b) => b.toString(16).padStart(2, "0")).join("");
  // constant-time compare
  if (hex.length !== v1.length) return false;
  let diff = 0;
  for (let i = 0; i < hex.length; i++) diff |= hex.charCodeAt(i) ^ v1.charCodeAt(i);
  return diff === 0;
}

// ------------------------------------------------------------------ issuing

function tierOfPrice(env: Env, priceId: string): Tier | null {
  if (priceId === env.PRICE_SUPPORTER) return "supporter";
  if (priceId === env.PRICE_PRO) return "pro";
  if (priceId === env.PRICE_TEAM) return "team";
  return null;
}

async function issueForSession(env: Env, session: any): Promise<{ key: string; payload: Payload }> {
  const email: string = session.customer_details?.email ?? session.customer_email ?? "";
  const cust: string | undefined = typeof session.customer === "string" ? session.customer : session.customer?.id;
  const lines = await stripe<any>(env, "GET", `/checkout/sessions/${session.id}/line_items?limit=5`);
  const priceId: string = lines.data?.[0]?.price?.id ?? "";
  const tier = tierOfPrice(env, priceId);
  if (!tier) throw new Error(`unknown price ${priceId}`);

  const payload: Payload = { tier, email, iat: isoDate(new Date()), cust };
  if (session.mode === "subscription") {
    const subId: string = typeof session.subscription === "string" ? session.subscription : session.subscription?.id;
    const sub = await stripe<any>(env, "GET", `/subscriptions/${subId}`);
    payload.sub = subId;
    payload.exp = periodEndWithGrace(env, sub);
    if (tier === "team") payload.org = session.metadata?.org || `org_${cust?.slice(-8) ?? subId.slice(-8)}`;
  }
  const key = await signKey(env, payload);
  await env.LICENSES.put(`session:${session.id}`, JSON.stringify({ key, payload }));
  if (cust) await env.LICENSES.put(`customer:${cust}`, JSON.stringify({ key, payload }));
  if (email) await env.LICENSES.put(`email:${email.toLowerCase()}`, JSON.stringify({ key, payload }));
  return { key, payload };
}

function periodEndWithGrace(env: Env, sub: any): string {
  // Basil API: period end lives on the subscription item.
  const end: number = sub.items?.data?.[0]?.current_period_end ?? sub.current_period_end ?? Math.floor(Date.now() / 1000) + 366 * 86400;
  const grace = Number(env.GRACE_DAYS || "3");
  return isoDate(new Date((end + grace * 86400) * 1000));
}

// ------------------------------------------------------------------ routes

async function checkout(env: Env, url: URL): Promise<Response> {
  const tier = url.searchParams.get("tier") as Tier | null;
  if (!tier || !["supporter", "pro", "team"].includes(tier)) return err("bad_tier", "tier must be supporter, pro or team");
  const site = env.SITE_URL.replace(/\/$/, "");
  const form: Record<string, string> = {
    success_url: `${site}/thanks?session_id={CHECKOUT_SESSION_ID}`,
    cancel_url: `${site}/pricing`,
    allow_promotion_codes: "true",
    "automatic_tax[enabled]": "true",
    "tax_id_collection[enabled]": "true",
    "metadata[product]": "innards",
    "metadata[tier]": tier,
  };
  if (tier === "supporter") {
    // One-time, pay what you want (price has custom_unit_amount; see setup-stripe.sh).
    Object.assign(form, { mode: "payment", "line_items[0][price]": env.PRICE_SUPPORTER, "line_items[0][quantity]": "1", "invoice_creation[enabled]": "true" });
  } else if (tier === "pro") {
    Object.assign(form, { mode: "subscription", "line_items[0][price]": env.PRICE_PRO, "line_items[0][quantity]": "1" });
  } else {
    const min = Number(env.TEAM_MIN_MACHINES || "5");
    const qty = Math.max(min, Number(url.searchParams.get("qty") ?? min) || min);
    Object.assign(form, {
      mode: "subscription",
      "line_items[0][price]": env.PRICE_TEAM,
      "line_items[0][quantity]": String(qty),
      "line_items[0][adjustable_quantity][enabled]": "true",
      "line_items[0][adjustable_quantity][minimum]": String(min),
      "line_items[0][adjustable_quantity][maximum]": "1000",
    });
    const org = url.searchParams.get("org");
    if (org) form["metadata[org]"] = org.toLowerCase().replace(/[^a-z0-9-]/g, "-").slice(0, 40);
  }
  const session = await stripe<any>(env, "POST", "/checkout/sessions", form);
  return Response.redirect(session.url, 303);
}

async function webhook(env: Env, req: Request): Promise<Response> {
  const body = await req.text();
  if (!(await verifyStripeSignature(env, body, req.headers.get("stripe-signature")))) return err("bad_signature", "invalid signature", 400);
  const event = JSON.parse(body);
  switch (event.type) {
    case "checkout.session.completed": {
      const s = event.data.object;
      if (s.payment_status === "paid" || s.mode === "subscription") await issueForSession(env, s);
      break;
    }
    case "invoice.paid": {
      // Renewal: re-issue the customer's key with the new period end.
      const inv = event.data.object;
      const subId: string | undefined = typeof inv.subscription === "string" ? inv.subscription : inv.subscription?.id ?? inv.parent?.subscription_details?.subscription;
      const cust: string | undefined = typeof inv.customer === "string" ? inv.customer : inv.customer?.id;
      if (subId && cust) {
        const stored = await env.LICENSES.get(`customer:${cust}`, "json") as { payload: Payload } | null;
        if (stored?.payload) {
          const sub = await stripe<any>(env, "GET", `/subscriptions/${subId}`);
          const payload: Payload = { ...stored.payload, iat: isoDate(new Date()), exp: periodEndWithGrace(env, sub), sub: subId };
          const key = await signKey(env, payload);
          await env.LICENSES.put(`customer:${cust}`, JSON.stringify({ key, payload }));
          if (payload.email) await env.LICENSES.put(`email:${payload.email.toLowerCase()}`, JSON.stringify({ key, payload }));
        }
      }
      break;
    }
    case "customer.subscription.deleted": {
      const sub = event.data.object;
      const cust: string | undefined = typeof sub.customer === "string" ? sub.customer : sub.customer?.id;
      if (cust) await env.LICENSES.put(`revoked:${sub.id}`, isoDate(new Date()));
      break;
    }
    default:
      break;
  }
  return json({ received: true });
}

async function keyForSession(env: Env, url: URL): Promise<Response> {
  const id = url.searchParams.get("session_id");
  if (!id?.startsWith("cs_")) return err("bad_session", "session_id required");
  const stored = await env.LICENSES.get(`session:${id}`, "json") as { key: string; payload: Payload } | null;
  if (stored) return json({ key: stored.key, tier: stored.payload.tier, email: stored.payload.email, exp: stored.payload.exp ?? null });
  // Webhook may not have landed yet: check the session directly.
  const session = await stripe<any>(env, "GET", `/checkout/sessions/${id}`);
  if (session.payment_status !== "paid" && session.mode !== "subscription") return err("unpaid", "payment not completed", 402);
  const { key, payload } = await issueForSession(env, session);
  return json({ key, tier: payload.tier, email: payload.email, exp: payload.exp ?? null });
}

async function refresh(env: Env, url: URL): Promise<Response> {
  const key = url.searchParams.get("key") ?? "";
  const payload = await parseKey(env, key);
  if (!payload) return err("invalid", "key does not verify", 401);
  if (!payload.sub) return json({ key, renewed: false, exp: payload.exp ?? null }); // one-time keys never expire
  if (await env.LICENSES.get(`revoked:${payload.sub}`)) return err("cancelled", "subscription cancelled", 402);
  const sub = await stripe<any>(env, "GET", `/subscriptions/${payload.sub}`);
  if (!["active", "trialing", "past_due"].includes(sub.status)) return err("inactive", `subscription ${sub.status}`, 402);
  const renewed: Payload = { ...payload, iat: isoDate(new Date()), exp: periodEndWithGrace(env, sub) };
  const fresh = await signKey(env, renewed);
  return json({ key: fresh, renewed: fresh !== key, exp: renewed.exp });
}

async function status(env: Env, url: URL): Promise<Response> {
  const payload = await parseKey(env, url.searchParams.get("key") ?? "");
  if (!payload) return json({ valid: false });
  const expired = payload.exp ? payload.exp < isoDate(new Date()) : false;
  return json({ valid: !expired, tier: payload.tier, exp: payload.exp ?? null, subscription: payload.sub ?? null, org: payload.org ?? null });
}

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    const url = new URL(req.url);
    const cors = { "access-control-allow-origin": env.SITE_URL, "access-control-allow-methods": "GET, POST", "access-control-allow-headers": "content-type" };
    if (req.method === "OPTIONS") return new Response(null, { headers: cors });
    try {
      let res: Response;
      if (url.pathname === "/api/health") res = json({ ok: true, service: "innards-billing" });
      else if (url.pathname === "/api/checkout" && req.method === "GET") res = await checkout(env, url);
      else if (url.pathname === "/api/stripe/webhook" && req.method === "POST") res = await webhook(env, req);
      else if (url.pathname === "/api/key" && req.method === "GET") res = await keyForSession(env, url);
      else if (url.pathname === "/api/license/refresh" && req.method === "GET") res = await refresh(env, url);
      else if (url.pathname === "/api/license/status" && req.method === "GET") res = await status(env, url);
      else res = err("not_found", "no such route", 404);
      for (const [k, v] of Object.entries(cors)) if (!res.headers.has(k) && res.status !== 303) res.headers.set(k, v);
      return res;
    } catch (e) {
      return err("internal", e instanceof Error ? e.message : "error", 500);
    }
  },
} satisfies ExportedHandler<Env>;
