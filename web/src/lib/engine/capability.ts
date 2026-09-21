// Port of innards-core/src/capability.rs — same weights, same ramps.
// "What is this machine still good for?" — 0–100 per workload, bucketed.
import { GIB, systemDisk, type Snapshot } from './types';

export const WORKLOADS = [
  'everyday', 'web_dev', 'containers', 'home_server', 'photo_editing',
  'heavy_compile', 'video_editing', 'local_llm', 'gaming', 'ml_training'
] as const;
export type Workload = (typeof WORKLOADS)[number];
export type Grade = 'unsuitable' | 'poor' | 'ok' | 'good' | 'great';
export const GRADE_ORDER: Grade[] = ['unsuitable', 'poor', 'ok', 'good', 'great'];

export interface Capability { workload: Workload; score: number; grade: Grade; limits: string[] }

interface Facts { cores: number; ram_gib: number; fast_disk: boolean; discrete_gpu: boolean; vram_gib: number; age: number; avx2: boolean; apple: boolean }

const YEAR = new Date().getUTCFullYear();

export function facts(s: Snapshot): Facts {
  const gpu = s.gpus.find((g) => g.is_discrete);
  const sd = systemDisk(s);
  return {
    cores: Math.max(1, s.cpu.physical_cores ?? Math.floor(s.cpu.logical_cpus / 2)),
    ram_gib: s.memory.total_bytes / GIB,
    fast_disk: sd ? sd.kind === 'nvme' || sd.kind === 'ssd' : true,
    discrete_gpu: !!gpu,
    vram_gib: gpu?.vram_bytes != null ? gpu.vram_bytes / GIB : 0,
    age: s.cpu.launch_year != null ? Math.max(0, YEAR - s.cpu.launch_year) : 4,
    avx2: s.cpu.flags.length === 0 || s.cpu.flags.includes('avx2'),
    apple: s.cpu.brand.toLowerCase().includes('apple')
  };
}

const ramp = (v: number, lo: number, hi: number) => Math.min(1, Math.max(0, (v - lo) / (hi - lo)));

export function score(s: Snapshot): Capability[] {
  const f = facts(s);
  return WORKLOADS.map((w) => one(w, f));
}

function one(w: Workload, f: Facts): Capability {
  const limits: [string, number][] = [];
  let sc: number;
  switch (w) {
    case 'everyday': {
      const disk = f.fast_disk ? 1 : 0.4, ram = ramp(f.ram_gib, 2, 8), age = 1 - ramp(f.age, 8, 14);
      if (!f.fast_disk) limits.push(['hdd', 0.6]);
      if (f.ram_gib < 8) limits.push(['ram', ram]);
      sc = 0.4 * disk + 0.35 * ram + 0.25 * age; break;
    }
    case 'web_dev': {
      const cores = ramp(f.cores, 1, 4), ram = ramp(f.ram_gib, 4, 16), disk = f.fast_disk ? 1 : 0.3;
      if (f.ram_gib < 16) limits.push(['ram', ram]);
      if (f.cores < 4) limits.push(['cores', cores]);
      if (!f.fast_disk) limits.push(['hdd', 0.3]);
      sc = 0.3 * cores + 0.4 * ram + 0.3 * disk; break;
    }
    case 'heavy_compile': {
      const cores = ramp(f.cores, 2, 12), ram = ramp(f.ram_gib, 8, 32), disk = f.fast_disk ? 1 : 0.3;
      if (f.cores < 8) limits.push(['cores', cores]);
      if (f.ram_gib < 32) limits.push(['ram', ram]);
      sc = 0.55 * cores + 0.3 * ram + 0.15 * disk; break;
    }
    case 'containers': {
      const ram = ramp(f.ram_gib, 4, 24), cores = ramp(f.cores, 1, 6);
      if (f.ram_gib < 24) limits.push(['ram', ram]);
      if (f.cores < 4) limits.push(['cores', cores]);
      sc = 0.6 * ram + 0.4 * cores; break;
    }
    case 'home_server': {
      const ram = ramp(f.ram_gib, 4, 16), disk = f.fast_disk ? 1 : 0.7, cores = ramp(f.cores, 1, 4);
      if (f.ram_gib < 16) limits.push(['ram', ram]);
      sc = 0.5 * ram + 0.2 * disk + 0.3 * cores; break;
    }
    case 'photo_editing': {
      const ram = ramp(f.ram_gib, 8, 32), cores = ramp(f.cores, 2, 8), gpu = f.discrete_gpu || f.apple ? 1 : 0.6;
      if (f.ram_gib < 16) limits.push(['ram', ram]);
      if (!f.discrete_gpu && !f.apple) limits.push(['igpu', 0.6]);
      sc = 0.45 * ram + 0.3 * cores + 0.25 * gpu; break;
    }
    case 'video_editing': {
      const gpu = f.discrete_gpu ? Math.max(ramp(f.vram_gib, 2, 8), 0.6) : f.apple ? 0.9 : 0.35;
      const cores = ramp(f.cores, 2, 12), ram = ramp(f.ram_gib, 8, 32);
      if (!f.discrete_gpu && !f.apple) limits.push(['igpu', gpu]);
      if (f.cores < 8) limits.push(['cores', cores]);
      if (f.ram_gib < 32) limits.push(['ram', ram]);
      sc = 0.4 * gpu + 0.35 * cores + 0.25 * ram; break;
    }
    case 'local_llm': {
      const mem = f.apple ? ramp(f.ram_gib, 8, 64) : f.discrete_gpu ? ramp(f.vram_gib, 4, 24) : 0.35 * ramp(f.ram_gib, 8, 32);
      const simd = f.avx2 ? 1 : 0.3;
      if (!f.discrete_gpu && !f.apple) limits.push(['no_gpu', mem]);
      if (f.discrete_gpu && f.vram_gib < 12) limits.push(['vram', mem]);
      if (!f.avx2) limits.push(['no_avx2', 0.3]);
      sc = 0.8 * mem + 0.2 * simd; break;
    }
    case 'gaming': {
      const gpu = f.discrete_gpu ? Math.max(ramp(f.vram_gib, 4, 12), 0.5) : f.apple ? 0.5 : 0.15;
      const cores = ramp(f.cores, 2, 6), ram = ramp(f.ram_gib, 8, 16);
      if (!f.discrete_gpu) limits.push(['no_gpu', gpu]);
      if (f.cores < 6) limits.push(['cores', cores]);
      sc = 0.65 * gpu + 0.2 * cores + 0.15 * ram; break;
    }
    case 'ml_training': {
      const gpu = f.discrete_gpu ? ramp(f.vram_gib, 6, 24) : f.apple ? 0.4 * ramp(f.ram_gib, 16, 64) : 0.05;
      if (!f.discrete_gpu) limits.push(['no_gpu', gpu]);
      if (f.discrete_gpu && f.vram_gib < 16) limits.push(['vram', gpu]);
      sc = 0.85 * gpu + 0.15 * ramp(f.ram_gib, 16, 64); break;
    }
  }
  const n = Math.min(100, Math.max(0, Math.round(sc * 100)));
  const grade: Grade = n <= 24 ? 'unsuitable' : n <= 44 ? 'poor' : n <= 64 ? 'ok' : n <= 84 ? 'good' : 'great';
  limits.sort((a, b) => a[1] - b[1]);
  return { workload: w, score: n, grade, limits: limits.slice(0, 3).map(([k]) => k) };
}
