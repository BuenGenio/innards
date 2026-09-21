import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { mockInvoke } from "./mock";
import type {
  Answers, Catalog, Level, LicenseStatus, Narrative, Question,
  HistoryEntry, RenderedRecommendation, RenderedReport, Settings, ShopLink, UploadResult,
} from "./types";

// Outside Tauri (plain browser) fall back to fixture data so the UI can be
// developed and reviewed without the Rust backend.
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const invoke = <T,>(cmd: string, args?: Record<string, unknown>) => (isTauri ? tauriInvoke<T>(cmd, args) : mockInvoke<T>(cmd, args));

export const api = {
  analyze: (elevate: boolean) => invoke<RenderedReport>("analyze", { elevate }),
  render: (lang: string, level: Level) => invoke<RenderedReport>("render", { lang, level }),
  exportMarkdown: (lang: string, level: Level) => invoke<string>("export_markdown", { lang, level }),
  reportJson: () => invoke<unknown>("report_json"),
  saveText: (path: string, contents: string) => invoke<void>("save_text", { path, contents }),
  readText: (path: string) => invoke<string>("read_text", { path }),
  languages: () => invoke<[string, string][]>("languages"),
  catalog: (lang: string) => invoke<Catalog>("catalog", { lang }),
  advisorQuestions: () => invoke<Question[]>("advisor_questions"),
  advise: (answers: Answers, lang: string) => invoke<RenderedRecommendation[]>("advise", { answers, lang }),
  shopLinks: (recId: string, region: string) => invoke<ShopLink[]>("shop_links", { recId, region }),
  narrate: (lang: string, level: Level, includeRecs: boolean) => invoke<Narrative>("narrate", { lang, level, includeRecs }),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (s: Settings) => invoke<void>("set_settings", { s }),
  licenseStatus: () => invoke<LicenseStatus>("license_status"),
  activateLicense: (key: string) => invoke<LicenseStatus>("activate_license", { key }),
  cloudUpload: () => invoke<UploadResult>("cloud_upload"),
  licenseRefresh: () => invoke<LicenseStatus>("license_refresh"),
  history: (limit?: number) => invoke<HistoryEntry[]>("history", { limit }),
  historyClear: () => invoke<void>("history_clear"),
  loadReportJson: (text: string, lang: string, level: Level) => invoke<RenderedReport>("load_report_json", { text, lang, level }),
  viewingOtherMachine: () => invoke<boolean>("viewing_other_machine"),
  appInfo: () => invoke<{ version: string; core_version: string; os: string }>("app_info"),
};
