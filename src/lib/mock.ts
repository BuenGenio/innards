// Mock backend for running the UI in a plain browser ("design mode"):
// `pnpm dev` then open http://localhost:1420. Serves the JSON fixtures in
// static/fixtures, regenerated with `cargo run -p innards-core --example fixtures -- static/fixtures`.
// Query params: ?tier=supporter|pro to preview paid UI, ?lang=es, ?level=expert.

import type { Level, LicenseStatus, Settings } from "./types";

const q = typeof location !== "undefined" ? new URLSearchParams(location.search) : new URLSearchParams();
let settings: Settings = {
  lang: q.get("lang") ?? "en",
  level: (q.get("level") as Level) ?? "informed",
  region: "us",
  license_key: null,
  anthropic_api_key: null,
  elevate_for_smart: false,
  cloud_endpoint: null,
  cloud_token: null,
  cloud_auto_upload: false,
  machine_label: null,
  cloud_interval_hours: 24,
};
const tier = (q.get("tier") ?? "free") as LicenseStatus["tier"];

// ?snap=1: load synchronously and skip delays so a headless browser's
// load-event screenshot captures the finished page.
const snap = q.get("snap") === "1";
if (snap && typeof document !== "undefined") document.documentElement.classList.add("no-anim");

async function fx<T>(name: string): Promise<T> {
  if (snap) {
    const x = new XMLHttpRequest();
    x.open("GET", `/fixtures/${name}.json`, false);
    x.send();
    if (x.status !== 200) throw new Error(`fixture ${name} missing`);
    return JSON.parse(x.responseText);
  }
  const r = await fetch(`/fixtures/${name}.json`);
  if (!r.ok) throw new Error(`fixture ${name} missing`);
  return r.json();
}
const delay = (ms: number) => (snap ? Promise.resolve() : new Promise((r) => setTimeout(r, ms)));

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  switch (cmd) {
    case "analyze":
      await delay(900);
      return fx(`report-${settings.lang}-${settings.level}`);
    case "render":
      return fx(`report-${args.lang}-${args.level}`);
    case "export_markdown":
      return "# Innards report (mock)\n" as T;
    case "report_json":
      return fx("report-full");
    case "save_text":
      return undefined as T;
    case "read_text":
      return "{}" as T;
    case "languages":
      return fx("languages");
    case "catalog":
      return fx(`catalog-${args.lang}`);
    case "advisor_questions":
      return fx("questions");
    case "advise":
      await delay(400);
      return fx(`recs-${args.lang}`);
    case "shop_links":
      if (tier !== "pro" && tier !== "team" && tier !== "enterprise") throw "pro";
      return [
        { condition: "new", vendor: "Amazon", url: "https://www.amazon.com/" },
        { condition: "refurbished", vendor: "Back Market", url: "https://www.backmarket.com/" },
        { condition: "used", vendor: "eBay", url: "https://www.ebay.com/" },
      ] as T;
    case "narrate":
      if (tier === "free") throw "supporter";
      await delay(1200);
      return {
        text: "**What matters most**\n- The battery holds under half its original charge.\n- Two data drives are nearly full.\n\n**Still good for**\n- Everyday work, web development and home-server duty.\n\n**What to do**\n- Free the full drives; replace the battery if you travel.",
        model: "mock", input_tokens: 0, output_tokens: 0,
      } as T;
    case "get_settings":
      return settings as T;
    case "set_settings":
      settings = args.s as Settings;
      return undefined as T;
    case "license_status":
      return { tier, email: tier === "free" ? null : "you@example.com", expires: tier === "pro" || tier === "team" || tier === "enterprise" ? "2027-09-21" : null, org: tier === "team" || tier === "enterprise" ? "acme" : null, source: "mock", renewable: tier === "pro" || tier === "team" } as T;
    case "license_refresh":
      throw "no_key";
    case "history": {
      if (tier === "free") throw "supporter";
      // 30 days of plausible drift for the demo.
      const out = [];
      for (let i = 29; i >= 0; i--) {
        const d = new Date(Date.now() - i * 86_400_000);
        out.push({ at: d.toISOString(), health: 58 + Math.round(6 * Math.sin(i / 4)), critical: i < 10 ? 1 : 0, warning: 5 + (i % 3), battery_health: 46 - i * 0.02, root_free_pct: 46 - (29 - i) * 0.3, memory_available_pct: 25 + (i % 7), cpu_c: 62 + (i % 5) });
      }
      return out as T;
    }
    case "history_clear":
      return undefined as T;
    case "load_report_json":
      if (tier !== "pro" && tier !== "team" && tier !== "enterprise") throw "pro";
      return fx(`report-${args.lang}-${args.level}`);
    case "viewing_other_machine":
      return false as T;
    case "cloud_upload":
      if (tier !== "team" && tier !== "enterprise") throw "team";
      await delay(600);
      return { machine_id: "m_0123456789abcdef", report_id: "r_1", dashboard_url: "https://crm.example.com/x/innards.fleet/machines/m_0123456789abcdef" } as T;
    case "activate_license":
      throw "invalid";
    case "app_info":
      return { version: "0.1.0-mock", core_version: "0.1.0", os: "browser" } as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}
