// Syntax-checks every browser module with `node --check` and enforces the service-layer
// rules: only services/ may call fetch or XMLHttpRequest; only services/prefs.js may touch
// localStorage (theme choice), and it may not mention tokens or import the session; no file
// touches sessionStorage, cookies or IndexedDB, so session tokens are never stored. Desktop
// (docs/desktop/phases/phase-1-core-storage.md §6): only services/api-desktop.js may use Tauri IPC,
// it may not call fetch or XMLHttpRequest, and it must export exactly the names services/api.js does.
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const webRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** Lists the browser JS sources (app.js plus services/*.js and ui/*.js). */
function sources() {
  const inDir = (d) => readdirSync(join(webRoot, d)).filter((f) => f.endsWith('.js')).map((f) => join(webRoot, d, f));
  return [join(webRoot, 'app.js'), ...inDir('services'), ...inDir('ui')];
}

/** Returns the sorted `export` names of a module (declarations and `export { a, b as c }`). */
export function exportNames(code) {
  const names = new Set();
  for (const m of code.matchAll(/^export\s+(?:async\s+)?(?:function\*?|class|const|let|var)\s+([A-Za-z_$][\w$]*)/gm)) {
    names.add(m[1]);
  }
  for (const m of code.matchAll(/^export\s*\{([^}]*)\}/gm)) {
    for (const part of m[1].split(',').map((x) => x.trim()).filter(Boolean)) names.add(part.split(/\s+as\s+/).pop());
  }
  return [...names].sort();
}

/** Problems when api-desktop.js and api.js do not export the same names. */
function exportParity() {
  const read = (f) => exportNames(readFileSync(join(webRoot, 'services', f), 'utf8'));
  const web = read('api.js');
  const desktop = read('api-desktop.js');
  const missing = web.filter((n) => !desktop.includes(n));
  const extra = desktop.filter((n) => !web.includes(n));
  const out = [];
  if (missing.length) out.push(`services/api-desktop.js: missing exports ${missing.join(', ')}`);
  if (extra.length) out.push(`services/api-desktop.js: extra exports ${extra.join(', ')}`);
  return out;
}

/**
 * Throws when a source fails to parse or breaks the service-layer rules.
 * @returns Nothing; logs one line on success.
 */
export function checkSources() {
  const problems = exportParity();
  for (const file of sources()) {
    const rel = relative(webRoot, file);
    execFileSync(process.execPath, ['--check', file], { stdio: 'inherit' });
    const code = readFileSync(file, 'utf8');
    const isService = rel.startsWith('services/');
    if (!isService && /\bfetch\s*\(|XMLHttpRequest/.test(code)) problems.push(`${rel}: network call outside services/`);
    const isDesktopApi = rel === 'services/api-desktop.js';
    if (!isDesktopApi && /__TAURI__|\binvoke\s*\(/.test(code)) problems.push(`${rel}: Tauri IPC outside services/api-desktop.js`);
    if (isDesktopApi && /\bfetch\s*\(|XMLHttpRequest/.test(code)) problems.push(`${rel}: network call in the desktop API (IPC only)`);
    const isPrefs = rel === 'services/prefs.js';
    if (/sessionStorage|document\.cookie|indexedDB/.test(code)) problems.push(`${rel}: web storage use`);
    if (!isPrefs && /localStorage/.test(code)) problems.push(`${rel}: localStorage outside services/prefs.js`);
    if (isPrefs && /token|session/i.test(code)) {
      problems.push(`${rel}: prefs must not handle session or token values`);
    }
    if (/\balert\s*\(|\bconfirm\s*\(|\bprompt\s*\(/.test(code)) problems.push(`${rel}: blocking dialog`);
  }
  if (problems.length) throw new Error(`check failed:\n${problems.join('\n')}`);
  console.log(`check: ${sources().length} files OK, api.js / api-desktop.js exports match`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) checkSources();
