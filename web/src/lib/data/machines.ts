// Two real machines, probed with `cargo run -p innards-core --example report`
// on 2026-09-21. Findings are an excerpt of each report (transient load /
// external-drive findings dropped); params and evidence are verbatim.
import type { Finding, Snapshot, Summary } from '$lib/engine/types';

export interface Machine {
  id: string;
  label: string;      // short tab label
  caption: string;    // one line under the report
  snapshot: Snapshot;
  summary: Summary;
  findings: Finding[];
}

export const x1: Machine = {
  id: 'x1',
  label: 'ThinkPad X1 (2017)',
  caption: 'Excerpt from a real report · Lenovo ThinkPad X1 Carbon 5th · Ubuntu 25.10',
  snapshot: {
    system: { vendor: 'LENOVO', product: 'ThinkPad X1 Carbon 5th', chassis: 'laptop' },
    cpu: { brand: 'Intel(R) Core(TM) i7-7500U CPU @ 2.70GHz', physical_cores: 2, logical_cpus: 4, launch_year: 2017, flags: ['avx2'] },
    memory: { total_bytes: 15984377856, swap_total_bytes: 12884893696, kind: 'LPDDR3', upgradeable: false, tmpfs_used_bytes: 2648276992 },
    storage: [{ name: 'nvme0n1', model: 'KINGSTON SNV2S2000G', size_bytes: 2000398934016, kind: 'nvme', is_system_disk: true, smart: null, temperature_c: null }],
    filesystems: [{ mount_point: '/', total_bytes: 1183705350144, available_bytes: 503176941568, is_root: true }],
    gpus: [{ name: 'HD Graphics 620', is_discrete: false, vram_bytes: null }],
    battery: { present: true, health_pct: 45.334972 },
    thermal: { cpu_c: 63 }
  },
  summary: {
    machine: 'LENOVO ThinkPad X1 Carbon 5th',
    cpu: 'Intel(R) Core(TM) i7-7500U CPU @ 2.70GHz · 2C/4T',
    memory: '14.9 GiB LPDDR3',
    storage: '1.8 TiB NVMe',
    gpu: 'HD Graphics 620',
    os: 'Ubuntu 25.10',
    battery: '45%',
    health_score: 0
  },
  findings: [
    { id: 'battery.worn', severity: 'warning', category: 'battery', params: { health: 45.3, cycles: 1729, full_wh: 25.9, design_wh: 57.0 }, evidence: ['energy_full=25.9 Wh design=57.0 Wh cycles=Some(1729)'] },
    { id: 'cpu.old', severity: 'warning', category: 'cpu', params: { age: 9, brand: 'Intel(R) Core(TM) i7-7500U CPU @ 2.70GHz', year: 2017 }, evidence: [] },
    { id: 'memory.tmpfs_heavy', severity: 'warning', category: 'memory', params: { pct: 16.6, tmpfs: '2.5 GiB' }, evidence: ['tmpfs used=2.5 GiB'] },
    { id: 'memory.soldered', severity: 'info', category: 'memory', params: { kind: 'LPDDR3', total: 14.9 }, evidence: [] },
    { id: 'cpu.few_cores', severity: 'info', category: 'cpu', params: { cores: 2, threads: 4 }, evidence: [] },
    { id: 'gpu.integrated_only', severity: 'info', category: 'gpu', params: { name: 'HD Graphics 620' }, evidence: [] },
    { id: 'memory.swap_ok', severity: 'good', category: 'memory', params: { swap: '12.0 GiB', backends: '8.0 GiB (File, prio 5), 4.0 GiB (Zram, prio 100)' }, evidence: ['swap_total=12.0 GiB, used=4.3 GiB'] },
    { id: 'storage.system_nvme', severity: 'good', category: 'storage', params: { model: 'KINGSTON SNV2S2000G', size: '1.8 TiB', link: 'PCIe 8.0 GT/s x4' }, evidence: [] }
  ]
};

export const m2: Machine = {
  id: 'm2',
  label: 'MacBook Air M2',
  caption: 'Excerpt from a real report · Apple Mac14,2 · Asahi Ubuntu 26.04',
  snapshot: {
    system: { vendor: 'Apple Inc.', product: 'Mac14,2', chassis: 'laptop' },
    cpu: { brand: 'Blizzard-M2', physical_cores: 8, logical_cpus: 8, launch_year: null, flags: [] },
    memory: { total_bytes: 15660384256, swap_total_bytes: 52938833920, kind: null, upgradeable: null, tmpfs_used_bytes: 152895488 },
    storage: [{ name: 'nvme0n1', model: 'APPLE SSD AP1024Z', size_bytes: 1000555581440, kind: 'nvme', is_system_disk: true, smart: null, temperature_c: null }],
    filesystems: [{ mount_point: '/', total_bytes: 733852729344, available_bytes: 38083268608, is_root: true }],
    gpus: [],
    battery: { present: true, health_pct: 86.32478 },
    thermal: { cpu_c: null }
  },
  summary: {
    machine: 'Apple Inc. Mac14,2',
    cpu: 'Blizzard-M2 · 8C/8T',
    memory: '14.6 GiB',
    storage: '932 GiB NVMe',
    gpu: '',
    os: 'Ubuntu 26.04',
    battery: '86%',
    health_score: 0
  },
  findings: [
    { id: 'memory.low_available', severity: 'critical', category: 'memory', params: { available: '984 MiB', available_pct: 6.6 }, evidence: ['available=984 MiB of 14.6 GiB'] },
    { id: 'storage.root_nearly_full', severity: 'warning', category: 'storage', params: { free: '35.5 GiB', mount: '/', total: '683 GiB', used_pct: 94.8 }, evidence: ['/dev/nvme0n1p11 ext4 648 GiB/683 GiB used'] },
    { id: 'network.wifi_only', severity: 'info', category: 'network', params: {}, evidence: [] },
    { id: 'memory.swap_ok', severity: 'good', category: 'memory', params: { swap: '49.3 GiB', backends: '36.0 GiB (Partition, prio -1), 7.3 GiB (Zram, prio 100), 6.0 GiB (Zram, prio 95)' }, evidence: ['swap_total=49.3 GiB, used=23.0 GiB'] },
    { id: 'battery.ok', severity: 'good', category: 'battery', params: { health: 86.3, cycles: 420, full_wh: 44.9, design_wh: 52.0 }, evidence: ['energy_full=44.9 Wh design=52.0 Wh cycles=Some(420)'] }
  ]
};

export const machines: Machine[] = [x1, m2];

// Same arithmetic as report.rs `summarize`: health from severities, verdict from counts.
export function healthScore(findings: Finding[]): number {
  let s = 100;
  for (const f of findings) s -= { critical: 20, warning: 8, info: 0, good: -2 }[f.severity];
  return Math.min(100, Math.max(0, s));
}
export function verdictKey(findings: Finding[]): { key: string; critical: number; warnings: number } {
  const critical = findings.filter((f) => f.severity === 'critical').length;
  const warnings = findings.filter((f) => f.severity === 'warning').length;
  const key = critical === 0 && warnings === 0 ? 'verdict/great' : critical === 0 ? 'verdict/fine' : critical === 1 ? 'verdict/attention' : 'verdict/urgent';
  return { key, critical, warnings };
}
