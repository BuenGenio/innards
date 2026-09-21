// App-wide state with Svelte 5 runes. One object, imported everywhere.

import { api } from "./api";
import { ui } from "./ui-strings";
import type { Catalog, Level, LicenseStatus, RenderedReport, Settings, Tier } from "./types";

const FALLBACK_SETTINGS: Settings = {
  lang: "en", level: "informed", region: "us", license_key: null, anthropic_api_key: null, elevate_for_smart: false,
  cloud_endpoint: null, cloud_token: null, cloud_auto_upload: false, machine_label: null, cloud_interval_hours: 24,
};

class AppState {
  settings = $state<Settings>(FALLBACK_SETTINGS);
  languages = $state<[string, string][]>([["en", "English"]]);
  catalog = $state<Catalog | null>(null);
  report = $state<RenderedReport | null>(null);
  license = $state<LicenseStatus>({ tier: "free", email: null, expires: null, org: null, source: "none", renewable: false });
  scanning = $state(false);
  /** True while showing a report opened from a file (Pro). */
  otherMachine = $state(false);
  error = $state<string | null>(null);

  get lang() { return this.settings.lang; }
  get level() { return this.settings.level; }
  get tier() { return this.license.tier; }

  /** Tier ordering: a Pro key also unlocks Supporter features, and so on. */
  has(t: Tier): boolean {
    const order: Tier[] = ["free", "supporter", "pro", "team", "enterprise"];
    return order.indexOf(this.license.tier) >= order.indexOf(t);
  }

  /** Look up a UI string; falls back to the key so missing strings are visible. */
  t(path: string): string {
    const parts = path.split("/");
    let cur: unknown = this.catalog;
    for (const p of parts) {
      if (cur && typeof cur === "object" && p in (cur as Record<string, unknown>)) cur = (cur as Record<string, unknown>)[p];
      else return path;
    }
    return typeof cur === "string" ? cur : path;
  }

  /** UI chrome string (tabs, buttons, paywall copy…) from `ui-strings.ts`; `t()` is for catalog strings. */
  u(key: string, params?: Record<string, string | number>): string {
    return ui(this.lang, key, params);
  }

  async init() {
    const [settings, languages, license] = await Promise.all([api.getSettings(), api.languages(), api.licenseStatus()]);
    this.settings = settings;
    this.languages = languages;
    this.license = license;
    this.maybeRenew();
    this.catalog = await api.catalog(settings.lang);
    await this.scan();
  }

  /** Subscription keys are renewed silently when within 30 days of expiry. */
  private maybeRenew() {
    const exp = this.license.expires;
    if (!this.license.renewable || !exp) return;
    const days = (new Date(exp).getTime() - Date.now()) / 86_400_000;
    if (days > 30) return;
    api.licenseRefresh().then((st) => (this.license = st)).catch(() => {});
  }

  async scan() {
    this.scanning = true;
    this.error = null;
    try {
      this.report = await api.analyze(this.settings.elevate_for_smart);
      this.otherMachine = false;
    } catch (e) {
      this.error = String(e);
    } finally {
      this.scanning = false;
    }
  }

  async setLang(lang: string) {
    this.settings.lang = lang;
    this.catalog = await api.catalog(lang);
    await this.rerender();
    await api.setSettings($state.snapshot(this.settings));
  }

  async setLevel(level: Level) {
    this.settings.level = level;
    await this.rerender();
    await api.setSettings($state.snapshot(this.settings));
  }

  async rerender() {
    if (!this.report) return;
    this.report = await api.render(this.settings.lang, this.settings.level);
  }

  /** Pro: view a report exported by another machine. */
  async openReport(text: string) {
    this.report = await api.loadReportJson(text, this.settings.lang, this.settings.level);
    this.otherMachine = true;
  }

  async saveSettings() {
    await api.setSettings($state.snapshot(this.settings));
    this.license = await api.licenseStatus();
  }
}

export const app = new AppState();
