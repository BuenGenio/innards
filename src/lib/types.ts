// Mirrors of the Rust types that cross the IPC boundary.

export type Severity = "critical" | "warning" | "info" | "good";
export type Grade = "unsuitable" | "poor" | "ok" | "good" | "great";
export type Level = "plain" | "informed" | "expert";
export type Tier = "free" | "supporter" | "pro" | "team" | "enterprise";

export interface Summary {
  machine: string;
  cpu: string;
  memory: string;
  storage: string;
  gpu: string;
  os: string;
  battery: string | null;
  health_score: number;
}

export interface RenderedFinding {
  id: string;
  severity: Severity;
  category: string;
  title: string;
  body: string;
  action: string | null;
  evidence: string[];
}

export interface RenderedCapability {
  workload: string;
  label: string;
  score: number;
  grade: Grade;
  grade_label: string;
  limits: string[];
}

export interface RenderedReport {
  lang: string;
  level: Level;
  summary: Summary;
  verdict: string;
  findings: RenderedFinding[];
  capabilities: RenderedCapability[];
  notes: string[];
}

export interface Question {
  id: "uses" | "pains" | "budget" | "horizon" | "portability";
  multi: boolean;
  options: string[];
}

export interface Answers {
  uses: string[];
  pains: string[];
  budget: string;
  horizon: string;
  needs_portability: boolean;
}

export interface RenderedRecommendation {
  id: string;
  kind: string;
  impact: string;
  cost: string;
  over_budget: boolean;
  title: string;
  body: string;
  why: string;
  helps: string[];
  shopping_query: string | null;
}

export interface ShopLink {
  condition: "new" | "refurbished" | "used";
  vendor: string;
  url: string;
}

export interface Settings {
  lang: string;
  level: Level;
  region: string;
  license_key: string | null;
  anthropic_api_key: string | null;
  elevate_for_smart: boolean;
  cloud_endpoint: string | null;
  cloud_token: string | null;
  cloud_auto_upload: boolean;
  machine_label: string | null;
  cloud_interval_hours: number;
}

export interface HistoryEntry {
  at: string;
  health: number;
  critical: number;
  warning: number;
  battery_health: number | null;
  root_free_pct: number | null;
  memory_available_pct: number | null;
  cpu_c: number | null;
}

export interface LicenseStatus {
  tier: Tier;
  email: string | null;
  expires: string | null;
  org: string | null;
  source: string;
  renewable: boolean;
}

export interface UploadResult {
  machine_id: string;
  report_id: string;
  dashboard_url: string | null;
}

export interface Narrative {
  text: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
}

export interface Catalog {
  ui: Record<string, string>;
  severities: Record<Severity, string>;
  categories: Record<string, string>;
  grades: Record<Grade, string>;
  workloads: Record<string, string>;
  advisor: { uses: string; pains: string; budget: string; horizon: string; portability: string; options: Record<string, string> };
  rec_kinds: Record<string, string>;
  impacts: Record<string, string>;
}
