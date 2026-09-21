#!/usr/bin/env node
// Verifies src/lib/ui-strings.ts: every language table has exactly the keys of
// `en`, and no component still carries inline bilingual strings.
// Usage: node scripts/check-ui-strings.mjs   (exit 1 on any problem)

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(join(root, "src/lib/ui-strings.ts"), "utf8");

// --- (a) key parity -------------------------------------------------------
// Parse the STRINGS literal without a TS toolchain: find `<lang>: {` blocks
// at two-space indent and collect `"dotted.key":` entries inside each.
const tables = {};
let lang = null;
for (const line of src.split("\n")) {
  const open = line.match(/^  ([a-z][a-z0-9_-]*): \{\s*$/i);
  if (open) { lang = open[1]; tables[lang] = new Set(); continue; }
  if (/^  \},?\s*$/.test(line)) { lang = null; continue; }
  if (!lang) continue;
  const kv = line.match(/^\s*"([^"]+)":\s*"/);
  if (kv) {
    if (tables[lang].has(kv[1])) fail(`${lang}: duplicate key "${kv[1]}"`);
    tables[lang].add(kv[1]);
  }
}

let failed = false;
function fail(msg) { console.error("check-ui-strings: " + msg); failed = true; }

if (!tables.en || tables.en.size === 0) fail("could not find the `en` table in src/lib/ui-strings.ts");
const en = tables.en ?? new Set();
for (const [code, keys] of Object.entries(tables)) {
  if (code === "en") continue;
  const missing = [...en].filter((k) => !keys.has(k));
  const extra = [...keys].filter((k) => !en.has(k));
  for (const k of missing) fail(`${code}: missing key "${k}"`);
  for (const k of extra) fail(`${code}: extra key "${k}" (not in en)`);
}

// --- (b) leftover inline bilingual strings ----------------------------------
const dirs = [join(root, "src/lib/components"), join(root, "src/routes")];
// `{ en: "<text>"` must contain a letter so a language-code → flag-emoji map does not trip it.
const patterns = [/\bes \?/, /\{ en: "[^"]*\p{L}/u, /app\.lang === "es"/];
const files = [];
function walk(d) {
  for (const e of readdirSync(d, { withFileTypes: true })) {
    const p = join(d, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.name.endsWith(".svelte")) files.push(p);
  }
}
for (const d of dirs) walk(d);
for (const f of files) {
  const lines = readFileSync(f, "utf8").split("\n");
  lines.forEach((line, i) => {
    for (const re of patterns) {
      if (re.test(line)) fail(`${f.replace(root + "/", "")}:${i + 1}: leftover inline bilingual string: ${line.trim()}`);
    }
  });
}

if (failed) process.exit(1);
const summary = Object.entries(tables).map(([c, k]) => `${c}=${k.size}`).join(", ");
console.log(`check-ui-strings: ok (${summary}; ${files.length} component files clean)`);
