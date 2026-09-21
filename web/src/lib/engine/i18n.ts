// Mirrors innards-core/src/i18n.rs: dotted-path lookup into the catalog and
// `{param}` substitution. Missing params stay visible on purpose.
import en from '$lib/data/en.json';

export type Level = 'plain' | 'informed' | 'expert';
export const LEVELS: Level[] = ['plain', 'informed', 'expert'];

export type Params = Record<string, string | number | boolean>;

export function get(path: string): string | undefined {
  let cur: unknown = en;
  for (const seg of path.split('/')) {
    if (cur === null || typeof cur !== 'object') return undefined;
    cur = (cur as Record<string, unknown>)[seg];
  }
  return typeof cur === 'string' ? cur : undefined;
}

export function fill(template: string, params: Params = {}): string {
  let out = template;
  for (const [k, v] of Object.entries(params)) out = out.replaceAll(`{${k}}`, String(v));
  return out;
}

export function t(path: string): string {
  return get(path) ?? `⟨${path}⟩`;
}

export function tf(path: string, params: Params): string {
  return fill(t(path), params);
}
