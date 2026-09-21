// Port of innards-core/src/advisor.rs: questionnaire answers + snapshot →
// ranked, costed recommendations. Same thresholds and ordering as the crate.
import { fmtBytes, GIB, isLaptop, rootFs, systemDisk, type Snapshot } from './types';
import { score as scoreCaps, GRADE_ORDER, type Workload } from './capability';

export type Pain = 'slow' | 'out_of_memory' | 'storage' | 'battery' | 'heat_noise' | 'crashes' | 'none';
export type Budget = 'under_100' | 'under_300' | 'under_800' | 'under_1500' | 'no_limit';
export type Horizon = 'now' | 'six_months' | 'year';
export type RecKind = 'software' | 'upgrade' | 'service' | 'replace' | 'keep';
export type Impact = 'low' | 'medium' | 'high';

export interface Answers { uses: Workload[]; pains: Pain[]; budget: Budget; horizon: Horizon; needs_portability: boolean }

export interface Recommendation {
  id: string;
  kind: RecKind;
  impact: Impact;
  cost_usd: [number, number];
  over_budget: boolean;
  params: Record<string, string | number | boolean>;
  helps: Workload[];
  shopping_query: string | null;
}

const BUDGET_MAX: Record<Budget, number> = { under_100: 100, under_300: 300, under_800: 800, under_1500: 1500, no_limit: Infinity };
const IMPACT_RANK: Record<Impact, number> = { low: 0, medium: 1, high: 2 };
const YEAR = new Date().getUTCFullYear();

export const QUESTIONS = [
  { id: 'uses', multi: true, options: ['everyday', 'web_dev', 'containers', 'home_server', 'photo_editing', 'heavy_compile', 'video_editing', 'local_llm', 'gaming', 'ml_training'] },
  { id: 'pains', multi: true, options: ['slow', 'out_of_memory', 'storage', 'battery', 'heat_noise', 'crashes', 'none'] },
  { id: 'budget', multi: false, options: ['under_100', 'under_300', 'under_800', 'under_1500', 'no_limit'] },
  { id: 'horizon', multi: false, options: ['now', 'six_months', 'year'] },
  { id: 'portability', multi: false, options: ['yes', 'no'] }
] as const;

function rec(id: string, kind: RecKind, impact: Impact, cost: [number, number], params: Recommendation['params'] = {}, helps: Workload[] = [], shop: string | null = null): Recommendation {
  return { id, kind, impact, cost_usd: cost, over_budget: false, params, helps, shopping_query: shop };
}

function ramTarget(uses: Workload[]): number {
  const want = uses.map((w) => ({ everyday: 8, web_dev: 16, home_server: 16, gaming: 16, photo_editing: 32, containers: 32, local_llm: 32, heavy_compile: 32, video_editing: 32, ml_training: 64 })[w]);
  return want.length ? Math.max(...want) : 8;
}
function coresTarget(uses: Workload[]): number {
  const want = uses.map((w) => ({ everyday: 2, home_server: 2, web_dev: 4, gaming: 4, photo_editing: 4, containers: 6, local_llm: 6, heavy_compile: 8, video_editing: 8, ml_training: 8 })[w]);
  return want.length ? Math.max(...want) : 2;
}
const GPU_USES: Workload[] = ['gaming', 'ml_training', 'local_llm', 'video_editing'];
const wantsGpu = (uses: Workload[]) => uses.some((w) => GPU_USES.includes(w));

