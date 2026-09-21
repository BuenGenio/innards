// The slice of innards-core's Snapshot that capability scoring and the
// advisor read. Field names match the Rust structs (serde snake_case).
export type StorageKind = 'unknown' | 'nvme' | 'ssd' | 'hdd' | 'emmc' | 'sd';
export type Chassis = 'unknown' | 'laptop' | 'desktop' | 'server' | 'convertible' | 'mini_pc' | 'vm';

export interface Smart {
  healthy: boolean | null;
  percentage_used: number | null;
  media_errors: number | null;
  reallocated_sectors: number | null;
  critical_temp_minutes: number | null;
}

export interface Snapshot {
  system: { vendor: string | null; product: string | null; chassis: Chassis };
  cpu: { brand: string; physical_cores: number | null; logical_cpus: number; launch_year: number | null; flags: string[] };
  memory: { total_bytes: number; swap_total_bytes: number; kind: string | null; upgradeable: boolean | null; tmpfs_used_bytes: number };
  storage: { name: string; model: string | null; size_bytes: number; kind: StorageKind; is_system_disk: boolean; smart: Smart | null; temperature_c: number | null }[];
  filesystems: { mount_point: string; total_bytes: number; available_bytes: number; is_root: boolean }[];
  gpus: { name: string; is_discrete: boolean; vram_bytes: number | null }[];
  battery: { present: boolean; health_pct: number | null } | null;
  thermal: { cpu_c: number | null };
}

export type Severity = 'critical' | 'warning' | 'info' | 'good';
export type Category = 'memory' | 'storage' | 'cpu' | 'thermal' | 'battery' | 'gpu' | 'network' | 'system';

export interface Finding {
  id: string;
  severity: Severity;
  category: Category;
  params: Record<string, string | number | boolean>;
  evidence: string[];
}

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

export const GIB = 2 ** 30;

export function fmtBytes(b: number): string {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  if (i === 0) return `${b} ${units[i]}`;
  return v >= 100 ? `${Math.round(v)} ${units[i]}` : `${v.toFixed(1)} ${units[i]}`;
}

export function systemDisk(s: Snapshot) { return s.storage.find((d) => d.is_system_disk); }
export function rootFs(s: Snapshot) { return s.filesystems.find((f) => f.is_root); }
export function isLaptop(s: Snapshot) {
  return s.system.chassis === 'laptop' || s.system.chassis === 'convertible' || !!s.battery?.present;
}