export function recommend(snap: Snapshot, a: Answers): Recommendation[] {
  const recs: Recommendation[] = [];
  const caps = scoreCaps(snap);
  const gradeOf = (w: Workload) => caps.find((c) => c.workload === w)?.grade ?? 'ok';
  const laptop = isLaptop(snap);
  const ramGib = Math.floor(snap.memory.total_bytes / GIB);
  const ramWant = ramTarget(a.uses);
  const cores = Math.max(1, snap.cpu.physical_cores ?? Math.floor(snap.cpu.logical_cpus / 2));
  const age = snap.cpu.launch_year != null ? Math.max(0, YEAR - snap.cpu.launch_year) : 4;
  const hasPain = (p: Pain) => a.pains.includes(p);
  const sysDisk = systemDisk(snap);

  // Free software fixes first.
  if (snap.memory.swap_total_bytes === 0) recs.push(rec('rec.add_swap', 'software', 'high', [0, 0], { ram: ramGib }, a.uses));
  const root = rootFs(snap);
  if (root) {
    const used = 100 - (root.available_bytes * 100) / Math.max(1, root.total_bytes);
    if (used >= 85) recs.push(rec('rec.free_space', 'software', 'medium', [0, 0], { used_pct: Math.round(used), free: fmtBytes(root.available_bytes) }));
  }
  if (snap.memory.tmpfs_used_bytes > snap.memory.total_bytes / 10) recs.push(rec('rec.tmpfs_to_disk', 'software', 'medium', [0, 0], { tmpfs: fmtBytes(snap.memory.tmpfs_used_bytes) }));

  // Storage
  if (sysDisk) {
    const d = sysDisk, s = d.smart;
    const badHealth = !!s && (s.healthy === false || (s.media_errors ?? 0) > 0 || (s.reallocated_sectors ?? 0) > 0 || (s.percentage_used ?? 0) >= 80);
    if (d.kind === 'hdd' || badHealth || (hasPain('storage') && d.size_bytes < 512 * GIB)) {
      const targetGb = d.size_bytes < 512 * GIB ? 1000 : 2000;
      const nvme = d.kind === 'nvme' || snap.storage.some((x) => x.kind === 'nvme');
      const id = badHealth ? 'rec.ssd_replace_failing' : d.kind === 'hdd' ? 'rec.ssd_replace_hdd' : 'rec.ssd_bigger';
      const cost: [number, number] = targetGb >= 2000 ? [90, 160] : [50, 90];
      recs.push(rec(id, badHealth ? 'replace' : 'upgrade', d.kind === 'hdd' || badHealth ? 'high' : 'medium', cost,
        { model: d.model ?? d.name, size: fmtBytes(d.size_bytes), target_gb: targetGb, interface: nvme ? 'NVMe M.2' : 'SATA 2.5"' },
        ['everyday', 'web_dev', 'containers'], `${nvme ? 'NVMe M.2 2280' : '2.5 inch SATA'} ${targetGb}GB SSD`));
    }
  }

  // RAM
  if (ramGib < ramWant || hasPain('out_of_memory')) {
    const target = Math.min(128, Math.max(ramWant, ramGib * 2));
    if (snap.memory.upgradeable === false) {
      recs.push(rec('rec.ram_soldered', 'keep', 'high', [0, 0], { have: ramGib, target }, a.uses));
    } else {
      const kind = snap.memory.kind ?? 'DDR4';
      const delta = Math.max(8, target - ramGib);
      const id = snap.memory.upgradeable == null ? 'rec.ram_add_check' : 'rec.ram_add';
      recs.push(rec(id, 'upgrade', 'high', [delta * 3, delta * 6], { have: ramGib, target, kind }, a.uses, `${target}GB ${kind} ${laptop ? 'SODIMM' : 'DIMM'}`));
    }
  }

  // Battery
  if (laptop) {
    const h = snap.battery?.health_pct;
    if (h != null && h < 65 && (a.needs_portability || hasPain('battery'))) {
      const model = snap.system.product ?? '';
      recs.push(rec('rec.battery_replace', 'service', 'high', [40, 120], { health: Math.round(h), model }, [], `${model || 'laptop'} replacement battery`));
    }
  }

  // Thermal
  const hotDisk = snap.storage.some((d) => (d.smart?.critical_temp_minutes ?? 0) >= 30 || (d.temperature_c ?? 0) >= 70);
  const hotCpu = (snap.thermal.cpu_c ?? 0) >= 85;
  if (hotCpu || hotDisk || hasPain('heat_noise')) {
    recs.push(rec('rec.thermal_service', 'service', hotCpu ? 'medium' : 'low', [0, 40],
      { cpu_temp: Math.round(snap.thermal.cpu_c ?? 0), hot_disk: hotDisk }, [], laptop ? 'laptop thermal paste pads kit' : 'CPU thermal paste'));
  }

  // GPU-bound workloads
  if (wantsGpu(a.uses) && !snap.gpus.some((g) => g.is_discrete)) {
    const gpuUses = a.uses.filter((w) => GPU_USES.includes(w));
    if (laptop) {
      recs.push(rec('rec.gpu_laptop_no_slot', 'keep', 'high', [0, 0], {}, gpuUses));
    } else {
      const vram = gpuUses.includes('ml_training') || gpuUses.includes('local_llm') ? 16 : 8;
      recs.push(rec('rec.gpu_add', 'upgrade', 'high', vram >= 16 ? [450, 1000] : [250, 450], { vram }, gpuUses, `graphics card ${vram}GB`));
    }
  }

  // Whole-machine replacement
  const poorlyServed = a.uses.filter((w) => GRADE_ORDER.indexOf(gradeOf(w)) <= GRADE_ORDER.indexOf('poor'));
  const ceilingHit = snap.memory.upgradeable === false && ramGib < ramWant;
  const cpuShort = cores < coresTarget(a.uses);
  if (poorlyServed.length && (age >= 6 || ceilingHit || cpuShort)) {
    const targetCores = Math.max(4, coresTarget(a.uses));
    const targetRam = Math.max(16, ramWant);
    const needsGpu = wantsGpu(a.uses);
    const cost: [number, number] = needsGpu ? (laptop ? [1200, 2500] : [900, 2000]) : laptop ? [600, 1400] : [400, 1000];
    recs.push(rec('rec.machine_replace', 'replace', 'high', cost,
      { age, cores_now: cores, ram_now: ramGib, target_cores: targetCores, target_ram: targetRam, needs_gpu: needsGpu, form: a.needs_portability ? 'laptop' : 'desktop_or_mini' },
      poorlyServed, `${a.needs_portability ? 'laptop' : 'mini PC'} ${targetCores} cores ${targetRam}GB RAM${needsGpu ? ' dedicated GPU' : ''}`));
  }

  if (recs.length === 0 || recs.every((r) => r.kind === 'software')) recs.push(rec('rec.keep', 'keep', 'low', [0, 0], { age }));

  const max = BUDGET_MAX[a.budget];
  for (const r of recs) r.over_budget = r.cost_usd[0] > max;
  recs.sort((x, y) => IMPACT_RANK[y.impact] - IMPACT_RANK[x.impact] || x.cost_usd[0] - y.cost_usd[0]);
  return recs;
}
